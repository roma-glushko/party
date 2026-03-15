//! P language formatter. Parses and re-emits .p files with consistent style.

use super::ast::*;

pub fn format_program(program: &Program) -> String {
    let mut f = Formatter::new();
    f.fmt_program(program);
    f.output
}

struct Formatter {
    output: String,
    indent: usize,
}

impl Formatter {
    fn new() -> Self {
        Self {
            output: String::new(),
            indent: 0,
        }
    }

    fn push(&mut self, s: &str) {
        self.output.push_str(s);
    }

    fn newline(&mut self) {
        self.output.push('\n');
        for _ in 0..self.indent {
            self.output.push_str("    ");
        }
    }

    fn blank_line(&mut self) {
        // Ensure we have exactly one blank line (two newlines total)
        // Trim trailing spaces/indentation from current line
        while self.output.ends_with(' ') {
            self.output.pop();
        }
        if !self.output.ends_with("\n\n") {
            if !self.output.ends_with('\n') {
                self.output.push('\n');
            }
            self.output.push('\n');
        }
    }

    // ---- Program ----

    fn fmt_program(&mut self, prog: &Program) {
        let mut prev_kind = DeclKind::None;
        for decl in &prog.decls {
            let kind = decl_kind(decl);
            // Blank line between different declaration kinds
            if prev_kind != DeclKind::None && (kind != prev_kind || matches!(kind, DeclKind::Machine | DeclKind::Spec)) {
                self.blank_line();
            }
            self.fmt_top_decl(decl);
            prev_kind = kind;
        }
        // Ensure trailing newline
        if !self.output.ends_with('\n') {
            self.output.push('\n');
        }
    }

    fn fmt_top_decl(&mut self, decl: &TopDecl) {
        match decl {
            TopDecl::TypeDef(td) => self.fmt_typedef(td),
            TopDecl::EnumTypeDef(e) => self.fmt_enum(e),
            TopDecl::EventDecl(e) => self.fmt_event(e),
            TopDecl::EventSetDecl(es) => self.fmt_eventset(es),
            TopDecl::InterfaceDecl(i) => self.fmt_interface(i),
            TopDecl::MachineDecl(m) | TopDecl::SpecMachineDecl(m) => self.fmt_machine(m),
            TopDecl::FunDecl(f) => self.fmt_fun(f),
            TopDecl::ModuleDecl(m) => self.fmt_module(m),
            TopDecl::TestDecl(t) => self.fmt_test(t),
            TopDecl::ImplementationDecl(i) => self.fmt_implementation(i),
            TopDecl::GlobalParamDecl(g) => self.fmt_global_param(g),
        }
    }

    // ---- Types ----

    fn fmt_type(&mut self, ty: &PType) {
        match ty {
            PType::Bool => self.push("bool"),
            PType::Int => self.push("int"),
            PType::Float => self.push("float"),
            PType::StringType => self.push("string"),
            PType::Event => self.push("event"),
            PType::Machine => self.push("machine"),
            PType::Data => self.push("data"),
            PType::Any => self.push("any"),
            PType::Seq(inner) => {
                self.push("seq[");
                self.fmt_type(inner);
                self.push("]");
            }
            PType::Set(inner) => {
                self.push("set[");
                self.fmt_type(inner);
                self.push("]");
            }
            PType::Map(k, v) => {
                self.push("map[");
                self.fmt_type(k);
                self.push(", ");
                self.fmt_type(v);
                self.push("]");
            }
            PType::Tuple(ts) => {
                self.push("(");
                for (i, t) in ts.iter().enumerate() {
                    if i > 0 { self.push(", "); }
                    self.fmt_type(t);
                }
                self.push(")");
            }
            PType::NamedTuple(fields) => {
                self.push("(");
                for (i, (name, ty)) in fields.iter().enumerate() {
                    if i > 0 { self.push(", "); }
                    self.push(name);
                    self.push(": ");
                    self.fmt_type(ty);
                }
                self.push(")");
            }
            PType::Named(name) => self.push(name),
        }
    }

    // ---- Declarations ----

    fn fmt_typedef(&mut self, td: &TypeDefDecl) {
        self.push("type ");
        self.push(&td.name);
        if let Some(ty) = &td.ty {
            self.push(" = ");
            self.fmt_type(ty);
        }
        self.push(";");
        self.newline();
    }

