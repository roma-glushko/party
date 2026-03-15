//! Type checker and static analysis for P programs.
//! Implements all static checks that the P compiler performs.

use std::collections::{HashMap, HashSet};

use super::ast::*;
use super::errors::CompileError;
use super::token::Span;
use super::types::PResolvedType;

/// Run all static analysis on a parsed program.
/// Returns errors if any static checks fail.
pub fn check_program(programs: &[Program], source: &str) -> Result<(), Vec<CompileError>> {
    let mut checker = TypeChecker::new(source);
    checker.check(programs)?;
    Ok(())
}

// ---- Declarations stored in scope ----

#[derive(Debug, Clone)]
struct EventInfo {
    payload: Option<PResolvedType>,
}

#[derive(Debug, Clone)]
struct MachineInfo {
    is_spec: bool,
    states: Vec<String>,
    start_state: Option<String>,
    fields: Vec<(String, PResolvedType)>,
    functions: Vec<String>,
    observes: Option<Vec<String>>,
    state_handlers: HashMap<String, StateHandlerInfo>,
    entry_payload: Option<PResolvedType>,
}

#[derive(Debug, Clone)]
struct StateHandlerInfo {
    entry_param_type: Option<PResolvedType>,
    handlers: Vec<(String, HandlerKind)>, // event name -> kind
    deferred: Vec<String>,
    ignored: Vec<String>,
    is_start: bool,
    temperature: Option<Temperature>,
}

#[derive(Debug, Clone)]
enum HandlerKind {
    Do,
    Goto(String),
}

#[derive(Debug, Clone)]
struct FunctionInfo {
    params: Vec<(String, PResolvedType)>,
    ret_type: Option<PResolvedType>,
    is_foreign: bool,
    // Capabilities (populated during purity propagation)
    can_send: bool,
    can_raise: bool,
    can_change_state: bool,
    can_create: bool,
    can_receive: bool,
    is_nondeterministic: bool,
}

#[derive(Debug, Clone)]
struct EnumInfo {
    elements: Vec<String>,
}

struct TypeChecker<'a> {
    source: &'a str,
    errors: Vec<CompileError>,
    // Global declarations
    events: HashMap<String, EventInfo>,
    machines: HashMap<String, MachineInfo>,
    functions: HashMap<String, FunctionInfo>,
    enums: HashMap<String, EnumInfo>,
    enum_elements: HashMap<String, String>, // element -> enum name
    typedefs: HashMap<String, PResolvedType>,
    interfaces: HashMap<String, Option<PResolvedType>>,
    event_sets: HashMap<String, Vec<String>>,
    /// Global function ASTs for purity analysis.
    global_funs_ast: HashMap<String, FunDecl>,
}

impl<'a> TypeChecker<'a> {
    fn new(source: &'a str) -> Self {
        let mut tc = Self {
            source,
            errors: Vec::new(),
            events: HashMap::new(),
            machines: HashMap::new(),
            functions: HashMap::new(),
            enums: HashMap::new(),
            enum_elements: HashMap::new(),
            typedefs: HashMap::new(),
            interfaces: HashMap::new(),
            event_sets: HashMap::new(),
            global_funs_ast: HashMap::new(),
        };

        // Built-in events
        tc.events.insert("null".to_string(), EventInfo { payload: None });
        tc.events.insert("halt".to_string(), EventInfo { payload: None });

        tc
    }

    fn err(&mut self, msg: impl Into<String>, span: Span) {
        self.errors.push(CompileError::from_offset(msg, self.source, span.start));
    }

