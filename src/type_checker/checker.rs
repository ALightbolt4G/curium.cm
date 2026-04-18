use crate::parser::ast::*;
use crate::lexer::Span;
use super::symbol_table::{Symbol, SymbolKind, SymbolTable, ScopeKind};
use std::collections::HashMap;

/// Type errors collected during checking.
#[derive(Debug, Clone)]
pub struct TypeError {
    pub message: String,
    pub line: usize,
    pub column: usize,
}

/// Walks the AST and validates type correctness.
pub struct TypeChecker {
    pub symbols: SymbolTable,
    pub errors: Vec<TypeError>,
    pub node_types: HashMap<Span, Type>,
    current_return_type: Option<Type>,
}

impl TypeChecker {
    pub fn new() -> Self {
        Self {
            symbols: SymbolTable::new(),
            errors: Vec::new(),
            node_types: HashMap::new(),
            current_return_type: None,
        }
    }

    /// Type check the entire program.
    pub fn check(ast: &AstNode) -> (SymbolTable, Vec<TypeError>, HashMap<Span, Type>) {
        let mut checker = TypeChecker::new();
        checker.check_node(ast);
        (checker.symbols, checker.errors, checker.node_types)
    }

    fn error(&mut self, span: &crate::lexer::Span, msg: &str) {
        self.errors.push(TypeError {
            message: msg.to_string(),
            line: span.start,
            column: span.end,
        });
    }

