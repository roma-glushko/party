//! Untyped AST for the P language, closely matching PParser.g4.

use super::token::Span;

pub type Iden = String;

/// A complete P program is a list of top-level declarations.
#[derive(Debug, Clone)]
pub struct Program {
    pub decls: Vec<TopDecl>,
}

// ---------- Top-level declarations ----------

#[derive(Debug, Clone)]
pub enum TopDecl {
    TypeDef(TypeDefDecl),
    EnumTypeDef(EnumTypeDefDecl),
    EventDecl(EventDecl),
    EventSetDecl(EventSetDecl),
    InterfaceDecl(InterfaceDecl),
    MachineDecl(MachineDecl),
    SpecMachineDecl(MachineDecl),
    FunDecl(FunDecl),
    ModuleDecl(ModuleDecl),
    TestDecl(TestDecl),
    ImplementationDecl(ImplementationDecl),
    GlobalParamDecl(GlobalParamDecl),
}

#[derive(Debug, Clone)]
pub struct TypeDefDecl {
    pub name: Iden,
    pub ty: Option<PType>, // None = foreign type
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct EnumTypeDefDecl {
    pub name: Iden,
    pub elements: Vec<EnumElem>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct EnumElem {
    pub name: Iden,
    pub value: Option<i64>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct EventDecl {
    pub name: Iden,
    pub payload: Option<PType>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct EventSetDecl {
    pub name: Iden,
    pub events: Vec<Iden>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct InterfaceDecl {
    pub name: Iden,
    pub payload: Option<PType>,
    pub receives: Option<Vec<Iden>>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct MachineDecl {
    pub name: Iden,
    pub is_spec: bool,
    pub observes: Option<Vec<Iden>>,
    pub receives: Option<Vec<Iden>>,
    pub sends: Option<Vec<Iden>>,
    pub body: MachineBody,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct MachineBody {
    pub vars: Vec<VarDecl>,
    pub funs: Vec<FunDecl>,
    pub states: Vec<StateDecl>,
}

#[derive(Debug, Clone)]
pub struct VarDecl {
    pub names: Vec<Iden>,
    pub ty: PType,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct FunDecl {
    pub name: Iden,
    pub params: Vec<FunParam>,
    pub ret_type: Option<PType>,
    pub body: Option<FunctionBody>,
    pub is_foreign: bool,
    pub creates: Option<Iden>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct FunParam {
    pub name: Iden,
    pub ty: PType,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct FunctionBody {
    pub var_decls: Vec<VarDecl>,
    pub stmts: Vec<Stmt>,
    pub span: Span,
}

// ---------- States ----------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Temperature {
    Hot,
    Cold,
}

#[derive(Debug, Clone)]
pub struct StateDecl {
    pub is_start: bool,
    pub temperature: Option<Temperature>,
    pub name: Iden,
    pub items: Vec<StateBodyItem>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum StateBodyItem {
    Entry(EntryExit),
    Exit(EntryExit),
    Defer(Vec<Iden>, Span),
    Ignore(Vec<Iden>, Span),
    OnEventDoAction(OnEventDoAction),
    OnEventGotoState(OnEventGotoState),
}

#[derive(Debug, Clone)]
pub struct EntryExit {
    pub fun_name: Option<Iden>,
    pub anon_handler: Option<AnonEventHandler>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct AnonEventHandler {
    pub param: Option<FunParam>,
    pub body: FunctionBody,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct OnEventDoAction {
    pub events: Vec<Iden>,
    pub fun_name: Option<Iden>,
    pub anon_handler: Option<AnonEventHandler>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct OnEventGotoState {
    pub events: Vec<Iden>,
    pub target: Iden,
    pub with_fun_name: Option<Iden>,
    pub with_anon_handler: Option<AnonEventHandler>,
    pub span: Span,
}

// ---------- Module system ----------

#[derive(Debug, Clone)]
pub struct ModuleDecl {
    pub name: Iden,
    pub expr: ModExpr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum ModExpr {
    Paren(Box<ModExpr>),
    Primitive(Vec<BindExpr>),
    Named(Iden),
    Compose(Vec<ModExpr>),
    Union(Vec<ModExpr>),
    HideEvents(Vec<Iden>, Box<ModExpr>),
    HideInterfaces(Vec<Iden>, Box<ModExpr>),
    AssertMod(Vec<Iden>, Box<ModExpr>),
    Rename(Iden, Iden, Box<ModExpr>),
    MainMachine(Iden, Box<ModExpr>),
}

#[derive(Debug, Clone)]
pub struct BindExpr {
    pub machine: Iden,
    pub interface: Option<Iden>,
}

#[derive(Debug, Clone)]
pub struct TestDecl {
    pub name: Iden,
    pub main_machine: Option<Iden>,
    pub module_expr: ModExpr,
    pub params: Option<Vec<ParamRange>>,
    pub assume_expr: Option<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ParamRange {
    pub name: Iden,
    pub values: Vec<i64>,
}

#[derive(Debug, Clone)]
pub struct ImplementationDecl {
    pub name: Iden,
    pub main_machine: Option<Iden>,
    pub module_expr: ModExpr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct GlobalParamDecl {
    pub names: Vec<Iden>,
    pub ty: PType,
    pub span: Span,
}

// ---------- Types ----------

#[derive(Debug, Clone)]
pub enum PType {
    Bool,
    Int,
    Float,
    StringType,
    Event,
    Machine,
    Data,
    Any,
    Seq(Box<PType>),
    Set(Box<PType>),
    Map(Box<PType>, Box<PType>),
    Tuple(Vec<PType>),
    NamedTuple(Vec<(Iden, PType)>),
    Named(Iden),
}

// ---------- Statements ----------

#[derive(Debug, Clone)]
pub enum Stmt {
    Compound(Vec<Stmt>, Span),
    Assert {
        expr: Expr,
        message: Option<Expr>,
        span: Span,
    },
    Assume {
        expr: Expr,
        message: Option<Expr>,
        span: Span,
    },
    Print {
        message: Expr,
        span: Span,
    },
    Return {
        value: Option<Expr>,
        span: Span,
    },
    Break(Span),
    Continue(Span),
    Assign {
        lvalue: LValue,
        rvalue: Expr,
        span: Span,
    },
    Insert {
        lvalue: LValue,
        index: Expr,
        value: Expr,
        span: Span,
    },
    AddToSet {
        lvalue: LValue,
        value: Expr,
        span: Span,
    },
    Remove {
        lvalue: LValue,
        key: Expr,
        span: Span,
    },
    While {
        cond: Expr,
        body: Box<Stmt>,
        span: Span,
    },
    Foreach {
        item: Iden,
        collection: Expr,
        body: Box<Stmt>,
        span: Span,
    },
    If {
        cond: Expr,
        then_branch: Box<Stmt>,
        else_branch: Option<Box<Stmt>>,
        span: Span,
    },
    CtorStmt {
        interface: Iden,
        args: Vec<Expr>,
        span: Span,
    },
    FunCall {
        name: Iden,
        args: Vec<Expr>,
        span: Span,
    },
    Raise {
        event: Expr,
        args: Vec<Expr>,
        span: Span,
    },
    Send {
        target: Expr,
        event: Expr,
        args: Vec<Expr>,
        span: Span,
    },
    Announce {
        event: Expr,
        args: Vec<Expr>,
        span: Span,
    },
    Goto {
        state: Iden,
        payload: Vec<Expr>,
        span: Span,
    },
    Receive {
        cases: Vec<RecvCase>,
        span: Span,
    },
    NoStmt(Span),
}

#[derive(Debug, Clone)]
pub struct RecvCase {
    pub events: Vec<Iden>,
    pub handler: AnonEventHandler,
    pub span: Span,
}

// ---------- LValues ----------

#[derive(Debug, Clone)]
pub enum LValue {
    Var(Iden, Span),
    NamedTupleField(Box<LValue>, Iden, Span),
    TupleField(Box<LValue>, usize, Span),
    Index(Box<LValue>, Expr, Span),
}

// ---------- Expressions ----------

#[derive(Debug, Clone)]
pub enum Expr {
    // Literals
    IntLit(i64, Span),
    FloatLit(f64, Span),
    BoolLit(bool, Span),
    StringLit(String, Span),
    NullLit(Span),
    This(Span),
    HaltEvent(Span),
    Nondet(Span),
    FairNondet(Span),

    // Identifiers (variable reference or enum element)
    Iden(Iden, Span),

    // Tuple construction
    UnnamedTuple(Vec<Expr>, Span),
    NamedTuple(Vec<(Iden, Expr)>, Span),

    // Field access
    NamedTupleAccess(Box<Expr>, Iden, Span),
    TupleAccess(Box<Expr>, usize, Span),

    // Collection access
    SeqMapAccess(Box<Expr>, Box<Expr>, Span),

    // Built-in functions
    Keys(Box<Expr>, Span),
    Values(Box<Expr>, Span),
    Sizeof(Box<Expr>, Span),
    Default(PType, Span),

    // Machine creation
    New(Iden, Vec<Expr>, Span),

    // Function call
    FunCall(Iden, Vec<Expr>, Span),

    // Unary
    Neg(Box<Expr>, Span),
    Not(Box<Expr>, Span),

    // Binary
    BinOp(BinOp, Box<Expr>, Box<Expr>, Span),

    // Cast
    Cast(Box<Expr>, PType, Span),

    // Choose (nondeterministic)
    Choose(Option<Box<Expr>>, Span),

    // Format string
    FormatString(String, Vec<Expr>, Span),

    // Parenthesized (removed during parsing, but useful for spans)
    Paren(Box<Expr>, Span),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
    And,
    Or,
    In,
}
