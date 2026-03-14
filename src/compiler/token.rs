use logos::Logos;
use std::fmt;

/// Byte-offset span in source code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
}

/// A token with its span.
#[derive(Debug, Clone)]
pub struct SpannedToken {
    pub kind: Token,
    pub span: Span,
}

// ---------- Token enum (matches PLexer.g4) ----------

#[derive(Logos, Debug, Clone, PartialEq)]
#[logos(skip r"[ \t\r\n\f]+")]
#[logos(skip(r"//[^\r\n]*", allow_greedy = true))]
#[logos(skip r"/\*[^*]*\*+(?:[^/*][^*]*\*+)*/")]
pub enum Token {
    // ---- Type keywords ----
    #[token("any")]
    Any,
    #[token("bool")]
    Bool,
    #[token("enum")]
    Enum,
    #[token("event")]
    Event,
    #[token("eventset")]
    EventSet,
    #[token("float")]
    Float,
    #[token("int")]
    Int,
    #[token("machine")]
    Machine,
    #[token("interface")]
    Interface,
    #[token("map")]
    Map,
    #[token("set")]
    Set,
    #[token("string")]
    StringType,
    #[token("seq")]
    Seq,
    #[token("data")]
    Data,

    // ---- Language keywords ----
    #[token("announce")]
    Announce,
    #[token("as")]
    As,
    #[token("assert")]
    Assert,
    #[token("assume")]
    Assume,
    #[token("break")]
    Break,
    #[token("case")]
    Case,
    #[token("cold")]
    Cold,
    #[token("continue")]
    Continue,
    #[token("default")]
    Default,
    #[token("defer")]
    Defer,
    #[token("do")]
    Do,
    #[token("else")]
    Else,
    #[token("entry")]
    Entry,
    #[token("exit")]
    Exit,
    #[token("foreach")]
    Foreach,
    #[token("format")]
    Format,
    #[token("fun")]
    Fun,
    #[token("goto")]
    Goto,
    #[token("halt")]
    Halt,
    #[token("hot")]
    Hot,
    #[token("if")]
    If,
    #[token("ignore")]
    Ignore,
    #[token("in")]
    In,
    #[token("keys")]
    Keys,
    #[token("new")]
    New,
    #[token("observes")]
    Observes,
    #[token("on")]
    On,
    #[token("print")]
    Print,
    #[token("raise")]
    Raise,
    #[token("receive")]
    Receive,
    #[token("return")]
    Return,
    #[token("send")]
    Send,
    #[token("sizeof")]
    Sizeof,
    #[token("spec")]
    Spec,
    #[token("start")]
    Start,
    #[token("state")]
    State,
    #[token("this")]
    This,
    #[token("type")]
    Type,
    #[token("values")]
    Values,
    #[token("var")]
    Var,
    #[token("while")]
    While,
    #[token("with")]
    With,
    #[token("choose")]
    Choose,

    // ---- Module system keywords ----
    #[token("module")]
    Module,
    #[token("implementation")]
    Implementation,
    #[token("test")]
    Test,
    #[token("refines")]
    Refines,
    #[token("compose")]
    Compose,
    #[token("union")]
    Union,
    #[token("hidee")]
    HideE,
    #[token("hidei")]
    HideI,
    #[token("rename")]
    Rename,
    #[token("safe")]
    Safe,
    #[token("main")]
    Main,
    #[token("receives")]
    Receives,
    #[token("sends")]
    Sends,
    #[token("creates")]
    Creates,
    #[token("to")]
    To,

    // ---- PVerifier keywords (parse but mostly ignore) ----
    #[token("invariant")]
    Invariant,
    #[token("axiom")]
    Axiom,
    #[token("is")]
    Is,
    #[token("inflight")]
    InFlight,
    #[token("targets")]
    Targets,
    #[token("sent")]
    Sent,
    #[token("Proof")]
    Proof,
    #[token("prove")]
    Prove,
    #[token("using")]
    Using,
    #[token("Lemma")]
    Lemma,
    #[token("Theorem")]
    Theorem,
    #[token("except")]
    Except,
    #[token("requires")]
    Requires,
    #[token("ensures")]
    Ensures,
    #[token("forall")]
    Forall,
    #[token("exists")]
    Exists,
    #[token("init-condition")]
    InitCondition,
    #[token("pure")]
    Pure,
    #[token("param")]
    Param,
    #[token("pairwise")]
    Pairwise,
    #[token("wise")]
    Wise,
    #[token("paramtest")]
    ParamTest,

    // ---- Literals ----
    #[token("true")]
    True,
    #[token("false")]
    False,
    #[regex("[0-9]+")]
    IntLiteral,
    #[token("null")]
    Null,
    #[regex(r#""([^"\\]|\\.)*""#)]
    StringLiteral,

    // ---- Symbols (longest match first via ordering) ----
    #[token("$$")]
    FairNondet,
    #[token("$")]
    Nondet,

    #[token("<==>")]
    LIff,
    #[token("==>")]
    LThen,

    #[token("!")]
    LNot,
    #[token("&&")]
    LAnd,
    #[token("||")]
    LOr,

    #[token("==")]
    Eq,
    #[token("!=")]
    Ne,
    #[token("<=")]
    Le,
    #[token(">=")]
    Ge,
    #[token("<")]
    Lt,
    #[token(">")]
    Gt,
    #[token("->")]
    RArrow,

    #[token("=")]
    Assign,
    #[token("+=")]
    Insert,
    #[token("-=")]
    Remove,

    #[token("+")]
    Add,
    #[token("-")]
    Sub,
    #[token("*")]
    Mul,
    #[token("/")]
    Div,
    #[token("%")]
    Mod,

    #[token("{")]
    LBrace,
    #[token("}")]
    RBrace,
    #[token("[")]
    LBrack,
    #[token("]")]
    RBrack,
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token(";")]
    Semi,
    #[token(",")]
    Comma,
    #[token(".")]
    Dot,
    #[token(":")]
    Colon,
    #[token("::")]
    ColonColon,

    // ---- Identifiers ----
    #[regex("[a-zA-Z_][a-zA-Z0-9_]*")]
    Iden,
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Token::IntLiteral => write!(f, "integer"),
            Token::StringLiteral => write!(f, "string"),
            Token::Iden => write!(f, "identifier"),
            Token::Semi => write!(f, "';'"),
            Token::Comma => write!(f, "','"),
            Token::LParen => write!(f, "'('"),
            Token::RParen => write!(f, "')'"),
            Token::LBrace => write!(f, "'{{'"),
            Token::RBrace => write!(f, "'}}'"),
            Token::LBrack => write!(f, "'['"),
            Token::RBrack => write!(f, "']'"),
            Token::Colon => write!(f, "':'"),
            Token::Assign => write!(f, "'='"),
            Token::Dot => write!(f, "'.'"),
            other => write!(f, "{other:?}"),
        }
    }
}