    fn fmt_enum(&mut self, e: &EnumTypeDefDecl) {
        self.push("enum ");
        self.push(&e.name);
        self.push(" { ");
        for (i, elem) in e.elements.iter().enumerate() {
            if i > 0 { self.push(", "); }
            self.push(&elem.name);
            if let Some(val) = elem.value {
                self.push(&format!(" = {val}"));
            }
        }
        self.push(" }");
        self.newline();
    }

    fn fmt_event(&mut self, e: &EventDecl) {
        self.push("event ");
        self.push(&e.name);
        if let Some(payload) = &e.payload {
            self.push(": ");
            self.fmt_type(payload);
        }
        self.push(";");
        self.newline();
    }

    fn fmt_eventset(&mut self, es: &EventSetDecl) {
        self.push("eventset ");
        self.push(&es.name);
        self.push(" = { ");
        self.push(&es.events.join(", "));
        self.push(" };");
        self.newline();
    }

    fn fmt_interface(&mut self, i: &InterfaceDecl) {
        self.push("interface ");
        self.push(&i.name);
        self.push("(");
        if let Some(payload) = &i.payload {
            self.fmt_type(payload);
        }
        self.push(")");
        if let Some(receives) = &i.receives {
            if !receives.is_empty() {
                self.push(" receives ");
                self.push(&receives.join(", "));
            }
        }
        self.push(";");
        self.newline();
    }

    fn fmt_global_param(&mut self, g: &GlobalParamDecl) {
        self.push("param ");
        self.push(&g.names.join(", "));
        self.push(": ");
        self.fmt_type(&g.ty);
        self.push(";");
        self.newline();
    }

    // ---- Machine ----

    fn fmt_machine(&mut self, m: &MachineDecl) {
        if m.is_spec {
            self.push("spec ");
            self.push(&m.name);
            self.push(" observes ");
            if let Some(obs) = &m.observes {
                self.push(&obs.join(", "));
            }
        } else {
            self.push("machine ");
            self.push(&m.name);
        }

        if let Some(receives) = &m.receives {
            if !receives.is_empty() {
                self.push(" receives ");
                self.push(&receives.join(", "));
                self.push(";");
            }
        }
        if let Some(sends) = &m.sends {
            if !sends.is_empty() {
                self.push(" sends ");
                self.push(&sends.join(", "));
                self.push(";");
            }
        }

        self.push(" {");
        self.indent += 1;

        // Variables
        for var in &m.body.vars {
            self.newline();
            self.fmt_var_decl(var);
        }

        // States
        for (i, state) in m.body.states.iter().enumerate() {
            if i > 0 || !m.body.vars.is_empty() {
                self.blank_line();
            }
            // Use indent without extra newline after blank_line
            for _ in 0..self.indent { self.output.push_str("    "); }
            self.fmt_state(state);
            self.output.push('\n');
        }

        // Functions
        for (i, fun) in m.body.funs.iter().enumerate() {
            if i > 0 || !m.body.states.is_empty() || !m.body.vars.is_empty() {
                self.blank_line();
            }
            for _ in 0..self.indent { self.output.push_str("    "); }
            self.fmt_fun(fun);
        }

        self.indent -= 1;
        // Close brace — trim trailing blank lines
        while self.output.ends_with("\n\n") {
            self.output.pop();
        }
        if !self.output.ends_with('\n') {
            self.output.push('\n');
        }
        self.push("}\n");
    }

    fn fmt_var_decl(&mut self, v: &VarDecl) {
        self.push("var ");
        self.push(&v.names.join(", "));
        self.push(": ");
        self.fmt_type(&v.ty);
        self.push(";");
    }

    // ---- State ----

    fn fmt_state(&mut self, s: &StateDecl) {
        if s.is_start { self.push("start "); }
        if let Some(Temperature::Hot) = s.temperature { self.push("hot "); }
        if let Some(Temperature::Cold) = s.temperature { self.push("cold "); }
        self.push("state ");
        self.push(&s.name);
        self.push(" {");
        self.indent += 1;

        for item in &s.items {
            self.newline();
            self.fmt_state_item(item);
        }

        self.indent -= 1;
        self.newline();
        self.push("}");
    }

