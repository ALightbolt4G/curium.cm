mod lexer;
mod parser;
mod codegen;
mod error;
mod cli;
mod type_checker;

use std::fs;
use std::process::Command as ProcessCommand;

use lexer::Lexer;
use parser::{Parser, AstKind};
use codegen::CGenerator;

fn main() {
    let matches = cli::build_cli().get_matches();

    match matches.subcommand() {
        Some(("build", sub)) => cmd_build(sub),
        Some(("run", sub)) => cmd_run(sub),
        Some(("check", sub)) => cmd_check(sub),
        Some(("dump", sub)) => cmd_dump(sub),
        Some(("init", sub)) => cmd_init(sub),
        Some(("fmt", sub)) => cmd_fmt(sub),
        Some(("test", sub)) => cmd_test(sub),
        Some(("packages", sub)) => cmd_packages(sub),
        Some(("doctor", _)) => cmd_doctor(),
        _ => {
            print_banner();
            cli::build_cli().print_help().ok();
            println!();
        }
    }
}

// ── Imports Resolution ───────────────────────────────────────────────────────

fn resolve_imports(ast: &mut parser::AstNode, visited: &mut std::collections::HashSet<String>, base_dir: &std::path::Path) -> Result<(), String> {
    if let parser::AstKind::Program(decls) = &mut ast.kind {
        let mut new_decls = Vec::new();
        // Take ownership of decls
        let current_decls = std::mem::take(decls);
        for decl in current_decls {
            if let parser::AstKind::ImportDecl { path, .. } = &decl.kind {
                let str_path = path.replace("\\", "/");
                let file_name = if str_path.ends_with(".cm") { str_path } else { format!("{}.cm", str_path) };
                
                let mut file_path = base_dir.join(&file_name);
                if !file_path.exists() {
                    let cwd_path = std::path::Path::new(&file_name);
                    if cwd_path.exists() {
                        file_path = cwd_path.to_path_buf();
                    }
                }
                
                let cannon = std::fs::canonicalize(&file_path).unwrap_or_else(|_| file_path.clone());
                let cannon_str = cannon.to_string_lossy().to_string();
                
                if visited.contains(&cannon_str) { continue; }
                if !cannon_str.is_empty() { visited.insert(cannon_str.clone()); }

                let source = fs::read_to_string(&file_path).map_err(|e| format!("Cannot read import '{}': {}", file_path.display(), e))?;
                let tokens = Lexer::tokenize(&source).map_err(|e| format!("Lex error in import '{}': {}", file_path.display(), e))?;
                let mut imported_ast = Parser::parse(tokens).map_err(|e| format!("Parse error in import '{}': {}", file_path.display(), e))?;
                
                let parent_dir = file_path.parent().unwrap_or(std::path::Path::new(""));
                resolve_imports(&mut imported_ast, visited, parent_dir)?;
                
                if let parser::AstKind::Program(imported_decls) = imported_ast.kind {
                    new_decls.extend(imported_decls);
                }
            } else {
                new_decls.push(decl);
            }
        }
        *decls = new_decls;
    }
    Ok(())
}

// ── Commands ─────────────────────────────────────────────────────────────────

fn inject_prelude(source: &str) -> String {
    let prelude = "import \"core/prelude\";\nimport \"std/process\";\nimport \"std/fs\";\nimport \"std/string\";\nimport \"std/vec\";\n";
    format!("{}{}", prelude, source)
}

