use crate::lexer::Span;

// ── Type System ──────────────────────────────────────────────────────────────

/// Curium type representation.
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    I8,
    I16,
    I32,
    I64,
    U8,
    U16,
    U32,
    U64,
    F32,
    F64,
    Usize,
    Bool,
    Char,
    String,
    Str,
    Void,
    Dyn,
    Strnum,
    Ptr(Box<Type>),
    Array(Box<Type>),
    Slice(Box<Type>),
    Named(String),
    Generic(String, Vec<Type>),
    Function {
        params: Vec<Type>,
        ret: Box<Type>,
    },
    Optional(Box<Type>),
    Inferred, // placeholder for type inference
}

impl std::fmt::Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::I8 => write!(f, "i8"),
            Type::I16 => write!(f, "i16"),
            Type::I32 => write!(f, "i32"),
            Type::I64 => write!(f, "i64"),
            Type::U8 => write!(f, "u8"),
            Type::U16 => write!(f, "u16"),
            Type::U32 => write!(f, "u32"),
            Type::U64 => write!(f, "u64"),
            Type::F32 => write!(f, "f32"),
            Type::F64 => write!(f, "f64"),
            Type::Usize => write!(f, "usize"),
            Type::Bool => write!(f, "bool"),
            Type::Char => write!(f, "char"),
            Type::String => write!(f, "string"),
            Type::Str => write!(f, "str"),
            Type::Void => write!(f, "void"),
            Type::Dyn => write!(f, "dyn"),
            Type::Strnum => write!(f, "strnum"),
            Type::Ptr(inner) => write!(f, "^{}", inner),
            Type::Array(inner) => write!(f, "[]{}", inner),
            Type::Slice(inner) => write!(f, "&[]{}", inner),
            Type::Named(name) => write!(f, "{}", name),
            Type::Generic(name, args) => {
                write!(f, "{}<", name)?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", arg)?;
                }
                write!(f, ">")
            }
            Type::Function { params, ret } => {
                write!(f, "fn(")?;
                for (i, p) in params.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", p)?;
                }
                write!(f, ") -> {}", ret)
            }
            Type::Optional(inner) => write!(f, "?{}", inner),
            Type::Inferred => write!(f, "_"),
        }
    }
}

// ── AST Node ─────────────────────────────────────────────────────────────────

/// A single AST node with location info.
#[derive(Debug, Clone)]
pub struct AstNode {
    pub kind: AstKind,
    pub span: Span,
}

impl AstNode {
    pub fn new(kind: AstKind, span: Span) -> Self {
        Self { kind, span }
    }
}

// ── Supporting structures ────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub ty: Type,
    pub mutable: bool,
}

#[derive(Debug, Clone)]
pub struct Field {
    pub name: String,
    pub ty: Type,
    pub is_pub: bool,
}

#[derive(Debug, Clone)]
pub struct EnumVariant {
    pub name: String,
    pub fields: Vec<Type>,
}

#[derive(Debug, Clone)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub body: Box<AstNode>,
}

#[derive(Debug, Clone)]
pub enum Pattern {
    Literal(Box<AstNode>),
    Identifier(String),
    EnumVariant {
        path: Vec<String>,
        bindings: Vec<String>,
    },
    Wildcard,
}

#[derive(Debug, Clone)]
pub struct CatchArm {
    pub error_type: Option<Type>,
    pub binding: String,
    pub body: Box<AstNode>,
}

#[derive(Debug, Clone)]
pub enum AllocatorKind {
    Arena,
    Manual,
    Gc,
}

// ── Binary / Unary operators ────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum BinOp {
    Add,      // +
    Sub,      // -
    Mul,      // *
    Div,      // /
    Mod,      // %
    Eq,       // ==
    Neq,      // !=
    Lt,       // <
    Gt,       // >
    LtEq,    // <=
    GtEq,    // >=
    And,      // &&
    Or,       // ||
    BitAnd,   // &
    BitOr,    // |
    BitXor,   // ^
    Range,    // ..
}

impl std::fmt::Display for BinOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BinOp::Add => write!(f, "+"),
            BinOp::Sub => write!(f, "-"),
            BinOp::Mul => write!(f, "*"),
            BinOp::Div => write!(f, "/"),
            BinOp::Mod => write!(f, "%"),
            BinOp::Eq => write!(f, "=="),
            BinOp::Neq => write!(f, "!="),
            BinOp::Lt => write!(f, "<"),
            BinOp::Gt => write!(f, ">"),
            BinOp::LtEq => write!(f, "<="),
            BinOp::GtEq => write!(f, ">="),
            BinOp::And => write!(f, "&&"),
            BinOp::Or => write!(f, "||"),
            BinOp::BitAnd => write!(f, "&"),
            BinOp::BitOr => write!(f, "|"),
            BinOp::BitXor => write!(f, "^"),
            BinOp::Range => write!(f, ".."),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum UnaryOp {
    Neg,    // -
    Not,    // !
    Deref,  // ^
    AddrOf, // &
    BitNot, // ~
}