    fn check_node(&mut self, node: &AstNode) {
        match &node.kind {
            AstKind::Program(decls) => {
                // First pass: register all top-level declarations
                for d in decls {
                    self.register_declaration(d);
                }
                // Second pass: type check bodies
                for d in decls {
                    self.check_node(d);
                }
            }

            AstKind::FnDecl {
                name: _,
                params,
                return_type,
                body,
                ..
            } => {
                let ret = return_type.clone().unwrap_or(Type::Void);
                let prev_return = self.current_return_type.replace(ret.clone());

                self.symbols.push_scope(ScopeKind::Function);

                for p in params {
                    self.symbols.define(Symbol {
                        name: p.name.clone(),
                        ty: p.ty.clone(),
                        mutable: p.mutable,
                        kind: SymbolKind::Parameter,
                    });
                }

                self.check_node(body);

                self.symbols.pop_scope();
                self.current_return_type = prev_return;
            }

            AstKind::StructDecl { name: _, fields: _, .. } => {
                // Already registered in first pass
            }

            AstKind::EnumDecl { name: _, variants: _, .. } => {
                // Already registered in first pass
            }

            AstKind::ImplBlock {
                target, methods, ..
            } => {
                // Verify target exists
                if self.symbols.resolve(target).is_none() {
                    self.error(&node.span, &format!("Impl target '{}' not found", target));
                    return;
                }

                for method in methods {
                    if let AstKind::FnDecl {
                        name,
                        params,
                        return_type,
                        body,
                        ..
                    } = &method.kind
                    {
                        let full_name = format!("{}_{}", target, name);
                        let param_types: Vec<Type> = params.iter().map(|p| p.ty.clone()).collect();
                        let ret = return_type.clone().unwrap_or(Type::Void);

                        self.symbols.define(Symbol {
                            name: full_name,
                            ty: ret.clone(),
                            mutable: false,
                            kind: SymbolKind::Function {
                                params: param_types,
                                return_type: ret.clone(),
                            },
                        });

                        let prev_return = self.current_return_type.replace(ret);
                        self.symbols.push_scope(ScopeKind::Impl(target.clone()));

                        // Define `self` in method scope
                        if params.first().map(|p| p.name == "self").unwrap_or(false) {
                            self.symbols.define(Symbol {
                                name: "self".to_string(),
                                ty: Type::Named(target.clone()),
                                mutable: params[0].mutable,
                                kind: SymbolKind::Parameter,
                            });
                        }

                        for p in params {
                            if p.name != "self" {
                                self.symbols.define(Symbol {
                                    name: p.name.clone(),
                                    ty: p.ty.clone(),
                                    mutable: p.mutable,
                                    kind: SymbolKind::Parameter,
                                });
                            }
                        }

                        self.check_node(body);
                        self.symbols.pop_scope();
                        self.current_return_type = prev_return;
                    }
                }
            }

            AstKind::Block(stmts) => {
                for s in stmts {
                    self.check_node(s);
                }
            }

            AstKind::LetDecl {
                name,
                type_annotation,
                init,
                mutable,
            } => {
                let declared_ty = type_annotation.clone();
                let init_ty = init.as_ref().map(|e| self.infer_expr(e));

                let resolved_ty = match (&declared_ty, &init_ty) {
                    (Some(declared), Some(inferred)) => {
                        if !self.types_compatible(declared, inferred) {
                            self.error(
                                &node.span,
                                &format!(
                                    "Type mismatch: declared '{}' but initialized with '{}'",
                                    declared, inferred
                                ),
                            );
                        }
                        declared.clone()
                    }
                    (Some(declared), None) => declared.clone(),
                    (None, Some(inferred)) => inferred.clone(),
                    (None, None) => {
                        self.error(&node.span, &format!("Cannot infer type for '{}'", name));
                        Type::Inferred
                    }
                };

                self.symbols.define(Symbol {
                    name: name.clone(),
                    ty: resolved_ty,
                    mutable: *mutable,
                    kind: SymbolKind::Variable,
                });
            }

            AstKind::ReturnStmt(value) => {
                if let Some(expected) = self.current_return_type.clone() {
                    match value {
                        Some(expr) => {
                            let actual = self.infer_expr(expr);
                            if !self.types_compatible(&expected, &actual) {
                                self.error(
                                    &node.span,
                                    &format!(
                                        "Return type mismatch: expected '{}', got '{}'",
                                        expected, actual
                                    ),
                                );
                            }
                        }
                        None => {
                            if expected != Type::Void {
                                self.error(
                                    &node.span,
                                    &format!(
                                        "Missing return value, expected '{}'",
                                        expected
                                    ),
                                );
                            }
                        }
                    }
                }
            }

            AstKind::IfStmt {
                condition,
                then_branch,
                else_branch,
            } => {
                let cond_ty = self.infer_expr(condition);
                if !self.types_compatible(&Type::Bool, &cond_ty) {
                    self.error(
                        &condition.span,
                        &format!("If condition must be bool, got '{}'", cond_ty),
                    );
                }
                self.symbols.push_scope(ScopeKind::Block);
                self.check_node(then_branch);
                self.symbols.pop_scope();
                if let Some(eb) = else_branch {
                    self.symbols.push_scope(ScopeKind::Block);
                    self.check_node(eb);
                    self.symbols.pop_scope();
                }
            }

            AstKind::WhileStmt { condition, body } => {
                let cond_ty = self.infer_expr(condition);
                if !self.types_compatible(&Type::Bool, &cond_ty) {
                    self.error(
                        &condition.span,
                        &format!("While condition must be bool, got '{}'", cond_ty),
                    );
                }
                self.symbols.push_scope(ScopeKind::Loop);
                self.check_node(body);
                self.symbols.pop_scope();
            }

            AstKind::ForStmt {
                variable,
                iterable,
                body,
            } => {
                let iter_ty = self.infer_expr(iterable);
                let elem_ty = match &iter_ty {
                    Type::Array(inner) => *inner.clone(),
                    _ => Type::I64, // Default for range expressions
                };

                self.symbols.push_scope(ScopeKind::Loop);
                self.symbols.define(Symbol {
                    name: variable.clone(),
                    ty: elem_ty,
                    mutable: false,
                    kind: SymbolKind::Variable,
                });
                self.check_node(body);
                self.symbols.pop_scope();
            }

            AstKind::LoopStmt { body } => {
                self.symbols.push_scope(ScopeKind::Loop);
                self.check_node(body);
                self.symbols.pop_scope();
            }

            AstKind::BreakStmt | AstKind::ContinueStmt => {
                if !self.symbols.in_loop() {
                    let kw = if matches!(&node.kind, AstKind::BreakStmt) {
                        "break"
                    } else {
                        "continue"
                    };
                    self.error(&node.span, &format!("'{}' outside of loop", kw));
                }
            }

            AstKind::Assignment { target, value, .. } => {
                // Check target is mutable
                if let AstKind::Identifier(name) = &target.kind {
                    let is_immutable = self.symbols.resolve(name)
                        .map(|sym| !sym.mutable)
                        .unwrap_or(false);
                    if is_immutable {
                        self.error(
                            &target.span,
                            &format!("Cannot assign to immutable variable '{}'", name),
                        );
                    }
                }

                let target_ty = self.infer_expr(target);
                let value_ty = self.infer_expr(value);
                if !self.types_compatible(&target_ty, &value_ty) {
                    self.error(
                        &node.span,
                        &format!(
                            "Assignment type mismatch: '{}' = '{}'",
                            target_ty, value_ty
                        ),
                    );
                }
            }

            AstKind::ExprStmt(expr) => {
                // Check assignments for mutability even when wrapped in ExprStmt
                if let AstKind::Assignment { target, .. } = &expr.kind {
                    if let AstKind::Identifier(name) = &target.kind {
                        let is_immutable = self.symbols.resolve(name)
                            .map(|sym| !sym.mutable)
                            .unwrap_or(false);
                        if is_immutable {
                            self.error(
                                &target.span,
                                &format!("Cannot assign to immutable variable '{}'", name),
                            );
                        }
                    }
                }
                self.infer_expr(expr);
            }

            AstKind::MatchStmt { expr, arms } => {
                let _match_ty = self.infer_expr(expr);
                for arm in arms {
                    self.symbols.push_scope(ScopeKind::Block);
                    self.check_node(&arm.body);
                    self.symbols.pop_scope();
                }
            }

            AstKind::TryBlock {
                body,
                catch_arms,
                finally_block,
            } => {
                self.symbols.push_scope(ScopeKind::Block);
                self.check_node(body);
                self.symbols.pop_scope();

                for catch in catch_arms {
                    self.symbols.push_scope(ScopeKind::Block);
                    let err_ty = catch.error_type.clone().unwrap_or(Type::String);
                    self.symbols.define(Symbol {
                        name: catch.binding.clone(),
                        ty: err_ty,
                        mutable: false,
                        kind: SymbolKind::Variable,
                    });
                    self.check_node(&catch.body);
                    self.symbols.pop_scope();
                }

                if let Some(finally_blk) = finally_block {
                    self.symbols.push_scope(ScopeKind::Block);
                    self.check_node(finally_blk);
                    self.symbols.pop_scope();
                }
            }

            AstKind::ReactorBlock { body, .. } => {
                self.symbols.push_scope(ScopeKind::Reactor);
                self.check_node(body);
                self.symbols.pop_scope();
            }

            AstKind::SpawnBlock(body) => {
                self.symbols.push_scope(ScopeKind::Block);
                self.check_node(body);
                self.symbols.pop_scope();
            }

            // C blocks, imports, etc. — no type checking needed
            AstKind::CBlock(_)
            | AstKind::CppBlock(_)
            | AstKind::ImportDecl { .. }
            | AstKind::ModuleDecl { .. }
            | AstKind::ThrowStmt(_)
            | AstKind::TraitDecl { .. } => {}

            _ => {}
        }
    }

