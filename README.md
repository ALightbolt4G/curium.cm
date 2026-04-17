<p align="center">
  <img src="https://img.shields.io/badge/version-5.0.0-00d4ff?style=for-the-badge&logo=data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHZpZXdCb3g9IjAgMCAyNCAyNCI+PHBhdGggZmlsbD0iIzAwZDRmZiIgZD0iTTEyIDJDNi40OCAyIDIgNi40OCAyIDEyczQuNDggMTAgMTAgMTAgMTAtNC40OCAxMC0xMFMxNy41MiAyIDEyIDJ6bTAgMThjLTQuNDEgMC04LTMuNTktOC04czMuNTktOCA4LTggOCAzLjU5IDggOC0zLjU5IDgtOCA4eiIvPjwvc3ZnPg==" alt="version"/>
  <img src="https://img.shields.io/badge/license-MIT-green?style=for-the-badge" alt="license"/>
  <img src="https://img.shields.io/badge/lang-Rust-orange?style=for-the-badge&logo=rust" alt="rust"/>
  <img src="https://img.shields.io/badge/target-C11-blue?style=for-the-badge" alt="c11"/>
</p>

<h1 align="center">
  🧪 Curium
</h1>

<p align="center">
  <strong>A modern systems programming language that transpiles to C11.</strong><br>
  <em>Safe pointers • Dynamic operators • Reactor memory model • Self-hosting path</em>
</p>

---

## Overview

**Curium** is a compiled systems programming language designed for performance and safety. The compiler (`cm`) transpiles `.cm` source files to clean C11 code, which is then compiled to native binaries using GCC or TCC.

This repository contains the **Rust bootstrap compiler** — the reference implementation of Curium v5.0 that will eventually compile the self-hosting compiler written in Curium itself.

## Quick Start

```bash
# Build the compiler
cargo build --release

# Create a new project
cm init my_project
cd my_project

# Build and run
cm build src/main.cm -o main
./main

# Or build + run in one step
cm run src/main.cm
```

## Hello, Curium!

```curium
fn main() -> i32 {
    println("Hello, Curium!");
    return 0;
}
```

## Language Features

### Variables & Types
```curium
let x: i32 = 42;           // Immutable binding
mut counter: i32 = 0;      // Mutable variable
let name: string = "Curium";
let pi: f64 = 3.14159;
let flag: bool = true;
```

### Functions
```curium
fn fibonacci(n: i32) -> i32 {
    if n <= 1 {
        return n;
    }
    return fibonacci(n - 1) + fibonacci(n - 2);
}
```

### Structs & impl
```curium
struct Point {
    x: f64;
    y: f64;
}

impl Point {
    fn distance(self, other: Point) -> f64 {
        let dx: f64 = self.x - other.x;
        let dy: f64 = self.y - other.y;
        return 0.0; // sqrt not yet available
    }
}

fn main() -> i32 {
    let p = Point { x: 1.0, y: 2.0 };
    return 0;
}
```

### Enums & Pattern Matching
```curium
enum Shape {
    Circle(f64),
    Rectangle(f64, f64),
}

fn area(shape: Shape) -> f64 {
    match shape {
        Shape::Circle => { return 3.14159; }
        Shape::Rectangle => { return 1.0; }
    }
}
```

### Control Flow
```curium
// If / else
if x > 0 {
    println("positive");
} else {
    println("non-positive");
}

// While loop
while i < 10 {
    i += 1;
}

// For loop with range
for i in 0..10 {
    println("iteration");
}

// Infinite loop
loop {
    if done { break; }
}
```

### Error Handling
```curium
try {
    let result = risky_operation();
} catch (e) {
    println("Error occurred");
} finally {
    cleanup();
}
```

### Memory Management — Reactor Blocks
```curium
reactor arena(4096) {
    // All allocations use bump allocator
    // Memory freed automatically at block exit
    let data = allocate_buffer(1024);
}
```

### Polyglot C Interop
```curium
fn fast_sqrt(x: f64) -> f64 {
    c {
        #include <math.h>
        return sqrt(x);
    }
}
```

### Concurrency
```curium
spawn {
    heavy_computation();
}
```

## Type System

| Curium Type | C11 Mapping | Description |
|-------------|-------------|-------------|
| `i8` .. `i64` | `int8_t` .. `int64_t` | Signed integers |
| `u8` .. `u64` | `uint8_t` .. `uint64_t` | Unsigned integers |
| `f32` / `f64` | `float` / `double` | Floating point |
| `bool` | `bool` | Boolean |
| `char` | `char` | Character |
| `string` | `curium_string_t*` | Heap-allocated string |
| `str` | `curium_string_t*` | String slice |
| `^T` | `T*` + refcount | Safe pointer |
| `dyn` | `curium_dyn_t` | Dynamic type |
| `strnum` | `curium_strnum_t` | Dual string/number |
| `?T` | Optional wrapper | Nullable type |
| `[]T` | `T*` | Array/slice |

## CLI Reference

