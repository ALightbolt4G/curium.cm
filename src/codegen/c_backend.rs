use crate::parser::ast::*;

/// C11 code generator — walks the AST and emits C source code.
pub struct CGenerator {
    output: String,
    indent: usize,
    forward_decls: Vec<String>,
    needs_curium_string: bool,
    needs_curium_arena: bool,
    needs_curium_dyn: bool,
    needs_curium_gc: bool,
}

impl CGenerator {
    pub fn new() -> Self {
        Self {
            output: String::new(),
            indent: 0,
            forward_decls: Vec::new(),
            needs_curium_string: false,
            needs_curium_arena: false,
            needs_curium_dyn: false,
            needs_curium_gc: false,
        }
    }

    /// Generate C11 source code from a Program AST node.
    pub fn generate(ast: &AstNode) -> String {
        let mut gen = CGenerator::new();
        gen.scan_features(ast);
        gen.emit_preamble();
        gen.gen_node(ast);
        gen.finalize()
    }

    fn finalize(self) -> String {
        let mut result = String::new();

        // Preamble
        result.push_str(&self.output);

        result
    }

    // ── Feature scanning ─────────────────────────────────────────────────

    fn scan_features(&mut self, node: &AstNode) {
        match &node.kind {
            AstKind::Program(decls) => {
                for d in decls {
                    self.scan_features(d);
                }
            }
            AstKind::FnDecl { body, return_type, params, .. } => {
                if let Some(ty) = return_type {
                    self.scan_type_features(ty);
                }
                for p in params {
                    self.scan_type_features(&p.ty);
                }
                self.scan_features(body);
            }
            AstKind::LetDecl { type_annotation, init, .. } => {
                if let Some(ty) = type_annotation {
                    self.scan_type_features(ty);
                }
                if let Some(init) = init {
                    self.scan_features(init);
                }
            }
            AstKind::Block(stmts) => {
                for s in stmts {
                    self.scan_features(s);
                }
            }
            AstKind::ExprStmt(expr) => self.scan_features(expr),
            AstKind::ReturnStmt(val) => {
                if let Some(v) = val {
                    self.scan_features(v);
                }
            }
            AstKind::IfStmt { condition, then_branch, else_branch } => {
                self.scan_features(condition);
                self.scan_features(then_branch);
                if let Some(eb) = else_branch {
                    self.scan_features(eb);
                }
            }
            AstKind::WhileStmt { condition, body } => {
                self.scan_features(condition);
                self.scan_features(body);
            }
            AstKind::ForStmt { iterable, body, .. } => {
                self.scan_features(iterable);
                self.scan_features(body);
            }
            AstKind::LoopStmt { body } => self.scan_features(body),
            AstKind::MatchStmt { expr, arms } => {
                self.scan_features(expr);
                for arm in arms {
                    self.scan_features(&arm.body);
                }
            }
            AstKind::TryBlock { body, catch_arms, finally_block } => {
                self.scan_features(body);
                for c in catch_arms {
                    self.scan_features(&c.body);
                }
                if let Some(f) = finally_block {
                    self.scan_features(f);
                }
            }
            AstKind::ThrowStmt(expr) => self.scan_features(expr),
            AstKind::ReactorBlock { allocator, body, size, .. } => {
                match allocator {
                    AllocatorKind::Arena => self.needs_curium_arena = true,
                    AllocatorKind::Gc => self.needs_curium_gc = true,
                    _ => {}
                }
                if let Some(s) = size {
                    self.scan_features(s);
                }
                self.scan_features(body);
            }
            AstKind::SpawnBlock(body) => self.scan_features(body),
            AstKind::ImplBlock { methods, .. } => {
                for m in methods {
                    self.scan_features(m);
                }
            }
            AstKind::StructDecl { fields, .. } => {
                for f in fields {
                    self.scan_type_features(&f.ty);
                }
            }
            AstKind::StringLiteral(_) => {
                self.needs_curium_string = true;
            }
            AstKind::Call { callee, args, .. } => {
                if let AstKind::Identifier(name) = &callee.kind {
                    if name == "print" || name == "println" {
                        self.needs_curium_string = true;
                    }
                }
                self.scan_features(callee);
                for a in args {
                    self.scan_features(a);
                }
            }
            AstKind::BinaryExpr { left, right, .. } => {
                self.scan_features(left);
                self.scan_features(right);
            }
            AstKind::UnaryExpr { expr, .. } => self.scan_features(expr),
            AstKind::Assignment { target, value, .. } => {
                self.scan_features(target);
                self.scan_features(value);
            }
            AstKind::MemberAccess { object, .. } => self.scan_features(object),
            AstKind::Index { object, index } => {
                self.scan_features(object);
                self.scan_features(index);
            }
            _ => {}
        }
    }

