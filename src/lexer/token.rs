use std::fmt;

/// Represents a source span for error reporting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    pub fn merge(&self, other: &Span) -> Span {
        Span {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }
}

/// All token kinds in the Curium v5 language.
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // ── Single-character tokens ──────────────────────────────────────
    LParen,       // (
    RParen,       // )
    LBrace,       // {
    RBrace,       // }
    LBracket,     // [
    RBracket,     // ]
    Semi,         // ;
    Comma,        // ,
    Colon,        // :
    Dot,          // .
    At,           // @
    Dollar,       // $
    Question,     // ?
    Bang,         // !
    Hash,         // #
    Tilde,        // ~

    // ── Operators ────────────────────────────────────────────────────
    Plus,           // +
    Minus,          // -
    Star,           // *
    Slash,          // /
    Percent,        // %
    Equal,          // =
    EqualEqual,     // ==
    BangEqual,      // !=
    Lt,             // <
    Gt,             // >
    LtEqual,        // <=
    GtEqual,        // >=
    AndAnd,         // &&
    PipePipe,       // ||
    Ampersand,      // &
    Pipe,           // |
    Caret,          // ^
    Arrow,          // ->
    FatArrow,       // =>
    ColonEqual,     // :=
    DoubleQuestion, // ??
    DoubleColon,    // ::
    PlusEqual,      // +=
    MinusEqual,     // -=
    StarEqual,      // *=
    SlashEqual,     // /=
    PercentEqual,   // %=
    DotDot,         // ..

    // ── Literals ─────────────────────────────────────────────────────
    Identifier(String),
    StringLiteral(String),
    NumberLiteral(String),
    CharLiteral(char),

    // ── Keywords ─────────────────────────────────────────────────────
    // Control flow
    KwFn,
    KwLet,
    KwMut,
    KwReturn,
    KwIf,
    KwElse,
    KwWhile,
    KwFor,
    KwLoop,
    KwBreak,
    KwContinue,
    KwIn,

    // Literals / constants
    KwTrue,
    KwFalse,
    KwNull,

    // Types
    KwString,
    KwVoid,
    KwDyn,
    KwI8,
    KwI16,
    KwI32,
    KwI64,
    KwU8,
    KwU16,
    KwU32,
    KwU64,
    KwF32,
    KwF64,
    KwUsize,
    KwBool,
    KwChar,
    KwStr,
    KwStrnum,
    KwPtr,

    // OOP / data
    KwStruct,
    KwEnum,
    KwUnion,
    KwTrait,
    KwImpl,
    KwClass,
    KwInterface,
    KwImplements,
    KwExtends,
    KwNew,
    KwSelf_,
    KwGet,
    KwSet,
    KwStatic,
    KwPub,

    // Pattern matching
    KwMatch,

    // Modules
    KwImport,
    KwModule,
    KwPackage,
    KwUsing,
    KwNamespace,
    KwFrom,
    KwRequire,

    // Error handling
    KwTry,
    KwCatch,
    KwThrow,
    KwFinally,

    // Concurrency
    KwAsync,
    KwAwait,
    KwTask,
    KwSpawn,
    KwCall,

    // Memory
    KwReactor,
    KwArena,
    KwManual,
    KwGc,
    KwGcCollect,
    KwMalloc,
    KwFree,

    // I/O
    KwPrint,
    KwPrintln,

    // ── Special tokens ───────────────────────────────────────────────
    CBlock(String),       // c { ... }
    CppBlock(String),     // cpp { ... }
    HashAttr(String),     // #[attr]
    Comment(String),      // // or /* */

    // ── Sentinel ─────────────────────────────────────────────────────
    Eof,
}

