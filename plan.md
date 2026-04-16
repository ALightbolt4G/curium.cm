

---

## 🦀 Complete Prompt: Curium Compiler in Rust

```markdown
# Project: Curium Rust Compiler - Path to Self-Hosting

## Overview
Rebuild the Curium v5.0 compiler in Rust with a clear path to self-hosting. 
The implementation will follow the architecture documented in INTERNALS.md, 
supporting the syntax defined in SYNTAX_REFERENCE.md and LANGUAGE_GUIDE.md.

**Repository:** https://github.com/ALightbolt4G/curium.cm

## Project Structure
```
curium-rust/
├── Cargo.toml
├── src/
│   ├── main.rs
│   ├── lexer/
│   │   ├── mod.rs
│   │   ├── token.rs      # TokenKind enum (all 80+ tokens from TokenKind)
│   │   └── scanner.rs    # Lexer implementation
│   ├── parser/
│   │   ├── mod.rs
│   │   ├── ast.rs        # AST node definitions (AstKind variants)
│   │   └── grammar.rs    # Recursive descent parser
│   ├── codegen/
│   │   ├── mod.rs
│   │   └── c_backend.rs  # Transpile to C11
│   ├── cli/
│   │   └── mod.rs        # cm init, build, run, check, doctor, fmt
│   └── error/
│       └── diagnostics.rs # Beautiful error messages with ariadne
├── tests/
│   └── integration_tests.rs
└── examples/
    ├── hello.cm
    ├── linked_list.cm
    └── fibonacci.cm
```

## Phase 1: Minimal Viable Transpiler (Weeks 1-2)

### 1.1 Lexer Implementation
Support the full Curium v5 token set from the documentation:

```rust
// src/lexer/token.rs
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Single-character tokens
    LParen, RParen, LBrace, RBrace, LBracket, RBracket,
    Semi, Comma, Colon, Dot, At, Dollar, Question, Bang,
    
    // Operators
    Plus, Minus, Star, Slash, Percent,
    Equal, EqualEqual, NotEqual,
    Lt, Gt, LtEqual, GtEqual,
    AndAnd, PipePipe, Ampersand, Pipe,
    Arrow, FatArrow, ColonEqual,
    Deref, AddrOf, DoubleQuestion, DoubleColon,
    
    // Literals
    Identifier(String),
    StringLiteral(String),
    NumberLiteral(String),
    InterpolatedString(Vec<String>),
    
    // Keywords (complete list from TokenKind enum)
    KwFn, KwLet, KwMut, KwReturn, KwIf, KwElse,
    KwWhile, KwFor, KwLoop, KwBreak, KwContinue,
    KwTrue, KwFalse, KwNull,
    KwString, KwVoid, KwDyn, KwCall, KwSpawn,
    KwI8, KwI16, KwI32, KwI64, KwU8, KwU16, KwU32, KwU64,
    KwF32, KwF64, KwUsize, KwBool,
    KwStruct, KwEnum, KwUnion, KwTrait, KwImpl,
    KwMatch, KwImport, KwModule, KwPub,
    KwTry, KwCatch, KwThrow, KwFinally,
    KwAsync, KwAwait, KwTask,
    KwReactor, KwArena, KwManual,
    KwNew, KwPtr, KwStr, KwStrnum,
    KwClass, KwInterface, KwImplements, KwExtends,
    KwGet, KwSet, KwStatic,
    KwPackage, KwUsing, KwNamespace, KwFrom, KwRequire,
    KwGc, KwGcCollect, KwMalloc, KwFree,
    KwPrint, KwPrintln,
    
    // Special
    CBlock(String),      // c { ... }
    CppBlock(String),    // cpp { ... }
    HashAttr(String),    // #[attr]
    Comment(String),
    Eof,
}
```

### 1.2 Parser - Core AST
Implement the AST nodes matching the documented structures:

```rust
// src/parser/ast.rs
#[derive(Debug, Clone)]
pub enum AstKind {
    // Declarations
    FnDecl {
        name: String,
        params: Vec<Param>,
        return_type: Option<Type>,
        body: Box<AstNode>,
        attributes: Vec<String>,
    },
    LetDecl {
        name: String,
        type_annotation: Option<Type>,
        init: Box<AstNode>,
        mutable: bool,
    },
    StructDecl {
        name: String,
        fields: Vec<Field>,
        methods: Vec<AstNode>,
    },
    EnumDecl {
        name: String,
        variants: Vec<EnumVariant>,
    },
    ImplBlock {
        trait_name: Option<String>,
        struct_name: String,
        methods: Vec<AstNode>,
    },
    
