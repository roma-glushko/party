//! Recursive descent parser for the P language.
//! Closely follows PParser.g4 production rules.

use super::ast::*;
use super::token::{Span, SpannedToken, Token};

#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    pub span: Span,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

pub struct Parser {
    tokens: Vec<SpannedToken>,
    pos: usize,
    source: String,
}

impl Parser {
    pub fn new(tokens: Vec<SpannedToken>, source: String) -> Self {
        Self {
            tokens,
            pos: 0,
            source,
        }
    }

    fn slice_at(&self, span: Span) -> &str {
        &self.source[span.start..span.end]
    }

    // ---- Basic helpers ----

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos).map(|t| &t.kind)
    }

    fn peek_span(&self) -> Span {
        self.tokens
            .get(self.pos)
            .map(|t| t.span)
            .unwrap_or(Span::new(self.source.len(), self.source.len()))
    }

    fn at_end(&self) -> bool {
        self.pos >= self.tokens.len()
    }

    fn advance(&mut self) -> SpannedToken {
        let tok = self.tokens[self.pos].clone();
        self.pos += 1;
        tok
    }

    fn expect(&mut self, expected: &Token) -> Result<Span, ParseError> {
        if self.peek() == Some(expected) {
            let span = self.tokens[self.pos].span;
            self.pos += 1;
            Ok(span)
        } else {
            Err(self.error(format!("expected {expected}, got {}", self.current_desc())))
        }
    }

    fn expect_iden(&mut self) -> Result<(Iden, Span), ParseError> {
        match self.peek() {
            Some(Token::Iden) => {
                let tok = self.advance();
                Ok((self.slice_at(tok.span).to_string(), tok.span))
            }
            _ => Err(self.error(format!(
                "expected identifier, got {}",
                self.current_desc()
            ))),
        }
    }

    fn try_consume(&mut self, tok: &Token) -> Option<Span> {
        if self.peek() == Some(tok) {
            let span = self.tokens[self.pos].span;
            self.pos += 1;
            Some(span)
        } else {
            None
        }
    }

    fn current_desc(&self) -> String {
        match self.tokens.get(self.pos) {
            Some(t) => {
                let text = self.slice_at(t.span);
                format!("'{}' ({:?})", text, t.kind)
            }
            None => "end of file".to_string(),
        }
    }

    fn error(&self, message: String) -> ParseError {
        ParseError {
            message,
            span: self.peek_span(),
        }
    }

    fn span_from(&self, start: usize) -> Span {
        let end = if self.pos > 0 {
            self.tokens[self.pos - 1].span.end
        } else {
            start
        };
        Span::new(start, end)
    }

    // ---- Program ----

    pub fn parse_program(&mut self) -> Result<Program, ParseError> {
        let mut decls = Vec::new();
        while !self.at_end() {
            decls.push(self.parse_top_decl()?);
        }
        Ok(Program { decls })
    }

    // ---- Top-level declarations ----

    fn parse_top_decl(&mut self) -> Result<TopDecl, ParseError> {
        match self.peek() {
            Some(Token::Type) => self.parse_type_def_decl(),
            Some(Token::Enum) => self.parse_enum_type_def_decl(),
            Some(Token::Event) => self.parse_event_decl(),
            Some(Token::EventSet) => self.parse_event_set_decl(),
            Some(Token::Interface) => self.parse_interface_decl(),
            Some(Token::Machine) => self.parse_machine_decl(false),
            Some(Token::Spec) => self.parse_spec_machine_decl(),
            Some(Token::Fun) => {
                let f = self.parse_fun_decl()?;
                Ok(TopDecl::FunDecl(f))
            }
            Some(Token::Module) => self.parse_module_decl(),
            Some(Token::Test | Token::ParamTest) => self.parse_test_decl(),
            Some(Token::Implementation) => self.parse_implementation_decl(),
            Some(Token::Param) => self.parse_global_param_decl(),
            // Skip PVerifier-only declarations
            Some(Token::Invariant | Token::Axiom | Token::InitCondition | Token::Proof | Token::Lemma | Token::Theorem | Token::Pure) => {
                self.skip_pverifier_decl()
            }
            _ => Err(self.error(format!(
                "expected top-level declaration, got {}",
                self.current_desc()
            ))),
        }
    }

    fn skip_pverifier_decl(&mut self) -> Result<TopDecl, ParseError> {
        // Skip PVerifier-only declarations by consuming tokens until we find a balanced end
        let start = self.peek_span().start;
        let tok = self.advance().kind.clone();
        match tok {
            Token::Pure => {
                // pure name(...) : type = expr ;  OR  pure name(...) : type ;
                let (name, _) = self.expect_iden()?;
                self.expect(&Token::LParen)?;
                self.skip_balanced(&Token::LParen, &Token::RParen)?;
                self.expect(&Token::Colon)?;
                let _ty = self.parse_type()?;
                if self.try_consume(&Token::Assign).is_some() {
                    self.parse_expr(0)?;
                }
                self.expect(&Token::Semi)?;
                // Return as a type def with no type (will be ignored)
                Ok(TopDecl::TypeDef(TypeDefDecl {
                    name,
                    ty: None,
                    span: self.span_from(start),
                }))
            }
            Token::Proof => {
                // Proof name? { ... }
                if self.peek() == Some(&Token::Iden) {
                    self.advance();
                }
                self.expect(&Token::LBrace)?;
                self.skip_balanced(&Token::LBrace, &Token::RBrace)?;
                Ok(TopDecl::TypeDef(TypeDefDecl {
                    name: "_proof".to_string(),
                    ty: None,
                    span: self.span_from(start),
                }))
            }
            Token::Lemma | Token::Theorem => {
                let _ = self.expect_iden()?;
                self.expect(&Token::LBrace)?;
                self.skip_balanced(&Token::LBrace, &Token::RBrace)?;
                Ok(TopDecl::TypeDef(TypeDefDecl {
                    name: "_proof_group".to_string(),
                    ty: None,
                    span: self.span_from(start),
                }))
            }
            _ => {
                // invariant/axiom/init-condition: skip to semicolon
                self.skip_to_semi()?;
                Ok(TopDecl::TypeDef(TypeDefDecl {
                    name: "_pverifier".to_string(),
                    ty: None,
                    span: self.span_from(start),
                }))
            }
        }
    }

    fn skip_balanced(&mut self, open: &Token, close: &Token) -> Result<(), ParseError> {
        let mut depth = 1;
        while depth > 0 && !self.at_end() {
            let tok = &self.advance().kind.clone();
            if tok == open {
                depth += 1;
            } else if tok == close {
                depth -= 1;
            }
        }
        if depth != 0 {
            Err(self.error("unbalanced brackets".to_string()))
        } else {
            Ok(())
        }
    }

    fn skip_to_semi(&mut self) -> Result<(), ParseError> {
        while !self.at_end() {
            if self.advance().kind == Token::Semi {
                return Ok(());
            }
        }
        Err(self.error("expected ';'".to_string()))
    }

    // ---- type T; / type T = Type; ----

    fn parse_type_def_decl(&mut self) -> Result<TopDecl, ParseError> {
        let start = self.peek_span().start;
        self.expect(&Token::Type)?;
        let (name, _) = self.expect_iden()?;
        let ty = if self.try_consume(&Token::Assign).is_some() {
            Some(self.parse_type()?)
        } else {
            None
        };
        self.expect(&Token::Semi)?;
        Ok(TopDecl::TypeDef(TypeDefDecl {
            name,
            ty,
            span: self.span_from(start),
        }))
    }

    // ---- enum E { A, B, C } ----

    fn parse_enum_type_def_decl(&mut self) -> Result<TopDecl, ParseError> {
        let start = self.peek_span().start;
        self.expect(&Token::Enum)?;
        let (name, _) = self.expect_iden()?;
        self.expect(&Token::LBrace)?;
        let mut elements = Vec::new();
        loop {
            if self.peek() == Some(&Token::RBrace) {
                break;
            }
            let elem_start = self.peek_span().start;
            let (elem_name, _) = self.expect_iden()?;
            let value = if self.try_consume(&Token::Assign).is_some() {
                let (v, _) = self.parse_int_literal()?;
                Some(v)
            } else {
                None
            };
            elements.push(EnumElem {
                name: elem_name,
                value,
                span: self.span_from(elem_start),
            });
            if self.try_consume(&Token::Comma).is_none() {
                break;
            }
        }
        self.expect(&Token::RBrace)?;
        Ok(TopDecl::EnumTypeDef(EnumTypeDefDecl {
            name,
            elements,
            span: self.span_from(start),
        }))
    }

    // ---- event E : Type; ----

    fn parse_event_decl(&mut self) -> Result<TopDecl, ParseError> {
        let start = self.peek_span().start;
        self.expect(&Token::Event)?;
        let (name, _) = self.expect_iden()?;
        let payload = if self.try_consume(&Token::Colon).is_some() {
            Some(self.parse_type()?)
        } else {
            None
        };
        self.expect(&Token::Semi)?;
        Ok(TopDecl::EventDecl(EventDecl {
            name,
            payload,
            span: self.span_from(start),
        }))
    }

    // ---- eventset ES = { e1, e2 }; ----

    fn parse_event_set_decl(&mut self) -> Result<TopDecl, ParseError> {
        let start = self.peek_span().start;
        self.expect(&Token::EventSet)?;
        let (name, _) = self.expect_iden()?;
        self.expect(&Token::Assign)?;
        self.expect(&Token::LBrace)?;
        let events = self.parse_non_default_event_list()?;
        self.expect(&Token::RBrace)?;
        self.expect(&Token::Semi)?;
        Ok(TopDecl::EventSetDecl(EventSetDecl {
            name,
            events,
            span: self.span_from(start),
        }))
    }

    // ---- interface I(Type) receives e1, e2; ----

    fn parse_interface_decl(&mut self) -> Result<TopDecl, ParseError> {
        let start = self.peek_span().start;
        self.expect(&Token::Interface)?;
        let (name, _) = self.expect_iden()?;
        self.expect(&Token::LParen)?;
        let payload = if self.peek() != Some(&Token::RParen) {
            Some(self.parse_type()?)
        } else {
            None
        };
        self.expect(&Token::RParen)?;
        let receives = if self.try_consume(&Token::Receives).is_some() {
            if self.peek() == Some(&Token::Semi) {
                Some(Vec::new())
            } else {
                Some(self.parse_non_default_event_list()?)
            }
        } else {
            None
        };
        self.expect(&Token::Semi)?;
        Ok(TopDecl::InterfaceDecl(InterfaceDecl {
            name,
            payload,
            receives,
            span: self.span_from(start),
        }))
    }

    // ---- machine M receives...; sends...; { ... } ----

    fn parse_machine_decl(&mut self, is_spec: bool) -> Result<TopDecl, ParseError> {
        let start = self.peek_span().start;
        if !is_spec {
            self.expect(&Token::Machine)?;
        }
        let (name, _) = self.expect_iden()?;

        let mut receives = None;
        let mut sends = None;
        let observes = if is_spec {
            self.expect(&Token::Observes)?;
            Some(self.parse_non_default_event_list()?)
        } else {
            // Parse receives/sends clauses
            loop {
                if self.try_consume(&Token::Receives).is_some() {
                    if self.peek() == Some(&Token::Semi) {
                        receives = Some(Vec::new());
                    } else {
                        receives = Some(self.parse_non_default_event_list()?);
                    }
                    self.expect(&Token::Semi)?;
                } else if self.try_consume(&Token::Sends).is_some() {
                    if self.peek() == Some(&Token::Semi) {
                        sends = Some(Vec::new());
                    } else {
                        sends = Some(self.parse_non_default_event_list()?);
                    }
                    self.expect(&Token::Semi)?;
                } else {
                    break;
                }
            }
            None
        };

        let body = self.parse_machine_body()?;
        let decl = MachineDecl {
            name,
            is_spec,
            observes,
            receives,
            sends,
            body,
            span: self.span_from(start),
        };
        if is_spec {
            Ok(TopDecl::SpecMachineDecl(decl))
        } else {
            Ok(TopDecl::MachineDecl(decl))
        }
    }

    fn parse_spec_machine_decl(&mut self) -> Result<TopDecl, ParseError> {
        self.expect(&Token::Spec)?;
        self.parse_machine_decl(true)
    }

    fn parse_machine_body(&mut self) -> Result<MachineBody, ParseError> {
        self.expect(&Token::LBrace)?;
        let mut vars = Vec::new();
        let mut funs = Vec::new();
        let mut states = Vec::new();

        while self.peek() != Some(&Token::RBrace) && !self.at_end() {
            match self.peek() {
                Some(Token::Var) => vars.push(self.parse_var_decl()?),
                Some(Token::Fun) => funs.push(self.parse_fun_decl()?),
                Some(Token::Start | Token::State | Token::Hot | Token::Cold) => {
                    states.push(self.parse_state_decl()?)
                }
                _ => {
                    return Err(self.error(format!(
                        "expected var, fun, or state in machine body, got {}",
                        self.current_desc()
                    )));
                }
            }
        }
        self.expect(&Token::RBrace)?;
        Ok(MachineBody { vars, funs, states })
    }

    // ---- var x, y : int; ----

    fn parse_var_decl(&mut self) -> Result<VarDecl, ParseError> {
        let start = self.peek_span().start;
        self.expect(&Token::Var)?;
        let names = self.parse_iden_list()?;
        self.expect(&Token::Colon)?;
        let ty = self.parse_type()?;
        self.expect(&Token::Semi)?;
        Ok(VarDecl {
            names,
            ty,
            span: self.span_from(start),
        })
    }

    fn parse_iden_list(&mut self) -> Result<Vec<Iden>, ParseError> {
        let mut names = Vec::new();
        let (name, _) = self.expect_iden()?;
        names.push(name);
        while self.try_consume(&Token::Comma).is_some() {
            let (name, _) = self.expect_iden()?;
            names.push(name);
        }
        Ok(names)
    }

    // ---- fun F(x: int, y: bool) : int { ... } ----

    fn parse_fun_decl(&mut self) -> Result<FunDecl, ParseError> {
        let start = self.peek_span().start;
        self.expect(&Token::Fun)?;
        let (name, _) = self.expect_iden()?;
        self.expect(&Token::LParen)?;
        let params = if self.peek() != Some(&Token::RParen) {
            self.parse_fun_param_list()?
        } else {
            Vec::new()
        };
        self.expect(&Token::RParen)?;

        let ret_type = if self.try_consume(&Token::Colon).is_some() {
            Some(self.parse_type()?)
        } else {
            None
        };

        let creates = if self.try_consume(&Token::Creates).is_some() {
            let (iface, _) = self.expect_iden()?;
            Some(iface)
        } else {
            None
        };

        // Foreign or has body?
        if self.try_consume(&Token::Semi).is_some() {
            // Foreign function or function with requires/ensures
            // Skip any requires/ensures
            Ok(FunDecl {
                name,
                params,
                ret_type,
                body: None,
                is_foreign: true,
                creates,
                span: self.span_from(start),
            })
        } else if self.peek() == Some(&Token::Return) || self.peek() == Some(&Token::Requires) || self.peek() == Some(&Token::Ensures) {
            // PVerifier foreign fun with return/requires/ensures
            while self.peek() != Some(&Token::Semi) && !self.at_end() {
                self.advance();
            }
            self.try_consume(&Token::Semi);
            Ok(FunDecl {
                name,
                params,
                ret_type,
                body: None,
                is_foreign: true,
                creates,
                span: self.span_from(start),
            })
        } else {
            let body = self.parse_function_body()?;
            Ok(FunDecl {
                name,
                params,
                ret_type,
                body: Some(body),
                is_foreign: false,
                creates,
                span: self.span_from(start),
            })
        }
    }

    fn parse_fun_param_list(&mut self) -> Result<Vec<FunParam>, ParseError> {
        let mut params = Vec::new();
        params.push(self.parse_fun_param()?);
        while self.try_consume(&Token::Comma).is_some() {
            params.push(self.parse_fun_param()?);
        }
        Ok(params)
    }

    fn parse_fun_param(&mut self) -> Result<FunParam, ParseError> {
        let start = self.peek_span().start;
        let (name, _) = self.expect_iden()?;
        self.expect(&Token::Colon)?;
        let ty = self.parse_type()?;
        Ok(FunParam {
            name,
            ty,
            span: self.span_from(start),
        })
    }

    fn parse_function_body(&mut self) -> Result<FunctionBody, ParseError> {
        let start = self.peek_span().start;
        self.expect(&Token::LBrace)?;
        let mut var_decls = Vec::new();
        while self.peek() == Some(&Token::Var) {
            var_decls.push(self.parse_var_decl()?);
        }
        let mut stmts = Vec::new();
        while self.peek() != Some(&Token::RBrace) && !self.at_end() {
            stmts.push(self.parse_statement()?);
        }
        self.expect(&Token::RBrace)?;
        Ok(FunctionBody {
            var_decls,
            stmts,
            span: self.span_from(start),
        })
    }

    // ---- State declarations ----

    fn parse_state_decl(&mut self) -> Result<StateDecl, ParseError> {
        let start = self.peek_span().start;
        let is_start = self.try_consume(&Token::Start).is_some();
        let temperature = if self.try_consume(&Token::Hot).is_some() {
            Some(Temperature::Hot)
        } else if self.try_consume(&Token::Cold).is_some() {
            Some(Temperature::Cold)
        } else {
            None
        };
        self.expect(&Token::State)?;
        let (name, _) = self.expect_iden()?;
        self.expect(&Token::LBrace)?;
        let mut items = Vec::new();
        while self.peek() != Some(&Token::RBrace) && !self.at_end() {
            items.push(self.parse_state_body_item()?);
        }
        self.expect(&Token::RBrace)?;
        Ok(StateDecl {
            is_start,
            temperature,
            name,
            items,
            span: self.span_from(start),
        })
    }

    fn parse_state_body_item(&mut self) -> Result<StateBodyItem, ParseError> {
        let start = self.peek_span().start;
        match self.peek() {
            Some(Token::Entry) => {
                self.advance();
                // entry funName; OR entry handler
                if self.peek() == Some(&Token::Iden) && self.lookahead(1) == Some(&Token::Semi) {
                    let (name, _) = self.expect_iden()?;
                    self.expect(&Token::Semi)?;
                    Ok(StateBodyItem::Entry(EntryExit {
                        fun_name: Some(name),
                        anon_handler: None,
                        span: self.span_from(start),
                    }))
                } else {
                    let handler = self.parse_anon_event_handler()?;
                    Ok(StateBodyItem::Entry(EntryExit {
                        fun_name: None,
                        anon_handler: Some(handler),
                        span: self.span_from(start),
                    }))
                }
            }
            Some(Token::Exit) => {
                self.advance();
                // exit funName; OR exit handler
                if self.peek() == Some(&Token::Iden) && self.lookahead(1) == Some(&Token::Semi) {
                    let (name, _) = self.expect_iden()?;
                    self.expect(&Token::Semi)?;
                    Ok(StateBodyItem::Exit(EntryExit {
                        fun_name: Some(name),
                        anon_handler: None,
                        span: self.span_from(start),
                    }))
                } else {
                    let body = self.parse_function_body()?;
                    Ok(StateBodyItem::Exit(EntryExit {
                        fun_name: None,
                        anon_handler: Some(AnonEventHandler {
                            param: None,
                            body,
                            span: self.span_from(start),
                        }),
                        span: self.span_from(start),
                    }))
                }
            }
            Some(Token::Defer) => {
                self.advance();
                let events = self.parse_non_default_event_list()?;
                self.expect(&Token::Semi)?;
                Ok(StateBodyItem::Defer(events, self.span_from(start)))
            }
            Some(Token::Ignore) => {
                self.advance();
                let events = self.parse_non_default_event_list()?;
                self.expect(&Token::Semi)?;
                Ok(StateBodyItem::Ignore(events, self.span_from(start)))
            }
            Some(Token::On) => {
                self.advance();
                let events = self.parse_event_list()?;
                if self.try_consume(&Token::Do).is_some() {
                    // on events do funName; OR on events do handler
                    if self.peek() == Some(&Token::Iden) && self.lookahead(1) == Some(&Token::Semi)
                    {
                        let (name, _) = self.expect_iden()?;
                        self.expect(&Token::Semi)?;
                        Ok(StateBodyItem::OnEventDoAction(OnEventDoAction {
                            events,
                            fun_name: Some(name),
                            anon_handler: None,
                            span: self.span_from(start),
                        }))
                    } else {
                        let handler = self.parse_anon_event_handler()?;
                        Ok(StateBodyItem::OnEventDoAction(OnEventDoAction {
                            events,
                            fun_name: None,
                            anon_handler: Some(handler),
                            span: self.span_from(start),
                        }))
                    }
                } else if self.try_consume(&Token::Goto).is_some() {
                    let (target, _) = self.expect_iden()?;
                    if self.try_consume(&Token::Semi).is_some() {
                        Ok(StateBodyItem::OnEventGotoState(OnEventGotoState {
                            events,
                            target,
                            with_fun_name: None,
                            with_anon_handler: None,
                            span: self.span_from(start),
                        }))
                    } else if self.try_consume(&Token::With).is_some() {
                        if self.peek() == Some(&Token::Iden)
                            && self.lookahead(1) == Some(&Token::Semi)
                        {
                            let (name, _) = self.expect_iden()?;
                            self.expect(&Token::Semi)?;
                            Ok(StateBodyItem::OnEventGotoState(OnEventGotoState {
                                events,
                                target,
                                with_fun_name: Some(name),
                                with_anon_handler: None,
                                span: self.span_from(start),
                            }))
                        } else {
                            let handler = self.parse_anon_event_handler()?;
                            Ok(StateBodyItem::OnEventGotoState(OnEventGotoState {
                                events,
                                target,
                                with_fun_name: None,
                                with_anon_handler: Some(handler),
                                span: self.span_from(start),
                            }))
                        }
                    } else {
                        self.expect(&Token::Semi)?;
                        unreachable!()
                    }
                } else {
                    Err(self.error("expected 'do' or 'goto' after event list".to_string()))
                }
            }
            _ => Err(self.error(format!(
                "expected state body item, got {}",
                self.current_desc()
            ))),
        }
    }

    fn lookahead(&self, offset: usize) -> Option<&Token> {
        self.tokens.get(self.pos + offset).map(|t| &t.kind)
    }

    fn parse_anon_event_handler(&mut self) -> Result<AnonEventHandler, ParseError> {
        let start = self.peek_span().start;
        // Optional parameter: (name : type)
        let param = if self.peek() == Some(&Token::LParen) && self.is_fun_param_ahead() {
            self.expect(&Token::LParen)?;
            let p = self.parse_fun_param()?;
            self.expect(&Token::RParen)?;
            Some(p)
        } else {
            None
        };
        let body = self.parse_function_body()?;
        Ok(AnonEventHandler {
            param,
            body,
            span: self.span_from(start),
        })
    }

    /// Check if what follows LParen is `iden : type )` (a function parameter)
    /// vs a parenthesized expression.
    fn is_fun_param_ahead(&self) -> bool {
        // ( iden : ... )
        matches!(
            (self.lookahead(1), self.lookahead(2)),
            (Some(Token::Iden), Some(Token::Colon))
        )
    }

    fn parse_event_list(&mut self) -> Result<Vec<Iden>, ParseError> {
        let mut events = Vec::new();
        events.push(self.parse_event_id()?);
        while self.try_consume(&Token::Comma).is_some() {
            events.push(self.parse_event_id()?);
        }
        Ok(events)
    }

    fn parse_event_id(&mut self) -> Result<Iden, ParseError> {
        match self.peek() {
            Some(Token::Null) => {
                self.advance();
                Ok("null".to_string())
            }
            Some(Token::Halt) => {
                self.advance();
                Ok("halt".to_string())
            }
            Some(Token::Iden) => {
                let (name, _) = self.expect_iden()?;
                Ok(name)
            }
            _ => Err(self.error(format!("expected event name, got {}", self.current_desc()))),
        }
    }

    fn parse_non_default_event_list(&mut self) -> Result<Vec<Iden>, ParseError> {
        let mut events = Vec::new();
        events.push(self.parse_non_default_event()?);
        while self.try_consume(&Token::Comma).is_some() {
            events.push(self.parse_non_default_event()?);
        }
        Ok(events)
    }

    fn parse_non_default_event(&mut self) -> Result<Iden, ParseError> {
        match self.peek() {
            Some(Token::Halt) => {
                self.advance();
                Ok("halt".to_string())
            }
            Some(Token::Iden) => {
                let (name, _) = self.expect_iden()?;
                Ok(name)
            }
            _ => Err(self.error(format!(
                "expected event name or 'halt', got {}",
                self.current_desc()
            ))),
        }
    }

    // ---- Types ----

    fn parse_type(&mut self) -> Result<PType, ParseError> {
        match self.peek() {
            Some(Token::Bool) => {
                self.advance();
                Ok(PType::Bool)
            }
            Some(Token::Int) => {
                self.advance();
                Ok(PType::Int)
            }
            Some(Token::Float) => {
                self.advance();
                Ok(PType::Float)
            }
            Some(Token::StringType) => {
                self.advance();
                Ok(PType::StringType)
            }
            Some(Token::Event) => {
                self.advance();
                Ok(PType::Event)
            }
            Some(Token::Machine) => {
                self.advance();
                Ok(PType::Machine)
            }
            Some(Token::Data) => {
                self.advance();
                Ok(PType::Data)
            }
            Some(Token::Any) => {
                self.advance();
                Ok(PType::Any)
            }
            Some(Token::Seq) => {
                self.advance();
                self.expect(&Token::LBrack)?;
                let elem = self.parse_type()?;
                self.expect(&Token::RBrack)?;
                Ok(PType::Seq(Box::new(elem)))
            }
            Some(Token::Set) => {
                self.advance();
                self.expect(&Token::LBrack)?;
                let elem = self.parse_type()?;
                self.expect(&Token::RBrack)?;
                Ok(PType::Set(Box::new(elem)))
            }
            Some(Token::Map) => {
                self.advance();
                self.expect(&Token::LBrack)?;
                let key = self.parse_type()?;
                self.expect(&Token::Comma)?;
                let val = self.parse_type()?;
                self.expect(&Token::RBrack)?;
                Ok(PType::Map(Box::new(key), Box::new(val)))
            }
            Some(Token::LParen) => {
                // Named tuple: (name: type, ...) or unnamed tuple: (type, type, ...)
                self.advance();
                if self.peek() == Some(&Token::RParen) {
                    self.advance();
                    return Ok(PType::Tuple(Vec::new()));
                }
                // Lookahead: if we see iden : then it's a named tuple type
                if matches!(
                    (self.peek(), self.lookahead(1)),
                    (Some(Token::Iden), Some(Token::Colon))
                ) {
                    let mut fields = Vec::new();
                    loop {
                        let (name, _) = self.expect_iden()?;
                        self.expect(&Token::Colon)?;
                        let ty = self.parse_type()?;
                        fields.push((name, ty));
                        if self.try_consume(&Token::Comma).is_none() {
                            break;
                        }
                        // Allow trailing comma before )
                        if self.peek() == Some(&Token::RParen) {
                            break;
                        }
                    }
                    self.expect(&Token::RParen)?;
                    Ok(PType::NamedTuple(fields))
                } else {
                    let mut types = Vec::new();
                    types.push(self.parse_type()?);
                    while self.try_consume(&Token::Comma).is_some() {
                        if self.peek() == Some(&Token::RParen) {
                            break;
                        }
                        types.push(self.parse_type()?);
                    }
                    self.expect(&Token::RParen)?;
                    Ok(PType::Tuple(types))
                }
            }
            Some(Token::Iden) => {
                let (name, _) = self.expect_iden()?;
                Ok(PType::Named(name))
            }
            _ => Err(self.error(format!("expected type, got {}", self.current_desc()))),
        }
    }

    // ---- Statements ----

    fn parse_statement(&mut self) -> Result<Stmt, ParseError> {
        let start = self.peek_span().start;
        match self.peek() {
            Some(Token::LBrace) => {
                self.advance();
                let mut stmts = Vec::new();
                while self.peek() != Some(&Token::RBrace) && !self.at_end() {
                    stmts.push(self.parse_statement()?);
                }
                self.expect(&Token::RBrace)?;
                Ok(Stmt::Compound(stmts, self.span_from(start)))
            }
            Some(Token::Assert) => {
                self.advance();
                let expr = self.parse_expr(0)?;
                let message = if self.try_consume(&Token::Comma).is_some() {
                    Some(self.parse_expr(0)?)
                } else {
                    None
                };
                self.expect(&Token::Semi)?;
                Ok(Stmt::Assert {
                    expr,
                    message,
                    span: self.span_from(start),
                })
            }
            Some(Token::Assume) => {
                self.advance();
                let expr = self.parse_expr(0)?;
                let message = if self.try_consume(&Token::Comma).is_some() {
                    Some(self.parse_expr(0)?)
                } else {
                    None
                };
                self.expect(&Token::Semi)?;
                Ok(Stmt::Assume {
                    expr,
                    message,
                    span: self.span_from(start),
                })
            }
            Some(Token::Print) => {
                self.advance();
                let message = self.parse_expr(0)?;
                self.expect(&Token::Semi)?;
                Ok(Stmt::Print {
                    message,
                    span: self.span_from(start),
                })
            }
            Some(Token::Return) => {
                self.advance();
                let value = if self.peek() != Some(&Token::Semi) {
                    Some(self.parse_expr(0)?)
                } else {
                    None
                };
                self.expect(&Token::Semi)?;
                Ok(Stmt::Return {
                    value,
                    span: self.span_from(start),
                })
            }
            Some(Token::Break) => {
                self.advance();
                self.expect(&Token::Semi)?;
                Ok(Stmt::Break(self.span_from(start)))
            }
            Some(Token::Continue) => {
                self.advance();
                self.expect(&Token::Semi)?;
                Ok(Stmt::Continue(self.span_from(start)))
            }
            Some(Token::While) => {
                self.advance();
                self.expect(&Token::LParen)?;
                let cond = self.parse_expr(0)?;
                self.expect(&Token::RParen)?;
                let body = self.parse_statement()?;
                Ok(Stmt::While {
                    cond,
                    body: Box::new(body),
                    span: self.span_from(start),
                })
            }
            Some(Token::Foreach) => {
                self.advance();
                self.expect(&Token::LParen)?;
                let (item, _) = self.expect_iden()?;
                self.expect(&Token::In)?;
                let collection = self.parse_expr(0)?;
                self.expect(&Token::RParen)?;
                // Skip optional invariant clauses (PVerifier)
                while self.try_consume(&Token::Invariant).is_some() {
                    self.parse_expr(0)?;
                    self.expect(&Token::Semi)?;
                }
                let body = self.parse_statement()?;
                Ok(Stmt::Foreach {
                    item,
                    collection,
                    body: Box::new(body),
                    span: self.span_from(start),
                })
            }
            Some(Token::If) => {
                self.advance();
                self.expect(&Token::LParen)?;
                let cond = self.parse_expr(0)?;
                self.expect(&Token::RParen)?;
                let then_branch = self.parse_statement()?;
                let else_branch = if self.try_consume(&Token::Else).is_some() {
                    Some(Box::new(self.parse_statement()?))
                } else {
                    None
                };
                Ok(Stmt::If {
                    cond,
                    then_branch: Box::new(then_branch),
                    else_branch,
                    span: self.span_from(start),
                })
            }
            Some(Token::New) => {
                self.advance();
                let (name, _) = self.expect_iden()?;
                self.expect(&Token::LParen)?;
                let args = if self.peek() != Some(&Token::RParen) {
                    self.parse_rvalue_list()?
                } else {
                    Vec::new()
                };
                self.expect(&Token::RParen)?;
                self.expect(&Token::Semi)?;
                Ok(Stmt::CtorStmt {
                    interface: name,
                    args,
                    span: self.span_from(start),
                })
            }
            Some(Token::Raise) => {
                self.advance();
                let event = self.parse_expr(0)?;
                let args = if self.try_consume(&Token::Comma).is_some() {
                    self.parse_rvalue_list()?
                } else {
                    Vec::new()
                };
                self.expect(&Token::Semi)?;
                Ok(Stmt::Raise {
                    event,
                    args,
                    span: self.span_from(start),
                })
            }
            Some(Token::Send) => {
                self.advance();
                let target = self.parse_expr(0)?;
                self.expect(&Token::Comma)?;
                let event = self.parse_expr(0)?;
                let args = if self.try_consume(&Token::Comma).is_some() {
                    self.parse_rvalue_list()?
                } else {
                    Vec::new()
                };
                self.expect(&Token::Semi)?;
                Ok(Stmt::Send {
                    target,
                    event,
                    args,
                    span: self.span_from(start),
                })
            }
            Some(Token::Announce) => {
                self.advance();
                let event = self.parse_expr(0)?;
                let args = if self.try_consume(&Token::Comma).is_some() {
                    self.parse_rvalue_list()?
                } else {
                    Vec::new()
                };
                self.expect(&Token::Semi)?;
                Ok(Stmt::Announce {
                    event,
                    args,
                    span: self.span_from(start),
                })
            }
            Some(Token::Goto) => {
                self.advance();
                let (state, _) = self.expect_iden()?;
                let payload = if self.try_consume(&Token::Comma).is_some() {
                    self.parse_rvalue_list()?
                } else {
                    Vec::new()
                };
                self.expect(&Token::Semi)?;
                Ok(Stmt::Goto {
                    state,
                    payload,
                    span: self.span_from(start),
                })
            }
            Some(Token::Receive) => {
                self.advance();
                self.expect(&Token::LBrace)?;
                let mut cases = Vec::new();
                while self.peek() == Some(&Token::Case) {
                    let case_start = self.peek_span().start;
                    self.advance();
                    let events = self.parse_event_list()?;
                    self.expect(&Token::Colon)?;
                    let handler = self.parse_anon_event_handler()?;
                    cases.push(RecvCase {
                        events,
                        handler,
                        span: self.span_from(case_start),
                    });
                }
                self.expect(&Token::RBrace)?;
                Ok(Stmt::Receive {
                    cases,
                    span: self.span_from(start),
                })
            }
            Some(Token::Semi) => {
                self.advance();
                Ok(Stmt::NoStmt(self.span_from(start)))
            }
            Some(Token::Iden) => {
                // Could be: assignment, insert, remove, or function call
                self.parse_iden_leading_stmt(start)
            }
            _ => Err(self.error(format!(
                "expected statement, got {}",
                self.current_desc()
            ))),
        }
    }

    /// Parse a statement that starts with an identifier.
    /// Could be: lvalue = expr; | lvalue += (...); | lvalue -= expr; | fun(...);
    fn parse_iden_leading_stmt(&mut self, start: usize) -> Result<Stmt, ParseError> {
        // Try to parse as lvalue first
        let lvalue = self.parse_lvalue()?;

        match self.peek() {
            Some(Token::Assign) => {
                self.advance();
                let rvalue = self.parse_expr(0)?;
                self.expect(&Token::Semi)?;
                Ok(Stmt::Assign {
                    lvalue,
                    rvalue,
                    span: self.span_from(start),
                })
            }
            Some(Token::Insert) => {
                self.advance();
                self.expect(&Token::LParen)?;
                let first = self.parse_expr(0)?;
                if self.try_consume(&Token::Comma).is_some() {
                    // Insert: lvalue += (index, value)
                    let value = self.parse_expr(0)?;
                    self.expect(&Token::RParen)?;
                    self.expect(&Token::Semi)?;
                    Ok(Stmt::Insert {
                        lvalue,
                        index: first,
                        value,
                        span: self.span_from(start),
                    })
                } else {
                    // Add to set: lvalue += (value)
                    self.expect(&Token::RParen)?;
                    self.expect(&Token::Semi)?;
                    Ok(Stmt::AddToSet {
                        lvalue,
                        value: first,
                        span: self.span_from(start),
                    })
                }
            }
            Some(Token::Remove) => {
                self.advance();
                let key = self.parse_expr(0)?;
                self.expect(&Token::Semi)?;
                Ok(Stmt::Remove {
                    lvalue,
                    key,
                    span: self.span_from(start),
                })
            }
            Some(Token::LParen) => {
                // Function call: name(args);
                let name = match lvalue {
                    LValue::Var(name, _) => name,
                    _ => return Err(self.error("invalid function call target".to_string())),
                };
                self.advance();
                let args = if self.peek() != Some(&Token::RParen) {
                    self.parse_rvalue_list()?
                } else {
                    Vec::new()
                };
                self.expect(&Token::RParen)?;
                self.expect(&Token::Semi)?;
                Ok(Stmt::FunCall {
                    name,
                    args,
                    span: self.span_from(start),
                })
            }
            _ => Err(self.error(format!(
                "expected '=', '+=', '-=', or '(' after identifier, got {}",
                self.current_desc()
            ))),
        }
    }

    fn parse_lvalue(&mut self) -> Result<LValue, ParseError> {
        let start = self.peek_span().start;
        let (name, _) = self.expect_iden()?;
        let mut lv = LValue::Var(name, self.span_from(start));

        loop {
            if self.try_consume(&Token::Dot).is_some() {
                if let Some(Token::IntLiteral) = self.peek() {
                    let tok = self.advance();
                    let idx: usize = self.slice_at(tok.span).parse().map_err(|_| {
                        self.error("invalid tuple index".to_string())
                    })?;
                    lv = LValue::TupleField(Box::new(lv), idx, self.span_from(start));
                } else {
                    let (field, _) = self.expect_iden()?;
                    lv = LValue::NamedTupleField(Box::new(lv), field, self.span_from(start));
                }
            } else if self.try_consume(&Token::LBrack).is_some() {
                let index = self.parse_expr(0)?;
                self.expect(&Token::RBrack)?;
                lv = LValue::Index(Box::new(lv), index, self.span_from(start));
            } else {
                break;
            }
        }

        Ok(lv)
    }

    // ---- Expressions (Pratt parser) ----

    pub fn parse_expr(&mut self, min_prec: u8) -> Result<Expr, ParseError> {
        let mut left = self.parse_unary_expr()?;

        loop {
            // Handle `as`/`to` cast at precedence 5 (between add/sub and comparison)
            if matches!(self.peek(), Some(Token::As | Token::To)) {
                let cast_prec: u8 = 5;
                if cast_prec < min_prec {
                    break;
                }
                self.advance();
                let start = left.span().start;
                let ty = self.parse_type()?;
                left = Expr::Cast(Box::new(left), ty, self.span_from(start));
                continue;
            }

            let Some(op) = self.peek_binop() else { break };
            let (prec, right_assoc) = binop_precedence(op);
            if prec < min_prec {
                break;
            }
            self.advance(); // consume operator
            let start = left.span().start;
            let next_min = if right_assoc { prec } else { prec + 1 };
            let right = self.parse_expr(next_min)?;
            let span = Span::new(start, right.span().end);
            left = Expr::BinOp(op, Box::new(left), Box::new(right), span);
        }

        Ok(left)
    }

    fn peek_binop(&self) -> Option<BinOp> {
        match self.peek()? {
            Token::Add => Some(BinOp::Add),
            Token::Sub => Some(BinOp::Sub),
            Token::Mul => Some(BinOp::Mul),
            Token::Div => Some(BinOp::Div),
            Token::Mod => Some(BinOp::Mod),
            Token::Eq => Some(BinOp::Eq),
            Token::Ne => Some(BinOp::Ne),
            Token::Lt => Some(BinOp::Lt),
            Token::Gt => Some(BinOp::Gt),
            Token::Le => Some(BinOp::Le),
            Token::Ge => Some(BinOp::Ge),
            Token::LAnd => Some(BinOp::And),
            Token::LOr => Some(BinOp::Or),
            Token::In => Some(BinOp::In),
            _ => None,
        }
    }

    fn parse_unary_expr(&mut self) -> Result<Expr, ParseError> {
        let start = self.peek_span().start;
        match self.peek() {
            Some(Token::Sub) => {
                self.advance();
                let expr = self.parse_unary_expr()?;
                Ok(Expr::Neg(Box::new(expr), self.span_from(start)))
            }
            Some(Token::LNot) => {
                self.advance();
                let expr = self.parse_unary_expr()?;
                Ok(Expr::Not(Box::new(expr), self.span_from(start)))
            }
            _ => self.parse_postfix_expr(),
        }
    }

    fn parse_postfix_expr(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_primary_expr()?;

        loop {
            let start = expr.span().start;
            if self.try_consume(&Token::Dot).is_some() {
                // Field access: .iden or .int
                if let Some(Token::IntLiteral) = self.peek() {
                    let tok = self.advance();
                    let idx: usize = self.slice_at(tok.span).parse().map_err(|_| {
                        self.error("invalid tuple index".to_string())
                    })?;
                    expr = Expr::TupleAccess(Box::new(expr), idx, self.span_from(start));
                } else {
                    let (field, _) = self.expect_iden()?;
                    expr =
                        Expr::NamedTupleAccess(Box::new(expr), field, self.span_from(start));
                }
            } else if self.try_consume(&Token::LBrack).is_some() {
                let index = self.parse_expr(0)?;
                self.expect(&Token::RBrack)?;
                expr = Expr::SeqMapAccess(Box::new(expr), Box::new(index), self.span_from(start));
            } else {
                break;
            }
        }

        Ok(expr)
    }

    fn parse_primary_expr(&mut self) -> Result<Expr, ParseError> {
        let start = self.peek_span().start;
        match self.peek() {
            Some(Token::IntLiteral) => {
                let tok = self.advance();
                // Check for float literal: IntLiteral DOT IntLiteral
                if self.peek() == Some(&Token::Dot) && matches!(self.lookahead(1), Some(Token::IntLiteral)) {
                    let int_str = self.slice_at(tok.span).to_string();
                    self.advance(); // consume DOT
                    let frac_tok = self.advance();
                    let frac_str = self.slice_at(frac_tok.span).to_string();
                    let val: f64 = format!("{int_str}.{frac_str}").parse().unwrap_or(0.0);
                    Ok(Expr::FloatLit(val, self.span_from(start)))
                } else {
                    let val: i64 = self.slice_at(tok.span).parse().unwrap_or(0);
                    Ok(Expr::IntLit(val, self.span_from(start)))
                }
            }
            Some(Token::True) => {
                self.advance();
                Ok(Expr::BoolLit(true, self.span_from(start)))
            }
            Some(Token::False) => {
                self.advance();
                Ok(Expr::BoolLit(false, self.span_from(start)))
            }
            Some(Token::Null) => {
                self.advance();
                Ok(Expr::NullLit(self.span_from(start)))
            }
            Some(Token::This) => {
                self.advance();
                Ok(Expr::This(self.span_from(start)))
            }
            Some(Token::Halt) => {
                self.advance();
                Ok(Expr::HaltEvent(self.span_from(start)))
            }
            Some(Token::Nondet) => {
                self.advance();
                Ok(Expr::Nondet(self.span_from(start)))
            }
            Some(Token::FairNondet) => {
                self.advance();
                Ok(Expr::FairNondet(self.span_from(start)))
            }
            Some(Token::StringLiteral) => {
                let tok = self.advance();
                let text = self.slice_at(tok.span);
                // Remove quotes
                let s = text[1..text.len() - 1].to_string();
                Ok(Expr::StringLit(s, self.span_from(start)))
            }
            Some(Token::Format) => {
                self.advance();
                self.expect(&Token::LParen)?;
                // format("...", args...)
                let tok = self.advance();
                if tok.kind != Token::StringLiteral {
                    return Err(self.error("expected string literal in format".to_string()));
                }
                let text = self.slice_at(tok.span);
                let fmt_str = text[1..text.len() - 1].to_string();
                let args = if self.try_consume(&Token::Comma).is_some() {
                    self.parse_rvalue_list()?
                } else {
                    Vec::new()
                };
                self.expect(&Token::RParen)?;
                Ok(Expr::FormatString(fmt_str, args, self.span_from(start)))
            }
            Some(Token::Default) => {
                self.advance();
                self.expect(&Token::LParen)?;
                let ty = self.parse_type()?;
                self.expect(&Token::RParen)?;
                Ok(Expr::Default(ty, self.span_from(start)))
            }
            Some(Token::Keys) => {
                self.advance();
                self.expect(&Token::LParen)?;
                let expr = self.parse_expr(0)?;
                self.expect(&Token::RParen)?;
                Ok(Expr::Keys(Box::new(expr), self.span_from(start)))
            }
            Some(Token::Values) => {
                self.advance();
                self.expect(&Token::LParen)?;
                let expr = self.parse_expr(0)?;
                self.expect(&Token::RParen)?;
                Ok(Expr::Values(Box::new(expr), self.span_from(start)))
            }
            Some(Token::Sizeof) => {
                self.advance();
                self.expect(&Token::LParen)?;
                let expr = self.parse_expr(0)?;
                self.expect(&Token::RParen)?;
                Ok(Expr::Sizeof(Box::new(expr), self.span_from(start)))
            }
            Some(Token::New) => {
                self.advance();
                let (name, _) = self.expect_iden()?;
                self.expect(&Token::LParen)?;
                let args = if self.peek() != Some(&Token::RParen) {
                    self.parse_rvalue_list()?
                } else {
                    Vec::new()
                };
                self.expect(&Token::RParen)?;
                Ok(Expr::New(name, args, self.span_from(start)))
            }
            Some(Token::Choose) => {
                self.advance();
                self.expect(&Token::LParen)?;
                let arg = if self.peek() != Some(&Token::RParen) {
                    Some(Box::new(self.parse_expr(0)?))
                } else {
                    None
                };
                self.expect(&Token::RParen)?;
                Ok(Expr::Choose(arg, self.span_from(start)))
            }
            Some(Token::Iden) => {
                // iden or iden(args)  -- function call
                let (name, name_span) = self.expect_iden()?;
                if self.peek() == Some(&Token::LParen) {
                    // Function call expression
                    self.advance();
                    let args = if self.peek() != Some(&Token::RParen) {
                        self.parse_rvalue_list()?
                    } else {
                        Vec::new()
                    };
                    self.expect(&Token::RParen)?;
                    Ok(Expr::FunCall(name, args, self.span_from(start)))
                } else {
                    Ok(Expr::Iden(name, name_span))
                }
            }
            Some(Token::LParen) => {
                self.advance();
                // Could be: (expr), (expr, expr, ...) unnamed tuple, (name = expr, ...) named tuple
                if self.peek() == Some(&Token::RParen) {
                    self.advance();
                    return Ok(Expr::UnnamedTuple(Vec::new(), self.span_from(start)));
                }

                // Check for named tuple: (iden = ...)
                if matches!(
                    (self.peek(), self.lookahead(1)),
                    (Some(Token::Iden), Some(Token::Assign))
                ) {
                    return self.parse_named_tuple_expr(start);
                }

                // Parse first expression
                let first = self.parse_expr(0)?;

                if self.try_consume(&Token::Comma).is_some() {
                    // Unnamed tuple
                    let mut fields = vec![first];
                    if self.peek() != Some(&Token::RParen) {
                        fields.extend(self.parse_comma_separated_exprs()?);
                    }
                    self.expect(&Token::RParen)?;
                    Ok(Expr::UnnamedTuple(fields, self.span_from(start)))
                } else {
                    // Parenthesized expression
                    self.expect(&Token::RParen)?;
                    Ok(Expr::Paren(Box::new(first), self.span_from(start)))
                }
            }
            Some(Token::Dot) if matches!(self.lookahead(1), Some(Token::IntLiteral)) => {
                // Float literal with leading dot: .123
                self.advance(); // consume DOT
                let frac_tok = self.advance();
                let frac_part = self.slice_at(frac_tok.span);
                let val: f64 = format!("0.{frac_part}").parse().unwrap_or(0.0);
                Ok(Expr::FloatLit(val, self.span_from(start)))
            }
            Some(Token::Float) => {
                // float(base, exp) syntax
                self.advance();
                self.expect(&Token::LParen)?;
                let base_tok = self.advance();
                let base: i64 = self.slice_at(base_tok.span).parse().unwrap_or(0);
                self.expect(&Token::Comma)?;
                let exp_tok = self.advance();
                let exp: i32 = self.slice_at(exp_tok.span).parse().unwrap_or(0);
                self.expect(&Token::RParen)?;
                let val = (base as f64) * 10f64.powi(exp);
                Ok(Expr::FloatLit(val, self.span_from(start)))
            }
            _ => Err(self.error(format!(
                "expected expression, got {}",
                self.current_desc()
            ))),
        }
    }

    fn parse_named_tuple_expr(&mut self, start: usize) -> Result<Expr, ParseError> {
        let mut fields = Vec::new();
        loop {
            let (name, _) = self.expect_iden()?;
            self.expect(&Token::Assign)?;
            let value = self.parse_expr(0)?;
            fields.push((name, value));
            if self.try_consume(&Token::Comma).is_none() {
                break;
            }
            if self.peek() == Some(&Token::RParen) {
                break;
            }
        }
        self.expect(&Token::RParen)?;
        Ok(Expr::NamedTuple(fields, self.span_from(start)))
    }

    fn parse_comma_separated_exprs(&mut self) -> Result<Vec<Expr>, ParseError> {
        let mut exprs = Vec::new();
        exprs.push(self.parse_expr(0)?);
        while self.try_consume(&Token::Comma).is_some() {
            if self.peek() == Some(&Token::RParen) {
                break;
            }
            exprs.push(self.parse_expr(0)?);
        }
        Ok(exprs)
    }

    fn parse_rvalue_list(&mut self) -> Result<Vec<Expr>, ParseError> {
        let mut exprs = Vec::new();
        exprs.push(self.parse_expr(0)?);
        while self.try_consume(&Token::Comma).is_some() {
            exprs.push(self.parse_expr(0)?);
        }
        Ok(exprs)
    }

    fn parse_int_literal(&mut self) -> Result<(i64, Span), ParseError> {
        match self.peek() {
            Some(Token::IntLiteral) => {
                let tok = self.advance();
                let val: i64 = self.slice_at(tok.span).parse().unwrap_or(0);
                Ok((val, tok.span))
            }
            Some(Token::Sub) => {
                self.advance();
                let tok = self.advance();
                if tok.kind != Token::IntLiteral {
                    return Err(self.error("expected integer literal".to_string()));
                }
                let val: i64 = self.slice_at(tok.span).parse::<i64>().unwrap_or(0);
                Ok((-val, tok.span))
            }
            _ => Err(self.error(format!(
                "expected integer literal, got {}",
                self.current_desc()
            ))),
        }
    }

    // ---- Module expressions ----

    fn parse_mod_expr(&mut self) -> Result<ModExpr, ParseError> {
        match self.peek() {
            Some(Token::LParen) => {
                self.advance();
                let inner = self.parse_mod_expr()?;
                self.expect(&Token::RParen)?;
                Ok(ModExpr::Paren(Box::new(inner)))
            }
            Some(Token::LBrace) => {
                self.advance();
                let mut binds = Vec::new();
                loop {
                    let (machine, _) = self.expect_iden()?;
                    let interface = if self.try_consume(&Token::RArrow).is_some() {
                        let (iface, _) = self.expect_iden()?;
                        Some(iface)
                    } else {
                        None
                    };
                    binds.push(BindExpr { machine, interface });
                    if self.try_consume(&Token::Comma).is_none() {
                        break;
                    }
                }
                self.expect(&Token::RBrace)?;
                Ok(ModExpr::Primitive(binds))
            }
            Some(Token::Compose) => {
                self.advance();
                let mut exprs = Vec::new();
                exprs.push(self.parse_mod_expr()?);
                while self.try_consume(&Token::Comma).is_some() {
                    exprs.push(self.parse_mod_expr()?);
                }
                Ok(ModExpr::Compose(exprs))
            }
            Some(Token::Union) => {
                self.advance();
                let mut exprs = Vec::new();
                exprs.push(self.parse_mod_expr()?);
                while self.try_consume(&Token::Comma).is_some() {
                    exprs.push(self.parse_mod_expr()?);
                }
                Ok(ModExpr::Union(exprs))
            }
            Some(Token::HideE) => {
                self.advance();
                let events = self.parse_non_default_event_list()?;
                self.expect(&Token::In)?;
                let inner = self.parse_mod_expr()?;
                Ok(ModExpr::HideEvents(events, Box::new(inner)))
            }
            Some(Token::HideI) => {
                self.advance();
                let ifaces = self.parse_iden_list()?;
                self.expect(&Token::In)?;
                let inner = self.parse_mod_expr()?;
                Ok(ModExpr::HideInterfaces(ifaces, Box::new(inner)))
            }
            Some(Token::Assert) => {
                self.advance();
                let monitors = self.parse_iden_list()?;
                self.expect(&Token::In)?;
                let inner = self.parse_mod_expr()?;
                Ok(ModExpr::AssertMod(monitors, Box::new(inner)))
            }
            Some(Token::Rename) => {
                self.advance();
                let (old, _) = self.expect_iden()?;
                self.expect(&Token::To)?;
                let (new, _) = self.expect_iden()?;
                self.expect(&Token::In)?;
                let inner = self.parse_mod_expr()?;
                Ok(ModExpr::Rename(old, new, Box::new(inner)))
            }
            Some(Token::Main) => {
                self.advance();
                let (machine, _) = self.expect_iden()?;
                self.expect(&Token::In)?;
                let inner = self.parse_mod_expr()?;
                Ok(ModExpr::MainMachine(machine, Box::new(inner)))
            }
            Some(Token::Iden) => {
                let (name, _) = self.expect_iden()?;
                Ok(ModExpr::Named(name))
            }
            _ => Err(self.error(format!(
                "expected module expression, got {}",
                self.current_desc()
            ))),
        }
    }

    // ---- Module-level declarations ----

    fn parse_module_decl(&mut self) -> Result<TopDecl, ParseError> {
        let start = self.peek_span().start;
        self.expect(&Token::Module)?;
        let (name, _) = self.expect_iden()?;
        self.expect(&Token::Assign)?;
        let expr = self.parse_mod_expr()?;
        self.expect(&Token::Semi)?;
        Ok(TopDecl::ModuleDecl(ModuleDecl {
            name,
            expr,
            span: self.span_from(start),
        }))
    }

    fn parse_test_decl(&mut self) -> Result<TopDecl, ParseError> {
        let start = self.peek_span().start;
        self.expect(&Token::Test)?;

        // Optional: param (name in [values])
        let params = if self.try_consume(&Token::Param).is_some() {
            Some(self.parse_param_ranges()?)
        } else {
            None
        };

        // Optional: assume expr
        let assume_expr = if self.try_consume(&Token::Assume).is_some() {
            Some(self.parse_expr(0)?)
        } else {
            None
        };

        // Optional: pairwise or (N wise)
        if self.try_consume(&Token::Pairwise).is_some() {
            // skip
        } else if self.peek() == Some(&Token::LParen)
            && matches!(self.lookahead(1), Some(Token::IntLiteral))
            && matches!(self.lookahead(2), Some(Token::Wise))
        {
            self.advance(); // (
            let n_tok = self.advance();
            let n_val: i64 = self.slice_at(n_tok.span).parse().unwrap_or(0);
            if n_val < 1 {
                return Err(self.error(format!("invalid wise coverage value: {n_val} (must be >= 1)")));
            }
            self.advance(); // wise
            self.expect(&Token::RParen)?;
        }

        let (name, _) = self.expect_iden()?;
        let main_machine = if self.try_consume(&Token::LBrack).is_some() {
            self.expect(&Token::Main)?;
            self.expect(&Token::Assign)?;
            let (m, _) = self.expect_iden()?;
            self.expect(&Token::RBrack)?;
            Some(m)
        } else {
            None
        };
        self.expect(&Token::Colon)?;
        let module_expr = self.parse_mod_expr()?;

        // Optional: refines modExpr
        if self.try_consume(&Token::Refines).is_some() {
            let _refines = self.parse_mod_expr()?;
        }

        self.expect(&Token::Semi)?;
        Ok(TopDecl::TestDecl(TestDecl {
            name,
            main_machine,
            module_expr,
            params,
            assume_expr,
            span: self.span_from(start),
        }))
    }

    fn parse_param_ranges(&mut self) -> Result<Vec<ParamRange>, ParseError> {
        self.expect(&Token::LParen)?;
        let mut ranges = Vec::new();
        loop {
            let (name, _) = self.expect_iden()?;
            self.expect(&Token::In)?;
            self.expect(&Token::LBrack)?;
            let mut values = Vec::new();
            loop {
                // seqPrimitive: BoolLiteral | IntLiteral | SUB IntLiteral
                match self.peek() {
                    Some(Token::True) => {
                        self.advance();
                        values.push(1);
                    }
                    Some(Token::False) => {
                        self.advance();
                        values.push(0);
                    }
                    _ => {
                        let (v, _) = self.parse_int_literal()?;
                        values.push(v);
                    }
                }
                if self.try_consume(&Token::Comma).is_none() {
                    break;
                }
            }
            self.expect(&Token::RBrack)?;
            ranges.push(ParamRange { name, values });
            if self.try_consume(&Token::Comma).is_none() {
                break;
            }
        }
        self.expect(&Token::RParen)?;
        Ok(ranges)
    }

    fn parse_implementation_decl(&mut self) -> Result<TopDecl, ParseError> {
        let start = self.peek_span().start;
        self.expect(&Token::Implementation)?;
        let (name, _) = self.expect_iden()?;
        let main_machine = if self.try_consume(&Token::LBrack).is_some() {
            self.expect(&Token::Main)?;
            self.expect(&Token::Assign)?;
            let (m, _) = self.expect_iden()?;
            self.expect(&Token::RBrack)?;
            Some(m)
        } else {
            None
        };
        self.expect(&Token::Colon)?;
        let module_expr = self.parse_mod_expr()?;
        self.expect(&Token::Semi)?;
        Ok(TopDecl::ImplementationDecl(ImplementationDecl {
            name,
            main_machine,
            module_expr,
            span: self.span_from(start),
        }))
    }

    fn parse_global_param_decl(&mut self) -> Result<TopDecl, ParseError> {
        let start = self.peek_span().start;
        self.expect(&Token::Param)?;
        let names = self.parse_iden_list()?;
        self.expect(&Token::Colon)?;
        let ty = self.parse_type()?;
        self.expect(&Token::Semi)?;
        Ok(TopDecl::GlobalParamDecl(GlobalParamDecl {
            names,
            ty,
            span: self.span_from(start),
        }))
    }
}

