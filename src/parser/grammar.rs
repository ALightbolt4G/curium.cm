use crate::lexer::{Span, Token, TokenKind};
use crate::parser::ast::*;

/// Recursive descent parser for Curium source code.
pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    /// Parse the entire token stream into a Program AST node.
    pub fn parse(tokens: Vec<Token>) -> Result<AstNode, String> {
        let mut parser = Parser::new(tokens);
        parser.parse_program()
    }

    // ── Program ──────────────────────────────────────────────────────────

    fn parse_program(&mut self) -> Result<AstNode, String> {
        let start = self.current_span();
        let mut declarations = Vec::new();

        while !self.is_at_end() {
            // Skip comment tokens
            if self.check_comment() {
                self.advance();
                continue;
            }
            declarations.push(self.parse_top_level()?);
        }

        let end = self.current_span();
        Ok(AstNode::new(AstKind::Program(declarations), start.merge(&end)))
    }

    // ── Top-level declarations ───────────────────────────────────────────

    fn parse_top_level(&mut self) -> Result<AstNode, String> {
        let mut attributes = Vec::new();
        let mut is_pub = false;

        // Collect attributes
        while self.check_kind_fn(|k| matches!(k, TokenKind::HashAttr(_))) {
            if let TokenKind::HashAttr(attr) = &self.peek().kind {
                attributes.push(attr.clone());
            }
            self.advance();
        }

        // Check for `pub`
        if self.check(&TokenKind::KwPub) {
            is_pub = true;
            self.advance();
        }

        match &self.peek().kind {
            TokenKind::KwFn => self.parse_fn_decl(is_pub, attributes),
            TokenKind::KwStruct => self.parse_struct_decl(is_pub),
            TokenKind::KwEnum => self.parse_enum_decl(is_pub),
            TokenKind::KwTrait => self.parse_trait_decl(is_pub),
            TokenKind::KwImpl => self.parse_impl_block(),
            TokenKind::KwImport => self.parse_import_decl(),
            TokenKind::KwModule => self.parse_module_decl(),
            TokenKind::KwLet | TokenKind::KwMut => self.parse_let_decl(),
            _ => Err(self.error(&format!(
                "Expected declaration, found '{}'",
                self.peek().kind
            ))),
        }
    }

    // ── Function declaration ─────────────────────────────────────────────

    fn parse_fn_decl(
        &mut self,
        is_pub: bool,
        attributes: Vec<String>,
    ) -> Result<AstNode, String> {
        let start = self.current_span();
        self.expect(&TokenKind::KwFn)?;

        let name = self.expect_identifier()?;
        self.expect(&TokenKind::LParen)?;
        let params = self.parse_param_list()?;
        self.expect(&TokenKind::RParen)?;

        let return_type = if self.check(&TokenKind::Arrow) {
            self.advance();
            Some(self.parse_type()?)
        } else {
            None
        };

        let body = self.parse_block()?;
        let end = body.span.clone();

        Ok(AstNode::new(
            AstKind::FnDecl {
                name,
                params,
                return_type,
                body: Box::new(body),
                is_pub,
                attributes,
            },
            start.merge(&end),
        ))
    }

    fn parse_param_list(&mut self) -> Result<Vec<Param>, String> {
        let mut params = Vec::new();

        if self.check(&TokenKind::RParen) {
            return Ok(params);
        }

        loop {
            let mutable = if self.check(&TokenKind::KwMut) {
                self.advance();
                true
            } else {
                false
            };

            // Handle `self` parameter
            if self.check(&TokenKind::KwSelf_) {
                self.advance();
                params.push(Param {
                    name: "self".to_string(),
                    ty: Type::Inferred,
                    mutable,
                });
            } else {
                let name = self.expect_identifier()?;
                self.expect(&TokenKind::Colon)?;
                let ty = self.parse_type()?;
                params.push(Param { name, ty, mutable });
            }

            if !self.check(&TokenKind::Comma) {
                break;
            }
            self.advance(); // consume comma
        }

        Ok(params)
    }

    // ── Struct declaration ───────────────────────────────────────────────

    fn parse_struct_decl(&mut self, is_pub: bool) -> Result<AstNode, String> {
        let start = self.current_span();
        self.expect(&TokenKind::KwStruct)?;
        let name = self.expect_identifier()?;
        self.expect(&TokenKind::LBrace)?;

        let mut fields = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            let field_pub = if self.check(&TokenKind::KwPub) {
                self.advance();
                true
            } else {
                false
            };

            let field_name = self.expect_identifier()?;
            self.expect(&TokenKind::Colon)?;
            let ty = self.parse_type()?;

            fields.push(Field {
                name: field_name,
                ty,
                is_pub: field_pub,
            });

            if self.check(&TokenKind::Comma) {
                self.advance();
            } else if !self.check(&TokenKind::RBrace) {
                // Allow trailing comma or no comma before }
                break;
            }
        }

        let end = self.current_span();
        self.expect(&TokenKind::RBrace)?;

        Ok(AstNode::new(
            AstKind::StructDecl {
                name,
                fields,
                is_pub,
            },
            start.merge(&end),
        ))
    }

    // ── Enum declaration ─────────────────────────────────────────────────

    fn parse_enum_decl(&mut self, is_pub: bool) -> Result<AstNode, String> {
        let start = self.current_span();
        self.expect(&TokenKind::KwEnum)?;
        let name = self.expect_identifier()?;
        self.expect(&TokenKind::LBrace)?;

        let mut variants = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            let variant_name = self.expect_identifier()?;
            let mut fields = Vec::new();

            if self.check(&TokenKind::LParen) {
                self.advance();
                while !self.check(&TokenKind::RParen) && !self.is_at_end() {
                    fields.push(self.parse_type()?);
                    if !self.check(&TokenKind::Comma) {
                        break;
                    }
                    self.advance();
                }
                self.expect(&TokenKind::RParen)?;
            }

            variants.push(EnumVariant {
                name: variant_name,
                fields,
            });

            if self.check(&TokenKind::Comma) {
                self.advance();
            }
        }

        let end = self.current_span();
        self.expect(&TokenKind::RBrace)?;

        Ok(AstNode::new(
            AstKind::EnumDecl {
                name,
                variants,
                is_pub,
            },
            start.merge(&end),
        ))
    }

    // ── Trait declaration ────────────────────────────────────────────────

    fn parse_trait_decl(&mut self, is_pub: bool) -> Result<AstNode, String> {
        let start = self.current_span();
        self.expect(&TokenKind::KwTrait)?;
        let name = self.expect_identifier()?;
        self.expect(&TokenKind::LBrace)?;

        let mut methods = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            // Skip comments
            if self.check_comment() {
                self.advance();
                continue;
            }
            methods.push(self.parse_fn_decl(false, vec![])?);
        }

        let end = self.current_span();
        self.expect(&TokenKind::RBrace)?;

        Ok(AstNode::new(
            AstKind::TraitDecl {
                name,
                methods,
                is_pub,
            },
            start.merge(&end),
        ))
    }

    // ── Impl block ───────────────────────────────────────────────────────

    fn parse_impl_block(&mut self) -> Result<AstNode, String> {
        let start = self.current_span();
        self.expect(&TokenKind::KwImpl)?;

        let first_name = self.expect_identifier()?;

        let (trait_name, target) = if self.check(&TokenKind::KwFor) {
            self.advance();
            let target = self.expect_identifier()?;
            (Some(first_name), target)
        } else {
            (None, first_name)
        };

        self.expect(&TokenKind::LBrace)?;

        let mut methods = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            if self.check_comment() {
                self.advance();
                continue;
            }

            let mut is_pub = false;
            let mut attributes = Vec::new();

            while self.check_kind_fn(|k| matches!(k, TokenKind::HashAttr(_))) {
                if let TokenKind::HashAttr(attr) = &self.peek().kind {
                    attributes.push(attr.clone());
                }
                self.advance();
            }

            if self.check(&TokenKind::KwPub) {
                is_pub = true;
                self.advance();
            }

            methods.push(self.parse_fn_decl(is_pub, attributes)?);
        }

        let end = self.current_span();
        self.expect(&TokenKind::RBrace)?;

        Ok(AstNode::new(
            AstKind::ImplBlock {
                trait_name,
                target,
                methods,
            },
            start.merge(&end),
        ))
    }

    // ── Import / Module ──────────────────────────────────────────────────

    fn parse_import_decl(&mut self) -> Result<AstNode, String> {
        let start = self.current_span();
        self.expect(&TokenKind::KwImport)?;

        let path = self.expect_string()?;
        let alias = if self.check_kind_fn(|k| matches!(k, TokenKind::Identifier(s) if s == "as")) {
            self.advance();
            Some(self.expect_identifier()?)
        } else {
            None
        };

        self.expect(&TokenKind::Semi)?;
        let end = self.current_span();

        Ok(AstNode::new(
            AstKind::ImportDecl { path, alias },
            start.merge(&end),
        ))
    }

    fn parse_module_decl(&mut self) -> Result<AstNode, String> {
        let start = self.current_span();
        self.expect(&TokenKind::KwModule)?;
        let name = self.expect_identifier()?;
        self.expect(&TokenKind::Semi)?;
        let end = self.current_span();
        Ok(AstNode::new(AstKind::ModuleDecl { name }, start.merge(&end)))
    }

    // ── Statements ───────────────────────────────────────────────────────

    fn parse_statement(&mut self) -> Result<AstNode, String> {
        // Skip comments
        while self.check_comment() {
            self.advance();
        }

        match &self.peek().kind {
            TokenKind::KwLet | TokenKind::KwMut => self.parse_let_decl(),
            TokenKind::KwReturn => self.parse_return_stmt(),
            TokenKind::KwBreak => {
                let span = self.current_span();
                self.advance();
                self.expect(&TokenKind::Semi)?;
                Ok(AstNode::new(AstKind::BreakStmt, span))
            }
            TokenKind::KwContinue => {
                let span = self.current_span();
                self.advance();
                self.expect(&TokenKind::Semi)?;
                Ok(AstNode::new(AstKind::ContinueStmt, span))
            }
            TokenKind::KwIf => self.parse_if_stmt(),
            TokenKind::KwWhile => self.parse_while_stmt(),
            TokenKind::KwFor => self.parse_for_stmt(),
            TokenKind::KwLoop => self.parse_loop_stmt(),
            TokenKind::KwMatch => self.parse_match_stmt(),
            TokenKind::KwTry => self.parse_try_block(),
            TokenKind::KwThrow => self.parse_throw_stmt(),
            TokenKind::KwReactor => self.parse_reactor_block(),
            TokenKind::KwSpawn => self.parse_spawn_block(),
            TokenKind::LBrace => self.parse_block(),
            TokenKind::CBlock(_) => {
                let start = self.current_span();
                if let TokenKind::CBlock(code) = &self.peek().kind {
                    let code = code.clone();
                    self.advance();
                    Ok(AstNode::new(AstKind::CBlock(code), start))
                } else {
                    unreachable!()
                }
            }
            TokenKind::CppBlock(_) => {
                let start = self.current_span();
                if let TokenKind::CppBlock(code) = &self.peek().kind {
                    let code = code.clone();
                    self.advance();
                    Ok(AstNode::new(AstKind::CppBlock(code), start))
                } else {
                    unreachable!()
                }
            }
            _ => self.parse_expr_statement(),
        }
    }

    fn parse_let_decl(&mut self) -> Result<AstNode, String> {
        let start = self.current_span();

        let mutable = if self.check(&TokenKind::KwMut) {
            self.advance();
            true
        } else {
            self.expect(&TokenKind::KwLet)?;
            if self.check(&TokenKind::KwMut) {
                self.advance();
                true
            } else {
                false
            }
        };

        let name = self.expect_identifier()?;

        let type_annotation = if self.check(&TokenKind::Colon) {
            self.advance();
            Some(self.parse_type()?)
        } else {
            None
        };

        let init = if self.check(&TokenKind::Equal) {
            self.advance();
            Some(Box::new(self.parse_expression()?))
        } else {
            None
        };

        self.expect(&TokenKind::Semi)?;
        let end = self.current_span();

        Ok(AstNode::new(
            AstKind::LetDecl {
                name,
                type_annotation,
                init,
                mutable,
            },
            start.merge(&end),
        ))
    }

    fn parse_return_stmt(&mut self) -> Result<AstNode, String> {
        let start = self.current_span();
        self.expect(&TokenKind::KwReturn)?;

        let value = if !self.check(&TokenKind::Semi) {
            Some(Box::new(self.parse_expression()?))
        } else {
            None
        };

        self.expect(&TokenKind::Semi)?;
        let end = self.current_span();
        Ok(AstNode::new(AstKind::ReturnStmt(value), start.merge(&end)))
    }

    fn parse_if_stmt(&mut self) -> Result<AstNode, String> {
        let start = self.current_span();
        self.expect(&TokenKind::KwIf)?;

        let condition = self.parse_expression()?;
        let then_branch = self.parse_block()?;

        let else_branch = if self.check(&TokenKind::KwElse) {
            self.advance();
            if self.check(&TokenKind::KwIf) {
                Some(Box::new(self.parse_if_stmt()?))
            } else {
                Some(Box::new(self.parse_block()?))
            }
        } else {
            None
        };

        let end = else_branch
            .as_ref()
            .map(|e| e.span.clone())
            .unwrap_or_else(|| then_branch.span.clone());

        Ok(AstNode::new(
            AstKind::IfStmt {
                condition: Box::new(condition),
                then_branch: Box::new(then_branch),
                else_branch,
            },
            start.merge(&end),
        ))
    }

    fn parse_while_stmt(&mut self) -> Result<AstNode, String> {
        let start = self.current_span();
        self.expect(&TokenKind::KwWhile)?;
        let condition = self.parse_expression()?;
        let body = self.parse_block()?;
        let end = body.span.clone();

        Ok(AstNode::new(
            AstKind::WhileStmt {
                condition: Box::new(condition),
                body: Box::new(body),
            },
            start.merge(&end),
        ))
    }

    fn parse_for_stmt(&mut self) -> Result<AstNode, String> {
        let start = self.current_span();
        self.expect(&TokenKind::KwFor)?;
        let variable = self.expect_identifier()?;
        self.expect(&TokenKind::KwIn)?;
        let iterable = self.parse_expression()?;
        let body = self.parse_block()?;
        let end = body.span.clone();

        Ok(AstNode::new(
            AstKind::ForStmt {
                variable,
                iterable: Box::new(iterable),
                body: Box::new(body),
            },
            start.merge(&end),
        ))
    }

    fn parse_loop_stmt(&mut self) -> Result<AstNode, String> {
        let start = self.current_span();
        self.expect(&TokenKind::KwLoop)?;
        let body = self.parse_block()?;
        let end = body.span.clone();
        Ok(AstNode::new(
            AstKind::LoopStmt {
                body: Box::new(body),
            },
            start.merge(&end),
        ))
    }

    fn parse_match_stmt(&mut self) -> Result<AstNode, String> {
        let start = self.current_span();
        self.expect(&TokenKind::KwMatch)?;
        let expr = self.parse_expression()?;
        self.expect(&TokenKind::LBrace)?;

        let mut arms = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            let pattern = self.parse_pattern()?;
            self.expect(&TokenKind::FatArrow)?;

            let body = if self.check(&TokenKind::LBrace) {
                self.parse_block()?
            } else {
                let expr = self.parse_expression()?;
                if self.check(&TokenKind::Comma) {
                    self.advance();
                }
                expr
            };

            arms.push(MatchArm {
                pattern,
                body: Box::new(body),
            });

            // Optional comma between arms
            if self.check(&TokenKind::Comma) {
                self.advance();
            }
        }

        let end = self.current_span();
        self.expect(&TokenKind::RBrace)?;

        Ok(AstNode::new(
            AstKind::MatchStmt {
                expr: Box::new(expr),
                arms,
            },
            start.merge(&end),
        ))
    }

    fn parse_pattern(&mut self) -> Result<Pattern, String> {
        match &self.peek().kind {
            TokenKind::Identifier(_) => {
                let name = self.expect_identifier()?;

                if self.check(&TokenKind::DoubleColon) {
                    // Enum path: Foo::Bar(x, y)
                    let mut path = vec![name];
                    while self.check(&TokenKind::DoubleColon) {
                        self.advance();
                        path.push(self.expect_identifier()?);
                    }

                    let mut bindings = Vec::new();
                    if self.check(&TokenKind::LParen) {
                        self.advance();
                        while !self.check(&TokenKind::RParen) && !self.is_at_end() {
                            bindings.push(self.expect_identifier()?);
                            if !self.check(&TokenKind::Comma) {
                                break;
                            }
                            self.advance();
                        }
                        self.expect(&TokenKind::RParen)?;
                    }

                    Ok(Pattern::EnumVariant { path, bindings })
                } else {
                    Ok(Pattern::Identifier(name))
                }
            }
            TokenKind::NumberLiteral(_) | TokenKind::StringLiteral(_) | TokenKind::KwTrue
            | TokenKind::KwFalse | TokenKind::KwNull => {
                let expr = self.parse_primary()?;
                Ok(Pattern::Literal(Box::new(expr)))
            }
            TokenKind::Identifier(s) if s == "_" => {
                self.advance();
                Ok(Pattern::Wildcard)
            }
            _ => {
                // Check for `_` wildcard
                if let TokenKind::Identifier(s) = &self.peek().kind {
                    if s == "_" {
                        self.advance();
                        return Ok(Pattern::Wildcard);
                    }
                }
                Err(self.error(&format!("Expected pattern, found '{}'", self.peek().kind)))
            }
        }
    }

    fn parse_try_block(&mut self) -> Result<AstNode, String> {
        let start = self.current_span();
        self.expect(&TokenKind::KwTry)?;
        let body = self.parse_block()?;

        let mut catch_arms = Vec::new();
        while self.check(&TokenKind::KwCatch) {
            self.advance();
            self.expect(&TokenKind::LParen)?;

            let binding = self.expect_identifier()?;
            let error_type = if self.check(&TokenKind::Colon) {
                self.advance();
                Some(self.parse_type()?)
            } else {
                None
            };

            self.expect(&TokenKind::RParen)?;
            let catch_body = self.parse_block()?;

            catch_arms.push(CatchArm {
                error_type,
                binding,
                body: Box::new(catch_body),
            });
        }

        let finally_block = if self.check(&TokenKind::KwFinally) {
            self.advance();
            Some(Box::new(self.parse_block()?))
        } else {
            None
        };

        let end = finally_block
            .as_ref()
            .map(|f| f.span.clone())
            .or_else(|| catch_arms.last().map(|c| c.body.span.clone()))
            .unwrap_or_else(|| body.span.clone());

        Ok(AstNode::new(
            AstKind::TryBlock {
                body: Box::new(body),
                catch_arms,
                finally_block,
            },
            start.merge(&end),
        ))
    }

    fn parse_throw_stmt(&mut self) -> Result<AstNode, String> {
        let start = self.current_span();
        self.expect(&TokenKind::KwThrow)?;
        let expr = self.parse_expression()?;
        self.expect(&TokenKind::Semi)?;
        let end = self.current_span();
        Ok(AstNode::new(
            AstKind::ThrowStmt(Box::new(expr)),
            start.merge(&end),
        ))
    }

    fn parse_reactor_block(&mut self) -> Result<AstNode, String> {
        let start = self.current_span();
        self.expect(&TokenKind::KwReactor)?;

        let allocator = match &self.peek().kind {
            TokenKind::KwArena => {
                self.advance();
                AllocatorKind::Arena
            }
            TokenKind::KwManual => {
                self.advance();
                AllocatorKind::Manual
            }
            TokenKind::KwGc => {
                self.advance();
                AllocatorKind::Gc
            }
            _ => AllocatorKind::Arena,
        };

        let size = if self.check(&TokenKind::LParen) {
            self.advance();
            let s = self.parse_expression()?;
            self.expect(&TokenKind::RParen)?;
            Some(Box::new(s))
        } else {
            None
        };

        let body = self.parse_block()?;
        let end = body.span.clone();

        Ok(AstNode::new(
            AstKind::ReactorBlock {
                allocator,
                size,
                body: Box::new(body),
            },
            start.merge(&end),
        ))
    }

    fn parse_spawn_block(&mut self) -> Result<AstNode, String> {
        let start = self.current_span();
        self.expect(&TokenKind::KwSpawn)?;
        let body = self.parse_block()?;
        let end = body.span.clone();
        Ok(AstNode::new(
            AstKind::SpawnBlock(Box::new(body)),
            start.merge(&end),
        ))
    }

    fn parse_block(&mut self) -> Result<AstNode, String> {
        let start = self.current_span();
        self.expect(&TokenKind::LBrace)?;

        let mut stmts = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            if self.check_comment() {
                self.advance();
                continue;
            }
            stmts.push(self.parse_statement()?);
        }

        let end = self.current_span();
        self.expect(&TokenKind::RBrace)?;

        Ok(AstNode::new(AstKind::Block(stmts), start.merge(&end)))
    }

    fn parse_expr_statement(&mut self) -> Result<AstNode, String> {
        let start = self.current_span();
        let expr = self.parse_expression()?;
        self.expect(&TokenKind::Semi)?;
        let end = self.current_span();
        Ok(AstNode::new(
            AstKind::ExprStmt(Box::new(expr)),
            start.merge(&end),
        ))
    }

    // ── Expressions (Pratt parsing) ──────────────────────────────────────

    fn parse_expression(&mut self) -> Result<AstNode, String> {
        self.parse_assignment_expr()
    }

    fn parse_assignment_expr(&mut self) -> Result<AstNode, String> {
        let expr = self.parse_or_expr()?;

        let assign_op = match &self.peek().kind {
            TokenKind::Equal => Some(AssignOp::Assign),
            TokenKind::PlusEqual => Some(AssignOp::AddAssign),
            TokenKind::MinusEqual => Some(AssignOp::SubAssign),
            TokenKind::StarEqual => Some(AssignOp::MulAssign),
            TokenKind::SlashEqual => Some(AssignOp::DivAssign),
            TokenKind::PercentEqual => Some(AssignOp::ModAssign),
            TokenKind::ColonEqual => Some(AssignOp::Assign),
            _ => None,
        };

        if let Some(op) = assign_op {
            self.advance();
            let value = self.parse_expression()?; // right-associative
            let span = expr.span.merge(&value.span);
            Ok(AstNode::new(
                AstKind::Assignment {
                    op,
                    target: Box::new(expr),
                    value: Box::new(value),
                },
                span,
            ))
        } else {
            Ok(expr)
        }
    }

    fn parse_or_expr(&mut self) -> Result<AstNode, String> {
        let mut left = self.parse_and_expr()?;
        while self.check(&TokenKind::PipePipe) {
            self.advance();
            let right = self.parse_and_expr()?;
            let span = left.span.merge(&right.span);
            left = AstNode::new(
                AstKind::BinaryExpr {
                    op: BinOp::Or,
                    left: Box::new(left),
                    right: Box::new(right),
                },
                span,
            );
        }
        Ok(left)
    }

    fn parse_and_expr(&mut self) -> Result<AstNode, String> {
        let mut left = self.parse_bitor_expr()?;
        while self.check(&TokenKind::AndAnd) {
            self.advance();
            let right = self.parse_bitor_expr()?;
            let span = left.span.merge(&right.span);
            left = AstNode::new(
                AstKind::BinaryExpr {
                    op: BinOp::And,
                    left: Box::new(left),
                    right: Box::new(right),
                },
                span,
            );
        }
        Ok(left)
    }

    fn parse_bitor_expr(&mut self) -> Result<AstNode, String> {
        let mut left = self.parse_bitxor_expr()?;
        while self.check(&TokenKind::Pipe) {
            self.advance();
            let right = self.parse_bitxor_expr()?;
            let span = left.span.merge(&right.span);
            left = AstNode::new(
                AstKind::BinaryExpr {
                    op: BinOp::BitOr,
                    left: Box::new(left),
                    right: Box::new(right),
                },
                span,
            );
        }
        Ok(left)
    }

    fn parse_bitxor_expr(&mut self) -> Result<AstNode, String> {
        let mut left = self.parse_bitand_expr()?;
        while self.check(&TokenKind::Caret) {
            self.advance();
            let right = self.parse_bitand_expr()?;
            let span = left.span.merge(&right.span);
            left = AstNode::new(
                AstKind::BinaryExpr {
                    op: BinOp::BitXor,
                    left: Box::new(left),
                    right: Box::new(right),
                },
                span,
            );
        }
        Ok(left)
    }

    fn parse_bitand_expr(&mut self) -> Result<AstNode, String> {
        let mut left = self.parse_equality_expr()?;
        while self.check(&TokenKind::Ampersand) {
            self.advance();
            let right = self.parse_equality_expr()?;
            let span = left.span.merge(&right.span);
            left = AstNode::new(
                AstKind::BinaryExpr {
                    op: BinOp::BitAnd,
                    left: Box::new(left),
                    right: Box::new(right),
                },
                span,
            );
        }
        Ok(left)
    }

    fn parse_equality_expr(&mut self) -> Result<AstNode, String> {
        let mut left = self.parse_comparison_expr()?;
        loop {
            let op = match &self.peek().kind {
                TokenKind::EqualEqual => BinOp::Eq,
                TokenKind::BangEqual => BinOp::Neq,
                _ => break,
            };
            self.advance();
            let right = self.parse_comparison_expr()?;
            let span = left.span.merge(&right.span);
            left = AstNode::new(
                AstKind::BinaryExpr {
                    op,
                    left: Box::new(left),
                    right: Box::new(right),
                },
                span,
            );
        }
        Ok(left)
    }

    fn parse_comparison_expr(&mut self) -> Result<AstNode, String> {
        let mut left = self.parse_range_expr()?;
        loop {
            let op = match &self.peek().kind {
                TokenKind::Lt => BinOp::Lt,
                TokenKind::Gt => BinOp::Gt,
                TokenKind::LtEqual => BinOp::LtEq,
                TokenKind::GtEqual => BinOp::GtEq,
                _ => break,
            };
            self.advance();
            let right = self.parse_range_expr()?;
            let span = left.span.merge(&right.span);
            left = AstNode::new(
                AstKind::BinaryExpr {
                    op,
                    left: Box::new(left),
                    right: Box::new(right),
                },
                span,
            );
        }
        Ok(left)
    }

    fn parse_range_expr(&mut self) -> Result<AstNode, String> {
        let left = self.parse_additive_expr()?;
        if self.check(&TokenKind::DotDot) {
            self.advance();
            let right = self.parse_additive_expr()?;
            let span = left.span.merge(&right.span);
            Ok(AstNode::new(
                AstKind::BinaryExpr {
                    op: BinOp::Range,
                    left: Box::new(left),
                    right: Box::new(right),
                },
                span,
            ))
        } else {
            Ok(left)
        }
    }

    fn parse_additive_expr(&mut self) -> Result<AstNode, String> {
        let mut left = self.parse_multiplicative_expr()?;
        loop {
            let op = match &self.peek().kind {
                TokenKind::Plus => BinOp::Add,
                TokenKind::Minus => BinOp::Sub,
                _ => break,
            };
            self.advance();
            let right = self.parse_multiplicative_expr()?;
            let span = left.span.merge(&right.span);
            left = AstNode::new(
                AstKind::BinaryExpr {
                    op,
                    left: Box::new(left),
                    right: Box::new(right),
                },
                span,
            );
        }
        Ok(left)
    }

    fn parse_multiplicative_expr(&mut self) -> Result<AstNode, String> {
        let mut left = self.parse_unary_expr()?;
        loop {
            let op = match &self.peek().kind {
                TokenKind::Star => BinOp::Mul,
                TokenKind::Slash => BinOp::Div,
                TokenKind::Percent => BinOp::Mod,
                _ => break,
            };
            self.advance();
            let right = self.parse_unary_expr()?;
            let span = left.span.merge(&right.span);
            left = AstNode::new(
                AstKind::BinaryExpr {
                    op,
                    left: Box::new(left),
                    right: Box::new(right),
                },
                span,
            );
        }
        Ok(left)
    }

    fn parse_unary_expr(&mut self) -> Result<AstNode, String> {
        let op = match &self.peek().kind {
            TokenKind::Minus => Some(UnaryOp::Neg),
            TokenKind::Bang => Some(UnaryOp::Not),
            TokenKind::Tilde => Some(UnaryOp::BitNot),
            TokenKind::Ampersand => Some(UnaryOp::AddrOf),
            TokenKind::Caret => Some(UnaryOp::Deref),
            _ => None,
        };

        if let Some(op) = op {
            let start = self.current_span();
            self.advance();
            let expr = self.parse_unary_expr()?;
            let span = start.merge(&expr.span);
            Ok(AstNode::new(
                AstKind::UnaryExpr {
                    op,
                    expr: Box::new(expr),
                },
                span,
            ))
        } else {
            self.parse_postfix_expr()
        }
    }

    fn parse_postfix_expr(&mut self) -> Result<AstNode, String> {
        let mut expr = self.parse_primary()?;

        loop {
            match &self.peek().kind {
                TokenKind::LParen => {
                    // Function call
                    self.advance();
                    let mut args = Vec::new();
                    if !self.check(&TokenKind::RParen) {
                        loop {
                            args.push(self.parse_expression()?);
                            if !self.check(&TokenKind::Comma) {
                                break;
                            }
                            self.advance();
                        }
                    }
                    let end = self.current_span();
                    self.expect(&TokenKind::RParen)?;
                    let span = expr.span.merge(&end);
                    expr = AstNode::new(
                        AstKind::Call {
                            callee: Box::new(expr),
                            args,
                        },
                        span,
                    );
                }
                TokenKind::Dot => {
                    // Member access
                    self.advance();
                    let field = self.expect_identifier()?;
                    let span = expr.span.merge(&self.current_span());
                    expr = AstNode::new(
                        AstKind::MemberAccess {
                            object: Box::new(expr),
                            field,
                        },
                        span,
                    );
                }
                TokenKind::LBracket => {
                    // Index
                    self.advance();
                    let index = self.parse_expression()?;
                    let end = self.current_span();
                    self.expect(&TokenKind::RBracket)?;
                    let span = expr.span.merge(&end);
                    expr = AstNode::new(
                        AstKind::Index {
                            object: Box::new(expr),
                            index: Box::new(index),
                        },
                        span,
                    );
                }
                TokenKind::DoubleColon => {
                    // Path expression: Foo::Bar
                    let mut path = match &expr.kind {
                        AstKind::Identifier(name) => vec![name.clone()],
                        AstKind::PathExpr(p) => p.clone(),
                        _ => break,
                    };
                    self.advance();
                    let segment = self.expect_identifier()?;
                    path.push(segment);
                    let span = expr.span.merge(&self.current_span());
                    expr = AstNode::new(AstKind::PathExpr(path), span);
                }
                TokenKind::Question => {
                    // Try operator: expr?
                    let span = expr.span.merge(&self.current_span());
                    self.advance();
                    expr = AstNode::new(AstKind::TryExpr(Box::new(expr)), span);
                }
                _ => break,
            }
        }

        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<AstNode, String> {
        let start = self.current_span();

        match self.peek().kind.clone() {
            TokenKind::NumberLiteral(n) => {
                self.advance();
                Ok(AstNode::new(AstKind::NumberLiteral(n), start))
            }
            TokenKind::StringLiteral(s) => {
                self.advance();
                Ok(AstNode::new(AstKind::StringLiteral(s), start))
            }
            TokenKind::CharLiteral(c) => {
                self.advance();
                Ok(AstNode::new(AstKind::CharLiteral(c), start))
            }
            TokenKind::KwTrue => {
                self.advance();
                Ok(AstNode::new(AstKind::BoolLiteral(true), start))
            }
            TokenKind::KwFalse => {
                self.advance();
                Ok(AstNode::new(AstKind::BoolLiteral(false), start))
            }
            TokenKind::KwNull => {
                self.advance();
                Ok(AstNode::new(AstKind::NullLiteral, start))
            }
            TokenKind::KwSelf_ => {
                self.advance();
                Ok(AstNode::new(AstKind::SelfLiteral, start))
            }
            TokenKind::Identifier(name) => {
                self.advance();

                // Check for struct literal: Foo { field: value }
                if self.check(&TokenKind::LBrace) && self.is_struct_literal_context(&name) {
                    self.advance(); // consume '{'
                    let mut fields = Vec::new();
                    while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
                        let field_name = self.expect_identifier()?;
                        self.expect(&TokenKind::Colon)?;
                        let value = self.parse_expression()?;
                        let fspan = start.merge(&value.span);
                        fields.push(AstNode::new(
                            AstKind::FieldInit {
                                name: field_name,
                                value: Box::new(value),
                            },
                            fspan,
                        ));
                        if self.check(&TokenKind::Comma) {
                            self.advance();
                        }
                    }
                    let end = self.current_span();
                    self.expect(&TokenKind::RBrace)?;
                    Ok(AstNode::new(
                        AstKind::StructLiteral {
                            name: name.clone(),
                            fields,
                        },
                        start.merge(&end),
                    ))
                } else {
                    Ok(AstNode::new(AstKind::Identifier(name), start))
                }
            }
            // Builtin calls: print, println
            TokenKind::KwPrint | TokenKind::KwPrintln => {
                let kw_name = if self.peek().kind == TokenKind::KwPrint {
                    "print"
                } else {
                    "println"
                };
                self.advance();

                // Treat as function call
                Ok(AstNode::new(
                    AstKind::Identifier(kw_name.to_string()),
                    start,
                ))
            }
            TokenKind::LParen => {
                // Grouped expression
                self.advance();
                let expr = self.parse_expression()?;
                self.expect(&TokenKind::RParen)?;
                Ok(expr)
            }
            TokenKind::LBracket => {
                // Array literal: [1, 2, 3]
                self.advance();
                let mut elements = Vec::new();
                if !self.check(&TokenKind::RBracket) {
                    loop {
                        elements.push(self.parse_expression()?);
                        if !self.check(&TokenKind::Comma) {
                            break;
                        }
                        self.advance();
                    }
                }
                let end = self.current_span();
                self.expect(&TokenKind::RBracket)?;
                Ok(AstNode::new(AstKind::ArrayLiteral(elements), start.merge(&end)))
            }
            _ => Err(self.error(&format!(
                "Expected expression, found '{}'",
                self.peek().kind
            ))),
        }
    }

    // ── Type parsing ─────────────────────────────────────────────────────

    fn parse_type(&mut self) -> Result<Type, String> {
        // Check for pointer: ^Type
        if self.check(&TokenKind::Caret) {
            self.advance();
            let inner = self.parse_type()?;
            return Ok(Type::Ptr(Box::new(inner)));
        }

        // Check for array: []Type
        if self.check(&TokenKind::LBracket) {
            self.advance();
            self.expect(&TokenKind::RBracket)?;
            let inner = self.parse_type()?;
            return Ok(Type::Array(Box::new(inner)));
        }

        // Check for optional: ?Type
        if self.check(&TokenKind::Question) {
            self.advance();
            let inner = self.parse_type()?;
            return Ok(Type::Optional(Box::new(inner)));
        }

        // Check for function type: fn(A, B) -> C
        if self.check(&TokenKind::KwFn) {
            self.advance();
            self.expect(&TokenKind::LParen)?;
            let mut params = Vec::new();
            if !self.check(&TokenKind::RParen) {
                loop {
                    params.push(self.parse_type()?);
                    if !self.check(&TokenKind::Comma) {
                        break;
                    }
                    self.advance();
                }
            }
            self.expect(&TokenKind::RParen)?;
            let ret = if self.check(&TokenKind::Arrow) {
                self.advance();
                self.parse_type()?
            } else {
                Type::Void
            };
            return Ok(Type::Function {
                params,
                ret: Box::new(ret),
            });
        }

        let ty = match &self.peek().kind {
            TokenKind::KwI8 => { self.advance(); Type::I8 }
            TokenKind::KwI16 => { self.advance(); Type::I16 }
            TokenKind::KwI32 => { self.advance(); Type::I32 }
            TokenKind::KwI64 => { self.advance(); Type::I64 }
            TokenKind::KwU8 => { self.advance(); Type::U8 }
            TokenKind::KwU16 => { self.advance(); Type::U16 }
            TokenKind::KwU32 => { self.advance(); Type::U32 }
            TokenKind::KwU64 => { self.advance(); Type::U64 }
            TokenKind::KwF32 => { self.advance(); Type::F32 }
            TokenKind::KwF64 => { self.advance(); Type::F64 }
            TokenKind::KwUsize => { self.advance(); Type::Usize }
            TokenKind::KwBool => { self.advance(); Type::Bool }
            TokenKind::KwChar => { self.advance(); Type::Char }
            TokenKind::KwString => { self.advance(); Type::String }
            TokenKind::KwStr => { self.advance(); Type::Str }
            TokenKind::KwVoid => { self.advance(); Type::Void }
            TokenKind::KwDyn => { self.advance(); Type::Dyn }
            TokenKind::KwStrnum => { self.advance(); Type::Strnum }
            TokenKind::Identifier(_) => {
                let name = self.expect_identifier()?;
                // Check for generic parameters: Name<T, U>
                if self.check(&TokenKind::Lt) {
                    self.advance();
                    let mut args = Vec::new();
                    loop {
                        args.push(self.parse_type()?);
                        if !self.check(&TokenKind::Comma) {
                            break;
                        }
                        self.advance();
                    }
                    self.expect(&TokenKind::Gt)?;
                    Type::Generic(name, args)
                } else {
                    Type::Named(name)
                }
            }
            _ => {
                return Err(self.error(&format!(
                    "Expected type, found '{}'",
                    self.peek().kind
                )));
            }
        };

        Ok(ty)
    }

    // ── Token navigation helpers ─────────────────────────────────────────

    fn peek(&self) -> &Token {
        &self.tokens[self.pos.min(self.tokens.len() - 1)]
    }

    fn advance(&mut self) -> &Token {
        let token = &self.tokens[self.pos];
        if self.pos < self.tokens.len() - 1 {
            self.pos += 1;
        }
        token
    }

    fn is_at_end(&self) -> bool {
        self.peek().kind == TokenKind::Eof
    }

    fn check(&self, kind: &TokenKind) -> bool {
        std::mem::discriminant(&self.peek().kind) == std::mem::discriminant(kind)
    }

    fn check_kind_fn<F: Fn(&TokenKind) -> bool>(&self, f: F) -> bool {
        f(&self.peek().kind)
    }

    fn check_comment(&self) -> bool {
        matches!(&self.peek().kind, TokenKind::Comment(_))
    }

    fn expect(&mut self, kind: &TokenKind) -> Result<&Token, String> {
        if self.check(kind) {
            Ok(self.advance())
        } else {
            Err(self.error(&format!(
                "Expected '{}', found '{}'",
                kind, self.peek().kind
            )))
        }
    }

    fn expect_identifier(&mut self) -> Result<String, String> {
        if let TokenKind::Identifier(name) = &self.peek().kind {
            let name = name.clone();
            self.advance();
            Ok(name)
        } else {
            Err(self.error(&format!(
                "Expected identifier, found '{}'",
                self.peek().kind
            )))
        }
    }

    fn expect_string(&mut self) -> Result<String, String> {
        if let TokenKind::StringLiteral(s) = &self.peek().kind {
            let s = s.clone();
            self.advance();
            Ok(s)
        } else {
            Err(self.error(&format!(
                "Expected string literal, found '{}'",
                self.peek().kind
            )))
        }
    }

    fn current_span(&self) -> Span {
        self.peek().span.clone()
    }

    fn error(&self, msg: &str) -> String {
        let token = self.peek();
        format!("{}:{}: Parse error: {}", token.line, token.column, msg)
    }

    /// Simple heuristic: if the next token after `{` looks like `identifier:`,
    /// it's probably a struct literal rather than a block.
    fn is_struct_literal_context(&self, _name: &str) -> bool {
        // Look ahead: `{ identifier : ...` means struct literal
        if self.pos + 2 < self.tokens.len() {
            let t1 = &self.tokens[self.pos + 1]; // token after `{`
            let t2 = &self.tokens[self.pos + 2]; // token after identifier
            matches!(&t1.kind, TokenKind::Identifier(_))
                && t2.kind == TokenKind::Colon
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;

    #[test]
    fn test_parse_empty() {
        let tokens = Lexer::tokenize("").unwrap();
        let ast = Parser::parse(tokens).unwrap();
        match ast.kind {
            AstKind::Program(decls) => assert!(decls.is_empty()),
            _ => panic!("Expected Program node"),
        }
    }

    #[test]
    fn test_parse_fn_decl() {
        let tokens = Lexer::tokenize("fn main() -> i32 { return 0; }").unwrap();
        let ast = Parser::parse(tokens).unwrap();
        match &ast.kind {
            AstKind::Program(decls) => {
                assert_eq!(decls.len(), 1);
                match &decls[0].kind {
                    AstKind::FnDecl { name, params, return_type, .. } => {
                        assert_eq!(name, "main");
                        assert!(params.is_empty());
                        assert_eq!(*return_type, Some(Type::I32));
                    }
                    _ => panic!("Expected FnDecl"),
                }
            }
            _ => panic!("Expected Program"),
        }
    }

    #[test]
    fn test_parse_let_decl() {
        let tokens = Lexer::tokenize("fn test() { let x: i32 = 42; }").unwrap();
        let ast = Parser::parse(tokens).unwrap();
        match &ast.kind {
            AstKind::Program(decls) => {
                match &decls[0].kind {
                    AstKind::FnDecl { body, .. } => {
                        match &body.kind {
                            AstKind::Block(stmts) => {
                                match &stmts[0].kind {
                                    AstKind::LetDecl { name, type_annotation, mutable, .. } => {
                                        assert_eq!(name, "x");
                                        assert_eq!(*type_annotation, Some(Type::I32));
                                        assert!(!mutable);
                                    }
                                    _ => panic!("Expected LetDecl"),
                                }
                            }
                            _ => panic!("Expected Block"),
                        }
                    }
                    _ => panic!("Expected FnDecl"),
                }
            }
            _ => panic!("Expected Program"),
        }
    }

    #[test]
    fn test_parse_binary_expr() {
        let tokens = Lexer::tokenize("fn test() { let x = 1 + 2 * 3; }").unwrap();
        let ast = Parser::parse(tokens).unwrap();
        // Should parse as (1 + (2 * 3)) due to precedence
        match &ast.kind {
            AstKind::Program(decls) => {
                match &decls[0].kind {
                    AstKind::FnDecl { body, .. } => {
                        match &body.kind {
                            AstKind::Block(stmts) => {
                                match &stmts[0].kind {
                                    AstKind::LetDecl { init: Some(expr), .. } => {
                                        match &expr.kind {
                                            AstKind::BinaryExpr { op, .. } => {
                                                assert_eq!(*op, BinOp::Add);
                                            }
                                            _ => panic!("Expected BinaryExpr"),
                                        }
                                    }
                                    _ => panic!("Expected LetDecl"),
                                }
                            }
                            _ => panic!("Expected Block"),
                        }
                    }
                    _ => panic!("Expected FnDecl"),
                }
            }
            _ => panic!("Expected Program"),
        }
    }
}
