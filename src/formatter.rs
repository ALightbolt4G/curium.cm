use crate::parser::ast::{AstNode, AstKind};

pub struct Formatter {
    output: String,
    indent: usize,
}

impl Formatter {
    pub fn new() -> Self {
        Self {
            output: String::new(),
            indent: 0,
        }
    }

    pub fn format(ast: &AstNode) -> String {
        let mut fmt = Formatter::new();
        fmt.format_node(ast);
        fmt.output
    }

    fn push(&mut self, s: &str) {
        self.output.push_str(s);
    }

    fn push_line(&mut self, s: &str) {
        if !self.output.ends_with('\n') && !self.output.is_empty() {
            self.output.push('\n');
        }
        let ind = "    ".repeat(self.indent);
        self.output.push_str(&ind);
        self.output.push_str(s);
    }

    fn new_line(&mut self) {
        self.output.push('\n');
    }

    fn format_node(&mut self, node: &AstNode) {
        match &node.kind {
            AstKind::Program(decls) => {
                for (i, d) in decls.iter().enumerate() {
                    self.format_node(d);
                    if i < decls.len() - 1 {
                        self.new_line();
                        self.new_line();
                    }
                }
                self.new_line();
            }
            AstKind::FnDecl { name, params, return_type, body, is_pub, attributes } => {
                for attr in attributes {
                    self.push_line(&format!("#[{}]", attr));
                }
                let pub_kw = if *is_pub { "pub " } else { "" };
                let mut params_str = Vec::new();
                for p in params {
                    params_str.push(format!("{}: {}", p.name, p.ty));
                }
                let ret_str = match return_type {
                    Some(t) => format!(" -> {}", t),
                    None => String::new(),
                };
                self.push_line(&format!("{}fn {}({}){} ", pub_kw, name, params_str.join(", "), ret_str));
                self.format_node(body);
            }
            AstKind::LetDecl { name, type_annotation, init, mutable } => {
                let mut_kw = if *mutable { "mut " } else { "" };
                let type_str = match type_annotation {
                    Some(t) => format!(": {}", t),
                    None => String::new(),
                };
                self.push_line(&format!("let {}{}{}", mut_kw, name, type_str));
                if let Some(i) = init {
                    self.push(" = ");
                    self.format_node(i);
                }
                self.push(";");
            }
            AstKind::StructDecl { name, fields, is_pub } => {
                let pub_kw = if *is_pub { "pub " } else { "" };
                self.push_line(&format!("{}struct {} {{", pub_kw, name));
                self.indent += 1;
                for f in fields {
                    let f_pub = if f.is_pub { "pub " } else { "" };
                    self.push_line(&format!("{}{}: {};", f_pub, f.name, f.ty));
                }
                self.indent -= 1;
                self.push_line("}");
            }
            AstKind::EnumDecl { name, variants, is_pub } => {
                let pub_kw = if *is_pub { "pub " } else { "" };
                self.push_line(&format!("{}enum {} {{", pub_kw, name));
                self.indent += 1;
                for v in variants {
                    if v.fields.is_empty() {
                        self.push_line(&format!("{},", v.name));
                    } else {
                        let ts: Vec<String> = v.fields.iter().map(|t| format!("{}", t)).collect();
                        self.push_line(&format!("{}({}),", v.name, ts.join(", ")));
                    }
                }
                self.indent -= 1;
                self.push_line("}");
            }
            AstKind::ImportDecl { path, alias } => {
                self.push_line(&format!("import \"{}\"", path));
                if let Some(a) = alias {
                    self.push(&format!(" as {}", a));
                }
                self.push(";");
            }
            AstKind::Block(stmts) => {
                self.push("{");
                self.indent += 1;
                for s in stmts {
                    self.format_node(s);
                }
                self.indent -= 1;
                self.push_line("}");
            }
            AstKind::IfStmt { condition, then_branch, else_branch } => {
                self.push_line("if ");
                self.format_node(condition);
                self.push(" ");
                self.format_node(then_branch);
                if let Some(e) = else_branch {
                    self.push(" else ");
                    self.format_node(e);
                }
            }
            AstKind::WhileStmt { condition, body } => {
                self.push_line("while ");
                self.format_node(condition);
                self.push(" ");
                self.format_node(body);
            }
            AstKind::ExprStmt(expr) => {
                self.push_line(""); // establish indent
                self.format_node(expr);
                self.push(";");
            }
            AstKind::ReturnStmt(expr) => {
                self.push_line("return");
                if let Some(e) = expr {
                    self.push(" ");
                    self.format_node(e);
                }
                self.push(";");
            }
            AstKind::Assignment { op, target, value } => {
                let op_str = match op {
                    crate::parser::ast::AssignOp::Assign => "=",
                    crate::parser::ast::AssignOp::AddAssign => "+=",
                    crate::parser::ast::AssignOp::SubAssign => "-=",
                    crate::parser::ast::AssignOp::MulAssign => "*=",
                    crate::parser::ast::AssignOp::DivAssign => "/=",
                    crate::parser::ast::AssignOp::ModAssign => "%=",
                };
                self.format_node(target);
                self.push(&format!(" {} ", op_str));
                self.format_node(value);
            }
            AstKind::BinaryExpr { op, left, right } => {
                self.format_node(left);
                self.push(&format!(" {} ", op));
                self.format_node(right);
            }
            AstKind::UnaryExpr { op, expr } => {
                self.push(&format!("{}", op));
                self.format_node(expr);
            }
            AstKind::Call { callee, args } => {
                self.format_node(callee);
                self.push("(");
                for (i, a) in args.iter().enumerate() {
                    self.format_node(a);
                    if i < args.len() - 1 {
                        self.push(", ");
                    }
                }
                self.push(")");
            }
            AstKind::MemberAccess { object, field } => {
                self.format_node(object);
                self.push(&format!(".{}", field));
            }
            AstKind::Identifier(name) => self.push(name),
            AstKind::NumberLiteral(n) => self.push(n),
            AstKind::StringLiteral(s) => self.push(&format!("\"{}\"", s)),
            AstKind::BoolLiteral(b) => self.push(if *b { "true" } else { "false" }),
            AstKind::NullLiteral => self.push("null"),
            _ => {
                // Fallback for missing nodes to avoid data loss
                self.push("/* TODO FMT NODE */");
            }
        }
    }
}