    fn scan_type_features(&mut self, ty: &Type) {
        match ty {
            Type::String | Type::Str => self.needs_curium_string = true,
            Type::Dyn => self.needs_curium_dyn = true,
            Type::Ptr(inner) | Type::Array(inner) | Type::Optional(inner) => {
                self.scan_type_features(inner);
            }
            Type::Generic(_, args) => {
                for a in args {
                    self.scan_type_features(a);
                }
            }
            _ => {}
        }
    }

    // ── Preamble ─────────────────────────────────────────────────────────

    fn emit_preamble(&mut self) {
        self.emit_line("/* Generated by Curium v5.0 compiler */");
        self.emit_line("#include <stdio.h>");
        self.emit_line("#include <stdlib.h>");
        self.emit_line("#include <stdint.h>");
        self.emit_line("#include <stdbool.h>");
        self.emit_line("#include <string.h>");
        self.emit_line("");

        if self.needs_curium_string {
            self.emit_line("/* ── Curium String Runtime ── */");
            self.emit_line("typedef struct curium_string_t {");
            self.emit_line("    char* data;");
            self.emit_line("    size_t length;");
            self.emit_line("    size_t capacity;");
            self.emit_line("} curium_string_t;");
            self.emit_line("");
            self.emit_line("static curium_string_t* curium_string_from(const char* s) {");
            self.emit_line("    curium_string_t* str = (curium_string_t*)malloc(sizeof(curium_string_t));");
            self.emit_line("    str->length = strlen(s);");
            self.emit_line("    str->capacity = str->length + 1;");
            self.emit_line("    str->data = (char*)malloc(str->capacity);");
            self.emit_line("    memcpy(str->data, s, str->capacity);");
            self.emit_line("    return str;");
            self.emit_line("}");
            self.emit_line("");
            self.emit_line("static void curium_string_free(curium_string_t* s) {");
            self.emit_line("    if (s) { free(s->data); free(s); }");
            self.emit_line("}");
            self.emit_line("");
        }

        if self.needs_curium_arena {
            self.emit_line("/* ── Curium Arena Allocator ── */");
            self.emit_line("typedef struct curium_arena_t {");
            self.emit_line("    uint8_t* memory;");
            self.emit_line("    size_t capacity;");
            self.emit_line("    size_t offset;");
            self.emit_line("} curium_arena_t;");
            self.emit_line("");
            self.emit_line("static curium_arena_t curium_arena_create(size_t cap) {");
            self.emit_line("    curium_arena_t a;");
            self.emit_line("    a.memory = (uint8_t*)malloc(cap);");
            self.emit_line("    a.capacity = cap;");
            self.emit_line("    a.offset = 0;");
            self.emit_line("    return a;");
            self.emit_line("}");
            self.emit_line("");
            self.emit_line("static void* curium_arena_alloc(curium_arena_t* a, size_t size) {");
            self.emit_line("    size = (size + 7) & ~7; /* align to 8 */");
            self.emit_line("    if (a->offset + size > a->capacity) return NULL;");
            self.emit_line("    void* ptr = a->memory + a->offset;");
            self.emit_line("    a->offset += size;");
            self.emit_line("    return ptr;");
            self.emit_line("}");
            self.emit_line("");
            self.emit_line("static void curium_arena_destroy(curium_arena_t* a) {");
            self.emit_line("    free(a->memory);");
            self.emit_line("    a->memory = NULL;");
            self.emit_line("}");
            self.emit_line("");
        }

        if self.needs_curium_dyn {
            self.emit_line("/* ── Curium Dynamic Type ── */");
            self.emit_line("typedef struct curium_dyn_t {");
            self.emit_line("    enum { DYN_INT, DYN_FLOAT, DYN_STRING, DYN_PTR, DYN_NULL } tag;");
            self.emit_line("    union {");
            self.emit_line("        int64_t int_val;");
            self.emit_line("        double float_val;");
            self.emit_line("        curium_string_t* string_val;");
            self.emit_line("        void* ptr_val;");
            self.emit_line("    };");
            self.emit_line("} curium_dyn_t;");
            self.emit_line("");
        }

        // Print / Println builtins
        self.emit_line("/* ── Curium Builtins ── */");
        if self.needs_curium_string {
            self.emit_line("static void curium_print(curium_string_t* s) {");
            self.emit_line("    if (s) printf(\"%s\", s->data);");
            self.emit_line("}");
            self.emit_line("");
            self.emit_line("static void curium_println(curium_string_t* s) {");
            self.emit_line("    if (s) printf(\"%s\\n\", s->data);");
            self.emit_line("}");
        } else {
            self.emit_line("static void curium_print_i32(int32_t v) { printf(\"%d\", v); }");
            self.emit_line("static void curium_println_i32(int32_t v) { printf(\"%d\\n\", v); }");
        }
        self.emit_line("");
    }