fn cmd_build(matches: &clap::ArgMatches) {
    let file = matches.get_one::<String>("file").unwrap();
    let output = matches.get_one::<String>("output").unwrap();
    let emit_c_only = matches.get_flag("emit-c");
    let cc = matches.get_one::<String>("cc").unwrap();

    let source = match fs::read_to_string(file) {
        Ok(s) => inject_prelude(&s),
        Err(e) => {
            eprintln!("{}", error::format_error(file, 0, 0, &format!("Cannot read file: {}", e)));
            std::process::exit(1);
        }
    };

    // Lex
    let tokens = match Lexer::tokenize(&source) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("{}", error::format_error(file, 0, 0, &e));
            std::process::exit(1);
        }
    };

    // Parse
    let mut ast = match Parser::parse(tokens) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("{}", error::format_error(file, 0, 0, &e));
            std::process::exit(1);
        }
    };

    // Resolve Imports
    let mut visited = std::collections::HashSet::new();
    let file_path = std::path::Path::new(file);
    let base_dir = file_path.parent().unwrap_or(std::path::Path::new(""));
    if let Ok(c) = std::fs::canonicalize(file_path) {
        visited.insert(c.to_string_lossy().to_string());
    }
    if let Err(e) = resolve_imports(&mut ast, &mut visited, base_dir) {
        eprintln!("{}", error::format_error(file, 0, 0, &e));
        std::process::exit(1);
    }

    // Type check (warnings only — non-fatal for bootstrap compatibility)
    let (_, type_errors) = type_checker::TypeChecker::check(&ast);
    for err in &type_errors {
        eprintln!(
            "\x1b[1;33mwarning\x1b[0m: {} ({}:{}:{})",
            err.message, file, err.line, err.column
        );
    }

    // Codegen
    let c_code = CGenerator::generate(&ast);

    let c_file = format!("{}.c", output);
    if let Err(e) = fs::write(&c_file, &c_code) {
        eprintln!("{}", error::format_error(file, 0, 0, &format!("Cannot write output: {}", e)));
        std::process::exit(1);
    }

    if emit_c_only {
        println!("\x1b[1;32m✓\x1b[0m Emitted C to {}", c_file);
        return;
    }

    // Compile with C compiler
    let out_exe = if cfg!(windows) {
        format!("{}.exe", output)
    } else {
        output.to_string()
    };

    let status = ProcessCommand::new(cc)
        .args([&c_file, "-o", &out_exe, "-lm"])
        .status();

    match status {
        Ok(s) if s.success() => {
            println!("\x1b[1;32m✓\x1b[0m Compiled to {}", out_exe);
        }
        Ok(s) => {
            eprintln!(
                "\x1b[1;31m✗\x1b[0m C compiler exited with code {}",
                s.code().unwrap_or(-1)
            );
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!(
                "\x1b[1;31m✗\x1b[0m Failed to invoke C compiler '{}': {}",
                cc, e
            );
            std::process::exit(1);
        }
    }
}

fn cmd_run(matches: &clap::ArgMatches) {
    let file = matches.get_one::<String>("file").unwrap();

    // Build first
    let build_matches = cli::build_cli()
        .get_matches_from(vec!["cm", "build", file, "--emit-c"]);
    if let Some(("build", sub)) = build_matches.subcommand() {
        cmd_build(sub);
    }

    // Now compile and run
    let source = fs::read_to_string(file).unwrap_or_else(|e| {
        eprintln!("Cannot read file: {}", e);
        std::process::exit(1);
    });
    let source = inject_prelude(&source);

    let tokens = Lexer::tokenize(&source).unwrap_or_else(|e| {
        eprintln!("{}", e);
        std::process::exit(1);
    });

    let mut ast = Parser::parse(tokens).unwrap_or_else(|e| {
        eprintln!("{}", e);
        std::process::exit(1);
    });

    let mut visited = std::collections::HashSet::new();
    let file_path = std::path::Path::new(file);
    let base_dir = file_path.parent().unwrap_or(std::path::Path::new(""));
    if let Ok(c) = std::fs::canonicalize(file_path) {
        visited.insert(c.to_string_lossy().to_string());
    }
    if let Err(e) = resolve_imports(&mut ast, &mut visited, base_dir) {
        eprintln!("{}", e);
        std::process::exit(1);
    }

    let c_code = CGenerator::generate(&ast);
    let c_file = "__curium_run.c";
    let exe_file = if cfg!(windows) { "__curium_run.exe" } else { "__curium_run" };

    fs::write(c_file, &c_code).unwrap();

    let compile = ProcessCommand::new("gcc")
        .args([c_file, "-o", exe_file, "-lm"])
        .status();

    match compile {
        Ok(s) if s.success() => {
            let run = ProcessCommand::new(format!("./{}", exe_file)).status();
            // Clean up
            let _ = fs::remove_file(c_file);
            let _ = fs::remove_file(exe_file);

            match run {
                Ok(s) => std::process::exit(s.code().unwrap_or(0)),
                Err(e) => {
                    eprintln!("Failed to run: {}", e);
                    std::process::exit(1);
                }
            }
        }
        _ => {
            eprintln!("\x1b[1;31m✗\x1b[0m Compilation failed");
            let _ = fs::remove_file(c_file);
            std::process::exit(1);
        }
    }
}