    // ── Declaration registration (first pass) ────────────────────────────

    fn register_declaration(&mut self, node: &AstNode) {
        match &node.kind {
            AstKind::FnDecl {
                name,
                params,
                return_type,
                ..
            } => {
                let param_types: Vec<Type> = params.iter().map(|p| p.ty.clone()).collect();
                let ret = return_type.clone().unwrap_or(Type::Void);
                self.symbols.define(Symbol {
                    name: name.clone(),
                    ty: ret.clone(),
                    mutable: false,
                    kind: SymbolKind::Function {
                        params: param_types,
                        return_type: ret,
                    },
                });
            }
            AstKind::StructDecl { name, fields, .. } => {
                let field_types: Vec<(String, Type)> = fields
                    .iter()
                    .map(|f| (f.name.clone(), f.ty.clone()))
                    .collect();
                self.symbols.define(Symbol {
                    name: name.clone(),
                    ty: Type::Named(name.clone()),
                    mutable: false,
                    kind: SymbolKind::Struct {
                        fields: field_types,
                    },
                });
            }
            AstKind::EnumDecl { name, variants, .. } => {
                let variant_types: Vec<(String, Vec<Type>)> = variants
                    .iter()
                    .map(|v| (v.name.clone(), v.fields.clone()))
                    .collect();
                self.symbols.define(Symbol {
                    name: name.clone(),
                    ty: Type::Named(name.clone()),
                    mutable: false,
                    kind: SymbolKind::Enum {
                        variants: variant_types,
                    },
                });
            }
            _ => {}
        }
    }

