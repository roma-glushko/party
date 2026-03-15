//! P language formatter. Parses and re-emits .p files with consistent style.
//!
//! Style conventions:
//! - 2-space indentation
//! - Space before colon in type annotations: `x : int`
//! - Opening brace on same line, content on next line
//! - Blank line before event handlers (on/defer/ignore) in states
//! - Short bodies on one line: `on E do { }`, `entry { x = 0; }`
//! - Trailing comma in single-field named tuples: `(s = null,)`
//! - Comments are preserved from the original source

use super::ast::*;
use super::token::Span;

pub fn format_program(program: &Program) -> String {
    format_program_with_source(program, "")
}

pub fn format_program_with_source(program: &Program, source: &str) -> String {
    let comments = extract_comments(source);
    let mut f = Formatter::new(comments, source);
    f.fmt_program(program);
    f.output
}

const INDENT: &str = "  ";

// ---- Comment extraction ----

#[derive(Debug, Clone)]
struct Comment {
    text: String,
    start: usize,   // byte offset
    end: usize,     // byte offset
    inline: bool,   // true if code precedes this comment on the same line
}

/// Extract all comments from source with their byte offsets.
fn extract_comments(source: &str) -> Vec<Comment> {
    let mut comments = Vec::new();
    let bytes = source.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        if i + 1 < len && bytes[i] == b'/' && bytes[i + 1] == b'/' {
            // Line comment
            let start = i;
            while i < len && bytes[i] != b'\n' {
                i += 1;
            }
            let text = source[start..i].trim_end().to_string();
            let inline = is_inline_comment(source, start);
            comments.push(Comment { text, start, end: i, inline });
        } else if i + 1 < len && bytes[i] == b'/' && bytes[i + 1] == b'*' {
            // Block comment
            let start = i;
            i += 2;
            while i + 1 < len && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                i += 1;
            }
            if i + 1 < len { i += 2; } // skip */
            let text = source[start..i].to_string();
            let inline = is_inline_comment(source, start);
            comments.push(Comment { text, start, end: i, inline });
        } else if bytes[i] == b'"' {
            // Skip string literals
            i += 1;
            while i < len && bytes[i] != b'"' {
                if bytes[i] == b'\\' { i += 1; }
                i += 1;
            }
            if i < len { i += 1; }
        } else {
            i += 1;
        }
    }

    comments
}

/// Check if a comment at `offset` is inline (code precedes it on the same line).
fn is_inline_comment(source: &str, offset: usize) -> bool {
    // Walk backwards from offset to start of line
    let bytes = source.as_bytes();
    let mut j = offset;
    while j > 0 && bytes[j - 1] != b'\n' {
        j -= 1;
    }
    // Check if there's any non-whitespace between line start and comment
    source[j..offset].chars().any(|c| !c.is_whitespace())
}

// ---- Formatter ----

struct Formatter {
    output: String,
    indent: usize,
    comments: Vec<Comment>,
    comment_cursor: usize,  // index of next comment to consider
    source: String,
}

impl Formatter {
    fn new(comments: Vec<Comment>, source: &str) -> Self {
        Self {
            output: String::new(),
            indent: 0,
            comments,
            comment_cursor: 0,
            source: source.to_string(),
        }
    }

    fn push(&mut self, s: &str) {
        self.output.push_str(s);
    }

    fn newline(&mut self) {
        self.output.push('\n');
        for _ in 0..self.indent {
            self.push(INDENT);
        }
    }