```
cm — Curium compiler and package manager (v5.0.0)

SUBCOMMANDS:
    init <name>         Initialize a new Curium project
    build <file>        Compile a .cm source file
        -o <output>         Output file path (default: output)
        --emit-c            Only output the generated C file
        --cc <compiler>     C compiler to use (default: gcc)
        --standalone        Statically link C runtime for independent binary output
    run <file>          Build and execute a Curium program
    check <file>        Parse and type-check only (no codegen)
    fmt [file|dir]      Format Curium source files
    test [filter]       Run project tests
    doctor              Diagnose project health and environment
    dump tokens <file>  Print the token stream
    dump ast <file>     Print the AST as S-expressions
    dump types <file>   Print resolved types for all symbols
    packages install    Install a package
    packages remove     Remove a package
    packages list       List installed packages
```

## Architecture

```
┌──────────────────────────────────────────────────────────┐
│                     cm CLI (clap)                        │
├──────────┬──────────┬──────────┬──────────┬──────────────┤
│  build   │   run    │  check   │   fmt    │   dump       │
├──────────┴──────────┴──────────┴──────────┴──────────────┤
│                   Compiler Pipeline                      │
│  ┌────────┐   ┌────────┐   ┌───────────┐   ┌─────────┐  │
│  │ Lexer  │──▶│ Parser │──▶│TypeChecker│──▶│ Codegen │  │
│  │scanner │   │grammar │   │  checker  │   │c_backend│  │
│  └────────┘   └────────┘   └───────────┘   └─────────┘  │
│       │            │             │               │       │
│   TokenKind     AstKind       SymbolTable    C11 output  │
├──────────────────────────────────────────────────────────┤
│                  Error Reporting (ariadne)                │
└──────────────────────────────────────────────────────────┘
```

### Module Structure
```
src/
├── main.rs              # CLI dispatch and command implementations
├── lexer/
│   ├── mod.rs           # Module exports
│   ├── token.rs         # TokenKind enum (80+ tokens), Span, Token
│   └── scanner.rs       # Lexer implementation (char-by-char)
├── parser/
│   ├── mod.rs           # Module exports
│   ├── ast.rs           # AST node definitions, Type enum, operators
│   └── grammar.rs       # Recursive descent parser
├── type_checker/
│   ├── mod.rs           # Module exports
│   ├── checker.rs       # Type checking and inference engine
│   └── symbol_table.rs  # Scoped symbol table with lexical lookup
├── codegen/
│   ├── mod.rs           # Module exports
│   └── c_backend.rs     # C11 code generator with runtime embedding
├── error/
│   ├── mod.rs           # Module exports
│   └── diagnostics.rs   # Rich error messages via ariadne
└── cli/
    └── mod.rs           # CLI definition (clap)
```

## Project Phases

| Phase | Description | Status |
|-------|-------------|--------|
| **Phase 1** | Minimal Viable Transpiler — Lexer, Parser, C11 Codegen | ✅ Complete |
| **Phase 2** | Full v5.0 Feature Support — Type Checker, Runtime | ✅ Complete |
| **Phase 3** | CLI Implementation — fmt, test, packages, doctor | ✅ Complete |
| **Phase 4** | Self-Hosting Path — compiler.cm, bootstrap verification | ✅ Complete |
| **Phase 5** | Standard Library (`doc/stdlib.md`) — Process, FS, Vec, String globally available | ✅ Complete |
| **Phase 6** | Ecosystem Utilities — Package Manager (`cm get`), LSP, Test Harness, Formatter | 🔄 In Progress |

## Self-Hosting Bootstrap

The self-hosting path demonstrates that the Curium compiler can compile a version of itself written in Curium:

```bash
# Stage 1: Rust bootstrap compiles compiler.cm
cm build compiler.cm --emit-c -o compiler_v1
gcc compiler_v1.c -o compiler_v1

# Stage 2: compiler_v1 recompiles compiler.cm
./compiler_v1 compiler.cm compiler_v2.c
gcc compiler_v2.c -o compiler_v2

# Stage 3: Verify determinism
./compiler_v2 compiler.cm compiler_v3.c
diff compiler_v2.c compiler_v3.c    # Should be identical!
```

Run the automated verification:
```powershell
.\bootstrap.ps1
```

## Testing

```bash
# Run all unit tests (Rust)
cargo test

# Run Curium source tests
cm test

# Test a specific example
cm check examples/hello.cm
cm check examples/fibonacci.cm
cm check examples/structs.cm
cm check examples/linked_list.cm
```

## Examples

| File | Description |
|------|-------------|
| `examples/hello.cm` | Hello World — minimal program |
| `examples/fibonacci.cm` | Recursive and iterative Fibonacci |
| `examples/structs.cm` | Structs, enums, impl blocks, pattern matching |
| `examples/linked_list.cm` | Node-based data structure with value aggregation |
| `compiler.cm` | Self-hosting compiler source (bootstrap target) |

## Building from Source

### Prerequisites
- **Rust** 1.75+ (for the bootstrap compiler)
- **GCC** or **TCC** (for compiling generated C11 code)

### Build
```bash
git clone https://github.com/ALightbolt4G/curium.cm.git
cd curium.cm
cargo build --release
```

The compiler binary will be at `target/release/cm` (or `cm.exe` on Windows).

## License

MIT License — see [LICENSE](LICENSE) for details.

---

<p align="center">
  <strong>Curium v5.0.0</strong> — Rust Bootstrap Compiler<br>
  <em>Built with ❤️ for systems programming</em>
</p>
