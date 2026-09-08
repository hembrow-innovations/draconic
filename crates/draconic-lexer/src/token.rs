use crate::js_string::JsString;
use draconic_diagnostics::Span;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
    /// True when a LineTerminator was skipped immediately before this token
    /// (restricted productions: postfix `++`/`--`, `continue`/`break`/`return`/`throw`).
    pub preceded_by_line_terminator: bool,
    /// True when the identifier/keyword token contained a Unicode escape (`\u…`).
    /// Contextual keywords (`get`/`set`/`async`) must not be escaped (E19.39).
    pub escaped: bool,
    /// Annex B legacy octal / NonOctalDecimal numeric or string escape (E19.69).
    /// Strict mode (and always for templates) rejects these as early SyntaxError.
    pub legacy_octal: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    // punctuators
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Semi,
    Comma,
    Dot,
    /// `...` rest/spread
    DotDotDot,
    Colon,
    /// `@` decorator (E19.78).
    At,
    Question,
    /// `?.` optional chaining punctuator (not when followed by a decimal digit).
    QuestionDot,
    QuestionQuestion,
    QuestionQuestionEq,
    // operators
    Plus,
    PlusPlus,
    PlusEq,
    Minus,
    MinusMinus,
    MinusEq,
    Star,
    StarStar,
    StarStarEq,
    StarEq,
    Slash,
    SlashEq,
    Percent,
    PercentEq,
    Bang,
    Eq,
    EqEq,
    EqEqEq,
    /// `=>` arrow function punctuator
    Arrow,
    NotEq,
    NotEqEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    AndAnd,
    AndAndEq,
    OrOr,
    OrOrEq,
    BitAnd,
    BitAndEq,
    BitOr,
    BitOrEq,
    BitXor,
    BitXorEq,
    Tilde,
    Shl,
    ShlEq,
    Shr,
    ShrEq,
    UShr,
    UShrEq,
    // keywords / atoms
    Ident(String),
    /// `#name` private identifier (name without `#`).
    PrivateIdent(String),
    Number(String),
    /// BigInt integer literal including `n` suffix (e.g. `1n`, `0xffn`).
    BigInt(String),
    String(JsString),
    /// `` `foo` `` — no `${` interpolations.
    TemplateNoSubstitution(JsString),
    /// `` `foo${ `` — cooked head before first interpolation.
    TemplateHead(JsString),
    /// `` }foo${ `` — cooked middle between interpolations.
    TemplateMiddle(JsString),
    /// `` }foo` `` — cooked tail after last interpolation.
    TemplateTail(JsString),
    True,
    False,
    Null,
    Let,
    Const,
    Var,
    TypeOf,
    Void,
    Delete,
    If,
    Else,
    While,
    Do,
    For,
    Break,
    Continue,
    Switch,
    Case,
    Default,
    In,
    InstanceOf,
    Of,
    Function,
    Async,
    Await,
    Yield,
    Return,
    This,
    New,
    Class,
    Extends,
    Super,
    Static,
    Throw,
    Try,
    Catch,
    Finally,
    With,
    Import,
    Export,
    From,
    As,
    /// `/pattern/flags` regular expression literal (pattern body without slashes).
    RegExp {
        pattern: String,
        flags: String,
    },
    // other
    Eof,
}