    // Statements
    ReturnStmt(Option<Box<AstNode>>),
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
    MatchStmt {
        expr: Box<AstNode>,
        arms: Vec<MatchArm>,
    },
    TryBlock {
        body: Box<AstNode>,
        catch_arms: Vec<CatchArm>,
        finally_block: Option<Box<AstNode>>,
    },
    ReactorBlock {
        allocator: AllocatorKind,
        size: Option<usize>,
        body: Box<AstNode>,
    },
    CBlock(String),
    SpawnBlock(Box<AstNode>),
    
    // Expressions
    BinaryExpr {
        op: String,
        left: Box<AstNode>,
        right: Box<AstNode>,
    },
    UnaryExpr {
        op: String,
        expr: Box<AstNode>,
    },
    Call {
        callee: Box<AstNode>,
        args: Vec<AstNode>,
    },
    DynOperator {
        op_var: String,
        cases: Vec<(String, Box<AstNode>)>,
        fallback: Box<AstNode>,
    },
    
    // Literals
    Identifier(String),
    NumberLiteral(String),
    StringLiteral(String),
    BoolLiteral(bool),
    NullLiteral,
    
    // Block
    Block(Vec<AstNode>),
}

pub struct AstNode {
    pub kind: AstKind,
    pub line: usize,
    pub column: usize,
    pub span: Span,
}
```

### 1.3 C11 Code Generator
Transpile to clean C11 code as documented:

```rust
// src/codegen/c_backend.rs
pub struct CGenerator {
    output: String,
    indent_level: usize,
    // Track Curium runtime dependencies
    needs_gc: bool,
    needs_arena: bool,
    needs_strnum: bool,
}

impl CGenerator {
    pub fn generate(&mut self, ast: &[AstNode]) -> String {
        // Emit runtime includes
        self.emit_includes();
        
        for node in ast {
            self.gen_node(node);
        }
        
        self.output.clone()
    }
    
    fn emit_includes(&mut self) {
        self.emitln("#include <curium/core.h>");
        self.emitln("#include <curium/memory.h>");
        self.emitln("#include <curium/string.h>");
        if self.needs_arena {
            self.emitln("#include <curium/arena.h>");
        }
    }
    
    fn gen_fn_decl(&mut self, name: &str, params: &[Param], 
                   return_type: Option<&Type>, body: &AstNode) {
        let ret = return_type.map(|t| self.type_to_c(t))
                             .unwrap_or("void".to_string());
        let params_c = params.iter()
            .map(|p| format!("{} {}", self.type_to_c(&p.ty), p.name))
            .collect::<Vec<_>>()
            .join(", ");
        
        self.emitln(&format!("{} {}({}) {{", ret, name, params_c));
        self.indent_level += 1;
        self.gen_node(body);
        self.indent_level -= 1;
        self.emitln("}");
    }
    
    fn type_to_c(&self, ty: &Type) -> String {
        match ty {
            Type::I32 => "int32_t".to_string(),
            Type::String => "curium_string_t*".to_string(),
            Type::Dyn => "curium_dyn_t".to_string(),
            Type::Ptr(inner) => format!("{}*", self.type_to_c(inner)),
            Type::Strnum => "curium_strnum_t".to_string(),
            // ...
        }
    }
}
```

## Phase 2: Full v5.0 Feature Support (Weeks 3-6)

### 2.1 Type Checker
Implement the type system described in INTERNALS.md:

```rust
// src/type_checker/mod.rs
pub struct TypeChecker {
    symbol_table: SymbolTable,
    current_function: Option<FunctionType>,
    errors: Vec<TypeError>,
}