impl TokenKind {
    /// Returns `true` if this token is a keyword.
    #[allow(dead_code)]
    pub fn is_keyword(&self) -> bool {
        matches!(
            self,
            TokenKind::KwFn
                | TokenKind::KwLet
                | TokenKind::KwMut
                | TokenKind::KwReturn
                | TokenKind::KwIf
                | TokenKind::KwElse
                | TokenKind::KwWhile
                | TokenKind::KwFor
                | TokenKind::KwLoop
                | TokenKind::KwBreak
                | TokenKind::KwContinue
                | TokenKind::KwIn
                | TokenKind::KwTrue
                | TokenKind::KwFalse
                | TokenKind::KwNull
                | TokenKind::KwString
                | TokenKind::KwVoid
                | TokenKind::KwDyn
                | TokenKind::KwStruct
                | TokenKind::KwEnum
                | TokenKind::KwUnion
                | TokenKind::KwTrait
                | TokenKind::KwImpl
                | TokenKind::KwMatch
                | TokenKind::KwImport
                | TokenKind::KwModule
                | TokenKind::KwPub
                | TokenKind::KwTry
                | TokenKind::KwCatch
                | TokenKind::KwThrow
                | TokenKind::KwFinally
                | TokenKind::KwAsync
                | TokenKind::KwAwait
                | TokenKind::KwTask
                | TokenKind::KwSpawn
                | TokenKind::KwCall
                | TokenKind::KwReactor
                | TokenKind::KwArena
                | TokenKind::KwManual
                | TokenKind::KwNew
                | TokenKind::KwPtr
                | TokenKind::KwStr
                | TokenKind::KwStrnum
                | TokenKind::KwClass
                | TokenKind::KwInterface
                | TokenKind::KwImplements
                | TokenKind::KwExtends
                | TokenKind::KwGet
                | TokenKind::KwSet
                | TokenKind::KwStatic
                | TokenKind::KwPackage
                | TokenKind::KwUsing
                | TokenKind::KwNamespace
                | TokenKind::KwFrom
                | TokenKind::KwRequire
                | TokenKind::KwGc
                | TokenKind::KwGcCollect
                | TokenKind::KwMalloc
                | TokenKind::KwFree
                | TokenKind::KwPrint
                | TokenKind::KwPrintln
                | TokenKind::KwSelf_
                | TokenKind::KwI8
                | TokenKind::KwI16
                | TokenKind::KwI32
                | TokenKind::KwI64
                | TokenKind::KwU8
                | TokenKind::KwU16
                | TokenKind::KwU32
                | TokenKind::KwU64
                | TokenKind::KwF32
                | TokenKind::KwF64
                | TokenKind::KwUsize
                | TokenKind::KwBool
                | TokenKind::KwChar
        )
    }
}