    fn fmt_state_item(&mut self, item: &StateBodyItem) {
        match item {
            StateBodyItem::Entry(ee) => {
                self.push("entry");
                if let Some(fun_name) = &ee.fun_name {
                    self.push(" ");
                    self.push(fun_name);
                    self.push(";");
                } else if let Some(handler) = &ee.anon_handler {
                    if let Some(param) = &handler.param {
                        self.push(" (");
                        self.push(&param.name);
                        self.push(": ");
                        self.fmt_type(&param.ty);
                        self.push(")");
                    }
                    self.push(" ");
                    self.fmt_function_body(&handler.body);
                }
            }
            StateBodyItem::Exit(ee) => {
                self.push("exit");
                if let Some(fun_name) = &ee.fun_name {
                    self.push(" ");
                    self.push(fun_name);
                    self.push(";");
                } else if let Some(handler) = &ee.anon_handler {
                    self.push(" ");
                    self.fmt_function_body(&handler.body);
                }
            }
            StateBodyItem::Defer(events, _) => {
                self.push("defer ");
                self.push(&events.join(", "));
                self.push(";");
            }
            StateBodyItem::Ignore(events, _) => {
                self.push("ignore ");
                self.push(&events.join(", "));
                self.push(";");
            }
            StateBodyItem::OnEventDoAction(on) => {
                self.push("on ");
                self.push(&on.events.join(", "));
                self.push(" do");
                if let Some(fun_name) = &on.fun_name {
                    self.push(" ");
                    self.push(fun_name);
                    self.push(";");
                } else if let Some(handler) = &on.anon_handler {
                    if let Some(param) = &handler.param {
                        self.push(" (");
                        self.push(&param.name);
                        self.push(": ");
                        self.fmt_type(&param.ty);
                        self.push(")");
                    }
                    self.push(" ");
                    self.fmt_function_body(&handler.body);
                }
            }
            StateBodyItem::OnEventGotoState(on) => {
                self.push("on ");
                self.push(&on.events.join(", "));
                self.push(" goto ");
                self.push(&on.target);
                if let Some(fun_name) = &on.with_fun_name {
                    self.push(" with ");
                    self.push(fun_name);
                }
                if let Some(handler) = &on.with_anon_handler {
                    self.push(" with");
                    if let Some(param) = &handler.param {
                        self.push(" (");
                        self.push(&param.name);
                        self.push(": ");
                        self.fmt_type(&param.ty);
                        self.push(")");
                    }
                    self.push(" ");
                    self.fmt_function_body(&handler.body);
                } else {
                    self.push(";");
                }
            }
        }
    }

    // ---- Function ----

    fn fmt_fun(&mut self, f: &FunDecl) {
        self.push("fun ");
        self.push(&f.name);
        self.push("(");
        for (i, p) in f.params.iter().enumerate() {
            if i > 0 { self.push(", "); }
            self.push(&p.name);
            self.push(": ");
            self.fmt_type(&p.ty);
        }
        self.push(")");
        if let Some(ret) = &f.ret_type {
            self.push(": ");
            self.fmt_type(ret);
        }
        if let Some(body) = &f.body {
            self.push(" ");
            self.fmt_function_body(body);
        } else {
            self.push(";");
        }
        self.newline();
    }

    fn fmt_function_body(&mut self, body: &FunctionBody) {
        self.push("{");
        self.indent += 1;
        for var in &body.var_decls {
            self.newline();
            self.fmt_var_decl(var);
        }
        for stmt in &body.stmts {
            self.newline();
            self.fmt_stmt(stmt);
        }
        self.indent -= 1;
        self.newline();
        self.push("}");
    }

    // ---- Statements ----