// ---- Expression helpers ----

impl Expr {
    pub fn span(&self) -> Span {
        match self {
            Expr::IntLit(_, s)
            | Expr::FloatLit(_, s)
            | Expr::BoolLit(_, s)
            | Expr::StringLit(_, s)
            | Expr::NullLit(s)
            | Expr::This(s)
            | Expr::HaltEvent(s)
            | Expr::Nondet(s)
            | Expr::FairNondet(s)
            | Expr::Iden(_, s)
            | Expr::UnnamedTuple(_, s)
            | Expr::NamedTuple(_, s)
            | Expr::NamedTupleAccess(_, _, s)
            | Expr::TupleAccess(_, _, s)
            | Expr::SeqMapAccess(_, _, s)
            | Expr::Keys(_, s)
            | Expr::Values(_, s)
            | Expr::Sizeof(_, s)
            | Expr::Default(_, s)
            | Expr::New(_, _, s)
            | Expr::FunCall(_, _, s)
            | Expr::Neg(_, s)
            | Expr::Not(_, s)
            | Expr::BinOp(_, _, _, s)
            | Expr::Cast(_, _, s)
            | Expr::Choose(_, s)
            | Expr::FormatString(_, _, s)
            | Expr::Paren(_, s) => *s,
        }
    }
}