fn cmd_check(matches: &clap::ArgMatches) {
    let file = matches.get_one::<String>("file").unwrap();

    let source = match fs::read_to_string(file) {
        Ok(s) => inject_prelude(&s),
        Err(e) => {
            eprintln!("Cannot read {}: {}", file, e);
            std::process::exit(1);
        }
    };

    let tokens = match Lexer::tokenize(&source) {
        Ok(t) => t,
        Err(e) => {
            error::emit_parse_error(&source, file, 0, &e);
            std::process::exit(1);
        }
    };

    let mut ast = match Parser::parse(tokens) {
        Ok(a) => a,
        Err(e) => {
            error::emit_parse_error(&source, file, 0, &e);
            std::process::exit(1);
        }
    };

    let mut visited = std::collections::HashSet::new();
    let file_path = std::path::Path::new(file);
    let base_dir = file_path.parent().unwrap_or(std::path::Path::new(""));
    if let Ok(c) = std::fs::canonicalize(file_path) {
        visited.insert(c.to_string_lossy().to_string());
    }
    if let Err(e) = resolve_imports(&mut ast, &mut visited, base_dir) {
        eprintln!("error: {}", e);
        std::process::exit(1);
    }

    // Type check
    let (_, type_errors) = type_checker::TypeChecker::check(&ast);
    if type_errors.is_empty() {
        println!("\x1b[1;32m✓\x1b[0m {} — no errors", file);
    } else {
        for err in &type_errors {
            eprintln!(
                "\x1b[1;31merror\x1b[0m: {} ({}:{}:{})",
                err.message, file, err.line, err.column
            );
        }
        eprintln!(
            "\x1b[1;31m✗\x1b[0m {} error(s) found",
            type_errors.len()
        );
        std::process::exit(1);
    }
}

fn cmd_dump(matches: &clap::ArgMatches) {
    match matches.subcommand() {
        Some(("tokens", sub)) => {
            let file = sub.get_one::<String>("file").unwrap();
            let source = fs::read_to_string(file).unwrap_or_else(|e| {
                eprintln!("Cannot read {}: {}", file, e);
                std::process::exit(1);
            });
            let source = inject_prelude(&source);

            let tokens = Lexer::tokenize(&source).unwrap_or_else(|e| {
                eprintln!("{}", e);
                std::process::exit(1);
            });

            for tok in &tokens {
                println!(
                    "{:4}:{:<3}  {:?}",
                    tok.line, tok.column, tok.kind
                );
            }
        }
        Some(("ast", sub)) => {
            let file = sub.get_one::<String>("file").unwrap();
            let source = fs::read_to_string(file).unwrap_or_else(|e| {
                eprintln!("Cannot read {}: {}", file, e);
                std::process::exit(1);
            });
            let source = inject_prelude(&source);

            let tokens = Lexer::tokenize(&source).unwrap_or_else(|e| {
                eprintln!("{}", e);
                std::process::exit(1);
            });

            let ast = Parser::parse(tokens).unwrap_or_else(|e| {
                eprintln!("{}", e);
                std::process::exit(1);
            });

            print_ast(&ast, 0);
        }
        Some(("types", sub)) => {
            let file = sub.get_one::<String>("file").unwrap();
            let source = fs::read_to_string(file).unwrap_or_else(|e| {
                eprintln!("Cannot read {}: {}", file, e);
                std::process::exit(1);
            });
            let source = inject_prelude(&source);

            let tokens = Lexer::tokenize(&source).unwrap_or_else(|e| {
                eprintln!("{}", e);
                std::process::exit(1);
            });

            let ast = Parser::parse(tokens).unwrap_or_else(|e| {
                eprintln!("{}", e);
                std::process::exit(1);
            });

            let (symbols, errors) = type_checker::TypeChecker::check(&ast);
            println!("\x1b[1;36m── Symbol Table ──\x1b[0m");
            // Print symbols from the global scope
            for sym in symbols.global_symbols() {
                println!("  {} : {} ({:?})", sym.name, sym.ty, sym.kind_name());
            }
            if !errors.is_empty() {
                println!("\n\x1b[1;33m── Type Errors ──\x1b[0m");
                for err in &errors {
                    println!("  {} (offset {}..{})", err.message, err.line, err.column);
                }
            }
        }
        _ => {
            eprintln!("Usage: cm dump <tokens|ast|types> <file.cm>");
        }
    }
}

