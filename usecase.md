
---

## 📊 Curium Compiler System - Use Cases Analysis Table

| **ID** | **Phase** | **Use Case Name** | **Actor** | **Description** | **Preconditions** | **Postconditions** | **Priority** | **Complexity** |
|--------|-----------|-------------------|-----------|-----------------|-------------------|-------------------|--------------|----------------|
| **UC-001** | Phase 1 | Initialize Project | Developer | Create new Curium project with standard structure | cm CLI installed | `curium.json` created, `src/` folder ready | High | Low |
| **UC-002** | Phase 1 | Lex Source File | Compiler | Tokenize `.cm` source file into token stream | Valid `.cm` file exists | Vector of Token structs | Critical | Medium |
| **UC-003** | Phase 1 | Parse Tokens to AST | Compiler | Convert token stream into Abstract Syntax Tree | Token stream available | AST root node with all declarations | Critical | High |
| **UC-004** | Phase 1 | Generate C11 Code | Compiler | Transpile AST to C11 source code | Valid AST exists | `output.c` file generated | Critical | High |
| **UC-005** | Phase 1 | Compile C to Binary | Compiler | Invoke TCC/GCC on generated C code | C compiler available | Executable binary produced | High | Low |
| **UC-006** | Phase 1 | Run Program | Developer | Build and execute Curium program in one command | Project is valid | Program output displayed | High | Low |
| **UC-007** | Phase 1 | Check Syntax Only | Developer | Validate syntax without generating code | `.cm` file exists | Syntax errors reported | Medium | Low |
| **UC-008** | Phase 2 | Type Check AST | Compiler | Perform semantic analysis and type inference | AST available | Type-annotated AST or type errors | Critical | High |
| **UC-009** | Phase 2 | Infer strnum Type | Compiler | Deduce if strnum is used as string or number | Context of usage known | Correct C type selected | High | Medium |
| **UC-010** | Phase 2 | Resolve Safe Pointers | Compiler | Convert `^T` to reference-counted C pointers | Type info available | `curium_safe_ptr_t` in output | Critical | High |
| **UC-011** | Phase 2 | Process dyn Operators | Compiler | Transform dynamic operator blocks to switch/match | Cases defined | C switch statement or if-else chain | High | Medium |
| **UC-012** | Phase 2 | Handle Reactor Blocks | Compiler | Generate arena allocator scope for reactor | Size specified | Bump allocator code emitted | High | Medium |
| **UC-013** | Phase 2 | Process Polyglot c{} Blocks | Compiler | Pass raw C code through unchanged | Block is syntactically valid | Raw C inserted verbatim | Medium | Low |
| **UC-014** | Phase 2 | Expand Pattern Matching | Compiler | Desugar match statement to if-else or switch | Enum type known | Nested conditionals in output | High | High |
| **UC-015** | Phase 2 | Generate Error Handling | Compiler | Implement try/catch with setjmp/longjmp | Error types defined | Exception handling code emitted | High | High |
| **UC-016** | Phase 2 | Process spawn Blocks | Compiler | Generate thread creation code | Concurrency runtime available | pthread or std::thread code | Medium | Medium |
| **UC-017** | Phase 3 | Format Source Code | Developer | Auto-format `.cm` file to standard style | File exists | Consistently formatted file | Medium | Medium |
| **UC-018** | Phase 3 | Run Tests | Developer | Discover and execute all tests in `tests/` | Test files exist | Test results reported | High | Low |
| **UC-019** | Phase 3 | Install Package | Developer | Download and install package from registry | Package name provided | Package in `packages/` folder | Medium | Medium |
| **UC-020** | Phase 3 | Remove Package | Developer | Uninstall package from project | Package is installed | Package removed from manifest | Low | Low |
| **UC-021** | Phase 3 | List Packages | Developer | Display all installed packages | Project has packages | Table of packages printed | Low | Low |
| **UC-022** | Phase 3 | Run Doctor | Developer | Diagnose project health and environment | Project exists | Health report generated | Medium | Medium |
| **UC-023** | Phase 3 | Emit C Only | Developer | Output transpiled C without compilation | `.cm` file valid | `.c` file written | Medium | Low |
| **UC-024** | Phase 4 | Bootstrap Compile | Developer | Compile compiler.cm with Rust bootstrap | Rust compiler exists | `compiler_v1` executable | Critical | Very High |
| **UC-025** | Phase 4 | Self-Compile | Compiler | Compile compiler.cm with itself | v1 compiler works | `compiler_v2` identical to v1 | Critical | Very High |
| **UC-026** | Phase 4 | Verify Determinism | System | Compare v2 and v3 compiler outputs | Both compilers run | Binary identical outputs | Critical | High |
| **UC-027** | All | Report Error with Context | Compiler | Display beautiful error with source snippet | Error occurred | User sees location and suggestion | High | Medium |
| **UC-028** | All | Track Memory Usage | Compiler | Monitor arena and heap allocations | Runtime tracking enabled | Statistics available via `--stat` | Low | Low |
| **UC-029** | All | Handle Unicode Identifiers | Lexer | Support non-ASCII in identifiers | UTF-8 source | Correctly tokenized | Medium | Medium |
| **UC-030** | All | Process Attributes | Parser | Parse `#[attr]` syntax and attach to AST | Valid attribute syntax | Attributes stored on node | Medium | Medium |
| **UC-031** | Phase 5 | File I/O Operations | Standard Library | Read/write files for source and output generation | FS permissions | File contents loaded/written | Critical | Medium |
| **UC-032** | Phase 5 | String & Path Utilities | Standard Library | Parse and manipulate paths and string builders | String input | Formatted/joined strings | High | Medium |
| **UC-033** | Phase 5 | Dynamic Collections | Standard Library | Use resizable arrays (Vectors) and maps | Types defined | Scalable memory containers | High | High |
| **UC-034** | Phase 5 | Process Control | Standard Library | Terminate compilation with specific exit codes | Failure state | Process exits with code | Medium | Low |