    // ── Expression type inference ────────────────────────────────────────

    fn infer_expr(&mut self, node: &AstNode) -> Type {
        let ty = match &node.kind {
            AstKind::NumberLiteral(n) => {
                if n.contains('.') {
                    Type::F64
                } else {
                    Type::I32
                }
            }
            AstKind::StringLiteral(_) => Type::String,
            AstKind::CharLiteral(_) => Type::Char,
            AstKind::BoolLiteral(_) => Type::Bool,
            AstKind::NullLiteral => Type::Optional(Box::new(Type::Inferred)),
            AstKind::SelfLiteral => Type::Named("Self".to_string()),

            AstKind::Identifier(name) => {
                if let Some(sym) = self.symbols.resolve(name) {
                    sym.ty.clone()
                } else {
                    // Don't error for builtins handled specially
                    match name.as_str() {
                        "print" | "println" => Type::Void,
                        _ => {
                            self.error(&node.span, &format!("Undefined variable '{}'", name));
                            Type::Inferred
                        }
                    }
                }
            }

            AstKind::BinaryExpr { op, left, right } => {
                let left_ty = self.infer_expr(left);
                let right_ty = self.infer_expr(right);
                self.infer_binary_op(op, &left_ty, &right_ty, &node.span)
            }

            AstKind::UnaryExpr { op, expr } => {
                let inner = self.infer_expr(expr);
                match op {
                    UnaryOp::Neg => inner,
                    UnaryOp::Not => Type::Bool,
                    UnaryOp::BitNot => inner,
                    UnaryOp::AddrOf => Type::Ptr(Box::new(inner)),
                    UnaryOp::Deref => match inner {
                        Type::Ptr(inner) => *inner,
                        _ => {
                            self.error(&node.span, "Cannot dereference non-pointer");
                            Type::Inferred
                        }
                    },
                }
            }

            AstKind::Call { callee, args } => {
                let callee_ty = self.infer_expr(callee);

                // Check argument count for known functions
                if let AstKind::Identifier(name) = &callee.kind {
                    if let Some(sym) = self.symbols.resolve(name).cloned() {
                        if let SymbolKind::Function {
                            params,
                            return_type,
                        } = &sym.kind
                        {
                            if args.len() != params.len() {
                                self.error(
                                    &node.span,
                                    &format!(
                                        "'{}' expects {} args, got {}",
                                        name,
                                        params.len(),
                                        args.len()
                                    ),
                                );
                            }
                            // Infer all args (even if count doesn't match, for error collection)
                            for a in args {
                                self.infer_expr(a);
                            }
                            return return_type.clone();
                        }
                    }
                }

                // Infer args anyway
                for a in args {
                    self.infer_expr(a);
                }

                // Generic function call: return type is Inferred
                match callee_ty {
                    Type::Function { ret, .. } => *ret,
                    Type::Void => Type::Void,
                    _ => Type::Inferred,
                }
            }

            AstKind::MemberAccess { object, field } => {
                let obj_ty = self.infer_expr(object);
                if let Type::Named(struct_name) = &obj_ty {
                    if let Some(sym) = self.symbols.resolve(struct_name).cloned() {
                        if let SymbolKind::Struct { fields } = &sym.kind {
                            if let Some((_, fty)) = fields.iter().find(|(n, _)| n == field) {
                                return fty.clone();
                            }
                        }
                    }
                }
                Type::Inferred
            }

            AstKind::Index { object, index } => {
                let obj_ty = self.infer_expr(object);
                let _idx_ty = self.infer_expr(index);
                match obj_ty {
                    Type::Array(inner) => *inner,
                    Type::String | Type::Str => Type::Char,
                    _ => Type::Inferred,
                }
            }

            AstKind::ArrayLiteral(elements) => {
                if elements.is_empty() {
                    Type::Array(Box::new(Type::Inferred))
                } else {
                    let first = self.infer_expr(&elements[0]);
                    for e in &elements[1..] {
                        let ety = self.infer_expr(e);
                        if !self.types_compatible(&first, &ety) {
                            self.error(
                                &e.span,
                                &format!(
                                    "Array element type mismatch: expected '{}', got '{}'",
                                    first, ety
                                ),
                            );
                        }
                    }
                    Type::Array(Box::new(first))
                }
            }

            AstKind::StructLiteral { name, fields } => {
                // Verify struct exists and fields match
                if let Some(sym) = self.symbols.resolve(name).cloned() {
                    if let SymbolKind::Struct {
                        fields: struct_fields,
                    } = &sym.kind
                    {
                        for f in fields {
                            if let AstKind::FieldInit {
                                name: fname,
                                value,
                            } = &f.kind
                            {
                                let val_ty = self.infer_expr(value);
                                if let Some((_, expected)) =
                                    struct_fields.iter().find(|(n, _)| n == fname)
                                {
                                    if !self.types_compatible(expected, &val_ty) {
                                        self.error(
                                            &f.span,
                                            &format!(
                                                "Field '{}': expected '{}', got '{}'",
                                                fname, expected, val_ty
                                            ),
                                        );
                                    }
                                } else {
                                    self.error(
                                        &f.span,
                                        &format!(
                                            "Unknown field '{}' in struct '{}'",
                                            fname, name
                                        ),
                                    );
                                }
                            }
                        }
                    }
                }
                Type::Named(name.clone())
            }

            AstKind::PathExpr(path) => {
                if path.len() >= 2 {
                    // Enum variant: Type::Named(enum_name)
                    Type::Named(path[0].clone())
                } else {
                    Type::Inferred
                }
            }

            AstKind::TryExpr(expr) => {
                let inner = self.infer_expr(expr);
                // Try operator unwraps Optional/Result
                match inner {
                    Type::Optional(inner) => *inner,
                    _ => inner,
                }
            }

            AstKind::Assignment { target, value, .. } => {
                let _t = self.infer_expr(target);
                self.infer_expr(value)
            }

            _ => Type::Inferred,
        };
        self.node_types.insert(node.span, ty.clone());
        ty
    }