/// Operator precedence: higher = tighter binding.
fn binop_precedence(op: BinOp) -> (u8, bool) {
    match op {
        BinOp::Or => (1, false),
        BinOp::And => (2, false),
        BinOp::Eq | BinOp::Ne => (3, false),
        BinOp::Lt | BinOp::Gt | BinOp::Le | BinOp::Ge | BinOp::In => (4, false),
        BinOp::Add | BinOp::Sub => (5, false),
        BinOp::Mul | BinOp::Div | BinOp::Mod => (6, false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::lexer;
    use std::path::Path;

    fn parse_source(source: &str) -> Result<Program, ParseError> {
        let tokens = lexer::lex(source).map_err(|e| ParseError {
            message: e.to_string(),
            span: Span::new(e.offset, e.offset + 1),
        })?;
        let mut parser = Parser::new(tokens, source.to_string());
        parser.parse_program()
    }

    #[test]
    fn parse_simple_machine() {
        let src = r#"
            event Ping : machine;
            event Pong;

            machine Main {
                var pong: machine;
                start state Init {
                    entry {
                        pong = new Pong();
                        send pong, Ping, this;
                    }
                    on Pong goto Done;
                }
                state Done {}
            }
        "#;
        let prog = parse_source(src).expect("should parse");
        assert_eq!(prog.decls.len(), 3);
    }

    #[test]
    fn parse_spec_monitor() {
        let src = r#"
            event E;
            spec S observes E {
                start state Init {
                    on E do {
                        assert true;
                    }
                }
            }
        "#;
        let prog = parse_source(src).expect("should parse");
        assert_eq!(prog.decls.len(), 2);
    }

    #[test]
    fn parse_all_correct_and_dynamic_error_testdata() {
        let testdata = Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata");
        let mut count = 0;
        let mut failures = Vec::new();

        for entry in walkdir(&testdata) {
            if entry.extension().is_some_and(|e| e == "p") {
                let path_str = entry.to_string_lossy();
                // Skip StaticError files — many have intentional parse errors
                if path_str.contains("StaticError") {
                    continue;
                }
                count += 1;
                let source = std::fs::read_to_string(&entry).unwrap();
                if let Err(e) = parse_source(&source) {
                    failures.push(format!("{}: {e}", entry.display()));
                }
            }
        }

        assert!(count > 250, "should find 250+ non-StaticError .p files, found {count}");
        if !failures.is_empty() {
            panic!(
                "{} of {count} files failed to parse:\n{}",
                failures.len(),
                failures.join("\n")
            );
        }
    }

    fn walkdir(dir: &Path) -> Vec<std::path::PathBuf> {
        let mut files = Vec::new();
        if dir.is_dir() {
            for entry in std::fs::read_dir(dir).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    files.extend(walkdir(&path));
                } else {
                    files.push(path);
                }
            }
        }
        files
    }
}