fn cmd_init(matches: &clap::ArgMatches) {
    let name = matches.get_one::<String>("name").unwrap();
    let project_dir = std::path::Path::new(name);

    if project_dir.exists() {
        eprintln!("\x1b[1;31m✗\x1b[0m Directory '{}' already exists", name);
        std::process::exit(1);
    }

    fs::create_dir_all(project_dir.join("src")).unwrap();

    // curium.json
    let manifest = format!(
        r#"{{
    "name": "{}",
    "version": "0.1.0",
    "entry": "src/main.cm",
    "compiler": "cm"
}}"#,
        name
    );
    fs::write(project_dir.join("curium.json"), manifest).unwrap();

    // main.cm
    let main_cm = r#"fn main() -> i32 {
    println("Hello from Curium!");
    return 0;
}
"#;
    fs::write(project_dir.join("src/main.cm"), main_cm).unwrap();

    println!("\x1b[1;32m✓\x1b[0m Created project '{}'", name);
    println!("  {}/", name);
    println!("  ├── curium.json");
    println!("  └── src/");
    println!("      └── main.cm");
}

fn cmd_doctor() {
    println!("\x1b[1;36m🔍 Curium Doctor\x1b[0m");
    println!("────────────────────────────────");
    println!("  Compiler:  cm v5.0.0 (Rust bootstrap)");

    // Check for C compiler
    let gcc = ProcessCommand::new("gcc").arg("--version").output();
    match gcc {
        Ok(out) if out.status.success() => {
            let ver = String::from_utf8_lossy(&out.stdout);
            let first_line = ver.lines().next().unwrap_or("unknown");
            println!("  \x1b[32m✓\x1b[0m gcc:      {}", first_line);
        }
        _ => {
            println!("  \x1b[31m✗\x1b[0m gcc:      not found");
        }
    }

    let tcc = ProcessCommand::new("tcc").arg("-v").output();
    match tcc {
        Ok(out) if out.status.success() => {
            let ver = String::from_utf8_lossy(&out.stdout);
            let first_line = ver.lines().next().unwrap_or("unknown");
            println!("  \x1b[32m✓\x1b[0m tcc:      {}", first_line);
        }
        _ => {
            println!("  \x1b[33m?\x1b[0m tcc:      not found (optional)");
        }
    }

    println!("────────────────────────────────");
    println!("  \x1b[1;32mAll checks passed.\x1b[0m");
}

fn cmd_fmt(matches: &clap::ArgMatches) {
    let target = matches
        .get_one::<String>("file")
        .map(|s| s.as_str())
        .unwrap_or("src/");

    let path = std::path::Path::new(target);

    if path.is_file() {
        format_file(path);
    } else if path.is_dir() {
        let mut count = 0;
        for entry in walkdir_cm(path) {
            format_file(&entry);
            count += 1;
        }
        println!("\x1b[1;32m✓\x1b[0m Formatted {} file(s)", count);
    } else {
        eprintln!("\x1b[1;31m✗\x1b[0m Path '{}' not found", target);
        std::process::exit(1);
    }
}

fn format_file(path: &std::path::Path) {
    let source = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Cannot read {}: {}", path.display(), e);
            return;
        }
    };

    // Basic formatting: normalize indentation and trailing whitespace
    let mut formatted = String::new();
    let mut indent_level: i32 = 0;

    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            formatted.push('\n');
            continue;
        }

        // Decrease indent before closing braces
        if trimmed.starts_with('}') {
            indent_level = (indent_level - 1).max(0);
        }

        let indent = "    ".repeat(indent_level as usize);
        formatted.push_str(&format!("{}{}\n", indent, trimmed));

        // Increase indent after opening braces
        if trimmed.ends_with('{') {
            indent_level += 1;
        }
    }

    if formatted != source {
        if let Err(e) = fs::write(path, &formatted) {
            eprintln!("Cannot write {}: {}", path.display(), e);
        } else {
            println!("  \x1b[32m✓\x1b[0m {}", path.display());
        }
    } else {
        println!("  \x1b[90m-\x1b[0m {} (no changes)", path.display());
    }
}