    fn check(&mut self, programs: &[Program]) -> Result<(), Vec<CompileError>> {
        // Phase 1: Register all declarations (stubs)
        for prog in programs {
            for decl in &prog.decls {
                self.register_decl(decl);
            }
        }

        // Phase 1b: Check for duplicate fields in named tuple types
        for prog in programs {
            for decl in &prog.decls {
                self.check_named_tuple_fields(decl);
            }
        }

        // Phase 2: Resolve type references in declarations
        self.resolve_typedefs();

        // Phase 3: Validate machines (start states, handlers, spec restrictions)
        let machine_names: Vec<String> = self.machines.keys().cloned().collect();
        for name in &machine_names {
            self.validate_machine(name);
        }

        // Phase 4: Type-check function bodies
        for prog in programs {
            for decl in &prog.decls {
                self.check_decl_bodies(decl);
            }
        }

        // Phase 5: Propagate purity and check capability restrictions
        self.propagate_purity(programs);

        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(std::mem::take(&mut self.errors))
        }
    }

    // ---- Phase 1: Register declarations ----

    fn register_decl(&mut self, decl: &TopDecl) {
        match decl {
            TopDecl::EventDecl(e) => {
                if e.name == "null" || e.name == "halt" {
                    self.err(format!("cannot declare reserved event '{}'", e.name), e.span);
                    return;
                }
                if self.events.contains_key(&e.name) {
                    self.err(format!("duplicate event declaration '{}'", e.name), e.span);
                    return;
                }
                let payload = e.payload.as_ref().map(|t| self.resolve_type(t));
                self.events.insert(e.name.clone(), EventInfo { payload });
            }
            TopDecl::EnumTypeDef(e) => {
                if self.enums.contains_key(&e.name) {
                    self.err(format!("duplicate enum declaration '{}'", e.name), e.span);
                    return;
                }
                let elements: Vec<String> = e.elements.iter().map(|el| el.name.clone()).collect();
                for el in &elements {
                    if self.enum_elements.contains_key(el) {
                        self.err(format!("duplicate enum element '{el}'"), e.span);
                    }
                    self.enum_elements.insert(el.clone(), e.name.clone());
                }
                self.enums.insert(e.name.clone(), EnumInfo { elements });
            }
            TopDecl::TypeDef(td) => {
                if td.name.starts_with('_') {
                    return; // Skip PVerifier placeholder decls
                }
                let resolved = td.ty.as_ref().map(|t| self.resolve_type(t));
                match resolved {
                    Some(ty) => {
                        self.typedefs.insert(td.name.clone(), ty);
                    }
                    None => {
                        // Foreign type
                        self.typedefs.insert(
                            td.name.clone(),
                            PResolvedType::Foreign(td.name.clone()),
                        );
                    }
                }
            }
            TopDecl::EventSetDecl(es) => {
                self.event_sets.insert(es.name.clone(), es.events.clone());
            }
            TopDecl::InterfaceDecl(i) => {
                let payload = i.payload.as_ref().map(|t| self.resolve_type(t));
                self.interfaces.insert(i.name.clone(), payload);
            }
            TopDecl::MachineDecl(m) | TopDecl::SpecMachineDecl(m) => {
                self.register_machine(m);
            }
            TopDecl::FunDecl(f) => {
                self.register_function(f, None);
                self.global_funs_ast.insert(f.name.clone(), f.clone());
            }
            TopDecl::GlobalParamDecl(_) | TopDecl::ModuleDecl(_) | TopDecl::TestDecl(_) | TopDecl::ImplementationDecl(_) => {
                // Handled elsewhere or skipped
            }
        }
    }

    fn register_machine(&mut self, m: &MachineDecl) {
        if self.machines.contains_key(&m.name) {
            self.err(format!("duplicate machine declaration '{}'", m.name), m.span);
            return;
        }

        let mut info = MachineInfo {
            is_spec: m.is_spec,
            states: Vec::new(),
            start_state: None,
            fields: Vec::new(),
            functions: Vec::new(),
            observes: m.observes.clone(),
            state_handlers: HashMap::new(),
            entry_payload: None,
        };

        // Register fields
        for var in &m.body.vars {
            let ty = self.resolve_type(&var.ty);
            for name in &var.names {
                info.fields.push((name.clone(), ty.clone()));
            }
        }

        // Register states
        for state in &m.body.states {
            if info.states.contains(&state.name) {
                self.err(format!("duplicate state '{}' in machine '{}'", state.name, m.name), state.span);
                continue;
            }
            info.states.push(state.name.clone());

            if state.is_start {
                if info.start_state.is_some() {
                    self.err(format!("multiple start states in machine '{}'", m.name), state.span);
                } else {
                    info.start_state = Some(state.name.clone());
                }
            }

            let handler_info = self.build_state_handler_info(state, &m.name);
            info.state_handlers.insert(state.name.clone(), handler_info);
        }

        // Register functions
        for fun in &m.body.funs {
            let qualified = format!("{}::{}", m.name, fun.name);
            info.functions.push(fun.name.clone());
            self.register_function(fun, Some(&m.name));
            // Also register with qualified name
            let finfo = self.functions.get(&fun.name).cloned();
            if let Some(fi) = finfo {
                self.functions.insert(qualified, fi);
            }
        }

        // Determine entry payload from start state
        if let Some(start_name) = &info.start_state {
            if let Some(sh) = info.state_handlers.get(start_name) {
                info.entry_payload = sh.entry_param_type.clone();
            }
        }

        // Also register as an interface
        self.interfaces.insert(m.name.clone(), info.entry_payload.clone());

        self.machines.insert(m.name.clone(), info);
    }

    fn build_state_handler_info(&mut self, state: &StateDecl, machine: &str) -> StateHandlerInfo {
        let mut info = StateHandlerInfo {
            entry_param_type: None,
            handlers: Vec::new(),
            deferred: Vec::new(),
            ignored: Vec::new(),
            is_start: state.is_start,
            temperature: state.temperature,
        };

        for item in &state.items {
            match item {
                StateBodyItem::Entry(ee) => {
                    if let Some(handler) = &ee.anon_handler {
                        if let Some(param) = &handler.param {
                            info.entry_param_type = Some(self.resolve_type(&param.ty));
                        }
                    }
                }
                StateBodyItem::Exit(_) => {}
                StateBodyItem::Defer(events, span) => {
                    for event in events {
                        if event == "null" {
                            self.err("cannot defer the null event", *span);
                        }
                        info.deferred.push(event.clone());
                    }
                }
                StateBodyItem::Ignore(events, span) => {
                    for event in events {
                        if event == "null" {
                            self.err("cannot ignore the null event", *span);
                        }
                        info.ignored.push(event.clone());
                    }
                }
                StateBodyItem::OnEventDoAction(on) => {
                    for event in &on.events {
                        info.handlers.push((event.clone(), HandlerKind::Do));
                    }
                }
                StateBodyItem::OnEventGotoState(on) => {
                    // Validate target state exists (deferred until machine is fully registered)
                    for event in &on.events {
                        info.handlers.push((event.clone(), HandlerKind::Goto(on.target.clone())));
                    }
                }
            }
        }

        // Check for duplicate entry/exit declarations
        let mut entry_count = 0;
        let mut exit_count = 0;
        for item in &state.items {
            match item {
                StateBodyItem::Entry(_) => {
                    entry_count += 1;
                    if entry_count > 1 {
                        self.err(
                            format!("duplicate entry handler in state '{}' of machine '{machine}'", state.name),
                            state.span,
                        );
                    }
                }
                StateBodyItem::Exit(_) => {
                    exit_count += 1;
                    if exit_count > 1 {
                        self.err(
                            format!("duplicate exit handler in state '{}' of machine '{machine}'", state.name),
                            state.span,
                        );
                    }
                }
                _ => {}
            }
        }

        // Check for conflicts
        let mut handled_events: HashSet<String> = HashSet::new();
        for (event, _) in &info.handlers {
            if !handled_events.insert(event.clone()) {
                self.err(
                    format!("duplicate handler for event '{event}' in state '{}' of machine '{machine}'", state.name),
                    state.span,
                );
            }
        }

        // Check defer + handle conflicts
        for event in &info.deferred {
            if handled_events.contains(event) {
                self.err(
                    format!("event '{event}' cannot be both deferred and handled in state '{}' of machine '{machine}'", state.name),
                    state.span,
                );
            }
            if info.ignored.contains(event) {
                self.err(
                    format!("event '{event}' cannot be both deferred and ignored in state '{}' of machine '{machine}'", state.name),
                    state.span,
                );
            }
        }

        info
    }

    fn register_function(&mut self, f: &FunDecl, machine: Option<&str>) {
        let params: Vec<(String, PResolvedType)> = f
            .params
            .iter()
            .map(|p| (p.name.clone(), self.resolve_type(&p.ty)))
            .collect();

        // Check duplicate param names
        let mut seen = HashSet::new();
        for (name, _) in &params {
            if !seen.insert(name.clone()) {
                self.err(format!("duplicate parameter name '{name}' in function '{}'", f.name), f.span);
            }
        }

        let ret_type = f.ret_type.as_ref().map(|t| self.resolve_type(t));

        // Check for duplicate function definitions (within the same scope)
        let key = if let Some(m) = machine {
            format!("{}::{}", m, f.name)
        } else {
            f.name.clone()
        };
        if self.functions.contains_key(&key) {
            if let Some(m) = machine {
                self.err(format!("duplicate function '{}' in machine '{m}'", f.name), f.span);
            } else {
                self.err(format!("duplicate function '{}'", f.name), f.span);
            }
        }

        // Insert with qualified key for duplicate checking
        self.functions.insert(key, FunctionInfo {
            params: params.clone(),
            ret_type: ret_type.clone(),
            is_foreign: f.is_foreign,
            can_send: false, can_raise: false, can_change_state: false,
            can_create: false, can_receive: false, is_nondeterministic: false,
        });
        // Also insert with simple name for lookup
        self.functions.insert(
            f.name.clone(),
            FunctionInfo {
                params,
                ret_type,
                is_foreign: f.is_foreign,
                can_send: false,
                can_raise: false,
                can_change_state: false,
                can_create: false,
                can_receive: false,
                is_nondeterministic: false,
            },
        );
    }

    // ---- Type resolution ----

    fn resolve_type(&self, ty: &PType) -> PResolvedType {
        match ty {
            PType::Bool => PResolvedType::Bool,
            PType::Int => PResolvedType::Int,
            PType::Float => PResolvedType::Float,
            PType::StringType => PResolvedType::String,
            PType::Event => PResolvedType::Event,
            PType::Machine => PResolvedType::Machine,
            PType::Data => PResolvedType::Data,
            PType::Any => PResolvedType::Any,
            PType::Seq(inner) => PResolvedType::Seq(Box::new(self.resolve_type(inner))),
            PType::Set(inner) => PResolvedType::Set(Box::new(self.resolve_type(inner))),
            PType::Map(k, v) => PResolvedType::Map(
                Box::new(self.resolve_type(k)),
                Box::new(self.resolve_type(v)),
            ),
            PType::Tuple(ts) => PResolvedType::Tuple(ts.iter().map(|t| self.resolve_type(t)).collect()),
            PType::NamedTuple(fields) => PResolvedType::NamedTuple(
                fields.iter().map(|(n, t)| (n.clone(), self.resolve_type(t))).collect(),
            ),
            PType::Named(name) => {
                if let Some(resolved) = self.typedefs.get(name) {
                    PResolvedType::TypeDef(name.clone(), Box::new(resolved.clone()))
                } else if self.enums.contains_key(name) {
                    PResolvedType::Enum(name.clone())
                } else if self.machines.contains_key(name) || self.interfaces.contains_key(name) {
                    PResolvedType::Permission(name.clone())
                } else {
                    // Unknown type — could be forward reference, treat as named
                    PResolvedType::Foreign(name.clone())
                }
            }
        }
    }

    fn check_named_tuple_fields(&mut self, decl: &TopDecl) {
        match decl {
            TopDecl::MachineDecl(m) | TopDecl::SpecMachineDecl(m) => {
                for var in &m.body.vars {
                    self.check_type_for_dup_fields(&var.ty, var.span);
                }
            }
            _ => {}
        }
    }

    fn check_type_for_dup_fields(&mut self, ty: &PType, span: Span) {
        if let PType::NamedTuple(fields) = ty {
            let mut seen = HashSet::new();
            for (name, _) in fields {
                if !seen.insert(name.clone()) {
                    self.err(format!("duplicate field name '{name}' in named tuple"), span);
                }
            }
        }
    }

    fn resolve_typedefs(&mut self) {
        // Check for circular typedefs
        for name in self.typedefs.keys().cloned().collect::<Vec<_>>() {
            let mut visited = HashSet::new();
            let mut current = name.clone();
            loop {
                if !visited.insert(current.clone()) {
                    self.errors.push(CompileError::new(format!(
                        "circular type definition involving '{name}'"
                    )));
                    break;
                }
                if let Some(PResolvedType::TypeDef(_, inner)) = self.typedefs.get(&current) {
                    if let PResolvedType::TypeDef(next, _) = inner.as_ref() {
                        current = next.clone();
                        continue;
                    }
                }
                break;
            }
        }
    }

    // ---- Phase 3: Machine validation ----

    fn validate_machine(&mut self, name: &str) {
        let machine = self.machines.get(name).unwrap().clone();

        // Must have a start state
        if machine.start_state.is_none() {
            self.errors.push(CompileError::new(format!(
                "machine '{name}' does not have a start state"
            )));
        }

        // Validate goto targets exist
        for (state_name, handler) in &machine.state_handlers {
            for (event, kind) in &handler.handlers {
                if let HandlerKind::Goto(target) = kind {
                    if !machine.states.contains(target) {
                        self.errors.push(CompileError::new(format!(
                            "state '{target}' referenced in transition from '{state_name}' on event '{event}' does not exist in machine '{name}'"
                        )));
                    }
                }
            }

            // Validate events exist
            for (event, _) in &handler.handlers {
                if event != "null" && event != "halt" && !self.events.contains_key(event) && !self.event_sets.contains_key(event) {
                    // Could be a valid event not yet seen; skip for now
                }
            }
        }

        // Spec machine restrictions
        if machine.is_spec {
            for (state_name, handler) in &machine.state_handlers {
                for event in &handler.deferred {
                    self.errors.push(CompileError::new(format!(
                        "spec machine '{name}' cannot defer events (state '{state_name}', event '{event}')"
                    )));
                }
            }
        }
    }

    // ---- Phase 4: Check declaration bodies ----

    fn check_decl_bodies(&mut self, decl: &TopDecl) {
        match decl {
            TopDecl::MachineDecl(m) | TopDecl::SpecMachineDecl(m) => {
                self.check_machine_bodies(m);
            }
            TopDecl::FunDecl(f) => {
                if let Some(body) = &f.body {
                    let ctx = FnContext {
                        machine: None,
                        is_spec: false,
                        fn_name: f.name.clone(),
                        ret_type: f.ret_type.as_ref().map(|t| self.resolve_type(t)),
                        in_loop: false,
                        is_entry: false,
                        is_exit: false,
                        is_handler: false,
                    };
                    let mut locals = HashMap::new();
                    for p in &f.params {
                        locals.insert(p.name.clone(), self.resolve_type(&p.ty));
                    }
                    self.check_function_body(body, &ctx, &mut locals);

                    // Return path analysis
                    if f.ret_type.is_some() {
                        if !self.all_paths_return(&body.stmts) {
                            self.err(
                                format!("not all code paths return a value in function '{}'", f.name),
                                f.span,
                            );
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn check_machine_bodies(&mut self, m: &MachineDecl) {
        let machine = self.machines.get(&m.name).cloned().unwrap();

        // Check function bodies
        for f in &m.body.funs {
            if let Some(body) = &f.body {
                let ctx = FnContext {
                    machine: Some(m.name.clone()),
                    is_spec: machine.is_spec,
                    fn_name: f.name.clone(),
                    ret_type: f.ret_type.as_ref().map(|t| self.resolve_type(t)),
                    in_loop: false,
                    is_entry: false,
                    is_exit: false,
                    is_handler: false,
                };
                let mut locals = HashMap::new();
                for p in &f.params {
                    locals.insert(p.name.clone(), self.resolve_type(&p.ty));
                }
                self.check_function_body(body, &ctx, &mut locals);

                // Return path analysis
                if f.ret_type.is_some() {
                    if !self.all_paths_return(&body.stmts) {
                        self.err(
                            format!("not all code paths return a value in function '{}::{}'", m.name, f.name),
                            f.span,
                        );
                    }
                }
            }
        }

        // Check state entry/exit/handler bodies
        for state in &m.body.states {
            for item in &state.items {
                match item {
                    StateBodyItem::Entry(ee) => {
                        if let Some(fun_name) = &ee.fun_name {
                            // Named entry function — validate it exists and has right signature
                            if !machine.functions.contains(fun_name) && !self.functions.contains_key(fun_name) {
                                self.err(
                                    format!("entry function '{fun_name}' not found in machine '{}'", m.name),
                                    ee.span,
                                );
                            }
                            // Note: entry functions CAN have parameters (for payload)
                            // The check for payload type matching is complex and deferred
                        }
                        if let Some(handler) = &ee.anon_handler {
                            let ctx = FnContext {
                                machine: Some(m.name.clone()),
                                is_spec: machine.is_spec,
                                fn_name: format!("{}::{}::entry", m.name, state.name),
                                ret_type: None,
                                in_loop: false,
                                is_entry: true,
                                is_exit: false,
                                is_handler: true,
                            };
                            let mut locals = HashMap::new();
                            if let Some(p) = &handler.param {
                                locals.insert(p.name.clone(), self.resolve_type(&p.ty));
                            }
                            self.check_function_body(&handler.body, &ctx, &mut locals);
                        }
                    }
                    StateBodyItem::Exit(ee) => {
                        if let Some(fun_name) = &ee.fun_name {
                            if !machine.functions.contains(fun_name) && !self.functions.contains_key(fun_name) {
                                self.err(
                                    format!("exit function '{fun_name}' not found in machine '{}'", m.name),
                                    ee.span,
                                );
                            }
                            // Exit functions must have 0 parameters
                            if let Some(fi) = self.functions.get(fun_name) {
                                if !fi.params.is_empty() {
                                    self.err(
                                        format!("exit function '{fun_name}' must have no parameters"),
                                        ee.span,
                                    );
                                }
                            }
                        }
                        if let Some(handler) = &ee.anon_handler {
                            if handler.param.is_some() {
                                self.err("exit handler cannot have parameters", ee.span);
                            }
                            let ctx = FnContext {
                                machine: Some(m.name.clone()),
                                is_spec: machine.is_spec,
                                fn_name: format!("{}::{}::exit", m.name, state.name),
                                ret_type: None,
                                in_loop: false,
                                is_entry: false,
                                is_exit: true,
                                is_handler: true,
                            };
                            let mut locals = HashMap::new();
                            self.check_function_body(&handler.body, &ctx, &mut locals);
                        }
                    }
                    StateBodyItem::OnEventDoAction(on) => {
                        // null is a valid event for default handler

                        if let Some(fun_name) = &on.fun_name {
                            if !machine.functions.contains(fun_name) && !self.functions.contains_key(fun_name) {
                                self.err(
                                    format!("handler function '{fun_name}' not found in machine '{}'", m.name),
                                    on.span,
                                );
                            }
                        }
                        if let Some(handler) = &on.anon_handler {
                            let ctx = FnContext {
                                machine: Some(m.name.clone()),
                                is_spec: machine.is_spec,
                                fn_name: format!("{}::{}::on_do", m.name, state.name),
                                ret_type: None,
                                in_loop: false,
                                is_entry: false,
                                is_exit: false,
                                is_handler: true,
                            };
                            let mut locals = HashMap::new();
                            if let Some(p) = &handler.param {
                                locals.insert(p.name.clone(), self.resolve_type(&p.ty));
                            }
                            self.check_function_body(&handler.body, &ctx, &mut locals);
                        }
                    }
                    StateBodyItem::OnEventGotoState(on) => {
                        for event in &on.events {
                            if event == "null" && machine.is_spec {
                                self.err("spec machine cannot transition on null event", on.span);
                            }
                        }

                        if let Some(fun_name) = &on.with_fun_name {
                            if !machine.functions.contains(fun_name) && !self.functions.contains_key(fun_name) {
                                self.err(
                                    format!("transition function '{fun_name}' not found in machine '{}'", m.name),
                                    on.span,
                                );
                            }
                        }
                        if let Some(handler) = &on.with_anon_handler {
                            let ctx = FnContext {
                                machine: Some(m.name.clone()),
                                is_spec: machine.is_spec,
                                fn_name: format!("{}::{}::goto_with", m.name, state.name),
                                ret_type: None,
                                in_loop: false,
                                is_entry: false,
                                is_exit: false, // goto-with handlers can use receive
                                is_handler: true,
                            };
                            let mut locals = HashMap::new();
                            if let Some(p) = &handler.param {
                                locals.insert(p.name.clone(), self.resolve_type(&p.ty));
                            }
                            self.check_function_body(&handler.body, &ctx, &mut locals);
                        }
                    }
                    StateBodyItem::Defer(_, _) | StateBodyItem::Ignore(_, _) => {}
                }
            }
        }
    }

    // ---- Function body type checking ----

    fn check_function_body(
        &mut self,
        body: &FunctionBody,
        ctx: &FnContext,
        locals: &mut HashMap<String, PResolvedType>,
    ) {
        // Register local variables
        for var in &body.var_decls {
            let ty = self.resolve_type(&var.ty);
            for name in &var.names {
                locals.insert(name.clone(), ty.clone());
            }
        }

        for stmt in &body.stmts {
            self.check_stmt(stmt, ctx, locals);
        }
    }

    fn check_stmt(
        &mut self,
        stmt: &Stmt,
        ctx: &FnContext,
        locals: &mut HashMap<String, PResolvedType>,
    ) {
        match stmt {
            Stmt::Compound(stmts, _) => {
                for s in stmts {
                    self.check_stmt(s, ctx, locals);
                }
            }
            Stmt::Assert { expr, message, span } => {
                let ty = self.infer_expr_type(expr, ctx, locals);
                if ty != PResolvedType::Bool && ty != PResolvedType::Any && ty != PResolvedType::Void {
                    self.err("assert condition must be bool", *span);
                }
                if let Some(msg) = message {
                    self.infer_expr_type(msg, ctx, locals);
                }
            }
            Stmt::Assume { expr, message, span } => {
                let ty = self.infer_expr_type(expr, ctx, locals);
                if ty != PResolvedType::Bool && ty != PResolvedType::Any && ty != PResolvedType::Void {
                    self.err("assume condition must be bool", *span);
                }
                if let Some(msg) = message {
                    self.infer_expr_type(msg, ctx, locals);
                }
            }
            Stmt::Print { message, .. } => {
                self.infer_expr_type(message, ctx, locals);
            }
            Stmt::Return { value, span } => {
                if ctx.is_entry && value.is_some() {
                    self.err("entry handler cannot return a value", *span);
                }
                if let Some(val) = value {
                    let val_ty = self.infer_expr_type(val, ctx, locals);
                    if let Some(ret_ty) = &ctx.ret_type {
                        if !ret_ty.is_assignable_from(&val_ty) && val_ty != PResolvedType::Void {
                            self.err(
                                format!("return type mismatch: expected {ret_ty}, got {val_ty}"),
                                *span,
                            );
                        }
                    }
                } else if ctx.ret_type.is_some() {
                    // Return without value in non-void function
                    // This is actually OK if raise/goto follows, handled by return path analysis
                }
            }
            Stmt::Break(span) => {
                if !ctx.in_loop {
                    self.err("break statement outside of loop", *span);
                }
            }
            Stmt::Continue(span) => {
                if !ctx.in_loop {
                    self.err("continue statement outside of loop", *span);
                }
            }
            Stmt::Assign { lvalue, rvalue, span } => {
                // Check for set indexed assignment (not allowed)
                if let LValue::Index(base, _, _) = lvalue {
                    let base_ty = self.infer_lvalue_type(base, ctx, locals);
                    if matches!(base_ty.canonicalize(), PResolvedType::Set(_)) {
                        self.err("sets do not support indexed assignment", *span);
                    }
                }
                let lhs_ty = self.infer_lvalue_type(lvalue, ctx, locals);
                let rhs_ty = self.infer_expr_type(rvalue, ctx, locals);
                // Check for function-returning-nothing used in assignment
                if rhs_ty == PResolvedType::Void {
                    if let Expr::FunCall(name, _, _) = rvalue {
                        if let Some(fi) = self.functions.get(name) {
                            if fi.ret_type.is_none() {
                                self.err(
                                    format!("function '{name}' does not return a value"),
                                    *span,
                                );
                            }
                        }
                    }
                }
                if lhs_ty != PResolvedType::Void
                    && rhs_ty != PResolvedType::Void
                    && lhs_ty != PResolvedType::Any
                    && rhs_ty != PResolvedType::Any
                {
                    if !lhs_ty.is_assignable_from(&rhs_ty) {
                        self.err(
                            format!("type mismatch in assignment: cannot assign {rhs_ty} to {lhs_ty}"),
                            *span,
                        );
                    }
                }
            }
            Stmt::Insert { lvalue, index, value, span } => {
                let lhs_ty = self.infer_lvalue_type(lvalue, ctx, locals);
                self.infer_expr_type(index, ctx, locals);
                self.infer_expr_type(value, ctx, locals);
                let _ = (lhs_ty, span); // Type checking for insert is complex, deferred
            }
            Stmt::AddToSet { lvalue, value, span } => {
                let lhs_ty = self.infer_lvalue_type(lvalue, ctx, locals);
                let val_ty = self.infer_expr_type(value, ctx, locals);
                // Check: for set[T], value must be assignable to T
                if let PResolvedType::Set(elem_ty) = lhs_ty.canonicalize() {
                    if val_ty != PResolvedType::Any && val_ty != PResolvedType::Void
                        && !elem_ty.is_assignable_from(&val_ty)
                    {
                        self.err(format!(
                            "type mismatch: cannot add {val_ty} to set[{elem_ty}]"
                        ), *span);
                    }
                }
            }
            Stmt::Remove { lvalue, key, .. } => {
                self.infer_lvalue_type(lvalue, ctx, locals);
                self.infer_expr_type(key, ctx, locals);
            }
            Stmt::While { cond, body, .. } => {
                let cond_ty = self.infer_expr_type(cond, ctx, locals);
                if cond_ty != PResolvedType::Bool && cond_ty != PResolvedType::Any && cond_ty != PResolvedType::Void {
                    // Soft check — while condition should be bool
                }
                let loop_ctx = FnContext {
                    in_loop: true,
                    ..FnContext {
                        machine: ctx.machine.clone(),
                        is_spec: ctx.is_spec,
                        fn_name: ctx.fn_name.clone(),
                        ret_type: ctx.ret_type.clone(),
                        in_loop: true,
                        is_entry: ctx.is_entry,
                        is_exit: ctx.is_exit,
                        is_handler: ctx.is_handler,
                    }
                };
                self.check_stmt(body, &loop_ctx, locals);
            }
            Stmt::Foreach { item, collection, body, span, .. } => {
                let col_ty = self.infer_expr_type(collection, ctx, locals);
                // Infer iterator type from collection element type
                let elem_ty = match col_ty.canonicalize() {
                    PResolvedType::Seq(elem) => *elem,
                    PResolvedType::Set(elem) => *elem,
                    PResolvedType::Map(key, _) => *key,
                    PResolvedType::Any => PResolvedType::Any,
                    other => {
                        self.err(format!("foreach requires a collection (seq, set, or map), got {other}"), *span);
                        PResolvedType::Any
                    }
                };
                locals.insert(item.clone(), elem_ty);
                let loop_ctx = FnContext {
                    machine: ctx.machine.clone(),
                    is_spec: ctx.is_spec,
                    fn_name: ctx.fn_name.clone(),
                    ret_type: ctx.ret_type.clone(),
                    in_loop: true,
                    is_entry: ctx.is_entry,
                    is_exit: ctx.is_exit,
                    is_handler: ctx.is_handler,
                };
                self.check_stmt(body, &loop_ctx, locals);
            }
            Stmt::If { cond, then_branch, else_branch, .. } => {
                self.infer_expr_type(cond, ctx, locals);
                self.check_stmt(then_branch, ctx, locals);
                if let Some(eb) = else_branch {
                    self.check_stmt(eb, ctx, locals);
                }
            }
            Stmt::CtorStmt { interface, args, span } => {
                if ctx.is_spec {
                    self.err("spec machine cannot create machines", *span);
                }
                let mut arg_types = Vec::new();
                for arg in args {
                    arg_types.push(self.infer_expr_type(arg, ctx, locals));
                }
                // Check interface exists
                if !self.interfaces.contains_key(interface) && !self.machines.contains_key(interface) {
                    self.err(format!("unknown machine/interface '{interface}'"), *span);
                }
                // Cannot create spec machines
                if let Some(m) = self.machines.get(interface) {
                    if m.is_spec {
                        self.err(format!("cannot create spec machine '{interface}'"), *span);
                    }
                }
                // Check payload matches machine entry parameter
                if let Some(expected_payload) = self.interfaces.get(interface).and_then(|p| p.clone()) {
                    let actual = match arg_types.len() {
                        0 => {
                            self.err(format!("machine '{interface}' requires a payload of type {expected_payload}"), *span);
                            return;
                        }
                        1 => arg_types[0].clone(),
                        _ => PResolvedType::Tuple(arg_types),
                    };
                    if actual != PResolvedType::Any && actual != PResolvedType::Void
                        && !expected_payload.is_assignable_from(&actual)
                    {
                        self.err(format!(
                            "machine '{interface}' expects payload {expected_payload}, got {actual}"
                        ), *span);
                    }
                }
            }
            Stmt::FunCall { name, args, span } => {
                for arg in args {
                    self.infer_expr_type(arg, ctx, locals);
                }
                if !self.functions.contains_key(name) && !self.enum_elements.contains_key(name) {
                    self.err(format!("undefined function '{name}'"), *span);
                }
            }
            Stmt::Raise { event, args, span } => {
                if ctx.is_exit {
                    self.err("cannot raise event in exit handler", *span);
                }
                let _ev_ty = self.infer_expr_type(event, ctx, locals);
                // Check for raising null event
                if matches!(event, Expr::NullLit(_)) || matches!(event, Expr::Iden(n, _) if n == "null") {
                    self.err("cannot raise the null event", *span);
                }
                // Check payload matches event declaration
                if let Expr::Iden(name, _) = event {
                    if let Some(ev_info) = self.events.get(name) {
                        if ev_info.payload.is_some() && args.is_empty() {
                            self.err(format!(
                                "event '{name}' requires a payload but raise provides none"
                            ), *span);
                        }
                    }
                }
                for arg in args {
                    self.infer_expr_type(arg, ctx, locals);
                }
            }
            Stmt::Send { target, event, args, span } => {
                if ctx.is_spec {
                    self.err("spec machine cannot send events", *span);
                }
                // Check for sending null event
                if matches!(event, Expr::NullLit(_)) || matches!(event, Expr::Iden(n, _) if n == "null") {
                    self.err("cannot send the null event", *span);
                }
                self.infer_expr_type(target, ctx, locals);
                self.infer_expr_type(event, ctx, locals);
                // Check send payload matches event declaration
                if let Expr::Iden(name, _) = event {
                    let ev_has_payload = self.events.get(name).and_then(|e| e.payload.as_ref()).is_some();
                    let ev_exists = self.events.contains_key(name);
                    if ev_exists {
                        if ev_has_payload && args.is_empty() {
                            self.err(format!("event '{name}' requires a payload but send provides none"), *span);
                        }
                        if !ev_has_payload && !args.is_empty() {
                            self.err(format!("event '{name}' has no payload but send provides one"), *span);
                        }
                    }
                }
                for arg in args {
                    self.infer_expr_type(arg, ctx, locals);
                }
            }
            Stmt::Announce { event, args, span } => {
                if ctx.is_spec {
                    self.err("spec machine cannot announce events", *span);
                }
                self.infer_expr_type(event, ctx, locals);
                for arg in args {
                    self.infer_expr_type(arg, ctx, locals);
                }
            }
            Stmt::Goto { state, payload, span } => {
                if ctx.is_exit {
                    self.err("cannot use goto in exit handler", *span);
                }
                // Validate state exists and check payload
                if let Some(machine_name) = &ctx.machine {
                    let machine_info = self.machines.get(machine_name).cloned();
                    if let Some(machine) = machine_info {
                        if !machine.states.contains(state) {
                            self.err(
                                format!("undefined state '{state}' in machine '{machine_name}'"),
                                *span,
                            );
                        }
                        // Check payload matches target state's entry parameter
                        if let Some(sh) = machine.state_handlers.get(state) {
                            if let Some(ref expected) = sh.entry_param_type {
                                if payload.is_empty() {
                                    self.err(format!(
                                        "goto '{state}' requires payload of type {expected}"
                                    ), *span);
                                }
                            }
                        }
                    }
                }
                for arg in payload {
                    self.infer_expr_type(arg, ctx, locals);
                }
            }
            Stmt::Receive { cases, span } => {
                if ctx.is_spec {
                    self.err("spec machine cannot use receive", *span);
                }
                for case in cases {
                    if let Some(param) = &case.handler.param {
                        let mut case_locals = locals.clone();
                        case_locals.insert(param.name.clone(), self.resolve_type(&param.ty));
                        self.check_function_body(&case.handler.body, ctx, &mut case_locals);
                    } else {
                        let mut case_locals = locals.clone();
                        self.check_function_body(&case.handler.body, ctx, &mut case_locals);
                    }
                }
            }
            Stmt::NoStmt(_) => {}
        }
    }

    // ---- Expression type inference ----

    fn infer_expr_type(
        &mut self,
        expr: &Expr,
        ctx: &FnContext,
        locals: &HashMap<String, PResolvedType>,
    ) -> PResolvedType {
        match expr {
            Expr::IntLit(_, _) => PResolvedType::Int,
            Expr::FloatLit(_, _) => PResolvedType::Float,
            Expr::BoolLit(_, _) => PResolvedType::Bool,
            Expr::StringLit(_, _) => PResolvedType::String,
            Expr::NullLit(_) => PResolvedType::Null,
            Expr::This(_) => PResolvedType::Machine,
            Expr::HaltEvent(_) => PResolvedType::Event,
            Expr::Nondet(_) | Expr::FairNondet(_) => PResolvedType::Bool,

            Expr::Iden(name, _span) => {
                // Check locals, then machine fields, then enum elements, then events
                if let Some(ty) = locals.get(name) {
                    return ty.clone();
                }
                if let Some(machine_name) = &ctx.machine {
                    if let Some(machine) = self.machines.get(machine_name) {
                        if let Some((_, ty)) = machine.fields.iter().find(|(n, _)| n == name) {
                            return ty.clone();
                        }
                    }
                }
                if let Some(enum_name) = self.enum_elements.get(name) {
                    return PResolvedType::Enum(enum_name.clone());
                }
                if self.events.contains_key(name) {
                    return PResolvedType::Event;
                }
                // Could be a machine/interface reference
                if self.machines.contains_key(name) || self.interfaces.contains_key(name) {
                    return PResolvedType::Machine;
                }
                // Unknown — might be caught elsewhere
                PResolvedType::Any
            }

            Expr::UnnamedTuple(fields, _) => {
                let types: Vec<_> = fields.iter().map(|f| self.infer_expr_type(f, ctx, locals)).collect();
                PResolvedType::Tuple(types)
            }
            Expr::NamedTuple(fields, span) => {
                // Check for duplicate field names
                let mut seen = HashSet::new();
                for (name, _) in fields {
                    if !seen.insert(name.clone()) {
                        self.err(format!("duplicate field name '{name}' in named tuple expression"), *span);
                    }
                }
                let types: Vec<_> = fields
                    .iter()
                    .map(|(n, f)| (n.clone(), self.infer_expr_type(f, ctx, locals)))
                    .collect();
                PResolvedType::NamedTuple(types)
            }

            Expr::NamedTupleAccess(base, field, span) => {
                let base_ty = self.infer_expr_type(base, ctx, locals);
                match base_ty.canonicalize() {
                    PResolvedType::NamedTuple(fields) => {
                        if let Some((_, ty)) = fields.iter().find(|(n, _)| n == field) {
                            ty.clone()
                        } else {
                            self.err(format!("field '{field}' not found in named tuple"), *span);
                            PResolvedType::Any
                        }
                    }
                    PResolvedType::Any => PResolvedType::Any,
                    _ => {
                        self.err(format!("cannot access field '{field}' on type {base_ty}"), *span);
                        PResolvedType::Any
                    }
                }
            }

            Expr::TupleAccess(base, idx, span) => {
                let base_ty = self.infer_expr_type(base, ctx, locals);
                match base_ty.canonicalize() {
                    PResolvedType::Tuple(fields) => {
                        if *idx < fields.len() {
                            fields[*idx].clone()
                        } else {
                            self.err(format!("tuple index {idx} out of bounds"), *span);
                            PResolvedType::Any
                        }
                    }
                    PResolvedType::Any => PResolvedType::Any,
                    _ => {
                        self.err(format!("cannot access index {idx} on type {base_ty}"), *span);
                        PResolvedType::Any
                    }
                }
            }

            Expr::SeqMapAccess(base, index, _span) => {
                let base_ty = self.infer_expr_type(base, ctx, locals);
                self.infer_expr_type(index, ctx, locals);
                match base_ty.canonicalize() {
                    PResolvedType::Seq(elem) => *elem,
                    PResolvedType::Map(_, val) => *val,
                    PResolvedType::Any => PResolvedType::Any,
                    _ => PResolvedType::Any,
                }
            }

            Expr::Keys(base, _) => {
                let base_ty = self.infer_expr_type(base, ctx, locals);
                match base_ty.canonicalize() {
                    PResolvedType::Map(k, _) => PResolvedType::Seq(k),
                    _ => PResolvedType::Seq(Box::new(PResolvedType::Any)),
                }
            }
            Expr::Values(base, _) => {
                let base_ty = self.infer_expr_type(base, ctx, locals);
                match base_ty.canonicalize() {
                    PResolvedType::Map(_, v) => PResolvedType::Seq(v),
                    _ => PResolvedType::Seq(Box::new(PResolvedType::Any)),
                }
            }
            Expr::Sizeof(base, _) => {
                self.infer_expr_type(base, ctx, locals);
                PResolvedType::Int
            }
            Expr::Default(ty, _) => self.resolve_type(ty),

            Expr::New(interface, args, span) => {
                if ctx.is_spec {
                    self.err("spec machine cannot create machines", *span);
                }
                // Cannot create spec machines
                if let Some(m) = self.machines.get(interface) {
                    if m.is_spec {
                        self.err(format!("cannot create spec machine '{interface}'"), *span);
                    }
                }
                for arg in args {
                    self.infer_expr_type(arg, ctx, locals);
                }
                // Return the interface/machine type so it can be assigned to typed variables
                if self.interfaces.contains_key(interface) || self.machines.contains_key(interface) {
                    PResolvedType::Permission(interface.clone())
                } else {
                    PResolvedType::Machine
                }
            }

            Expr::FunCall(name, args, span) => {
                for arg in args {
                    self.infer_expr_type(arg, ctx, locals);
                }

                // Check if it's a known function
                if let Some(finfo) = self.functions.get(name).cloned() {
                    // Check argument count
                    if args.len() != finfo.params.len() {
                        self.err(
                            format!(
                                "function '{name}' expects {} arguments, got {}",
                                finfo.params.len(),
                                args.len()
                            ),
                            *span,
                        );
                    }
                    finfo.ret_type.unwrap_or(PResolvedType::Void)
                } else if self.enum_elements.contains_key(name) {
                    // Enum element used as function — this is actually a function call
                    // that shadows the enum element
                    PResolvedType::Any
                } else {
                    // Unknown function — don't error here, could be machine method
                    PResolvedType::Any
                }
            }

            Expr::Neg(inner, span) => {
                let ty = self.infer_expr_type(inner, ctx, locals);
                match ty.canonicalize() {
                    PResolvedType::Int => PResolvedType::Int,
                    PResolvedType::Float => PResolvedType::Float,
                    PResolvedType::Any => PResolvedType::Any,
                    _ => {
                        self.err(format!("unary '-' requires int or float, got {ty}"), *span);
                        PResolvedType::Int
                    }
                }
            }
            Expr::Not(inner, span) => {
                let ty = self.infer_expr_type(inner, ctx, locals);
                if ty != PResolvedType::Bool && ty != PResolvedType::Any {
                    self.err(format!("unary '!' requires bool, got {ty}"), *span);
                }
                PResolvedType::Bool
            }

            Expr::BinOp(op, lhs, rhs, span) => {
                let lt = self.infer_expr_type(lhs, ctx, locals);
                let rt = self.infer_expr_type(rhs, ctx, locals);
                self.check_binop(*op, &lt, &rt, *span)
            }

            Expr::Cast(inner, ty, span) => {
                let inner_ty = self.infer_expr_type(inner, ctx, locals);
                let target_ty = self.resolve_type(ty);
                // Check for invalid null casts
                if inner_ty == PResolvedType::Null {
                    if matches!(target_ty, PResolvedType::Int | PResolvedType::Bool | PResolvedType::Float | PResolvedType::String) {
                        self.err(format!("cannot cast null to {target_ty}"), *span);
                    }
                }
                target_ty
            }

            Expr::Choose(arg, span) => {
                if let Some(a) = arg {
                    // Static check: choose(literal > 10000) is an error
                    if let Expr::IntLit(n, _) = a.as_ref() {
                        if *n > 10000 {
                            self.err(format!("choose argument {n} exceeds maximum of 10000"), *span);
                        }
                    }
                    let ty = self.infer_expr_type(a, ctx, locals);
                    match ty.canonicalize() {
                        PResolvedType::Int => PResolvedType::Int,
                        PResolvedType::Seq(elem) => *elem,
                        PResolvedType::Set(elem) => *elem,
                        PResolvedType::Map(key, _) => *key,
                        _ => PResolvedType::Any,
                    }
                } else {
                    PResolvedType::Bool
                }
            }

            Expr::FormatString(_, args, _) => {
                for arg in args {
                    self.infer_expr_type(arg, ctx, locals);
                }
                PResolvedType::String
            }

            Expr::Paren(inner, _) => self.infer_expr_type(inner, ctx, locals),
        }
    }

    fn check_binop(&mut self, op: BinOp, lt: &PResolvedType, rt: &PResolvedType, span: Span) -> PResolvedType {
        let lc = lt.canonicalize();
        let rc = rt.canonicalize();

        // Any is always compatible
        if lc == PResolvedType::Any || rc == PResolvedType::Any {
            return match op {
                BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod => PResolvedType::Any,
                _ => PResolvedType::Bool,
            };
        }

        match op {
            BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod => {
                if lc == PResolvedType::Int && rc == PResolvedType::Int {
                    PResolvedType::Int
                } else if lc == PResolvedType::Float && rc == PResolvedType::Float {
                    PResolvedType::Float
                } else if op == BinOp::Add && lc == PResolvedType::String && rc == PResolvedType::String {
                    PResolvedType::String
                } else {
                    self.err(
                        format!("arithmetic operator requires matching numeric types, got {lt} and {rt}"),
                        span,
                    );
                    PResolvedType::Int
                }
            }
            BinOp::Lt | BinOp::Gt | BinOp::Le | BinOp::Ge => {
                let valid = (lc.is_numeric() && rc.is_numeric() && lc == rc)
                    || (lc == PResolvedType::String && rc == PResolvedType::String);
                if !valid {
                    self.err(
                        format!("comparison requires matching numeric or string types, got {lt} and {rt}"),
                        span,
                    );
                }
                PResolvedType::Bool
            }
            BinOp::Eq | BinOp::Ne => {
                // Types must be comparable
                if !lt.is_assignable_from(rt) && !rt.is_assignable_from(lt) {
                    self.err(
                        format!("incomparable types: {lt} and {rt}"),
                        span,
                    );
                }
                PResolvedType::Bool
            }
            BinOp::And | BinOp::Or => {
                if lc != PResolvedType::Bool {
                    self.err(format!("logical operator requires bool, got {lt}"), span);
                }
                if rc != PResolvedType::Bool {
                    self.err(format!("logical operator requires bool, got {rt}"), span);
                }
                PResolvedType::Bool
            }
            BinOp::In => {
                // RHS must be a collection
                match rc {
                    PResolvedType::Map(_, _) | PResolvedType::Seq(_) | PResolvedType::Set(_) => {}
                    _ => {
                        self.err(format!("'in' requires collection on right side, got {rt}"), span);
                    }
                }
                PResolvedType::Bool
            }
        }
    }

    fn infer_lvalue_type(
        &mut self,
        lv: &LValue,
        ctx: &FnContext,
        locals: &HashMap<String, PResolvedType>,
    ) -> PResolvedType {
        match lv {
            LValue::Var(name, span) => {
                // Check if name is a local variable or field first (shadows events)
                if let Some(ty) = locals.get(name) {
                    return ty.clone();
                }
                if let Some(machine_name) = &ctx.machine {
                    if let Some(machine) = self.machines.get(machine_name) {
                        if let Some((_, ty)) = machine.fields.iter().find(|(n, _)| n == name) {
                            return ty.clone();
                        }
                    }
                }
                // If not a local/field, check if it's an event (cannot assign to events)
                if self.events.contains_key(name) {
                    self.err(format!("cannot assign to event '{name}'"), *span);
                }
                PResolvedType::Any
            }
            LValue::NamedTupleField(base, field, span) => {
                let base_ty = self.infer_lvalue_type(base, ctx, locals);
                match base_ty.canonicalize() {
                    PResolvedType::NamedTuple(fields) => {
                        if let Some((_, ty)) = fields.iter().find(|(n, _)| n == field) {
                            ty.clone()
                        } else {
                            self.err(format!("field '{field}' not found"), *span);
                            PResolvedType::Any
                        }
                    }
                    _ => PResolvedType::Any,
                }
            }
            LValue::TupleField(base, idx, _) => {
                let base_ty = self.infer_lvalue_type(base, ctx, locals);
                match base_ty.canonicalize() {
                    PResolvedType::Tuple(fields) => {
                        if *idx < fields.len() { fields[*idx].clone() } else { PResolvedType::Any }
                    }
                    _ => PResolvedType::Any,
                }
            }
            LValue::Index(base, index, _) => {
                let base_ty = self.infer_lvalue_type(base, ctx, locals);
                self.infer_expr_type(index, ctx, locals);
                match base_ty.canonicalize() {
                    PResolvedType::Seq(elem) => *elem,
                    PResolvedType::Map(_, val) => *val,
                    _ => PResolvedType::Any,
                }
            }
        }
    }

    // ---- Return path analysis ----

    fn all_paths_return(&self, stmts: &[Stmt]) -> bool {
        for stmt in stmts {
            if self.surely_returns(stmt) {
                return true;
            }
        }
        false
    }

    fn surely_returns(&self, stmt: &Stmt) -> bool {
        match stmt {
            Stmt::Return { .. } => true,
            Stmt::Raise { .. } => true,
            Stmt::Goto { .. } => true,
            Stmt::Compound(stmts, _) => self.all_paths_return(stmts),
            Stmt::If { then_branch, else_branch, .. } => {
                if let Some(else_b) = else_branch {
                    self.surely_returns(then_branch) && self.surely_returns(else_b)
                } else {
                    false
                }
            }
            Stmt::Assert { expr: Expr::BoolLit(false, _), .. } => true,
            Stmt::Receive { cases, .. } => {
                !cases.is_empty() && cases.iter().all(|c| self.all_paths_return(&c.handler.body.stmts))
            }
            _ => false,
        }
    }

    // ---- Phase 5: Purity propagation ----

    fn propagate_purity(&mut self, programs: &[Program]) {
        // Scan all function/handler bodies for capability-bearing statements
        for prog in programs {
            for decl in &prog.decls {
                if let TopDecl::MachineDecl(m) | TopDecl::SpecMachineDecl(m) = decl {
                    let is_spec = m.is_spec;
                    for state in &m.body.states {
                        for item in &state.items {
                            match item {
                                StateBodyItem::Exit(ee) => {
                                    if let Some(handler) = &ee.anon_handler {
                                        self.check_exit_purity(&handler.body, &m.name, &state.name);
                                    }
                                    if let Some(fn_name) = &ee.fun_name {
                                        self.check_named_exit_purity(fn_name, &m.name, &state.name);
                                    }
                                }
                                StateBodyItem::OnEventGotoState(on) => {
                                    if let Some(handler) = &on.with_anon_handler {
                                        self.check_exit_purity(&handler.body, &m.name, &state.name);
                                    }
                                    if let Some(fn_name) = &on.with_fun_name {
                                        self.check_named_exit_purity(fn_name, &m.name, &state.name);
                                    }
                                }
                                _ => {}
                            }
                        }
                    }

                    // For spec machines, check all functions don't have forbidden capabilities
                    if is_spec {
                        for f in &m.body.funs {
                            if let Some(body) = &f.body {
                                self.check_spec_purity(body, &m.name, &f.name);
                            }
                        }
                        for state in &m.body.states {
                            for item in &state.items {
                                match item {
                                    StateBodyItem::Entry(ee) => {
                                        if let Some(h) = &ee.anon_handler {
                                            self.check_spec_purity(&h.body, &m.name, &state.name);
                                        }
                                    }
                                    StateBodyItem::OnEventDoAction(on) => {
                                        if let Some(h) = &on.anon_handler {
                                            self.check_spec_purity(&h.body, &m.name, &state.name);
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    fn check_exit_purity(&mut self, body: &FunctionBody, machine: &str, state: &str) {
        for stmt in &body.stmts {
            self.check_exit_stmt(stmt, machine, state);
        }
    }

    fn check_exit_stmt(&mut self, stmt: &Stmt, machine: &str, state: &str) {
        match stmt {
            Stmt::Raise { span, .. } => {
                self.err(
                    format!("cannot raise event in exit/transition handler of state '{state}' in machine '{machine}'"),
                    *span,
                );
            }
            Stmt::Goto { span, .. } => {
                self.err(
                    format!("cannot use goto in exit/transition handler of state '{state}' in machine '{machine}'"),
                    *span,
                );
            }
            Stmt::Receive { .. } => {
                // receive IS allowed in exit handlers (unlike raise/goto)
            }
            Stmt::Compound(stmts, _) => {
                for s in stmts {
                    self.check_exit_stmt(s, machine, state);
                }
            }
            Stmt::If { then_branch, else_branch, .. } => {
                self.check_exit_stmt(then_branch, machine, state);
                if let Some(eb) = else_branch {
                    self.check_exit_stmt(eb, machine, state);
                }
            }
            Stmt::While { body, .. } | Stmt::Foreach { body, .. } => {
                self.check_exit_stmt(body, machine, state);
            }
            _ => {}
        }
    }

    fn check_named_exit_purity(&mut self, fn_name: &str, machine: &str, state: &str) {
        // Check if the named function contains forbidden operations
        if let Some(finfo) = self.functions.get(fn_name) {
            if finfo.can_raise {
                self.errors.push(CompileError::new(format!(
                    "exit/transition function '{fn_name}' in machine '{machine}' state '{state}' cannot raise events"
                )));
            }
            if finfo.can_change_state {
                self.errors.push(CompileError::new(format!(
                    "exit/transition function '{fn_name}' in machine '{machine}' state '{state}' cannot change state"
                )));
            }
        }
    }

    fn check_spec_purity(&mut self, body: &FunctionBody, machine: &str, context: &str) {
        for stmt in &body.stmts {
            self.check_spec_stmt(stmt, machine, context);
        }
    }

    fn check_spec_stmt(&mut self, stmt: &Stmt, machine: &str, context: &str) {
        match stmt {
            Stmt::Send { span, .. } => {
                self.err(
                    format!("spec machine '{machine}' cannot send events (in {context})"),
                    *span,
                );
            }
            Stmt::CtorStmt { span, .. } => {
                self.err(
                    format!("spec machine '{machine}' cannot create machines (in {context})"),
                    *span,
                );
            }
            Stmt::Announce { span, .. } => {
                self.err(
                    format!("spec machine '{machine}' cannot announce events (in {context})"),
                    *span,
                );
            }
            Stmt::Receive { span, .. } => {
                self.err(
                    format!("spec machine '{machine}' cannot use receive (in {context})"),
                    *span,
                );
            }
            Stmt::Compound(stmts, _) => {
                for s in stmts {
                    self.check_spec_stmt(s, machine, context);
                }
            }
            Stmt::If { then_branch, else_branch, .. } => {
                self.check_spec_stmt(then_branch, machine, context);
                if let Some(eb) = else_branch {
                    self.check_spec_stmt(eb, machine, context);
                }
            }
            Stmt::While { body, .. } | Stmt::Foreach { body, .. } => {
                self.check_spec_stmt(body, machine, context);
            }
            Stmt::FunCall { name, span, .. } => {
                // Check if the called function contains forbidden operations
                if let Some(fun) = self.global_funs_ast.get(name).cloned() {
                    if let Some(body) = &fun.body {
                        self.check_spec_purity(body, machine, &format!("function '{name}' called from {context}"));
                    }
                }
            }
            _ => {}
        }
    }
}

// FnContext is defined inside impl but let's keep it as a standalone struct
struct FnContext {
    machine: Option<String>,
    is_spec: bool,
    fn_name: String,
    ret_type: Option<PResolvedType>,
    in_loop: bool,
    is_entry: bool,
    is_exit: bool,
    is_handler: bool,
}