impl TypeChecker {
    fn check_node(&mut self, node: &mut AstNode) -> Result<Type, TypeError> {
        match &mut node.kind {
            AstKind::BinaryExpr { op, left, right } => {
                let left_ty = self.check_node(left)?;
                let right_ty = self.check_node(right)?;
                self.check_binary_op(op, left_ty, right_ty, node.span)
            }
            AstKind::DynOperator { op_var, cases, fallback } => {
                // Dynamic operators: all arms must return same type
                self.check_dyn_operator(op_var, cases, fallback)
            }
            AstKind::ReactorBlock { allocator, size, body } => {
                // Reactor creates new memory scope
                self.push_scope(ScopeKind::Reactor);
                let result = self.check_node(body);
                self.pop_scope();
                result
            }
            // ...
        }
    }
}
```

### 2.2 Runtime Library
Implement the Curium runtime in C (as referenced in INTERNALS.md):

```c
// runtime/curium/core.h
typedef struct curium_string_t {
    char* data;
    size_t length;
    size_t capacity;
} curium_string_t;

typedef struct curium_dyn_t {
    enum { DYN_INT, DYN_FLOAT, DYN_STRING, DYN_PTR } tag;
    union {
        int64_t int_val;
        double float_val;
        curium_string_t* string_val;
        void* ptr_val;
    };
} curium_dyn_t;

// strnum: dual string/number type
typedef struct curium_strnum_t {
    enum { STRNUM_INT, STRNUM_FLOAT, STRNUM_STRING } kind;
    union {
        int64_t i;
        double f;
        curium_string_t* s;
    };
} curium_strnum_t;

// Safe pointer with RC
typedef struct curium_safe_ptr_t {
    void* ptr;
    size_t* ref_count;
    void (*dtor)(void*);
} curium_safe_ptr_t;

// Arena allocator
typedef struct curium_arena_t {
    uint8_t* memory;
    size_t capacity;
    size_t offset;
} curium_arena_t;
```

## Phase 3: CLI Implementation (Week 7)

Implement the complete CLI as documented in CLI_REFERENCE.md:

```rust
// src/cli/mod.rs
use clap::{Command, Arg, Subcommand};

pub fn build_cli() -> Command {
    Command::new("cm")
        .version("5.0.0")
        .about("Curium compiler and package manager")
        .subcommand(
            Command::new("init")
                .about("Initialize a new Curium project")
                .arg(Arg::new("name").required(true))
        )
        .subcommand(
            Command::new("build")
                .about("Compile Curium source")
                .arg(Arg::new("entry").default_value("src/main.cm"))
                .arg(Arg::new("output").short('o').long("output"))
                .arg(Arg::new("emit-c").long("emit-c").action(clap::ArgAction::SetTrue))
        )
        .subcommand(Command::new("run").about("Build and execute"))
        .subcommand(Command::new("check").about("Type-check only"))
        .subcommand(Command::new("doctor").about("Diagnose project health"))
        .subcommand(Command::new("fmt").about("Format source code"))
        .subcommand(Command::new("test").about("Run tests"))
        .subcommand(
            Command::new("packages")
                .subcommand(Command::new("install").arg(Arg::new("package")))
                .subcommand(Command::new("remove").arg(Arg::new("package")))
                .subcommand(Command::new("list"))
        )
}
```

## Phase 4: Self-Hosting Path (Weeks 8-10)

### 4.1 Write Compiler in Curium
Once the Rust compiler supports enough features, write the same compiler in Curium:

```curium
// compiler.cm - Written in Curium
import "lexer/token.cm";
import "parser/ast.cm";
import "codegen/c_backend.cm";

struct Compiler {
    source: string;
    tokens: []Token;
    ast: AstNode;
}

impl Compiler {
    fn new(source: string) -> Compiler {
        Compiler { source, tokens: [], ast: AstNode::empty() }
    }
    
    fn compile(mut self, output_path: string) -> Result<(), string> {
        self.tokens = Lexer::tokenize(self.source)?;
        self.ast = Parser::parse(self.tokens)?;
        
        let c_code = CGenerator::generate(self.ast);
        fs::write(output_path, c_code)?;
        
        Ok(())
    }
}