impl fmt::Display for TokenKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenKind::LParen => write!(f, "("),
            TokenKind::RParen => write!(f, ")"),
            TokenKind::LBrace => write!(f, "{{"),
            TokenKind::RBrace => write!(f, "}}"),
            TokenKind::LBracket => write!(f, "["),
            TokenKind::RBracket => write!(f, "]"),
            TokenKind::Semi => write!(f, ";"),
            TokenKind::Comma => write!(f, ","),
            TokenKind::Colon => write!(f, ":"),
            TokenKind::Dot => write!(f, "."),
            TokenKind::At => write!(f, "@"),
            TokenKind::Dollar => write!(f, "$"),
            TokenKind::Question => write!(f, "?"),
            TokenKind::Bang => write!(f, "!"),
            TokenKind::Hash => write!(f, "#"),
            TokenKind::Tilde => write!(f, "~"),
            TokenKind::Plus => write!(f, "+"),
            TokenKind::Minus => write!(f, "-"),
            TokenKind::Star => write!(f, "*"),
            TokenKind::Slash => write!(f, "/"),
            TokenKind::Percent => write!(f, "%"),
            TokenKind::Equal => write!(f, "="),
            TokenKind::EqualEqual => write!(f, "=="),
            TokenKind::BangEqual => write!(f, "!="),
            TokenKind::Lt => write!(f, "<"),
            TokenKind::Gt => write!(f, ">"),
            TokenKind::LtEqual => write!(f, "<="),
            TokenKind::GtEqual => write!(f, ">="),
            TokenKind::AndAnd => write!(f, "&&"),
            TokenKind::PipePipe => write!(f, "||"),
            TokenKind::Ampersand => write!(f, "&"),
            TokenKind::Pipe => write!(f, "|"),
            TokenKind::Caret => write!(f, "^"),
            TokenKind::Arrow => write!(f, "->"),
            TokenKind::FatArrow => write!(f, "=>"),
            TokenKind::ColonEqual => write!(f, ":="),
            TokenKind::DoubleQuestion => write!(f, "??"),
            TokenKind::DoubleColon => write!(f, "::"),
            TokenKind::PlusEqual => write!(f, "+="),
            TokenKind::MinusEqual => write!(f, "-="),
            TokenKind::StarEqual => write!(f, "*="),
            TokenKind::SlashEqual => write!(f, "/="),
            TokenKind::PercentEqual => write!(f, "%="),
            TokenKind::DotDot => write!(f, ".."),
            TokenKind::Identifier(s) => write!(f, "{}", s),
            TokenKind::StringLiteral(s) => write!(f, "\"{}\"", s),
            TokenKind::NumberLiteral(s) => write!(f, "{}", s),
            TokenKind::CharLiteral(c) => write!(f, "'{}'", c),
            TokenKind::CBlock(s) => write!(f, "c {{ {} }}", s),
            TokenKind::CppBlock(s) => write!(f, "cpp {{ {} }}", s),
            TokenKind::HashAttr(s) => write!(f, "#[{}]", s),
            TokenKind::Comment(s) => write!(f, "// {}", s),
            TokenKind::Eof => write!(f, "<EOF>"),
            // Keywords
            kw => {
                let s = match kw {
                    TokenKind::KwFn => "fn",
                    TokenKind::KwLet => "let",
                    TokenKind::KwMut => "mut",
                    TokenKind::KwReturn => "return",
                    TokenKind::KwIf => "if",
                    TokenKind::KwElse => "else",
                    TokenKind::KwWhile => "while",
                    TokenKind::KwFor => "for",
                    TokenKind::KwLoop => "loop",
                    TokenKind::KwBreak => "break",
                    TokenKind::KwContinue => "continue",
                    TokenKind::KwIn => "in",
                    TokenKind::KwTrue => "true",
                    TokenKind::KwFalse => "false",
                    TokenKind::KwNull => "null",
                    TokenKind::KwString => "string",
                    TokenKind::KwVoid => "void",
                    TokenKind::KwDyn => "dyn",
                    TokenKind::KwI8 => "i8",
                    TokenKind::KwI16 => "i16",
                    TokenKind::KwI32 => "i32",
                    TokenKind::KwI64 => "i64",
                    TokenKind::KwU8 => "u8",
                    TokenKind::KwU16 => "u16",
                    TokenKind::KwU32 => "u32",
                    TokenKind::KwU64 => "u64",
                    TokenKind::KwF32 => "f32",
                    TokenKind::KwF64 => "f64",
                    TokenKind::KwUsize => "usize",
                    TokenKind::KwBool => "bool",
                    TokenKind::KwChar => "char",
                    TokenKind::KwStr => "str",
                    TokenKind::KwStrnum => "strnum",
                    TokenKind::KwPtr => "ptr",
                    TokenKind::KwStruct => "struct",
                    TokenKind::KwEnum => "enum",
                    TokenKind::KwUnion => "union",
                    TokenKind::KwTrait => "trait",
                    TokenKind::KwImpl => "impl",
                    TokenKind::KwClass => "class",
                    TokenKind::KwInterface => "interface",
                    TokenKind::KwImplements => "implements",
                    TokenKind::KwExtends => "extends",
                    TokenKind::KwNew => "new",
                    TokenKind::KwSelf_ => "self",
                    TokenKind::KwGet => "get",
                    TokenKind::KwSet => "set",
                    TokenKind::KwStatic => "static",
                    TokenKind::KwPub => "pub",
                    TokenKind::KwMatch => "match",
                    TokenKind::KwImport => "import",
                    TokenKind::KwModule => "module",
                    TokenKind::KwPackage => "package",
                    TokenKind::KwUsing => "using",
                    TokenKind::KwNamespace => "namespace",
                    TokenKind::KwFrom => "from",
                    TokenKind::KwRequire => "require",
                    TokenKind::KwTry => "try",
                    TokenKind::KwCatch => "catch",
                    TokenKind::KwThrow => "throw",
                    TokenKind::KwFinally => "finally",
                    TokenKind::KwAsync => "async",
                    TokenKind::KwAwait => "await",
                    TokenKind::KwTask => "task",
                    TokenKind::KwSpawn => "spawn",
                    TokenKind::KwCall => "call",
                    TokenKind::KwReactor => "reactor",
                    TokenKind::KwArena => "arena",
                    TokenKind::KwManual => "manual",
                    TokenKind::KwGc => "gc",
                    TokenKind::KwGcCollect => "gc_collect",
                    TokenKind::KwMalloc => "malloc",
                    TokenKind::KwFree => "free",
                    TokenKind::KwPrint => "print",
                    TokenKind::KwPrintln => "println",
                    _ => unreachable!(),
                };
                write!(f, "{}", s)
            }
        }
    }
}

/// A single token produced by the lexer.
#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub line: usize,
    pub column: usize,
    pub span: Span,
}

impl Token {
    pub fn new(kind: TokenKind, line: usize, column: usize, span: Span) -> Self {
        Self {
            kind,
            line,
            column,
            span,
        }
    }

    /// Returns `true` if the token is the EOF sentinel.
    #[allow(dead_code)]
    pub fn is_eof(&self) -> bool {
        self.kind == TokenKind::Eof
    }
}