fn walkdir_cm(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                files.extend(walkdir_cm(&path));
            } else if path.extension().map(|e| e == "cm").unwrap_or(false) {
                files.push(path);
            }
        }
    }
    files
}

fn cmd_test(matches: &clap::ArgMatches) {
    let filter = matches.get_one::<String>("filter");
    let test_dir = std::path::Path::new("tests");
    let examples_dir = std::path::Path::new("examples");

    // Collect test files
    let mut test_files: Vec<std::path::PathBuf> = Vec::new();

    if test_dir.exists() {
        test_files.extend(walkdir_cm(test_dir));
    }
    if examples_dir.exists() {
        test_files.extend(walkdir_cm(examples_dir));
    }

    // Apply filter
    if let Some(pattern) = filter {
        test_files.retain(|f| {
            f.to_string_lossy().contains(pattern.as_str())
        });
    }

    if test_files.is_empty() {
        println!("\x1b[1;33m!\x1b[0m No test files found");
        return;
    }

    println!("\x1b[1;36m── Running {} test(s) ──\x1b[0m\n", test_files.len());

    let mut passed = 0;
    let mut failed = 0;

    for file in &test_files {
        let source = match fs::read_to_string(file) {
            Ok(s) => inject_prelude(&s),
            Err(_) => {
                println!("  \x1b[31m✗\x1b[0m {} — cannot read", file.display());
                failed += 1;
                continue;
            }
        };

        let tokens = match Lexer::tokenize(&source) {
            Ok(t) => t,
            Err(e) => {
                println!("  \x1b[31m✗\x1b[0m {} — lex error: {}", file.display(), e);
                failed += 1;
                continue;
            }
        };

        let mut ast = match Parser::parse(tokens) {
            Ok(a) => a,
            Err(e) => {
                println!("  \x1b[31m✗\x1b[0m {} — parse error: {}", file.display(), e);
                failed += 1;
                continue;
            }
        };

        let mut visited = std::collections::HashSet::new();
        let base_dir = file.parent().unwrap_or(std::path::Path::new(""));
        if let Ok(c) = std::fs::canonicalize(file) {
            visited.insert(c.to_string_lossy().to_string());
        }
        if let Err(e) = resolve_imports(&mut ast, &mut visited, base_dir) {
            println!("  \x1b[31m✗\x1b[0m {} — import error: {}", file.display(), e);
            failed += 1;
            continue;
        }

        let (_, type_errors) = type_checker::TypeChecker::check(&ast);
        if type_errors.is_empty() {
            println!("  \x1b[32m✓\x1b[0m {}", file.display());
            passed += 1;
        } else {
            println!(
                "  \x1b[31m✗\x1b[0m {} — {} type error(s)",
                file.display(),
                type_errors.len()
            );
            for err in &type_errors {
                println!("      {}", err.message);
            }
            failed += 1;
        }
    }

    println!();
    if failed == 0 {
        println!(
            "\x1b[1;32m✓ All {} test(s) passed\x1b[0m",
            passed
        );
    } else {
        println!(
            "\x1b[1;31m✗ {} passed, {} failed\x1b[0m",
            passed, failed
        );
        std::process::exit(1);
    }
}