    // ── Binary operator type rules ───────────────────────────────────────

    fn infer_binary_op(
        &mut self,
        op: &BinOp,
        left: &Type,
        right: &Type,
        span: &crate::lexer::Span,
    ) -> Type {
        match op {
            // Arithmetic yields numeric type
            BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod => {
                if matches!(left, Type::Inferred) || matches!(right, Type::Inferred) {
                    // Can't check — one side is unresolved
                    Type::Inferred
                } else if self.is_numeric(left) && self.is_numeric(right) {
                    self.promote_numeric(left, right)
                } else if *op == BinOp::Add
                    && (matches!(left, Type::String) || matches!(right, Type::String))
                {
                    Type::String
                } else {
                    self.error(
                        span,
                        &format!("Cannot apply '{}' to '{}' and '{}'", op, left, right),
                    );
                    Type::Inferred
                }
            }

            // Comparison yields bool
            BinOp::Eq | BinOp::Neq | BinOp::Lt | BinOp::Gt | BinOp::LtEq | BinOp::GtEq => {
                Type::Bool
            }

            // Logical yields bool
            BinOp::And | BinOp::Or => Type::Bool,

            // Bitwise yields integer type
            BinOp::BitAnd | BinOp::BitOr | BinOp::BitXor => {
                self.promote_numeric(left, right)
            }

            // Range yields iterator/array
            BinOp::Range => Type::Array(Box::new(self.promote_numeric(left, right))),
        }
    }