fn main() -> i32 {
    let args = env::args();
    match args.len() {
        2 => {
            let source = fs::read_to_string(args[1])?;
            let compiler = Compiler::new(source);
            compiler.compile("output.c")?;
            0
        }
        _ => {
            println("Usage: curium <file.cm>");
            1
        }
    }
}
```

### 4.2 Bootstrap Verification
```bash
# Step 1: Compile the Curium compiler with Rust bootstrap
$ cargo run -- compile compiler.cm -o compiler_v1.c
$ gcc compiler_v1.c -o compiler_v1

# Step 2: Use v1 to compile itself
$ ./compiler_v1 compiler.cm -o compiler_v2.c
$ gcc compiler_v2.c -o compiler_v2

# Step 3: Verify determinism
$ ./compiler_v2 compiler.cm -o compiler_v3.c
$ diff compiler_v2.c compiler_v3.c  # Should be identical!
```

## Key Features to Support (from Documentation)

### From LANGUAGE_GUIDE.md:
- ✅ `strnum` dual-type
- ✅ `dyn` dynamic operators with fallback `dyn($)`
- ✅ Safe pointers (`^T`)
- ✅ `reactor arena(size) { ... }`
- ✅ `c { ... }` polyglot blocks
- ✅ `match` exhaustive pattern matching
- ✅ `spawn` concurrency
- ✅ `try/catch/throw` error handling

### From SYNTAX_REFERENCE.md:
- ✅ `#[hot]` register hint attribute
- ✅ `impl` and `trait` system
- ✅ `enum` with variant data
- ✅ `Result<T, E>` and `?` operator

### From CLI_REFERENCE.md:
- ✅ `cm init`, `build`, `run`, `check`, `doctor`
- ✅ `cm packages install/remove/list`
- ✅ `cm fmt`, `cm test`

## Error Messages (ariadne Integration)
```rust
// src/error/diagnostics.rs
use ariadne::{Color, Label, Report, ReportKind, Source};

pub fn emit_type_error(
    span: Span,
    expected: &Type,
    found: &Type,
    source: &str,
) {
    Report::build(ReportKind::Error, "type mismatch", span.start)
        .with_message(format!("Expected {}, found {}", expected, found))
        .with_label(
            Label::new(span.clone())
                .with_message(format!("This has type {}", found))
                .with_color(Color::Red),
        )
        .finish()
        .print(Source::from(source))
        .unwrap();
}
```

## Testing Strategy
```rust
// tests/integration_tests.rs
#[test]
fn test_hello_world() {
    let source = r#"
        fn main() {
            println("Hello, Curium!");
        }
    "#;
    
    let tokens = lexer::tokenize(source).unwrap();
    let ast = parser::parse(tokens).unwrap();
    let c_code = codegen::generate(ast);
    
    assert!(c_code.contains("curium_println"));
    assert!(c_code.contains("Hello, Curium!"));
}

#[test]
fn test_linked_list() {
    let source = include_str!("../examples/linked_list.cm");
    // Verify compilation succeeds
}

#[test]
fn test_dyn_operator() {
    let source = r#"
        fn main() {
            mut op = "+";
            dyn op in (
                "+" => { return 10 + 20; }
            ) dyn($) { return 0; };
        }
    "#;
    // Verify dyn operator parsing and codegen
}
```

## Success Criteria

1. **Phase 1**: Compile and run `hello.cm`, `fibonacci.cm`, `linked_list.cm`
2. **Phase 2**: Support all v5.0 syntax features from documentation
3. **Phase 3**: Full CLI parity with documented commands
4. **Phase 4**: Self-hosting - compile `compiler.cm` with itself

## References
- Repository: https://github.com/ALightbolt4G/curium.cm
- Documentation: README.md, LANGUAGE_GUIDE.md, SYNTAX_REFERENCE.md
- Internals: INTERNALS.md (AST v2 Arena, Type Checker, Codegen)
```

---

This prompt gives you a complete roadmap from simple transpiler to full self-hosting compiler in Rust, aligned with all the Curium v5.0 documentation you provided. Ready to start implementing? 🚀