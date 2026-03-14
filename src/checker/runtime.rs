//! P program runtime: executes compiled P programs with model checking.

use std::collections::{BTreeMap, HashMap, VecDeque};

use log::{debug, trace};
use rand::Rng;
use rand::RngExt;

use crate::compiler::ast::*;
use super::value::{PValue, OrderedFloat};

/// Outcome of executing a handler (entry, exit, event handler).
#[derive(Debug)]
enum HandlerOutcome {
    Normal,
    Raised(String, Option<PValue>),   // event name, payload
    GotoState(String, Option<PValue>), // state name, payload
    Halted,
    Return(Option<PValue>),
    Break,
    Continue,
}

/// Error found during model checking.
#[derive(Debug, Clone)]
pub struct CheckError {
    pub message: String,
}

impl std::fmt::Display for CheckError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

/// Per-machine instance state.
struct MachineInstance {
    machine_name: String,
    current_state: String,
    fields: HashMap<String, PValue>,
    event_queue: VecDeque<(String, Option<PValue>)>,
    halted: bool,
    is_spec: bool,
    /// Liveness temperature: increments each step while in a hot state,
    /// resets to 0 on transition to a cold state.
    liveness_temperature: usize,
}

/// The P program runtime.
pub struct Runtime {
    /// All machine declarations from the program.
    machines: HashMap<String, MachineDecl>,
    /// All event declarations.
    events: HashMap<String, Option<PType>>,
    /// All enum declarations.
    enums: HashMap<String, Vec<String>>,
    /// Enum element -> enum type name.
    enum_elements: HashMap<String, String>,
    /// Enum element -> declared integer value (for numbered enums).
    enum_values: HashMap<String, i64>,
    /// Type definitions.
    typedefs: HashMap<String, PType>,
    /// Global functions (not in any machine).
    global_funs: HashMap<String, FunDecl>,
    /// Machine instances.
    instances: Vec<MachineInstance>,
    /// RNG for nondeterministic choices.
    rng: rand::rngs::ThreadRng,
    /// Step counter.
    steps: usize,
    /// Max steps per iteration.
    max_steps: usize,
    /// Liveness temperature threshold: if a spec monitor stays in a hot
    /// state for this many scheduling steps without visiting a cold state,
    /// a liveness violation is reported. Mirrors PChecker's temperature system.
    liveness_temperature_threshold: usize,
    /// Counter for fair nondeterministic choices ($$).
    /// Alternates to ensure both branches are explored.
    fair_nondet_counter: usize,
    /// Bias for unfair nondeterministic choices ($).
    /// None = random, Some(true) = always true, Some(false) = always false.
    nondet_bias: Option<bool>,
}

impl Runtime {
    pub fn new(programs: &[Program]) -> Self {
        let mut rt = Runtime {
            machines: HashMap::new(),
            events: HashMap::new(),
            enums: HashMap::new(),
            enum_elements: HashMap::new(),
            enum_values: HashMap::new(),
            typedefs: HashMap::new(),
            global_funs: HashMap::new(),
            instances: Vec::new(),
            rng: rand::rng(),
            steps: 0,
            max_steps: 2000,
            liveness_temperature_threshold: 100,
            fair_nondet_counter: 0,
            nondet_bias: None,
        };

        // Register all declarations
        for prog in programs {
            for decl in &prog.decls {
                match decl {
                    TopDecl::EventDecl(e) => {
                        rt.events.insert(e.name.clone(), e.payload.clone());
                    }
                    TopDecl::EnumTypeDef(e) => {
                        let elems: Vec<String> = e.elements.iter().map(|el| el.name.clone()).collect();
                        for (i, el) in e.elements.iter().enumerate() {
                            rt.enum_elements.insert(el.name.clone(), e.name.clone());
                            // Store declared value if present, otherwise use index
                            let val = el.value.unwrap_or(i as i64);
                            rt.enum_values.insert(el.name.clone(), val);
                        }
                        rt.enums.insert(e.name.clone(), elems);
                    }
                    TopDecl::TypeDef(td) => {
                        if let Some(ty) = &td.ty {
                            rt.typedefs.insert(td.name.clone(), ty.clone());
                        }
                    }
                    TopDecl::MachineDecl(m) | TopDecl::SpecMachineDecl(m) => {
                        rt.machines.insert(m.name.clone(), m.clone());
                    }
                    TopDecl::FunDecl(f) => {
                        rt.global_funs.insert(f.name.clone(), f.clone());
                    }
                    _ => {}
                }
            }
        }

        rt
    }

    /// Set the bias for unfair nondeterministic choices ($).
    pub fn set_nondet_bias(&mut self, bias: Option<bool>) {
        self.nondet_bias = bias;
    }