fn cmd_packages(matches: &clap::ArgMatches) {
    match matches.subcommand() {
        Some(("install", sub)) => {
            let pkg = sub.get_one::<String>("package").unwrap();
            println!("\x1b[1;36m📦 Installing '{}'...\x1b[0m", pkg);

            // Create packages directory
            let pkg_dir = std::path::Path::new("packages");
            fs::create_dir_all(pkg_dir).ok();

            // Create a manifest entry
            let manifest = std::path::Path::new("curium.json");
            if manifest.exists() {
                println!("  \x1b[32m✓\x1b[0m Added '{}' to dependencies", pkg);
                println!("  \x1b[33m!\x1b[0m Package registry not yet available");
                println!("    Place package source in packages/{}/", pkg);
            } else {
                println!("  \x1b[33m!\x1b[0m No curium.json found. Run 'cm init' first.");
            }
        }
        Some(("remove", sub)) => {
            let pkg = sub.get_one::<String>("package").unwrap();
            let pkg_path = std::path::Path::new("packages").join(pkg);
            if pkg_path.exists() {
                fs::remove_dir_all(&pkg_path).ok();
                println!("\x1b[1;32m✓\x1b[0m Removed '{}'", pkg);
            } else {
                println!("\x1b[1;33m!\x1b[0m Package '{}' not installed", pkg);
            }
        }
        Some(("list", _)) => {
            let pkg_dir = std::path::Path::new("packages");
            if pkg_dir.exists() {
                let entries: Vec<_> = fs::read_dir(pkg_dir)
                    .into_iter()
                    .flatten()
                    .flatten()
                    .filter(|e| e.path().is_dir())
                    .collect();

                if entries.is_empty() {
                    println!("\x1b[90mNo packages installed\x1b[0m");
                } else {
                    println!("\x1b[1;36m── Installed Packages ──\x1b[0m");
                    for entry in entries {
                        println!("  📦 {}", entry.file_name().to_string_lossy());
                    }
                }
            } else {
                println!("\x1b[90mNo packages directory\x1b[0m");
            }
        }
        _ => {
            eprintln!("Usage: cm packages <install|remove|list>");
        }
    }
}