    // ── Code generation ──────────────────────────────────────────────────

    fn gen_node(&mut self, node: &AstNode) {
        match &node.kind {
            AstKind::Program(decls) => {
                // First pass: emit struct/enum forward declarations
                for d in decls {
                    match &d.kind {
                        AstKind::StructDecl { name, .. } => {
                            self.emit_line(&format!("typedef struct {} {};", name, name));
                        }
                        AstKind::EnumDecl { name, .. } => {
                            self.emit_line(&format!("typedef struct {} {};", name, name));
                        }
                        _ => {}
                    }
                }

                // Second pass: emit function forward declarations
                for d in decls {
                    if let AstKind::FnDecl { name, params, return_type, .. } = &d.kind {
                        if name != "main" {
                            let ret = self.type_to_c(return_type.as_ref().unwrap_or(&Type::Void));
                            let params_c = self.params_to_c(params);
                            self.emit_line(&format!("{} {}({});", ret, name, params_c));
                        }
                    }
                }
                self.emit_line("");

                // Third pass: emit all declarations
                for d in decls {
                    self.gen_node(d);
                    self.emit_line("");
                }
            }

            AstKind::FnDecl { name, params, return_type, body, .. } => {
                let ret = self.type_to_c(return_type.as_ref().unwrap_or(&Type::Void));
                let params_c = self.params_to_c(params);

                // main function gets special treatment
                if name == "main" {
                    self.emit_line(&format!("int main(int argc, char** argv) {{"));
                } else {
                    self.emit_line(&format!("{} {}({}) {{", ret, name, params_c));
                }

                self.indent += 1;
                self.gen_block_contents(body);
                self.indent -= 1;
                self.emit_line("}");
            }

            AstKind::StructDecl { name, fields, .. } => {
                self.emit_line(&format!("struct {} {{", name));
                self.indent += 1;
                for field in fields {
                    let ty_c = self.type_to_c(&field.ty);
                    self.emit_line(&format!("{} {};", ty_c, field.name));
                }
                self.indent -= 1;
                self.emit_line("};");
            }

            AstKind::EnumDecl { name, variants, .. } => {
                // Emit tag enum
                self.emit_line(&format!("enum {}_tag {{", name));
                self.indent += 1;
                for (i, v) in variants.iter().enumerate() {
                    let comma = if i < variants.len() - 1 { "," } else { "" };
                    self.emit_line(&format!("{}_{}{}", name.to_uppercase(), v.name.to_uppercase(), comma));
                }
                self.indent -= 1;
                self.emit_line("};");
                self.emit_line("");

                // Emit tagged union struct
                self.emit_line(&format!("struct {} {{", name));
                self.indent += 1;
                self.emit_line(&format!("enum {}_tag tag;", name));
                self.emit_line("union {");
                self.indent += 1;
                for v in variants {
                    if !v.fields.is_empty() {
                        self.emit_line(&format!("struct {{"));
                        self.indent += 1;
                        for (i, f) in v.fields.iter().enumerate() {
                            let ty_c = self.type_to_c(f);
                            self.emit_line(&format!("{} _{};", ty_c, i));
                        }
                        self.indent -= 1;
                        self.emit_line(&format!("}} {};", v.name.to_lowercase()));
                    }
                }
                self.indent -= 1;
                self.emit_line("} data;");
                self.indent -= 1;
                self.emit_line("};");
            }

            AstKind::ImplBlock { target, methods, .. } => {
                for method in methods {
                    if let AstKind::FnDecl { name, params, return_type, body, .. } = &method.kind {
                        let ret = self.type_to_c(return_type.as_ref().unwrap_or(&Type::Void));
                        let mut c_params = Vec::new();

                        // Add self parameter
                        if params.first().map(|p| p.name == "self").unwrap_or(false) {
                            let mutability = if params[0].mutable { "" } else { "const " };
                            c_params.push(format!("{}{}* self", mutability, target));
                            for p in &params[1..] {
                                c_params.push(format!("{} {}", self.type_to_c(&p.ty), p.name));
                            }
                        } else {
                            for p in params {
                                c_params.push(format!("{} {}", self.type_to_c(&p.ty), p.name));
                            }
                        }

                        let full_name = format!("{}_{}", target, name);
                        self.emit_line(&format!(
                            "{} {}({}) {{",
                            ret,
                            full_name,
                            c_params.join(", ")
                        ));
                        self.indent += 1;
                        self.gen_block_contents(body);
                        self.indent -= 1;
                        self.emit_line("}");
                        self.emit_line("");
                    }
                }
            }

            AstKind::Block(stmts) => {
                for s in stmts {
                    self.gen_node(s);
                }
            }

            AstKind::LetDecl { name, type_annotation, init, mutable } => {
                let ty = if let Some(ty) = type_annotation {
                    self.type_to_c(ty)
                } else if let Some(init) = init {
                    self.infer_c_type(init)
                } else {
                    "int32_t".to_string()
                };

                let qualifier = if !mutable { "const " } else { "" };

                if let Some(init_expr) = init {
                    let val = self.expr_to_c(init_expr);
                    self.emit_line(&format!("{}{} {} = {};", qualifier, ty, name, val));
                } else {
                    self.emit_line(&format!("{}{} {};", qualifier, ty, name));
                }
            }

            AstKind::ReturnStmt(value) => {
                if let Some(expr) = value {
                    let val = self.expr_to_c(expr);
                    self.emit_line(&format!("return {};", val));
                } else {
                    self.emit_line("return;");
                }
            }

            AstKind::ExprStmt(expr) => {
                let val = self.expr_to_c(expr);
                self.emit_line(&format!("{};", val));
            }

            AstKind::IfStmt { condition, then_branch, else_branch } => {
                let cond = self.expr_to_c(condition);
                self.emit_line(&format!("if ({}) {{", cond));
                self.indent += 1;
                self.gen_block_contents(then_branch);
                self.indent -= 1;
                if let Some(else_br) = else_branch {
                    match &else_br.kind {
                        AstKind::IfStmt { .. } => {
                            self.emit_raw("} else ");
                            self.gen_node(else_br);
                            return;
                        }
                        _ => {
                            self.emit_line("} else {");
                            self.indent += 1;
                            self.gen_block_contents(else_br);
                            self.indent -= 1;
                        }
                    }
                }
                self.emit_line("}");
            }

            AstKind::WhileStmt { condition, body } => {
                let cond = self.expr_to_c(condition);
                self.emit_line(&format!("while ({}) {{", cond));
                self.indent += 1;
                self.gen_block_contents(body);
                self.indent -= 1;
                self.emit_line("}");
            }

            AstKind::ForStmt { variable, iterable, body } => {
                // Simple range-based for: for i in 0..10 { }
                match &iterable.kind {
                    AstKind::BinaryExpr { op: BinOp::Range, left, right } => {
                        let start_val = self.expr_to_c(left);
                        let end_val = self.expr_to_c(right);
                        self.emit_line(&format!(
                            "for (int64_t {} = {}; {} < {}; {}++) {{",
                            variable, start_val, variable, end_val, variable
                        ));
                    }
                    _ => {
                        let iter_val = self.expr_to_c(iterable);
                        self.emit_line(&format!(
                            "/* for {} in {} */",
                            variable, iter_val
                        ));
                        self.emit_line("{ /* TODO: iterator protocol */");
                    }
                }
                self.indent += 1;
                self.gen_block_contents(body);
                self.indent -= 1;
                self.emit_line("}");
            }

            AstKind::LoopStmt { body } => {
                self.emit_line("for (;;) {");
                self.indent += 1;
                self.gen_block_contents(body);
                self.indent -= 1;
                self.emit_line("}");
            }

            AstKind::BreakStmt => {
                self.emit_line("break;");
            }

            AstKind::ContinueStmt => {
                self.emit_line("continue;");
            }

            AstKind::MatchStmt { expr, arms } => {
                let val = self.expr_to_c(expr);
                for (i, arm) in arms.iter().enumerate() {
                    let keyword = if i == 0 { "if" } else { "} else if" };
                    match &arm.pattern {
                        Pattern::Literal(lit) => {
                            let lit_c = self.expr_to_c(lit);
                            self.emit_line(&format!("{} ({} == {}) {{", keyword, val, lit_c));
                        }
                        Pattern::Identifier(name) => {
                            if name == "_" {
                                self.emit_line("} else {");
                            } else {
                                self.emit_line(&format!(
                                    "{} (1) {{ /* bind {} = {} */",
                                    keyword, name, val
                                ));
                            }
                        }
                        Pattern::Wildcard => {
                            if i == 0 {
                                self.emit_line("{");
                            } else {
                                self.emit_line("} else {");
                            }
                        }
                        Pattern::EnumVariant { path, .. } => {
                            let tag = path.join("_").to_uppercase();
                            self.emit_line(&format!("{} ({}.tag == {}) {{", keyword, val, tag));
                        }
                    }
                    self.indent += 1;
                    self.gen_block_contents(&arm.body);
                    self.indent -= 1;
                }
                self.emit_line("}");
            }

            AstKind::TryBlock { body, catch_arms, finally_block } => {
                self.emit_line("/* try { */");
                self.emit_line("{");
                self.indent += 1;
                self.gen_block_contents(body);
                self.indent -= 1;
                self.emit_line("}");
                for catch in catch_arms {
                    self.emit_line(&format!(
                        "/* catch ({}) {{ }} */",
                        catch.binding
                    ));
                }
                if let Some(finally_blk) = finally_block {
                    self.emit_line("/* finally */");
                    self.emit_line("{");
                    self.indent += 1;
                    self.gen_block_contents(finally_blk);
                    self.indent -= 1;
                    self.emit_line("}");
                }
            }

            AstKind::ThrowStmt(expr) => {
                let val = self.expr_to_c(expr);
                self.emit_line(&format!("/* throw {} */", val));
                self.emit_line(&format!("fprintf(stderr, \"Curium panic: throw\\n\");"));
                self.emit_line("exit(1);");
            }

            AstKind::ReactorBlock { allocator, size, body } => {
                match allocator {
                    AllocatorKind::Arena => {
                        let sz = size.as_ref().map(|s| self.expr_to_c(s)).unwrap_or_else(|| "4096".to_string());
                        self.emit_line(&format!("{{ /* reactor arena */"));
                        self.indent += 1;
                        self.emit_line(&format!("curium_arena_t __arena = curium_arena_create({});", sz));
                        self.gen_block_contents(body);
                        self.emit_line("curium_arena_destroy(&__arena);");
                        self.indent -= 1;
                        self.emit_line("}");
                    }
                    _ => {
                        self.emit_line("{ /* reactor block */");
                        self.indent += 1;
                        self.gen_block_contents(body);
                        self.indent -= 1;
                        self.emit_line("}");
                    }
                }
            }

            AstKind::SpawnBlock(body) => {
                self.emit_line("/* spawn { ... } — threading not yet implemented */");
                self.emit_line("{");
                self.indent += 1;
                self.gen_block_contents(body);
                self.indent -= 1;
                self.emit_line("}");
            }

            AstKind::CBlock(code) => {
                self.emit_line("/* c { } verbatim */");
                for line in code.lines() {
                    self.emit_line(line);
                }
            }

            AstKind::CppBlock(code) => {
                self.emit_line("/* cpp { } verbatim */");
                for line in code.lines() {
                    self.emit_line(line);
                }
            }

            _ => {
                // Expression nodes handled inline
            }
        }
    }