impl std::fmt::Display for UnaryOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UnaryOp::Neg => write!(f, "-"),
            UnaryOp::Not => write!(f, "!"),
            UnaryOp::Deref => write!(f, "^"),
            UnaryOp::AddrOf => write!(f, "&"),
            UnaryOp::BitNot => write!(f, "~"),
        }
    }
}

// ── Assignment operators ────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum AssignOp {
    Assign,    // =
    AddAssign, // +=
    SubAssign, // -=
    MulAssign, // *=
    DivAssign, // /=
    ModAssign, // %=
}

// ── AST Kind ─────────────────────────────────────────────────────────────────

/// All possible AST node kinds in the Curium language.
#[derive(Debug, Clone)]
pub enum AstKind {
    // ── Top-Level ────────────────────────────────────────────────
    Program(Vec<AstNode>),

    // ── Declarations ─────────────────────────────────────────────
    FnDecl {
        name: String,
        params: Vec<Param>,
        return_type: Option<Type>,
        body: Box<AstNode>,
        is_pub: bool,
        attributes: Vec<String>,
    },
    LetDecl {
        name: String,
        type_annotation: Option<Type>,
        init: Option<Box<AstNode>>,
        mutable: bool,
    },
    StructDecl {
        name: String,
        fields: Vec<Field>,
        is_pub: bool,
    },
    EnumDecl {
        name: String,
        variants: Vec<EnumVariant>,
        is_pub: bool,
    },
    UnionDecl {
        name: String,
        fields: Vec<Field>,
    },
    TraitDecl {
        name: String,
        methods: Vec<AstNode>,
        is_pub: bool,
    },
    ImplBlock {
        trait_name: Option<String>,
        target: String,
        methods: Vec<AstNode>,
    },
    ImportDecl {
        path: String,
        alias: Option<String>,
    },
    ModuleDecl {
        name: String,
    },

    // ── Statements ───────────────────────────────────────────────
    ExprStmt(Box<AstNode>),
    ReturnStmt(Option<Box<AstNode>>),
    BreakStmt,
    ContinueStmt,
    Block(Vec<AstNode>),

    IfStmt {
        condition: Box<AstNode>,
        then_branch: Box<AstNode>,
        else_branch: Option<Box<AstNode>>,
    },
    WhileStmt {
        condition: Box<AstNode>,
        body: Box<AstNode>,
    },
    ForStmt {
        variable: String,
        iterable: Box<AstNode>,
        body: Box<AstNode>,
    },
    LoopStmt {
        body: Box<AstNode>,
    },
    MatchStmt {
        expr: Box<AstNode>,
        arms: Vec<MatchArm>,
    },
    TryBlock {
        body: Box<AstNode>,
        catch_arms: Vec<CatchArm>,
        finally_block: Option<Box<AstNode>>,
    },
    ThrowStmt(Box<AstNode>),
    ReactorBlock {
        allocator: AllocatorKind,
        size: Option<Box<AstNode>>,
        body: Box<AstNode>,
    },
    SpawnBlock(Box<AstNode>),
    CBlock(String),
    CppBlock(String),

    // ── Expressions ──────────────────────────────────────────────
    BinaryExpr {
        op: BinOp,
        left: Box<AstNode>,
        right: Box<AstNode>,
    },
    UnaryExpr {
        op: UnaryOp,
        expr: Box<AstNode>,
    },
    Assignment {
        op: AssignOp,
        target: Box<AstNode>,
        value: Box<AstNode>,
    },
    Call {
        callee: Box<AstNode>,
        args: Vec<AstNode>,
    },
    MemberAccess {
        object: Box<AstNode>,
        field: String,
    },
    Index {
        object: Box<AstNode>,
        index: Box<AstNode>,
    },
    FieldInit {
        name: String,
        value: Box<AstNode>,
    },
    StructLiteral {
        name: String,
        fields: Vec<AstNode>,
    },
    ArrayLiteral(Vec<AstNode>),
    PathExpr(Vec<String>),
    CastExpr {
        expr: Box<AstNode>,
        target_type: Type,
    },
    TryExpr(Box<AstNode>), // expr?

    DynOperator {
        op_var: Box<AstNode>,
        cases: Vec<(Box<AstNode>, Box<AstNode>)>,
        fallback: Box<AstNode>,
    },

    // ── Literals ─────────────────────────────────────────────────
    Identifier(String),
    NumberLiteral(String),
    StringLiteral(String),
    CharLiteral(char),
    BoolLiteral(bool),
    NullLiteral,
    SelfLiteral,
}