---

## 📋 Summary Statistics

| **Metric** | **Count** |
|------------|-----------|
| Total Use Cases | 34 |
| Critical Priority | 9 |
| High Priority | 14 |
| Medium Priority | 9 |
| Low Priority | 2 |
| Very High Complexity | 3 |
| High Complexity | 10 |
| Medium Complexity | 11 |
| Low Complexity | 10 |

---

## 🔄 Phase Breakdown

| **Phase** | **Use Cases** | **Count** |
|-----------|---------------|-----------|
| Phase 1 (Basic Transpiler) | UC-001 to UC-007 | 7 |
| Phase 2 (Full Features) | UC-008 to UC-016 | 9 |
| Phase 3 (CLI & Tools) | UC-017 to UC-023 | 7 |
| Phase 4 (Self-Hosting) | UC-024 to UC-026 | 3 |
| Phase 5 (Standard Library) | UC-031 to UC-034 | 4 |
| Cross-Cutting | UC-027 to UC-030 | 4 |

---

## 🎯 Critical Path (MVP - Phase 1)

```
UC-002 (Lex) → UC-003 (Parse) → UC-004 (Generate C) → UC-005 (Compile) → UC-006 (Run)
```

---

## 📎 CSV Format for Excel Import

```csv
ID,Phase,Use Case Name,Actor,Description,Preconditions,Postconditions,Priority,Complexity
UC-001,Phase 1,Initialize Project,Developer,Create new Curium project with standard structure,cm CLI installed,curium.json created and src/ folder ready,High,Low
UC-002,Phase 1,Lex Source File,Compiler,Tokenize .cm source file into token stream,Valid .cm file exists,Vector of Token structs,Critical,Medium
UC-003,Phase 1,Parse Tokens to AST,Compiler,Convert token stream into Abstract Syntax Tree,Token stream available,AST root node with all declarations,Critical,High
UC-004,Phase 1,Generate C11 Code,Compiler,Transpile AST to C11 source code,Valid AST exists,output.c file generated,Critical,High
UC-005,Phase 1,Compile C to Binary,Compiler,Invoke TCC/GCC on generated C code,C compiler available,Executable binary produced,High,Low
UC-006,Phase 1,Run Program,Developer,Build and execute Curium program in one command,Project is valid,Program output displayed,High,Low
UC-007,Phase 1,Check Syntax Only,Developer,Validate syntax without generating code,.cm file exists,Syntax errors reported,Medium,Low
UC-008,Phase 2,Type Check AST,Compiler,Perform semantic analysis and type inference,AST available,Type-annotated AST or type errors,Critical,High
UC-009,Phase 2,Infer strnum Type,Compiler,Deduce if strnum is used as string or number,Context of usage known,Correct C type selected,High,Medium
UC-010,Phase 2,Resolve Safe Pointers,Compiler,Convert ^T to reference-counted C pointers,Type info available,curium_safe_ptr_t in output,Critical,High
UC-011,Phase 2,Process dyn Operators,Compiler,Transform dynamic operator blocks to switch/match,Cases defined,C switch statement or if-else chain,High,Medium
UC-012,Phase 2,Handle Reactor Blocks,Compiler,Generate arena allocator scope for reactor,Size specified,Bump allocator code emitted,High,Medium
UC-013,Phase 2,Process Polyglot c{} Blocks,Compiler,Pass raw C code through unchanged,Block is syntactically valid,Raw C inserted verbatim,Medium,Low
UC-014,Phase 2,Expand Pattern Matching,Compiler,Desugar match statement to if-else or switch,Enum type known,Nested conditionals in output,High,High
UC-015,Phase 2,Generate Error Handling,Compiler,Implement try/catch with setjmp/longjmp,Error types defined,Exception handling code emitted,High,High
UC-016,Phase 2,Process spawn Blocks,Compiler,Generate thread creation code,Concurrency runtime available,pthread or std::thread code,Medium,Medium
UC-017,Phase 3,Format Source Code,Developer,Auto-format .cm file to standard style,File exists,Consistently formatted file,Medium,Medium
UC-018,Phase 3,Run Tests,Developer,Discover and execute all tests in tests/,Test files exist,Test results reported,High,Low
UC-019,Phase 3,Install Package,Developer,Download and install package from registry,Package name provided,Package in packages/ folder,Medium,Medium
UC-020,Phase 3,Remove Package,Developer,Uninstall package from project,Package is installed,Package removed from manifest,Low,Low
UC-021,Phase 3,List Packages,Developer,Display all installed packages,Project has packages,Table of packages printed,Low,Low
UC-022,Phase 3,Run Doctor,Developer,Diagnose project health and environment,Project exists,Health report generated,Medium,Medium
UC-023,Phase 3,Emit C Only,Developer,Output transpiled C without compilation,.cm file valid,.c file written,Medium,Low
UC-024,Phase 4,Bootstrap Compile,Developer,Compile compiler.cm with Rust bootstrap,Rust compiler exists,compiler_v1 executable,Critical,Very High
UC-025,Phase 4,Self-Compile,Compiler,Compile compiler.cm with itself,v1 compiler works,compiler_v2 identical to v1,Critical,Very High
UC-026,Phase 4,Verify Determinism,System,Compare v2 and v3 compiler outputs,Both compilers run,Binary identical outputs,Critical,High
UC-027,All,Report Error with Context,Compiler,Display beautiful error with source snippet,Error occurred,User sees location and suggestion,High,Medium
UC-028,All,Track Memory Usage,Compiler,Monitor arena and heap allocations,Runtime tracking enabled,Statistics available via --stat,Low,Low
UC-029,All,Handle Unicode Identifiers,Lexer,Support non-ASCII in identifiers,UTF-8 source,Correctly tokenized,Medium,Medium
UC-030,All,Process Attributes,Parser,Parse #[attr] syntax and attach to AST,Valid attribute syntax,Attributes stored on node,Medium,Medium
UC-031,Phase 5,File I/O Operations,Standard Library,Read/write files for source and output generation,FS permissions,File contents loaded/written,Critical,Medium
UC-032,Phase 5,String & Path Utilities,Standard Library,Parse and manipulate paths and string builders,String input,Formatted/joined strings,High,Medium
UC-033,Phase 5,Dynamic Collections,Standard Library,Use resizable arrays (Vectors) and maps,Types defined,Scalable memory containers,High,High
UC-034,Phase 5,Process Control,Standard Library,Terminate compilation with specific exit codes,Failure state,Process exits with code,Medium,Low
UC-035,Phase 6,Package Manager,Ecosystem,Create `cm get` to fetch dependencies,Network access,Dependencies installed in local module repo,High,High
UC-036,Phase 6,Testing Framework,Ecosystem,Create `cm test` standard harness framework,Code with #[test] annotations,Runs tests and reports results,High,Medium
UC-037,Phase 6,Language Server (LSP),Ecosystem,Create an LSP using compiler typechecker API,Compiler as library,Hover/autocomplete info provided to IDE,Medium,Very High
UC-038,Phase 6,Code Formatter,Ecosystem,Create `cm fmt` to consistently format `.cm` files,Unformatted code,Code matches AST standard style,Low,High
UC-039,Phase 5,Binary Independence,Compiler,Compile with --standalone to statically link C runtime dependencies for true binary independence,Run binary on clean host,Binary executes without DLLs,High,Low
```