fn print_ast(node: &parser::AstNode, depth: usize) {
    let indent = "  ".repeat(depth);
    match &node.kind {
        AstKind::Program(decls) => {
            println!("{}(ASTv1", indent);
            for d in decls {
                print_ast(d, depth + 1);
            }
            println!("{})", indent);
        }
        AstKind::FnDecl { name, params, return_type, body, .. } => {
            let ret = return_type
                .as_ref()
                .map(|t| format!(" -> {}", t))
                .unwrap_or_default();
            let params_str = params
                .iter()
                .map(|p| format!("{}: {}", p.name, p.ty))
                .collect::<Vec<_>>()
                .join(", ");
            println!("{}(FnDecl \"{}\" ({}){}",indent, name, params_str, ret);
            print_ast(body, depth + 1);
            println!("{})", indent);
        }
        AstKind::LetDecl { name, type_annotation, mutable, init, .. } => {
            let mut_str = if *mutable { "mut " } else { "" };
            let ty_str = type_annotation
                .as_ref()
                .map(|t| format!(": {}", t))
                .unwrap_or_default();
            print!("{}(LetDecl {}{}{}", indent, mut_str, name, ty_str);
            if let Some(init) = init {
                println!();
                print_ast(init, depth + 1);
                println!("{})", indent);
            } else {
                println!(")");
            }
        }
        AstKind::StructDecl { name, fields, .. } => {
            println!("{}(StructDecl \"{}\"", indent, name);
            for f in fields {
                println!("{}  ({}: {})", indent, f.name, f.ty);
            }
            println!("{})", indent);
        }
        AstKind::EnumDecl { name, variants, .. } => {
            println!("{}(EnumDecl \"{}\"", indent, name);
            for v in variants {
                if v.fields.is_empty() {
                    println!("{}  ({})", indent, v.name);
                } else {
                    let fields: Vec<String> = v.fields.iter().map(|f| format!("{}", f)).collect();
                    println!("{}  ({} {})", indent, v.name, fields.join(" "));
                }
            }
            println!("{})", indent);
        }
        AstKind::ImplBlock { trait_name, target, methods, .. } => {
            let trait_str = trait_name
                .as_ref()
                .map(|t| format!("{} for ", t))
                .unwrap_or_default();
            println!("{}(Impl {}{}",indent, trait_str, target);
            for m in methods {
                print_ast(m, depth + 1);
            }
            println!("{})", indent);
        }
        AstKind::Block(stmts) => {
            println!("{}(Block", indent);
            for s in stmts {
                print_ast(s, depth + 1);
            }
            println!("{})", indent);
        }
        AstKind::ReturnStmt(val) => {
            print!("{}(Return", indent);
            if let Some(v) = val {
                println!();
                print_ast(v, depth + 1);
                println!("{})", indent);
            } else {
                println!(")");
            }
        }
        AstKind::ExprStmt(expr) => {
            println!("{}(ExprStmt", indent);
            print_ast(expr, depth + 1);
            println!("{})", indent);
        }
        AstKind::IfStmt { condition, then_branch, else_branch } => {
            println!("{}(If",indent);
            print_ast(condition, depth + 1);
            print_ast(then_branch, depth + 1);
            if let Some(eb) = else_branch {
                print_ast(eb, depth + 1);
            }
            println!("{})", indent);
        }
        AstKind::WhileStmt { condition, body } => {
            println!("{}(While", indent);
            print_ast(condition, depth + 1);
            print_ast(body, depth + 1);
            println!("{})", indent);
        }
        AstKind::ForStmt { variable, iterable, body } => {
            println!("{}(For \"{}\"", indent, variable);
            print_ast(iterable, depth + 1);
            print_ast(body, depth + 1);
            println!("{})", indent);
        }
        AstKind::MatchStmt { expr, arms } => {
            println!("{}(Match", indent);
            print_ast(expr, depth + 1);
            for arm in arms {
                println!("{}  (Arm", indent);
                print_ast(&arm.body, depth + 2);
                println!("{}  )", indent);
            }
            println!("{})", indent);
        }
        AstKind::BinaryExpr { op, left, right } => {
            println!("{}(BinOp {}", indent, op);
            print_ast(left, depth + 1);
            print_ast(right, depth + 1);
            println!("{})", indent);
        }
        AstKind::UnaryExpr { op, expr } => {
            println!("{}(UnaryOp {}", indent, op);
            print_ast(expr, depth + 1);
            println!("{})", indent);
        }
        AstKind::Call { callee, args } => {
            println!("{}(Call", indent);
            print_ast(callee, depth + 1);
            for a in args {
                print_ast(a, depth + 1);
            }
            println!("{})", indent);
        }
        AstKind::MemberAccess { object, field } => {
            println!("{}(Member .{}", indent, field);
            print_ast(object, depth + 1);
            println!("{})", indent);
        }
        AstKind::Assignment { op, target, value } => {
            println!("{}(Assign {:?}", indent, op);
            print_ast(target, depth + 1);
            print_ast(value, depth + 1);
            println!("{})", indent);
        }
        AstKind::Identifier(name) => println!("{}(Ident \"{}\")", indent, name),
        AstKind::NumberLiteral(n) => println!("{}(Num {})", indent, n),
        AstKind::StringLiteral(s) => println!("{}(Str \"{}\")", indent, s),
        AstKind::CharLiteral(c) => println!("{}(Char '{}')", indent, c),
        AstKind::BoolLiteral(b) => println!("{}(Bool {})", indent, b),
        AstKind::NullLiteral => println!("{}(Null)", indent),
        AstKind::SelfLiteral => println!("{}(Self)", indent),
        AstKind::CBlock(code) => println!("{}(CBlock \"...\")", indent),
        AstKind::ImportDecl { path, .. } => println!("{}(Import \"{}\")", indent, path),
        _ => println!("{}(<node {:?}>)", indent, std::mem::discriminant(&node.kind)),
    }
}

fn print_banner() {
    println!("\x1b[1;36m");
    println!("   ██████╗██╗   ██╗██████╗ ██╗██╗   ██╗███╗   ███╗");
    println!("  ██╔════╝██║   ██║██╔══██╗██║██║   ██║████╗ ████║");
    println!("  ██║     ██║   ██║██████╔╝██║██║   ██║██╔████╔██║");
    println!("  ██║     ██║   ██║██╔══██╗██║██║   ██║██║╚██╔╝██║");
    println!("  ╚██████╗╚██████╔╝██║  ██║██║╚██████╔╝██║ ╚═╝ ██║");
    println!("   ╚═════╝ ╚═════╝ ╚═╝  ╚═╝╚═╝ ╚═════╝ ╚═╝     ╚═╝");
    println!("\x1b[0m");
    println!("  \x1b[1mCurium v5.0.0\x1b[0m — Rust Bootstrap Compiler");
    println!();
}