    fn fmt_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Compound(stmts, _) => {
                self.push("{");
                self.indent += 1;
                for s in stmts {
                    self.newline();
                    self.fmt_stmt(s);
                }
                self.indent -= 1;
                self.newline();
                self.push("}");
            }
            Stmt::Assert { expr, message, .. } => {
                self.push("assert ");
                self.fmt_expr(expr);
                if let Some(msg) = message {
                    self.push(", ");
                    self.fmt_expr(msg);
                }
                self.push(";");
            }
            Stmt::Assume { expr, message, .. } => {
                self.push("assume ");
                self.fmt_expr(expr);
                if let Some(msg) = message {
                    self.push(", ");
                    self.fmt_expr(msg);
                }
                self.push(";");
            }
            Stmt::Print { message, .. } => {
                self.push("print ");
                self.fmt_expr(message);
                self.push(";");
            }
            Stmt::Return { value, .. } => {
                self.push("return");
                if let Some(v) = value {
                    self.push(" ");
                    self.fmt_expr(v);
                }
                self.push(";");
            }
            Stmt::Break(_) => self.push("break;"),
            Stmt::Continue(_) => self.push("continue;"),
            Stmt::Assign { lvalue, rvalue, .. } => {
                self.fmt_lvalue(lvalue);
                self.push(" = ");
                self.fmt_expr(rvalue);
                self.push(";");
            }
            Stmt::Insert { lvalue, index, value, .. } => {
                self.fmt_lvalue(lvalue);
                self.push(" += (");
                self.fmt_expr(index);
                self.push(", ");
                self.fmt_expr(value);
                self.push(");");
            }
            Stmt::AddToSet { lvalue, value, .. } => {
                self.fmt_lvalue(lvalue);
                self.push(" += (");
                self.fmt_expr(value);
                self.push(");");
            }
            Stmt::Remove { lvalue, key, .. } => {
                self.fmt_lvalue(lvalue);
                self.push(" -= (");
                self.fmt_expr(key);
                self.push(");");
            }
            Stmt::While { cond, body, .. } => {
                self.push("while (");
                self.fmt_expr(cond);
                self.push(") ");
                self.fmt_stmt(body);
            }
            Stmt::Foreach { item, collection, body, .. } => {
                self.push("foreach (");
                self.push(item);
                self.push(" in ");
                self.fmt_expr(collection);
                self.push(") ");
                self.fmt_stmt(body);
            }
            Stmt::If { cond, then_branch, else_branch, .. } => {
                self.push("if (");
                self.fmt_expr(cond);
                self.push(") ");
                self.fmt_stmt(then_branch);
                if let Some(eb) = else_branch {
                    self.push(" else ");
                    self.fmt_stmt(eb);
                }
            }
            Stmt::CtorStmt { interface, args, .. } => {
                self.push("new ");
                self.push(interface);
                self.push("(");
                self.fmt_expr_list(args);
                self.push(");");
            }
            Stmt::FunCall { name, args, .. } => {
                self.push(name);
                self.push("(");
                self.fmt_expr_list(args);
                self.push(");");
            }
            Stmt::Raise { event, args, .. } => {
                self.push("raise ");
                self.fmt_expr(event);
                if !args.is_empty() {
                    self.push(", ");
                    self.fmt_expr_list(args);
                }
                self.push(";");
            }
            Stmt::Send { target, event, args, .. } => {
                self.push("send ");
                self.fmt_expr(target);
                self.push(", ");
                self.fmt_expr(event);
                if !args.is_empty() {
                    self.push(", ");
                    self.fmt_expr_list(args);
                }
                self.push(";");
            }
            Stmt::Announce { event, args, .. } => {
                self.push("announce ");
                self.fmt_expr(event);
                if !args.is_empty() {
                    self.push(", ");
                    self.fmt_expr_list(args);
                }
                self.push(";");
            }
            Stmt::Goto { state, payload, .. } => {
                self.push("goto ");
                self.push(state);
                if !payload.is_empty() {
                    self.push(", ");
                    self.fmt_expr_list(payload);
                }
                self.push(";");
            }
            Stmt::Receive { cases, .. } => {
                self.push("receive {");
                self.indent += 1;
                for case in cases {
                    self.newline();
                    self.push("case ");
                    self.push(&case.events.join(", "));
                    self.push(":");
                    if let Some(param) = &case.handler.param {
                        self.push(" (");
                        self.push(&param.name);
                        self.push(": ");
                        self.fmt_type(&param.ty);
                        self.push(")");
                    }
                    self.push(" ");
                    self.fmt_function_body(&case.handler.body);
                }
                self.indent -= 1;
                self.newline();
                self.push("}");
            }
            Stmt::NoStmt(_) => {}
        }
    }

    // ---- Expressions ----

    fn fmt_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::IntLit(v, _) => self.push(&v.to_string()),
            Expr::FloatLit(v, _) => {
                let s = format!("{v}");
                if s.contains('.') {
                    self.push(&s);
                } else {
                    let s2 = format!("{v}.0");
                    self.push(&s2);
                }
            }
            Expr::BoolLit(v, _) => self.push(if *v { "true" } else { "false" }),
            Expr::StringLit(s, _) => {
                self.push("\"");
                self.push(s);
                self.push("\"");
            }
            Expr::NullLit(_) => self.push("null"),
            Expr::This(_) => self.push("this"),
            Expr::HaltEvent(_) => self.push("halt"),
            Expr::Nondet(_) => self.push("$"),
            Expr::FairNondet(_) => self.push("$$"),
            Expr::Iden(name, _) => self.push(name),
            Expr::UnnamedTuple(fields, _) => {
                self.push("(");
                for (i, f) in fields.iter().enumerate() {
                    if i > 0 { self.push(", "); }
                    self.fmt_expr(f);
                }
                if fields.len() == 1 { self.push(","); }
                self.push(")");
            }
            Expr::NamedTuple(fields, _) => {
                self.push("(");
                for (i, (name, val)) in fields.iter().enumerate() {
                    if i > 0 { self.push(", "); }
                    self.push(name);
                    self.push(" = ");
                    self.fmt_expr(val);
                }
                self.push(")");
            }
            Expr::NamedTupleAccess(base, field, _) => {
                self.fmt_expr(base);
                self.push(".");
                self.push(field);
            }
            Expr::TupleAccess(base, idx, _) => {
                self.fmt_expr(base);
                self.push(&format!(".{idx}"));
            }
            Expr::SeqMapAccess(base, index, _) => {
                self.fmt_expr(base);
                self.push("[");
                self.fmt_expr(index);
                self.push("]");
            }
            Expr::Keys(inner, _) => {
                self.push("keys(");
                self.fmt_expr(inner);
                self.push(")");
            }
            Expr::Values(inner, _) => {
                self.push("values(");
                self.fmt_expr(inner);
                self.push(")");
            }
            Expr::Sizeof(inner, _) => {
                self.push("sizeof(");
                self.fmt_expr(inner);
                self.push(")");
            }
            Expr::Default(ty, _) => {
                self.push("default(");
                self.fmt_type(ty);
                self.push(")");
            }
            Expr::New(name, args, _) => {
                self.push("new ");
                self.push(name);
                self.push("(");
                self.fmt_expr_list(args);
                self.push(")");
            }
            Expr::FunCall(name, args, _) => {
                self.push(name);
                self.push("(");
                self.fmt_expr_list(args);
                self.push(")");
            }
            Expr::Neg(inner, _) => {
                self.push("-");
                self.fmt_expr(inner);
            }
            Expr::Not(inner, _) => {
                self.push("!");
                self.fmt_expr(inner);
            }
            Expr::BinOp(op, lhs, rhs, _) => {
                self.fmt_expr(lhs);
                self.push(match op {
                    BinOp::Add => " + ",
                    BinOp::Sub => " - ",
                    BinOp::Mul => " * ",
                    BinOp::Div => " / ",
                    BinOp::Mod => " % ",
                    BinOp::Eq => " == ",
                    BinOp::Ne => " != ",
                    BinOp::Lt => " < ",
                    BinOp::Gt => " > ",
                    BinOp::Le => " <= ",
                    BinOp::Ge => " >= ",
                    BinOp::And => " && ",
                    BinOp::Or => " || ",
                    BinOp::In => " in ",
                });
                self.fmt_expr(rhs);
            }
            Expr::Cast(inner, ty, _) => {
                self.fmt_expr(inner);
                self.push(" as ");
                self.fmt_type(ty);
            }
            Expr::Choose(arg, _) => {
                self.push("choose(");
                if let Some(a) = arg {
                    self.fmt_expr(a);
                }
                self.push(")");
            }
            Expr::FormatString(fmt, args, _) => {
                self.push("format(\"");
                self.push(fmt);
                self.push("\"");
                for a in args {
                    self.push(", ");
                    self.fmt_expr(a);
                }
                self.push(")");
            }
            Expr::Paren(inner, _) => {
                self.push("(");
                self.fmt_expr(inner);
                self.push(")");
            }
        }
    }

    fn fmt_expr_list(&mut self, exprs: &[Expr]) {
        for (i, e) in exprs.iter().enumerate() {
            if i > 0 { self.push(", "); }
            self.fmt_expr(e);
        }
    }

    fn fmt_lvalue(&mut self, lv: &LValue) {
        match lv {
            LValue::Var(name, _) => self.push(name),
            LValue::NamedTupleField(base, field, _) => {
                self.fmt_lvalue(base);
                self.push(".");
                self.push(field);
            }
            LValue::TupleField(base, idx, _) => {
                self.fmt_lvalue(base);
                self.push(&format!(".{idx}"));
            }
            LValue::Index(base, index, _) => {
                self.fmt_lvalue(base);
                self.push("[");
                self.fmt_expr(index);
                self.push("]");
            }
        }
    }

    // ---- Module system ----

    fn fmt_mod_expr(&mut self, expr: &ModExpr) {
        match expr {
            ModExpr::Paren(inner) => {
                self.push("(");
                self.fmt_mod_expr(inner);
                self.push(")");
            }
            ModExpr::Primitive(binds) => {
                self.push("{ ");
                for (i, b) in binds.iter().enumerate() {
                    if i > 0 { self.push(", "); }
                    self.push(&b.machine);
                    if let Some(iface) = &b.interface {
                        self.push(" -> ");
                        self.push(iface);
                    }
                }
                self.push(" }");
            }
            ModExpr::Named(name) => self.push(name),
            ModExpr::Compose(exprs) => {
                self.push("compose ");
                for (i, e) in exprs.iter().enumerate() {
                    if i > 0 { self.push(", "); }
                    self.fmt_mod_expr(e);
                }
            }
            ModExpr::Union(exprs) => {
                self.push("union ");
                for (i, e) in exprs.iter().enumerate() {
                    if i > 0 { self.push(", "); }
                    self.fmt_mod_expr(e);
                }
            }
            ModExpr::HideEvents(events, inner) => {
                self.push("hidee ");
                self.push(&events.join(", "));
                self.push(" in ");
                self.fmt_mod_expr(inner);
            }
            ModExpr::HideInterfaces(ifaces, inner) => {
                self.push("hidei ");
                self.push(&ifaces.join(", "));
                self.push(" in ");
                self.fmt_mod_expr(inner);
            }
            ModExpr::AssertMod(monitors, inner) => {
                self.push("assert ");
                self.push(&monitors.join(", "));
                self.push(" in ");
                self.fmt_mod_expr(inner);
            }
            ModExpr::Rename(old, new, inner) => {
                self.push("rename ");
                self.push(old);
                self.push(" to ");
                self.push(new);
                self.push(" in ");
                self.fmt_mod_expr(inner);
            }
            ModExpr::MainMachine(machine, inner) => {
                self.push("main ");
                self.push(machine);
                self.push(" in ");
                self.fmt_mod_expr(inner);
            }
        }
    }

    fn fmt_module(&mut self, m: &ModuleDecl) {
        self.push("module ");
        self.push(&m.name);
        self.push(" = ");
        self.fmt_mod_expr(&m.expr);
        self.push(";");
        self.newline();
    }

    fn fmt_test(&mut self, t: &TestDecl) {
        self.push("test ");
        self.push(&t.name);
        if let Some(main) = &t.main_machine {
            self.push(" [main = ");
            self.push(main);
            self.push("]");
        }
        self.push(": ");
        self.fmt_mod_expr(&t.module_expr);
        self.push(";");
        self.newline();
    }

    fn fmt_implementation(&mut self, i: &ImplementationDecl) {
        self.push("implementation ");
        self.push(&i.name);
        if let Some(main) = &i.main_machine {
            self.push(" [main = ");
            self.push(main);
            self.push("]");
        }
        self.push(": ");
        self.fmt_mod_expr(&i.module_expr);
        self.push(";");
        self.newline();
    }
}

// ---- Helpers ----

#[derive(PartialEq, Clone, Copy)]
enum DeclKind {
    None,
    Type,
    Event,
    Machine,
    Spec,
    Fun,
    Module,
    Other,
}

fn decl_kind(decl: &TopDecl) -> DeclKind {
    match decl {
        TopDecl::TypeDef(_) | TopDecl::EnumTypeDef(_) => DeclKind::Type,
        TopDecl::EventDecl(_) | TopDecl::EventSetDecl(_) => DeclKind::Event,
        TopDecl::InterfaceDecl(_) => DeclKind::Other,
        TopDecl::MachineDecl(_) => DeclKind::Machine,
        TopDecl::SpecMachineDecl(_) => DeclKind::Spec,
        TopDecl::FunDecl(_) => DeclKind::Fun,
        TopDecl::ModuleDecl(_) | TopDecl::TestDecl(_) | TopDecl::ImplementationDecl(_) => DeclKind::Module,
        TopDecl::GlobalParamDecl(_) => DeclKind::Other,
    }
}