    // ── Type helpers ─────────────────────────────────────────────────────

    fn types_compatible(&self, expected: &Type, actual: &Type) -> bool {
        if expected == actual {
            return true;
        }

        // Inferred types are always compatible (type inference placeholder)
        if matches!(expected, Type::Inferred) || matches!(actual, Type::Inferred) {
            return true;
        }

        // Numeric promotion: i32 and i64, f32 and f64, etc.
        if self.is_numeric(expected) && self.is_numeric(actual) {
            return true;
        }

        // null is compatible with optionals and pointers
        if matches!(actual, Type::Optional(_)) && matches!(expected, Type::Optional(_)) {
            return true;
        }

        // Dyn is compatible with anything
        if matches!(expected, Type::Dyn) || matches!(actual, Type::Dyn) {
            return true;
        }

        // String ↔ Str compatible
        if matches!(
            (expected, actual),
            (Type::String, Type::Str) | (Type::Str, Type::String)
        ) {
            return true;
        }

        // Named types are compatible with themselves
        if let (Type::Named(a), Type::Named(b)) = (expected, actual) {
            return a == b;
        }

        false
    }

    fn is_numeric(&self, ty: &Type) -> bool {
        matches!(
            ty,
            Type::I8
                | Type::I16
                | Type::I32
                | Type::I64
                | Type::U8
                | Type::U16
                | Type::U32
                | Type::U64
                | Type::F32
                | Type::F64
                | Type::Usize
                | Type::Strnum
        )
    }

    fn promote_numeric(&self, a: &Type, b: &Type) -> Type {
        // Float wins over integer
        if matches!(a, Type::F64) || matches!(b, Type::F64) {
            return Type::F64;
        }
        if matches!(a, Type::F32) || matches!(b, Type::F32) {
            return Type::F32;
        }
        // Wider integer wins
        if matches!(a, Type::I64) || matches!(b, Type::I64) {
            return Type::I64;
        }
        // Default to i32
        Type::I32
    }
}

impl Default for TypeChecker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    fn check_source(src: &str) -> Vec<TypeError> {
        let tokens = Lexer::tokenize(src).unwrap();
        let ast = Parser::parse(tokens).unwrap();
        let (_, errors, _) = TypeChecker::check(&ast);
        errors
    }

    #[test]
    fn test_valid_program() {
        let errors = check_source("fn main() -> i32 { let x: i32 = 42; return 0; }");
        assert!(errors.is_empty(), "Expected no errors, got: {:?}", errors);
    }

    #[test]
    fn test_undefined_variable() {
        let errors = check_source("fn main() { let x: i32 = y; }");
        assert!(!errors.is_empty());
        assert!(errors[0].message.contains("Undefined variable 'y'"));
    }

    #[test]
    fn test_break_outside_loop() {
        let errors = check_source("fn main() { break; }");
        assert!(!errors.is_empty());
        assert!(errors[0].message.contains("outside of loop"));
    }

    #[test]
    fn test_immutable_assignment() {
        let errors = check_source("fn main() { let x: i32 = 1; x = 2; }");
        assert!(!errors.is_empty());
        assert!(errors[0].message.contains("immutable"));
    }
}