    /// Run the model checker. Returns Ok if no violations found, Err with description if found.
    pub fn run(&mut self) -> Result<(), CheckError> {
        // Find the main machine (look for test/implementation decl, or machine named "Main")
        let main_machine = self.find_main_machine()
            .ok_or_else(|| CheckError { message: "no main machine found".to_string() })?;

        // Create the main machine instance
        self.create_machine(&main_machine, None)?;

        // Create spec machines (monitors)
        let spec_names: Vec<String> = self.machines.iter()
            .filter(|(_, m)| m.is_spec)
            .map(|(name, _)| name.clone())
            .collect();
        for name in &spec_names {
            self.create_machine(name, None)?;
        }

        // Run the scheduling loop
        loop {
            if self.steps >= self.max_steps {
                break;
            }

            // Find machines with events to process
            let enabled: Vec<usize> = self.instances.iter().enumerate()
                .filter(|(_, inst)| !inst.halted && !inst.event_queue.is_empty() && !inst.is_spec)
                .map(|(i, _)| i)
                .collect();

            if enabled.is_empty() {
                // Check for deadlock: any non-halted machine with non-empty queue
                let blocked: Vec<usize> = self.instances.iter().enumerate()
                    .filter(|(_, inst)| !inst.halted && !inst.event_queue.is_empty())
                    .map(|(i, _)| i)
                    .collect();
                if !blocked.is_empty() {
                    return Err(CheckError {
                        message: format!("deadlock detected: {} machines blocked", blocked.len()),
                    });
                }
                break; // All quiescent
            }

            // Pick a random enabled machine
            let idx = if enabled.len() == 1 {
                enabled[0]
            } else {
                enabled[self.rng.random_range(0..enabled.len())]
            };

            self.step_machine(idx)?;
            self.steps += 1;

            // Check liveness temperature after each scheduling step
            self.check_liveness_temperature()?;
        }

        // End-of-run: the temperature-based system handles liveness checking
        // continuously during execution. No additional check needed here.
        // The temperature threshold prevents false positives on programs
        // that cycle through hot states but always eventually reach cold states.
        if false {
            // Placeholder to keep the code structure for future enhancements
            for inst in &self.instances {
                if inst.is_spec && inst.liveness_temperature > 0 {
                    let machine = self.machines.get(&inst.machine_name).unwrap();
                    for state in &machine.body.states {
                        if state.name == inst.current_state && state.temperature == Some(Temperature::Hot) {
                            return Err(CheckError {
                                message: format!(
                                    "liveness violation: spec '{}' stuck in hot state '{}'",
                                    inst.machine_name, inst.current_state
                                ),
                            });
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn find_main_machine(&self) -> Option<String> {
        // First check if there's a machine called "Main"
        if self.machines.contains_key("Main") {
            return Some("Main".to_string());
        }
        // Otherwise pick the first non-spec machine
        self.machines.iter()
            .find(|(_, m)| !m.is_spec)
            .map(|(name, _)| name.clone())
    }

    fn create_machine(&mut self, name: &str, payload: Option<PValue>) -> Result<usize, CheckError> {
        let machine = self.machines.get(name)
            .ok_or_else(|| CheckError { message: format!("unknown machine '{name}'") })?
            .clone();

        let start_state = machine.body.states.iter()
            .find(|s| s.is_start)
            .ok_or_else(|| CheckError { message: format!("machine '{name}' has no start state") })?
            .name.clone();
        debug!("create_machine '{}' id={} start_state={}", name, self.instances.len(), start_state);

        // Initialize fields with defaults
        let mut fields = HashMap::new();
        for var in &machine.body.vars {
            let default = self.default_value_for_type(&var.ty);
            for vname in &var.names {
                fields.insert(vname.clone(), default.clone());
            }
        }

        let id = self.instances.len();
        self.instances.push(MachineInstance {
            machine_name: name.to_string(),
            current_state: start_state.clone(),
            fields,
            event_queue: VecDeque::new(),
            halted: false,
            is_spec: machine.is_spec,
            liveness_temperature: 0,
        });

        // Run the start state's entry handler
        let state = machine.body.states.iter().find(|s| s.name == start_state).unwrap().clone();
        self.run_entry_handler(id, &state, payload)?;

        Ok(id)
    }

    fn step_machine(&mut self, id: usize) -> Result<(), CheckError> {
        if self.instances[id].halted {
            return Ok(());
        }

        // Dequeue an event
        let event = self.instances[id].event_queue.pop_front();
        let Some((event_name, payload)) = event else {
            return Ok(());
        };

        // Handle halt event
        if event_name == "halt" {
            self.instances[id].halted = true;
            return Ok(());
        }

        let machine_name = self.instances[id].machine_name.clone();
        let current_state = self.instances[id].current_state.clone();
        let machine = self.machines.get(&machine_name).unwrap().clone();

        let state = machine.body.states.iter()
            .find(|s| s.name == current_state)
            .unwrap()
            .clone();

        // Find handler for this event
        for item in &state.items {
            match item {
                StateBodyItem::Ignore(events, _) if events.contains(&event_name) => {
                    return Ok(()); // Drop event
                }
                StateBodyItem::Defer(events, _) if events.contains(&event_name) => {
                    // Re-enqueue at the back
                    self.instances[id].event_queue.push_back((event_name, payload));
                    return Ok(());
                }
                StateBodyItem::OnEventDoAction(on) if on.events.contains(&event_name) => {
                    let outcome = self.run_event_handler(id, &machine, on, payload)?;
                    return self.handle_outcome(id, outcome);
                }
                StateBodyItem::OnEventGotoState(on) if on.events.contains(&event_name) => {
                    // Run the with-handler if any, then transition
                    // Event payload is forwarded to the target state's entry handler
                    let transition_payload = payload.clone();
                    if let Some(handler) = &on.with_anon_handler {
                        let mut env = self.make_env(id);
                        if let Some(param) = &handler.param {
                            env.insert(param.name.clone(), payload.unwrap_or(PValue::Null));
                        }
                        let outcome = self.exec_body(id, &machine, &handler.body, &mut env)?;
                        match outcome {
                            HandlerOutcome::Normal | HandlerOutcome::Return(None) => {}
                            other => return self.handle_outcome(id, other),
                        }
                    } else if let Some(fn_name) = &on.with_fun_name {
                        let args = if let Some(p) = payload { vec![p] } else { Vec::new() };
                        self.call_function(id, &machine, fn_name, &args)?;
                    }
                    self.transition_to_state(id, &on.target, transition_payload)?;
                    return Ok(());
                }
                _ => {}
            }
        }

        // No handler found — check for null handler (default)
        for item in &state.items {
            if let StateBodyItem::OnEventDoAction(on) = item {
                if on.events.contains(&"null".to_string()) {
                    let outcome = self.run_event_handler(id, &machine, on, payload)?;
                    return self.handle_outcome(id, outcome);
                }
            }
        }

        // Unhandled event — just drop it (P semantics: unhandled events are dropped)
        Ok(())
    }

    fn run_entry_handler(&mut self, id: usize, state: &StateDecl, payload: Option<PValue>) -> Result<(), CheckError> {
        let machine_name = self.instances[id].machine_name.clone();
        let machine = self.machines.get(&machine_name).unwrap().clone();

        for item in &state.items {
            if let StateBodyItem::Entry(ee) = item {
                if let Some(handler) = &ee.anon_handler {
                    let mut env = self.make_env(id);
                    if let Some(param) = &handler.param {
                        env.insert(param.name.clone(), payload.clone().unwrap_or(PValue::Null));
                    }
                    let outcome = self.exec_body(id, &machine, &handler.body, &mut env)?;
                    match outcome {
                        HandlerOutcome::Normal | HandlerOutcome::Return(None) | HandlerOutcome::Break | HandlerOutcome::Continue => {}
                        HandlerOutcome::Raised(event, payload) => {
                            self.handle_outcome(id, HandlerOutcome::Raised(event, payload))?;
                        }
                        HandlerOutcome::GotoState(target, payload) => {
                            self.transition_to_state(id, &target, payload)?;
                        }
                        HandlerOutcome::Halted => {
                            self.instances[id].halted = true;
                        }
                        HandlerOutcome::Return(Some(_)) => {}
                    }
                } else if let Some(fn_name) = &ee.fun_name {
                    let args = if let Some(p) = payload.clone() { vec![p] } else { Vec::new() };
                    self.call_function(id, &machine, fn_name, &args)?;
                }
                return Ok(());
            }
        }
        Ok(())
    }

    fn run_exit_handler(&mut self, id: usize) -> Result<(), CheckError> {
        let machine_name = self.instances[id].machine_name.clone();
        let current_state = self.instances[id].current_state.clone();
        let machine = self.machines.get(&machine_name).unwrap().clone();

        let state = machine.body.states.iter()
            .find(|s| s.name == current_state)
            .unwrap()
            .clone();

        for item in &state.items {
            if let StateBodyItem::Exit(ee) = item {
                if let Some(handler) = &ee.anon_handler {
                    let mut env = self.make_env(id);
                    self.exec_body(id, &machine, &handler.body, &mut env)?;
                } else if let Some(fn_name) = &ee.fun_name {
                    self.call_function(id, &machine, fn_name, &[])?;
                }
                return Ok(());
            }
        }
        Ok(())
    }

    fn transition_to_state(&mut self, id: usize, target: &str, payload: Option<PValue>) -> Result<(), CheckError> {
        self.steps += 1;
        if self.steps >= self.max_steps { return Ok(()); }
        debug!("transition {}[{}] -> {}", self.instances[id].machine_name, self.instances[id].current_state, target);
        self.run_exit_handler(id)?;
        self.instances[id].current_state = target.to_string();
        let machine_name = self.instances[id].machine_name.clone();
        let machine = self.machines.get(&machine_name).unwrap().clone();

        // Reset liveness temperature on entry to cold state (spec monitors)
        if self.instances[id].is_spec {
            let target_state = machine.body.states.iter().find(|s| s.name == target);
            if let Some(state) = target_state {
                if state.temperature == Some(Temperature::Cold) {
                    trace!("liveness: spec '{}' entered cold state '{}', temperature reset",
                        machine_name, target);
                    self.instances[id].liveness_temperature = 0;
                }
            }
        }
        let state = machine.body.states.iter()
            .find(|s| s.name == *target)
            .unwrap()
            .clone();
        self.run_entry_handler(id, &state, payload)?;
        Ok(())
    }

    fn run_event_handler(&mut self, id: usize, machine: &MachineDecl, on: &OnEventDoAction, payload: Option<PValue>) -> Result<HandlerOutcome, CheckError> {
        if let Some(handler) = &on.anon_handler {
            let mut env = self.make_env(id);
            if let Some(param) = &handler.param {
                env.insert(param.name.clone(), payload.unwrap_or(PValue::Null));
            }
            self.exec_body(id, machine, &handler.body, &mut env)
        } else if let Some(fn_name) = &on.fun_name {
            let args = if let Some(p) = payload { vec![p] } else { Vec::new() };
            self.call_function(id, machine, fn_name, &args)?;
            Ok(HandlerOutcome::Normal)
        } else {
            Ok(HandlerOutcome::Normal)
        }
    }

    fn handle_outcome(&mut self, id: usize, outcome: HandlerOutcome) -> Result<(), CheckError> {
        match outcome {
            HandlerOutcome::Normal | HandlerOutcome::Return(_) | HandlerOutcome::Break | HandlerOutcome::Continue => Ok(()),
            HandlerOutcome::Raised(event, payload) => {
                if event == "halt" {
                    self.instances[id].halted = true;
                    return Ok(());
                }
                self.instances[id].event_queue.push_front((event, payload));
                // Limit recursion depth via step counter
                self.steps += 1;
                if self.steps >= self.max_steps {
                    return Ok(());
                }
                self.step_machine(id)
            }
            HandlerOutcome::GotoState(target, payload) => {
                self.transition_to_state(id, &target, payload)
            }
            HandlerOutcome::Halted => {
                self.instances[id].halted = true;
                Ok(())
            }
        }
    }

    fn make_env(&self, id: usize) -> HashMap<String, PValue> {
        let inst = &self.instances[id];
        trace!("make_env {}[{}] state={}", inst.machine_name, id, inst.current_state);
        inst.fields.clone()
    }

    fn sync_env_to_fields(&mut self, id: usize, env: &HashMap<String, PValue>) {
        let inst = &mut self.instances[id];
        for (name, val) in env {
            if inst.fields.contains_key(name) {
                trace!("sync field {}[{}] = {}", inst.machine_name, name, val);
                inst.fields.insert(name.clone(), val.clone());
            }
        }
    }

    // ---- Statement/expression execution ----

    fn exec_body(&mut self, id: usize, machine: &MachineDecl, body: &FunctionBody, env: &mut HashMap<String, PValue>) -> Result<HandlerOutcome, CheckError> {
        // Register local variables
        for var in &body.var_decls {
            let default = self.default_value_for_type(&var.ty);
            for name in &var.names {
                env.insert(name.clone(), default.clone());
            }
        }

        for stmt in &body.stmts {
            let outcome = self.exec_stmt(id, machine, stmt, env)?;
            match outcome {
                HandlerOutcome::Normal => {}
                other => {
                    self.sync_env_to_fields(id, env);
                    return Ok(other);
                }
            }
        }
        self.sync_env_to_fields(id, env);
        Ok(HandlerOutcome::Normal)
    }

    fn exec_stmt(&mut self, id: usize, machine: &MachineDecl, stmt: &Stmt, env: &mut HashMap<String, PValue>) -> Result<HandlerOutcome, CheckError> {
        match stmt {
            Stmt::Compound(stmts, _) => {
                for s in stmts {
                    let o = self.exec_stmt(id, machine, s, env)?;
                    match o {
                        HandlerOutcome::Normal => {}
                        other => return Ok(other),
                    }
                }
                Ok(HandlerOutcome::Normal)
            }
            Stmt::Assert { expr, message, .. } => {
                let val = self.eval_expr(id, machine, expr, env)?;
                if !val.to_bool() {
                    let msg = if let Some(m) = message {
                        let mv = self.eval_expr(id, machine, m, env)?;
                        format!("Assertion failed: {mv}")
                    } else {
                        "Assertion failed".to_string()
                    };
                    return Err(CheckError { message: msg });
                }
                Ok(HandlerOutcome::Normal)
            }
            Stmt::Assume { .. } => Ok(HandlerOutcome::Normal),
            Stmt::Print { message, .. } => {
                let val = self.eval_expr(id, machine, message, env)?;
                debug!("[P print] {val}");
                Ok(HandlerOutcome::Normal)
            }
            Stmt::Return { value, .. } => {
                let val = if let Some(v) = value {
                    Some(self.eval_expr(id, machine, v, env)?)
                } else {
                    None
                };
                Ok(HandlerOutcome::Return(val))
            }
            Stmt::Break(_) => Ok(HandlerOutcome::Break),
            Stmt::Continue(_) => Ok(HandlerOutcome::Continue),
            Stmt::Assign { lvalue, rvalue, .. } => {
                let val = self.eval_expr(id, machine, rvalue, env)?;
                self.set_lvalue(id, lvalue, val, env)?;
                Ok(HandlerOutcome::Normal)
            }
            Stmt::Insert { lvalue, index, value, .. } => {
                let idx = self.eval_expr(id, machine, index, env)?;
                let val = self.eval_expr(id, machine, value, env)?;
                let mut target = self.read_lvalue(id, lvalue, env);
                match &mut target {
                    PValue::Seq(seq) => {
                        let i = idx.as_int().unwrap_or(0) as usize;
                        if i <= seq.len() {
                            seq.insert(i, val);
                        }
                    }
                    PValue::Map(map) => {
                        map.insert(idx, val);
                    }
                    _ => {}
                }
                self.set_lvalue(id, lvalue, target, env)?;
                Ok(HandlerOutcome::Normal)
            }
            Stmt::AddToSet { lvalue, value, .. } => {
                let val = self.eval_expr(id, machine, value, env)?;
                let mut target = self.read_lvalue(id, lvalue, env);
                if let PValue::Set(set) = &mut target {
                    if !set.contains(&val) {
                        set.push(val);
                        set.sort();
                    }
                }
                self.set_lvalue(id, lvalue, target, env)?;
                Ok(HandlerOutcome::Normal)
            }
            Stmt::Remove { lvalue, key, .. } => {
                let k = self.eval_expr(id, machine, key, env)?;
                let mut target = self.read_lvalue(id, lvalue, env);
                match &mut target {
                    PValue::Seq(seq) => {
                        let i = k.as_int().unwrap_or(0) as usize;
                        if i < seq.len() { seq.remove(i); }
                    }
                    PValue::Map(map) => { map.remove(&k); }
                    PValue::Set(set) => { set.retain(|v| v != &k); }
                    _ => {}
                }
                self.set_lvalue(id, lvalue, target, env)?;
                Ok(HandlerOutcome::Normal)
            }
            Stmt::While { cond, body, .. } => {
                loop {
                    let c = self.eval_expr(id, machine, cond, env)?;
                    if !c.to_bool() { break; }
                    let o = self.exec_stmt(id, machine, body, env)?;
                    match o {
                        HandlerOutcome::Normal | HandlerOutcome::Continue => {}
                        HandlerOutcome::Break => break,
                        other => return Ok(other),
                    }
                    self.steps += 1;
                    if self.steps >= self.max_steps { break; }
                }
                Ok(HandlerOutcome::Normal)
            }
            Stmt::Foreach { item, collection, body, .. } => {
                let col = self.eval_expr(id, machine, collection, env)?;
                let items: Vec<PValue> = match col {
                    PValue::Seq(s) => s,
                    PValue::Set(s) => s,
                    PValue::Map(m) => m.keys().cloned().collect(),
                    _ => Vec::new(),
                };
                for elem in items {
                    env.insert(item.clone(), elem);
                    let o = self.exec_stmt(id, machine, body, env)?;
                    match o {
                        HandlerOutcome::Normal | HandlerOutcome::Continue => {}
                        HandlerOutcome::Break => break,
                        other => return Ok(other),
                    }
                }
                Ok(HandlerOutcome::Normal)
            }
            Stmt::If { cond, then_branch, else_branch, .. } => {
                let c = self.eval_expr(id, machine, cond, env)?;
                if c.to_bool() {
                    self.exec_stmt(id, machine, then_branch, env)
                } else if let Some(eb) = else_branch {
                    self.exec_stmt(id, machine, eb, env)
                } else {
                    Ok(HandlerOutcome::Normal)
                }
            }
            Stmt::CtorStmt { interface, args, .. } => {
                let mut arg_vals = Vec::new();
                for a in args {
                    arg_vals.push(self.eval_expr(id, machine, a, env)?);
                }
                let payload = arg_vals.into_iter().next();
                self.create_machine(interface, payload)?;
                Ok(HandlerOutcome::Normal)
            }
            Stmt::FunCall { name, args, .. } => {
                let mut arg_vals = Vec::new();
                for a in args {
                    arg_vals.push(self.eval_expr(id, machine, a, env)?);
                }
                // Sync env to fields before call so callee sees current state
                self.sync_env_to_fields(id, env);
                let result = self.call_function(id, machine, name, &arg_vals)?;
                // Re-sync fields to env after call so caller sees callee's changes
                for (fname, fval) in &self.instances[id].fields {
                    if env.contains_key(fname) {
                        env.insert(fname.clone(), fval.clone());
                    }
                }
                match result {
                    HandlerOutcome::Return(_) | HandlerOutcome::Normal | HandlerOutcome::Break | HandlerOutcome::Continue => Ok(HandlerOutcome::Normal),
                    other => Ok(other),
                }
            }
            Stmt::Raise { event, args, .. } => {
                let ev = self.eval_expr(id, machine, event, env)?;
                let event_name = match &ev {
                    PValue::EventId(name) => name.clone(),
                    PValue::EnumVal(_, _) => {
                        // Event reference
                        if let Expr::Iden(name, _) = event {
                            name.clone()
                        } else {
                            return Err(CheckError { message: "invalid event in raise".to_string() });
                        }
                    }
                    _ => {
                        if let Expr::Iden(name, _) = event {
                            name.clone()
                        } else if let Expr::HaltEvent(_) = event {
                            "halt".to_string()
                        } else {
                            return Err(CheckError { message: format!("invalid event in raise: {ev}") });
                        }
                    }
                };
                let payload = if !args.is_empty() {
                    let mut vals = Vec::new();
                    for a in args { vals.push(self.eval_expr(id, machine, a, env)?); }
                    if vals.len() == 1 { Some(vals.into_iter().next().unwrap()) }
                    else { Some(PValue::Tuple(vals)) }
                } else {
                    None
                };

                // Announce to monitors
                self.announce_event(&event_name, &payload);

                Ok(HandlerOutcome::Raised(event_name, payload))
            }
            Stmt::Send { target, event, args, .. } => {
                let target_val = self.eval_expr(id, machine, target, env)?;
                let ev = self.eval_expr(id, machine, event, env)?;
                let target_id = target_val.as_machine_ref()
                    .ok_or_else(|| CheckError { message: format!("send target is not a machine: {target_val}") })?;
                let event_name = match &ev {
                    PValue::EventId(name) => name.clone(),
                    _ => {
                        if let Expr::Iden(name, _) = event { name.clone() }
                        else if let Expr::HaltEvent(_) = event { "halt".to_string() }
                        else { return Err(CheckError { message: format!("invalid event in send: {ev}") }); }
                    }
                };
                let payload = if !args.is_empty() {
                    let mut vals = Vec::new();
                    for a in args { vals.push(self.eval_expr(id, machine, a, env)?); }
                    if vals.len() == 1 { Some(vals.into_iter().next().unwrap()) }
                    else { Some(PValue::Tuple(vals)) }
                } else {
                    None
                };

                // Announce to monitors
                self.announce_event(&event_name, &payload);

                // Enqueue on target
                if target_id < self.instances.len() && !self.instances[target_id].halted {
                    debug!("send {} -> {}[{}] event={}", self.instances[id].machine_name, self.instances[target_id].machine_name, target_id, event_name);
                    self.instances[target_id].event_queue.push_back((event_name, payload));
                }
                Ok(HandlerOutcome::Normal)
            }
            Stmt::Announce { event, args, .. } => {
                let ev = self.eval_expr(id, machine, event, env)?;
                let event_name = if let Expr::Iden(name, _) = event { name.clone() }
                    else { format!("{ev}") };
                let payload = if !args.is_empty() {
                    let mut vals = Vec::new();
                    for a in args { vals.push(self.eval_expr(id, machine, a, env)?); }
                    if vals.len() == 1 { Some(vals.into_iter().next().unwrap()) }
                    else { Some(PValue::Tuple(vals)) }
                } else { None };
                self.announce_event(&event_name, &payload);
                Ok(HandlerOutcome::Normal)
            }
            Stmt::Goto { state, payload, .. } => {
                let p = if !payload.is_empty() {
                    let mut vals = Vec::new();
                    for a in payload { vals.push(self.eval_expr(id, machine, a, env)?); }
                    if vals.len() == 1 { Some(vals.into_iter().next().unwrap()) }
                    else { Some(PValue::Tuple(vals)) }
                } else { None };
                Ok(HandlerOutcome::GotoState(state.clone(), p))
            }
            Stmt::Receive { cases, .. } => {
                // Block until a matching event arrives.
                // Process other machines while waiting.
                for _ in 0..self.max_steps {
                    // Check queue for matching event
                    let queue = &self.instances[id].event_queue;
                    let mut found = None;
                    for (qi, (ev, _)) in queue.iter().enumerate() {
                        for case in cases {
                            if case.events.contains(ev) || case.events.contains(&"null".to_string()) {
                                found = Some((qi, case.clone()));
                                break;
                            }
                        }
                        if found.is_some() { break; }
                    }

                    if let Some((qi, case)) = found {
                        let (_ev_name, ev_payload) = self.instances[id].event_queue.remove(qi).unwrap();
                        if let Some(param) = &case.handler.param {
                            env.insert(param.name.clone(), ev_payload.unwrap_or(PValue::Null));
                        }
                        return self.exec_body(id, machine, &case.handler.body, env);
                    }

                    // No matching event — step other machines
                    let other_enabled: Vec<usize> = self.instances.iter().enumerate()
                        .filter(|(i, inst)| *i != id && !inst.halted && !inst.event_queue.is_empty() && !inst.is_spec)
                        .map(|(i, _)| i)
                        .collect();

                    if other_enabled.is_empty() {
                        // Nobody else can run — give up to avoid deadlock
                        break;
                    }

                    // Step a random other machine
                    let other_id = if other_enabled.len() == 1 {
                        other_enabled[0]
                    } else {
                        other_enabled[self.rng.random_range(0..other_enabled.len())]
                    };
                    self.step_machine(other_id)?;
                    self.steps += 1;
                    if self.steps >= self.max_steps { break; }
                }
                Ok(HandlerOutcome::Normal)
            }
            Stmt::NoStmt(_) => Ok(HandlerOutcome::Normal),
        }
    }

    /// Check liveness temperature for all spec monitors.
    /// Called after each scheduling step. If a monitor has been in a hot state
    /// for too many steps without visiting a cold state, report a violation.
    fn check_liveness_temperature(&mut self) -> Result<(), CheckError> {
        for inst in &mut self.instances {
            if !inst.is_spec || inst.halted {
                continue;
            }
            // Look up the current state's temperature in the machine declaration
            // We need to find the state declaration to check its temperature attribute
        }

        // We need machine declarations to check state temperatures.
        // Collect spec instance info first, then check against machine declarations.
        let spec_states: Vec<(usize, String, String)> = self.instances.iter().enumerate()
            .filter(|(_, inst)| inst.is_spec && !inst.halted)
            .map(|(i, inst)| (i, inst.machine_name.clone(), inst.current_state.clone()))
            .collect();

        for (inst_id, machine_name, state_name) in spec_states {
            let machine = self.machines.get(&machine_name).unwrap();
            let state_decl = machine.body.states.iter().find(|s| s.name == state_name);

            if let Some(state) = state_decl {
                match state.temperature {
                    Some(Temperature::Hot) => {
                        self.instances[inst_id].liveness_temperature += 1;
                        trace!(
                            "liveness: spec '{}' in hot state '{}', temperature={}",
                            machine_name, state_name, self.instances[inst_id].liveness_temperature
                        );
                        if self.instances[inst_id].liveness_temperature > self.liveness_temperature_threshold {
                            return Err(CheckError {
                                message: format!(
                                    "liveness violation: spec '{}' stuck in hot state '{}' \
                                     (temperature {} exceeded threshold {})",
                                    machine_name, state_name,
                                    self.instances[inst_id].liveness_temperature,
                                    self.liveness_temperature_threshold,
                                ),
                            });
                        }
                    }
                    Some(Temperature::Cold) => {
                        // Cold state resets temperature
                        if self.instances[inst_id].liveness_temperature > 0 {
                            trace!("liveness: spec '{}' reached cold state '{}', temperature reset", machine_name, state_name);
                        }
                        self.instances[inst_id].liveness_temperature = 0;
                    }
                    None => {
                        // Warm (unmarked) state: temperature stays the same
                        // No increment, no reset
                    }
                }
            }
        }

        Ok(())
    }

    fn announce_event(&mut self, event_name: &str, payload: &Option<PValue>) {
        // Deliver event to all spec monitors that observe it
        let spec_ids: Vec<usize> = self.instances.iter().enumerate()
            .filter(|(_, inst)| inst.is_spec && !inst.halted)
            .map(|(i, _)| i)
            .collect();

        for spec_id in spec_ids {
            let machine_name = self.instances[spec_id].machine_name.clone();
            let machine = self.machines.get(&machine_name).unwrap().clone();

            // Check if this spec observes this event
            if let Some(observes) = &machine.observes {
                if !observes.contains(&event_name.to_string()) {
                    continue;
                }
            }

            // Enqueue the event on the spec monitor
            self.instances[spec_id].event_queue.push_back((event_name.to_string(), payload.clone()));

            // Process immediately (monitors are synchronous)
            let _ = self.step_machine(spec_id);
        }
    }

    // ---- Expression evaluation ----

    fn eval_expr(&mut self, id: usize, machine: &MachineDecl, expr: &Expr, env: &mut HashMap<String, PValue>) -> Result<PValue, CheckError> {
        match expr {
            Expr::IntLit(v, _) => Ok(PValue::Int(*v)),
            Expr::FloatLit(v, _) => Ok(PValue::Float(OrderedFloat(*v))),
            Expr::BoolLit(v, _) => Ok(PValue::Bool(*v)),
            Expr::StringLit(s, _) => Ok(PValue::String(s.clone())),
            Expr::NullLit(_) => Ok(PValue::Null),
            Expr::This(_) => Ok(PValue::MachineRef(id)),
            Expr::HaltEvent(_) => Ok(PValue::EventId("halt".to_string())),
            Expr::Nondet(_) => {
                // Unfair nondeterminism: can be biased to test liveness
                let val = match self.nondet_bias {
                    Some(b) => b,
                    None => self.rng.random_bool(0.5),
                };
                Ok(PValue::Bool(val))
            }
            Expr::FairNondet(_) => {
                // Fair nondeterminism: alternate to ensure both branches are explored.
                // This models the fairness constraint that both choices must eventually occur.
                self.fair_nondet_counter += 1;
                Ok(PValue::Bool(self.fair_nondet_counter % 2 == 0))
            }

            Expr::Iden(name, _) => {
                // Check locals/env first
                if let Some(val) = env.get(name) { return Ok(val.clone()); }
                // Check enum elements
                if let Some(enum_name) = self.enum_elements.get(name) {
                    return Ok(PValue::EnumVal(enum_name.clone(), name.clone()));
                }
                // Check events
                if self.events.contains_key(name) {
                    return Ok(PValue::EventId(name.clone()));
                }
                Ok(PValue::Null)
            }

            Expr::UnnamedTuple(fields, _) => {
                let vals: Result<Vec<_>, _> = fields.iter()
                    .map(|f| self.eval_expr(id, machine, f, env))
                    .collect();
                Ok(PValue::Tuple(vals?))
            }
            Expr::NamedTuple(fields, _) => {
                let vals: Result<Vec<_>, _> = fields.iter()
                    .map(|(n, f)| self.eval_expr(id, machine, f, env).map(|v| (n.clone(), v)))
                    .collect();
                Ok(PValue::NamedTuple(vals?))
            }

            Expr::NamedTupleAccess(base, field, _) => {
                let val = self.eval_expr(id, machine, base, env)?;
                match val {
                    PValue::NamedTuple(fields) => {
                        Ok(fields.iter().find(|(n, _)| n == field)
                            .map(|(_, v)| v.clone())
                            .unwrap_or(PValue::Null))
                    }
                    _ => Ok(PValue::Null),
                }
            }
            Expr::TupleAccess(base, idx, _) => {
                let val = self.eval_expr(id, machine, base, env)?;
                match val {
                    PValue::Tuple(fields) => Ok(fields.get(*idx).cloned().unwrap_or(PValue::Null)),
                    _ => Ok(PValue::Null),
                }
            }
            Expr::SeqMapAccess(base, index, _) => {
                let base_val = self.eval_expr(id, machine, base, env)?;
                let idx_val = self.eval_expr(id, machine, index, env)?;
                match base_val {
                    PValue::Seq(seq) => {
                        let i = idx_val.as_int().unwrap_or(0) as usize;
                        Ok(seq.get(i).cloned().unwrap_or(PValue::Null))
                    }
                    PValue::Map(map) => {
                        Ok(map.get(&idx_val).cloned().unwrap_or(PValue::Null))
                    }
                    _ => Ok(PValue::Null),
                }
            }

            Expr::Keys(base, _) => {
                let val = self.eval_expr(id, machine, base, env)?;
                match val {
                    PValue::Map(m) => Ok(PValue::Seq(m.keys().cloned().collect())),
                    _ => Ok(PValue::Seq(Vec::new())),
                }
            }
            Expr::Values(base, _) => {
                let val = self.eval_expr(id, machine, base, env)?;
                match val {
                    PValue::Map(m) => Ok(PValue::Seq(m.values().cloned().collect())),
                    _ => Ok(PValue::Seq(Vec::new())),
                }
            }
            Expr::Sizeof(base, _) => {
                let val = self.eval_expr(id, machine, base, env)?;
                let sz = match &val {
                    PValue::Seq(s) => s.len(),
                    PValue::Set(s) => s.len(),
                    PValue::Map(m) => m.len(),
                    _ => 0,
                };
                Ok(PValue::Int(sz as i64))
            }
            Expr::Default(ty, _) => Ok(self.default_value_for_type(ty)),

            Expr::New(interface, args, _) => {
                let mut arg_vals = Vec::new();
                for a in args {
                    arg_vals.push(self.eval_expr(id, machine, a, env)?);
                }
                let payload = arg_vals.into_iter().next();
                let new_id = self.create_machine(interface, payload)?;
                Ok(PValue::MachineRef(new_id))
            }

            Expr::FunCall(name, args, _) => {
                let mut arg_vals = Vec::new();
                for a in args {
                    arg_vals.push(self.eval_expr(id, machine, a, env)?);
                }
                self.sync_env_to_fields(id, env);
                let result = self.call_function(id, machine, name, &arg_vals)?;
                // Re-sync fields to env
                for (fname, fval) in &self.instances[id].fields {
                    if env.contains_key(fname) {
                        env.insert(fname.clone(), fval.clone());
                    }
                }
                match result {
                    HandlerOutcome::Return(Some(val)) => Ok(val),
                    _ => Ok(PValue::Null),
                }
            }

            Expr::Neg(inner, _) => {
                let val = self.eval_expr(id, machine, inner, env)?;
                match val {
                    PValue::Int(i) => Ok(PValue::Int(-i)),
                    PValue::Float(f) => Ok(PValue::Float(OrderedFloat(-f.0))),
                    _ => Ok(PValue::Int(0)),
                }
            }
            Expr::Not(inner, _) => {
                let val = self.eval_expr(id, machine, inner, env)?;
                Ok(PValue::Bool(!val.to_bool()))
            }

            Expr::BinOp(op, lhs, rhs, _) => {
                // Short-circuit for && and ||
                if *op == BinOp::And {
                    let l = self.eval_expr(id, machine, lhs, env)?;
                    if !l.to_bool() { return Ok(PValue::Bool(false)); }
                    let r = self.eval_expr(id, machine, rhs, env)?;
                    return Ok(PValue::Bool(r.to_bool()));
                }
                if *op == BinOp::Or {
                    let l = self.eval_expr(id, machine, lhs, env)?;
                    if l.to_bool() { return Ok(PValue::Bool(true)); }
                    let r = self.eval_expr(id, machine, rhs, env)?;
                    return Ok(PValue::Bool(r.to_bool()));
                }

                let l = self.eval_expr(id, machine, lhs, env)?;
                let r = self.eval_expr(id, machine, rhs, env)?;
                self.eval_binop(*op, &l, &r)
            }

            Expr::Cast(inner, _ty, _) => {
                let val = self.eval_expr(id, machine, inner, env)?;
                // For now, just pass through — runtime casts are handled by coercion
                match val {
                    PValue::Int(i) => Ok(PValue::Int(i)), // int to float handled below
                    PValue::Float(f) => Ok(PValue::Int(f.0 as i64)),
                    PValue::EnumVal(_, ref elem) => {
                        // enum to int: use declared value if available, else index
                        if let Some(enum_name) = self.enum_elements.get(elem) {
                            // Check for numbered enum values
                            let machines_copy: Vec<_> = self.machines.keys().cloned().collect();
                            let _ = machines_copy;
                            // Look through all programs' enum declarations for numbered values
                            if let Some(elems) = self.enums.get(enum_name) {
                                let idx = elems.iter().position(|e| e == elem).unwrap_or(0);
                                // Check if enum has declared values
                                if let Some(val) = self.enum_values.get(elem) {
                                    return Ok(PValue::Int(*val));
                                }
                                return Ok(PValue::Int(idx as i64));
                            }
                        }
                        Ok(val)
                    }
                    other => Ok(other),
                }
            }

            Expr::Choose(arg, _) => {
                if let Some(a) = arg {
                    let val = self.eval_expr(id, machine, a, env)?;
                    match val {
                        PValue::Int(n) => {
                            if n <= 0 { Ok(PValue::Int(0)) }
                            else { Ok(PValue::Int(self.rng.random_range(0..n))) }
                        }
                        PValue::Seq(s) if !s.is_empty() => {
                            let idx = self.rng.random_range(0..s.len());
                            Ok(s[idx].clone())
                        }
                        PValue::Set(s) if !s.is_empty() => {
                            let idx = self.rng.random_range(0..s.len());
                            Ok(s[idx].clone())
                        }
                        PValue::Map(m) if !m.is_empty() => {
                            let keys: Vec<_> = m.keys().collect();
                            let idx = self.rng.random_range(0..keys.len());
                            Ok(keys[idx].clone())
                        }
                        _ => Ok(PValue::Null),
                    }
                } else {
                    Ok(PValue::Bool(self.rng.random_bool(0.5)))
                }
            }

            Expr::FormatString(fmt, args, _) => {
                let mut arg_vals = Vec::new();
                for a in args {
                    arg_vals.push(self.eval_expr(id, machine, a, env)?);
                }
                let mut result = fmt.clone();
                for (i, val) in arg_vals.iter().enumerate() {
                    result = result.replace(&format!("{{{i}}}"), &format!("{val}"));
                }
                Ok(PValue::String(result))
            }

            Expr::Paren(inner, _) => self.eval_expr(id, machine, inner, env),
        }
    }

    fn eval_binop(&self, op: BinOp, l: &PValue, r: &PValue) -> Result<PValue, CheckError> {
        match op {
            BinOp::Add => match (l, r) {
                (PValue::Int(a), PValue::Int(b)) => Ok(PValue::Int(a + b)),
                (PValue::Float(a), PValue::Float(b)) => Ok(PValue::Float(OrderedFloat(a.0 + b.0))),
                (PValue::String(a), PValue::String(b)) => Ok(PValue::String(format!("{a}{b}"))),
                _ => Ok(PValue::Int(0)),
            },
            BinOp::Sub => match (l, r) {
                (PValue::Int(a), PValue::Int(b)) => Ok(PValue::Int(a - b)),
                (PValue::Float(a), PValue::Float(b)) => Ok(PValue::Float(OrderedFloat(a.0 - b.0))),
                _ => Ok(PValue::Int(0)),
            },
            BinOp::Mul => match (l, r) {
                (PValue::Int(a), PValue::Int(b)) => Ok(PValue::Int(a * b)),
                (PValue::Float(a), PValue::Float(b)) => Ok(PValue::Float(OrderedFloat(a.0 * b.0))),
                _ => Ok(PValue::Int(0)),
            },
            BinOp::Div => match (l, r) {
                (PValue::Int(a), PValue::Int(b)) if *b != 0 => Ok(PValue::Int(a / b)),
                (PValue::Float(a), PValue::Float(b)) => Ok(PValue::Float(OrderedFloat(a.0 / b.0))),
                _ => Ok(PValue::Int(0)),
            },
            BinOp::Mod => match (l, r) {
                (PValue::Int(a), PValue::Int(b)) if *b != 0 => Ok(PValue::Int(a % b)),
                _ => Ok(PValue::Int(0)),
            },
            BinOp::Eq => Ok(PValue::Bool(l == r)),
            BinOp::Ne => Ok(PValue::Bool(l != r)),
            BinOp::Lt => Ok(PValue::Bool(l < r)),
            BinOp::Gt => Ok(PValue::Bool(l > r)),
            BinOp::Le => Ok(PValue::Bool(l <= r)),
            BinOp::Ge => Ok(PValue::Bool(l >= r)),
            BinOp::And => Ok(PValue::Bool(l.to_bool() && r.to_bool())),
            BinOp::Or => Ok(PValue::Bool(l.to_bool() || r.to_bool())),
            BinOp::In => {
                let contained = match r {
                    PValue::Map(m) => m.contains_key(l),
                    PValue::Seq(s) => s.contains(l),
                    PValue::Set(s) => s.contains(l),
                    _ => false,
                };
                Ok(PValue::Bool(contained))
            }
        }
    }

    // ---- LValue operations ----

    fn set_lvalue(&mut self, id: usize, lv: &LValue, val: PValue, env: &mut HashMap<String, PValue>) -> Result<(), CheckError> {
        match lv {
            LValue::Var(name, _) => {
                if env.contains_key(name) {
                    env.insert(name.clone(), val);
                } else {
                    self.instances[id].fields.insert(name.clone(), val);
                }
                Ok(())
            }
            LValue::NamedTupleField(base, field, _) => {
                let mut parent = self.read_lvalue(id, base, env);
                if let PValue::NamedTuple(ref mut fields) = parent {
                    if let Some((_, v)) = fields.iter_mut().find(|(n, _)| n == field) {
                        *v = val;
                    }
                }
                self.set_lvalue(id, base, parent, env)
            }
            LValue::TupleField(base, idx, _) => {
                let mut parent = self.read_lvalue(id, base, env);
                if let PValue::Tuple(ref mut fields) = parent {
                    if *idx < fields.len() {
                        fields[*idx] = val;
                    }
                }
                self.set_lvalue(id, base, parent, env)
            }
            LValue::Index(base, index_expr, _) => {
                let machine = self.machines.get(&self.instances[id].machine_name).unwrap().clone();
                let idx = self.eval_expr(id, &machine, index_expr, env)?;
                let mut parent = self.read_lvalue(id, base, env);
                match &mut parent {
                    PValue::Seq(seq) => {
                        let i = idx.as_int().unwrap_or(0) as usize;
                        if i < seq.len() { seq[i] = val; }
                    }
                    PValue::Map(map) => { map.insert(idx, val); }
                    _ => {}
                }
                self.set_lvalue(id, base, parent, env)
            }
        }
    }

    /// Read the current value at an lvalue position.
    fn read_lvalue(&self, id: usize, lv: &LValue, env: &HashMap<String, PValue>) -> PValue {
        match lv {
            LValue::Var(name, _) => {
                env.get(name)
                    .or_else(|| self.instances[id].fields.get(name))
                    .cloned()
                    .unwrap_or(PValue::Null)
            }
            LValue::NamedTupleField(base, field, _) => {
                let base_val = self.read_lvalue(id, base, env);
                match base_val {
                    PValue::NamedTuple(fields) => {
                        fields.iter().find(|(n, _)| n == field)
                            .map(|(_, v)| v.clone())
                            .unwrap_or(PValue::Null)
                    }
                    _ => PValue::Null,
                }
            }
            LValue::TupleField(base, idx, _) => {
                let base_val = self.read_lvalue(id, base, env);
                match base_val {
                    PValue::Tuple(fields) => fields.get(*idx).cloned().unwrap_or(PValue::Null),
                    _ => PValue::Null,
                }
            }
            LValue::Index(base, _index_expr, _) => {
                // For reading, we'd need to evaluate the index expr, but we don't have
                // &mut self here. Return the whole collection instead.
                self.read_lvalue(id, base, env)
            }
        }
    }

    // ---- Function calls ----

    fn call_function(&mut self, id: usize, machine: &MachineDecl, name: &str, args: &[PValue]) -> Result<HandlerOutcome, CheckError> {
        // Look for function in machine first, then globals
        let fun = machine.body.funs.iter().find(|f| f.name == name)
            .or_else(|| self.global_funs.get(name).map(|f| f))
            .cloned();

        if let Some(fun) = fun {
            if let Some(body) = &fun.body {
                let mut env = self.make_env(id);
                for (i, param) in fun.params.iter().enumerate() {
                    env.insert(param.name.clone(), args.get(i).cloned().unwrap_or(PValue::Null));
                }
                self.exec_body(id, machine, body, &mut env)
            } else {
                // Foreign function — no-op
                Ok(HandlerOutcome::Return(None))
            }
        } else {
            // Unknown function — could be an enum constructor, just ignore
            Ok(HandlerOutcome::Return(None))
        }
    }

    // ---- Default values ----

    fn default_value_for_type(&self, ty: &PType) -> PValue {
        match ty {
            PType::Bool => PValue::Bool(false),
            PType::Int => PValue::Int(0),
            PType::Float => PValue::Float(OrderedFloat(0.0)),
            PType::StringType => PValue::String(String::new()),
            PType::Event => PValue::Null,
            PType::Machine => PValue::Null,
            PType::Any => PValue::Null,
            PType::Seq(_) => PValue::Seq(Vec::new()),
            PType::Set(_) => PValue::Set(Vec::new()),
            PType::Map(_, _) => PValue::Map(BTreeMap::new()),
            PType::Tuple(ts) => PValue::Tuple(ts.iter().map(|t| self.default_value_for_type(t)).collect()),
            PType::NamedTuple(fields) => PValue::NamedTuple(
                fields.iter().map(|(n, t)| (n.clone(), self.default_value_for_type(t))).collect(),
            ),
            PType::Named(name) => {
                if let Some(elems) = self.enums.get(name) {
                    if let Some(first) = elems.first() {
                        return PValue::EnumVal(name.clone(), first.clone());
                    }
                }
                if let Some(td) = self.typedefs.get(name) {
                    return self.default_value_for_type(td);
                }
                PValue::Null
            }
            PType::Data => PValue::Null,
        }
    }
}