    fn blank_line(&mut self) {
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

    /// Emit all standalone (non-inline) comments whose start offset is before `before_offset`.
    fn emit_leading_comments(&mut self, before_offset: usize) {
        while self.comment_cursor < self.comments.len() {
            if self.comments[self.comment_cursor].start >= before_offset { break; }
            if !self.comments[self.comment_cursor].inline {
                let text = self.comments[self.comment_cursor].text.clone();
                self.push(&text);
                self.newline();
            }
            self.comment_cursor += 1;
        }
    }

    /// Emit inline comments whose start offset is after `after_offset`.
    /// Only advances the cursor past inline comments; leaves standalone comments
    /// for `emit_leading_comments` to pick up later.
    fn emit_inline_comment_after(&mut self, after_offset: usize) {
        while self.comment_cursor < self.comments.len() {
            let c = &self.comments[self.comment_cursor];
            if c.start < after_offset { self.comment_cursor += 1; continue; }
            if !c.inline { break; }
            let text = c.text.clone();
            while self.output.ends_with(' ') { self.output.pop(); }
            self.push("  ");
            self.push(&text);
            self.comment_cursor += 1;
        }
    }

    /// Emit any remaining comments at the end of the file.
    fn emit_trailing_comments(&mut self) {
        while self.comment_cursor < self.comments.len() {
            let text = self.comments[self.comment_cursor].text.clone();
            let inline = self.comments[self.comment_cursor].inline;
            if inline {
                self.push("  ");
            }
            self.push(&text);
            self.comment_cursor += 1;
            if self.comment_cursor < self.comments.len() {
                self.newline();
            }
        }
    }

    /// Check if a function body is "short" enough to render on one line.
    fn is_short_body(body: &FunctionBody) -> bool {
        if !body.var_decls.is_empty() { return false; }
        if body.stmts.is_empty() { return true; }
        if body.stmts.len() > 1 { return false; }
        match &body.stmts[0] {
            Stmt::NoStmt(_) | Stmt::Break(_) | Stmt::Continue(_) => true,
            Stmt::Return { value: None, .. } => true,
            Stmt::Assign { .. } => true,
            _ => false,
        }
    }

    // ---- Program ----

    fn fmt_program(&mut self, prog: &Program) {
        let mut prev_kind = DeclKind::None;
        let mut prev_end: usize = 0;
        for decl in &prog.decls {
            let kind = decl_kind(decl);
            let span = decl_span(decl);

            if prev_kind != DeclKind::None
                && (kind != prev_kind
                    || matches!(kind, DeclKind::Machine | DeclKind::Spec))
            {
                self.blank_line();
            }

            // Emit leading comments before this declaration
            self.emit_leading_comments(span.start);

            self.fmt_top_decl(decl);

            // Emit inline comments after the declaration
            self.emit_inline_comment_after(span.end);
            prev_kind = kind;
        }

        // Emit any remaining comments at end of file
        if !self.output.ends_with('\n') {
            self.output.push('\n');
        }
        self.emit_trailing_comments();
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
            self.emit_leading_comments(var.span.start);
            self.fmt_var_decl(var);
        }

        // States
        for (i, state) in m.body.states.iter().enumerate() {
            if i > 0 || !m.body.vars.is_empty() {
                self.blank_line();
            } else {
                self.output.push('\n');
            }
            for _ in 0..self.indent { self.push(INDENT); }
            self.emit_leading_comments(state.span.start);
            self.fmt_state(state);
            self.output.push('\n');
        }

        // Functions
        for (i, fun) in m.body.funs.iter().enumerate() {
            if i > 0 || !m.body.states.is_empty() || !m.body.vars.is_empty() {
                self.blank_line();
            }
            for _ in 0..self.indent { self.push(INDENT); }
            self.emit_leading_comments(fun.span.start);
            self.fmt_fun(fun);
        }

        self.indent -= 1;
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

        let mut had_entry_exit = false;
        for item in &s.items {
            let item_span = state_item_span(item);
            let is_handler = matches!(item,
                StateBodyItem::OnEventDoAction(_) | StateBodyItem::OnEventGotoState(_)
                | StateBodyItem::Defer(_, _) | StateBodyItem::Ignore(_, _));

            if is_handler && had_entry_exit {
                self.blank_line();
                for _ in 0..self.indent { self.push(INDENT); }
                self.emit_leading_comments(item_span.start);
                self.fmt_state_item(item);
                had_entry_exit = false;
            } else {
                self.newline();
                self.emit_leading_comments(item_span.start);
                self.fmt_state_item(item);
            }

            if matches!(item, StateBodyItem::Entry(_) | StateBodyItem::Exit(_)) {
                had_entry_exit = true;
            }
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
                    self.fmt_handler_params_and_body(&handler.param, &handler.body);
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
                    self.fmt_body_inline_or_block(&handler.body);
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
                    self.fmt_handler_params_and_body(&handler.param, &handler.body);
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
                    self.push(";");
                } else if let Some(handler) = &on.with_anon_handler {
                    self.push(" with");
                    self.fmt_handler_params_and_body(&handler.param, &handler.body);
                } else {
                    self.push(";");
                }
            }
        }
    }

    fn fmt_handler_params_and_body(&mut self, param: &Option<FunParam>, body: &FunctionBody) {
        if let Some(p) = param {
            self.push(" (");
            self.push(&p.name);
            self.push(": ");
            self.fmt_type(&p.ty);
            self.push(")");
        }
        self.push(" ");
        self.fmt_body_inline_or_block(body);
    }

    fn fmt_body_inline_or_block(&mut self, body: &FunctionBody) {
        if Self::is_short_body(body) {
            self.push("{ ");
            for stmt in &body.stmts {
                self.fmt_stmt(stmt);
                self.push(" ");
            }
            self.push("}");
        } else {
            self.fmt_function_body(body);
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
            self.emit_leading_comments(stmt_span(stmt).start);
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
                    self.emit_leading_comments(stmt_span(s).start);
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
                // Unwrap Paren to avoid double-wrapping: -= ((x)) → -= (x)
                let inner = if let Expr::Paren(inner, _) = key { inner } else { key };
                self.fmt_expr(inner);
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
                    self.push(" :");
                    self.fmt_handler_params_and_body(&case.handler.param, &case.handler.body);
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
                if s.contains('.') { self.push(&s); }
                else { let s2 = format!("{v}.0"); self.push(&s2); }
            }
            Expr::BoolLit(v, _) => self.push(if *v { "true" } else { "false" }),
            Expr::StringLit(s, _) => { self.push("\""); self.push(s); self.push("\""); }
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
            Expr::Keys(inner, _) => { self.push("keys("); self.fmt_expr(inner); self.push(")"); }
            Expr::Values(inner, _) => { self.push("values("); self.fmt_expr(inner); self.push(")"); }
            Expr::Sizeof(inner, _) => { self.push("sizeof("); self.fmt_expr(inner); self.push(")"); }
            Expr::Default(ty, _) => { self.push("default("); self.fmt_type(ty); self.push(")"); }
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
            Expr::Neg(inner, _) => { self.push("-"); self.fmt_expr(inner); }
            Expr::Not(inner, _) => { self.push("!"); self.fmt_expr(inner); }
            Expr::BinOp(op, lhs, rhs, _) => {
                self.fmt_expr(lhs);
                self.push(match op {
                    BinOp::Add => " + ", BinOp::Sub => " - ",
                    BinOp::Mul => " * ", BinOp::Div => " / ", BinOp::Mod => " % ",
                    BinOp::Eq => " == ", BinOp::Ne => " != ",
                    BinOp::Lt => " < ", BinOp::Gt => " > ",
                    BinOp::Le => " <= ", BinOp::Ge => " >= ",
                    BinOp::And => " && ", BinOp::Or => " || ",
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
                if let Some(a) = arg { self.fmt_expr(a); }
                self.push(")");
            }
            Expr::FormatString(fmt, args, _) => {
                self.push("format(\"");
                self.push(fmt);
                self.push("\"");
                for a in args { self.push(", "); self.fmt_expr(a); }
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
                self.fmt_lvalue(base); self.push("."); self.push(field);
            }
            LValue::TupleField(base, idx, _) => {
                self.fmt_lvalue(base); self.push(&format!(".{idx}"));
            }
            LValue::Index(base, index, _) => {
                self.fmt_lvalue(base); self.push("["); self.fmt_expr(index); self.push("]");
            }
        }
    }

    // ---- Module system ----

    fn fmt_mod_expr(&mut self, expr: &ModExpr) {
        match expr {
            ModExpr::Paren(inner) => { self.push("("); self.fmt_mod_expr(inner); self.push(")"); }
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
                self.push("hidee "); self.push(&events.join(", "));
                self.push(" in "); self.fmt_mod_expr(inner);
            }
            ModExpr::HideInterfaces(ifaces, inner) => {
                self.push("hidei "); self.push(&ifaces.join(", "));
                self.push(" in "); self.fmt_mod_expr(inner);
            }
            ModExpr::AssertMod(monitors, inner) => {
                self.push("assert "); self.push(&monitors.join(", "));
                self.push(" in "); self.fmt_mod_expr(inner);
            }
            ModExpr::Rename(old, new, inner) => {
                self.push("rename "); self.push(old);
                self.push(" to "); self.push(new);
                self.push(" in "); self.fmt_mod_expr(inner);
            }
            ModExpr::MainMachine(machine, inner) => {
                self.push("main "); self.push(machine);
                self.push(" in "); self.fmt_mod_expr(inner);
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
enum DeclKind { None, Type, Event, Machine, Spec, Fun, Module, Other }

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

fn decl_span(decl: &TopDecl) -> Span {
    match decl {
        TopDecl::TypeDef(d) => d.span,
        TopDecl::EnumTypeDef(d) => d.span,
        TopDecl::EventDecl(d) => d.span,
        TopDecl::EventSetDecl(d) => d.span,
        TopDecl::InterfaceDecl(d) => d.span,
        TopDecl::MachineDecl(d) | TopDecl::SpecMachineDecl(d) => d.span,
        TopDecl::FunDecl(d) => d.span,
        TopDecl::ModuleDecl(d) => d.span,
        TopDecl::TestDecl(d) => d.span,
        TopDecl::ImplementationDecl(d) => d.span,
        TopDecl::GlobalParamDecl(d) => d.span,
    }
}

fn state_item_span(item: &StateBodyItem) -> Span {
    match item {
        StateBodyItem::Entry(ee) => ee.span,
        StateBodyItem::Exit(ee) => ee.span,
        StateBodyItem::Defer(_, s) | StateBodyItem::Ignore(_, s) => *s,
        StateBodyItem::OnEventDoAction(on) => on.span,
        StateBodyItem::OnEventGotoState(on) => on.span,
    }
}

fn stmt_span(stmt: &Stmt) -> Span {
    match stmt {
        Stmt::Compound(_, s) => *s,
        Stmt::Assert { span, .. } => *span,
        Stmt::Assume { span, .. } => *span,
        Stmt::Print { span, .. } => *span,
        Stmt::Return { span, .. } => *span,
        Stmt::Break(s) | Stmt::Continue(s) | Stmt::NoStmt(s) => *s,
        Stmt::Assign { span, .. } => *span,
        Stmt::Insert { span, .. } => *span,
        Stmt::AddToSet { span, .. } => *span,
        Stmt::Remove { span, .. } => *span,
        Stmt::While { span, .. } => *span,
        Stmt::Foreach { span, .. } => *span,
        Stmt::If { span, .. } => *span,
        Stmt::CtorStmt { span, .. } => *span,
        Stmt::FunCall { span, .. } => *span,
        Stmt::Raise { span, .. } => *span,
        Stmt::Send { span, .. } => *span,
        Stmt::Announce { span, .. } => *span,
        Stmt::Goto { span, .. } => *span,
        Stmt::Receive { span, .. } => *span,
    }
}