    fn gen_block_contents(&mut self, node: &AstNode) {
        match &node.kind {
            AstKind::Block(stmts) => {
                for s in stmts {
                    self.gen_node(s);
                }
            }
            _ => {
                self.gen_node(node);
            }
        }
    }

    // ── Expression to C string ───────────────────────────────────────────

    fn expr_to_c(&self, node: &AstNode) -> String {
        match &node.kind {
            AstKind::NumberLiteral(n) => n.clone(),
            AstKind::StringLiteral(s) => {
                format!("curium_string_from(\"{}\")", s.replace('\\', "\\\\").replace('"', "\\\""))
            }
            AstKind::CharLiteral(c) => format!("'{}'", c),
            AstKind::BoolLiteral(b) => if *b { "true".to_string() } else { "false".to_string() },
            AstKind::NullLiteral => "NULL".to_string(),
            AstKind::SelfLiteral => "self".to_string(),

            AstKind::Identifier(name) => {
                match name.as_str() {
                    "print" => "curium_print".to_string(),
                    "println" => "curium_println".to_string(),
                    _ => name.clone(),
                }
            }

            AstKind::BinaryExpr { op, left, right } => {
                let l = self.expr_to_c(left);
                let r = self.expr_to_c(right);
                format!("({} {} {})", l, op, r)
            }

            AstKind::UnaryExpr { op, expr } => {
                let e = self.expr_to_c(expr);
                match op {
                    UnaryOp::Deref => format!("(*{})", e),
                    UnaryOp::AddrOf => format!("(&{})", e),
                    _ => format!("({}{})", op, e),
                }
            }

            AstKind::Assignment { op, target, value } => {
                let t = self.expr_to_c(target);
                let v = self.expr_to_c(value);
                let op_str = match op {
                    AssignOp::Assign => "=",
                    AssignOp::AddAssign => "+=",
                    AssignOp::SubAssign => "-=",
                    AssignOp::MulAssign => "*=",
                    AssignOp::DivAssign => "/=",
                    AssignOp::ModAssign => "%=",
                };
                format!("{} {} {}", t, op_str, v)
            }

            AstKind::Call { callee, args } => {
                let func = self.expr_to_c(callee);
                let args_c: Vec<String> = args.iter().map(|a| self.expr_to_c(a)).collect();
                format!("{}({})", func, args_c.join(", "))
            }

            AstKind::MemberAccess { object, field } => {
                let obj = self.expr_to_c(object);
                format!("{}.{}", obj, field)
            }

            AstKind::Index { object, index } => {
                let obj = self.expr_to_c(object);
                let idx = self.expr_to_c(index);
                format!("{}[{}]", obj, idx)
            }

            AstKind::PathExpr(path) => {
                // Enum variant: Foo::Bar → FOO_BAR
                if path.len() == 2 {
                    format!("{}_{}", path[0].to_uppercase(), path[1].to_uppercase())
                } else {
                    path.join("_")
                }
            }

            AstKind::StructLiteral { name, fields } => {
                let mut parts = Vec::new();
                for f in fields {
                    if let AstKind::FieldInit { name: fname, value } = &f.kind {
                        let v = self.expr_to_c(value);
                        parts.push(format!(".{} = {}", fname, v));
                    }
                }
                format!("({}) {{ {} }}", name, parts.join(", "))
            }

            AstKind::ArrayLiteral(elements) => {
                let elems: Vec<String> = elements.iter().map(|e| self.expr_to_c(e)).collect();
                format!("{{ {} }}", elems.join(", "))
            }

            AstKind::CastExpr { expr, target_type } => {
                let e = self.expr_to_c(expr);
                let ty = self.type_to_c(target_type);
                format!("(({}){})", ty, e)
            }

            AstKind::TryExpr(expr) => {
                let e = self.expr_to_c(expr);
                format!("({}) /* ? */", e)
            }

            _ => format!("/* <unhandled expr> */"),
        }
    }

    // ── Type mapping ─────────────────────────────────────────────────────

    fn type_to_c(&self, ty: &Type) -> String {
        match ty {
            Type::I8 => "int8_t".to_string(),
            Type::I16 => "int16_t".to_string(),
            Type::I32 => "int32_t".to_string(),
            Type::I64 => "int64_t".to_string(),
            Type::U8 => "uint8_t".to_string(),
            Type::U16 => "uint16_t".to_string(),
            Type::U32 => "uint32_t".to_string(),
            Type::U64 => "uint64_t".to_string(),
            Type::F32 => "float".to_string(),
            Type::F64 => "double".to_string(),
            Type::Usize => "size_t".to_string(),
            Type::Bool => "bool".to_string(),
            Type::Char => "char".to_string(),
            Type::String | Type::Str => "curium_string_t*".to_string(),
            Type::Void => "void".to_string(),
            Type::Dyn => "curium_dyn_t".to_string(),
            Type::Strnum => "curium_dyn_t".to_string(),
            Type::Ptr(inner) => format!("{}*", self.type_to_c(inner)),
            Type::Array(inner) => format!("{}*", self.type_to_c(inner)),
            Type::Slice(inner) => format!("{}*", self.type_to_c(inner)),
            Type::Named(name) => name.clone(),
            Type::Generic(name, _) => name.clone(),
            Type::Function { ret, .. } => format!("{}*", self.type_to_c(ret)),
            Type::Optional(inner) => format!("{}*", self.type_to_c(inner)),
            Type::Inferred => "void*".to_string(),
        }
    }

    fn params_to_c(&self, params: &[Param]) -> String {
        if params.is_empty() {
            return "void".to_string();
        }
        params
            .iter()
            .map(|p| format!("{} {}", self.type_to_c(&p.ty), p.name))
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn infer_c_type(&self, expr: &AstNode) -> String {
        match &expr.kind {
            AstKind::NumberLiteral(n) => {
                if n.contains('.') {
                    "double".to_string()
                } else {
                    "int32_t".to_string()
                }
            }
            AstKind::StringLiteral(_) => "curium_string_t*".to_string(),
            AstKind::BoolLiteral(_) => "bool".to_string(),
            AstKind::CharLiteral(_) => "char".to_string(),
            AstKind::NullLiteral => "void*".to_string(),
            _ => "int32_t".to_string(),
        }
    }

    // ── Output helpers ───────────────────────────────────────────────────

    fn emit_line(&mut self, line: &str) {
        let indent_str = "    ".repeat(self.indent);
        self.output.push_str(&indent_str);
        self.output.push_str(line);
        self.output.push('\n');
    }

    fn emit_raw(&mut self, text: &str) {
        let indent_str = "    ".repeat(self.indent);
        self.output.push_str(&indent_str);
        self.output.push_str(text);
    }
}

impl Default for CGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    #[test]
    fn test_hello_world_codegen() {
        let tokens = Lexer::tokenize(r#"fn main() -> i32 { println("Hello, Curium!"); return 0; }"#).unwrap();
        let ast = Parser::parse(tokens).unwrap();
        let c_code = CGenerator::generate(&ast);
        assert!(c_code.contains("curium_println"));
        assert!(c_code.contains("Hello, Curium!"));
        assert!(c_code.contains("int main("));
        assert!(c_code.contains("return 0;"));
    }

    #[test]
    fn test_struct_codegen() {
        let tokens = Lexer::tokenize("struct Point { x: f64, y: f64 }").unwrap();
        let ast = Parser::parse(tokens).unwrap();
        let c_code = CGenerator::generate(&ast);
        assert!(c_code.contains("struct Point"));
        assert!(c_code.contains("double x;"));
        assert!(c_code.contains("double y;"));
    }

    #[test]
    fn test_variable_codegen() {
        let tokens = Lexer::tokenize("fn test() { let x: i32 = 42; mut y: i32 = 10; }").unwrap();
        let ast = Parser::parse(tokens).unwrap();
        let c_code = CGenerator::generate(&ast);
        assert!(c_code.contains("const int32_t x = 42;"));
        assert!(c_code.contains("int32_t y = 10;"));
    }
}
