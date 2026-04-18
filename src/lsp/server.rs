use std::io::{self, BufReader};
use std::collections::HashMap;
use crate::lexer::{Lexer, Span};
use crate::parser::{Parser, AstNode, AstKind, Type as CmType};
use crate::type_checker::checker::{TypeChecker, TypeError};
use super::jsonrpc::{self, Message, JsonValue};

pub struct LspServer {
    current_source: String,
    current_ast: Option<AstNode>,
    current_types: HashMap<Span, CmType>,
    current_errors: Vec<TypeError>,
}

impl LspServer {
    pub fn new() -> Self {
        Self {
            current_source: String::new(),
            current_ast: None,
            current_types: HashMap::new(),
            current_errors: Vec::new(),
        }
    }

    pub fn run(&mut self) -> Result<(), String> {
        let stdin = io::stdin();
        let mut reader = BufReader::new(stdin.lock());
        let mut stdout = io::stdout().lock();

        loop {
            let content = jsonrpc::read_message(&mut reader)?;
            let msg = Message::parse(&content)?;

            if let Some(method) = &msg.method {
                match method.as_str() {
                    "initialize" => {
                        let result = JsonValue::Object(vec![
                            ("capabilities".to_string(), JsonValue::Object(vec![
                                ("hoverProvider".to_string(), JsonValue::Bool(true)),
                                ("textDocumentSync".to_string(), JsonValue::Number(1.0)), // Full
                            ].into_iter().collect())),
                        ].into_iter().collect());
                        let resp = Message {
                            jsonrpc: "2.0".to_string(),
                            id: msg.id.clone(),
                            method: None,
                            params: None,
                            result: Some(result),
                            error: None,
                        };
                        jsonrpc::write_message(&mut stdout, &resp.to_json()).map_err(|e| e.to_string())?;
                    }
                    "textDocument/didOpen" | "textDocument/didChange" => {
                        if let Some(params) = &msg.params {
                            if let Some(doc) = params.get("textDocument") {
                                if let Some(text) = doc.get("text") {
                                    if let Some(s) = text.as_str() {
                                        self.update_source(s);
                                    }
                                }
                            }
                        }
                    }
                    "textDocument/hover" => {
                        if let Some(params) = &msg.params {
                            let resp = self.handle_hover(msg.id.clone(), params);
                            jsonrpc::write_message(&mut stdout, &resp.to_json()).map_err(|e| e.to_string())?;
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    fn update_source(&mut self, source: &str) {
        self.current_source = source.to_string();
        let tokens = Lexer::tokenize(source).unwrap_or_default();
        if let Ok(ast) = Parser::parse(tokens) {
            let (_, errors, node_types) = TypeChecker::check(&ast);
            self.current_ast = Some(ast);
            self.current_types = node_types;
            self.current_errors = errors;
        }
    }

    fn handle_hover(&self, id: Option<JsonValue>, params: &JsonValue) -> Message {
        let mut result = JsonValue::Null;

        if let (Some(ast), Some(pos)) = (&self.current_ast, params.get("position")) {
            if let (Some(JsonValue::Number(line)), Some(JsonValue::Number(character))) = (pos.get("line"), pos.get("character")) {
                let offset = self.pos_to_offset(*line as usize, *character as usize);
                if let Some(node) = self.find_node_at(ast, offset) {
                    if let Some(ty) = self.current_types.get(&node.span) {
                        result = JsonValue::Object(vec![
                            ("contents".to_string(), JsonValue::String(format!("Type: `{}`", ty))),
                        ].into_iter().collect());
                    }
                }
            }
        }

        Message {
            jsonrpc: "2.0".to_string(),
            id,
            method: None,
            params: None,
            result: Some(result),
            error: None,
        }
    }

    fn pos_to_offset(&self, line: usize, character: usize) -> usize {
        let mut offset = 0;
        let mut curr_line = 0;
        for c in self.current_source.chars() {
            if curr_line == line {
                return offset + character;
            }
            if c == '\n' {
                curr_line += 1;
            }
            offset += c.len_utf8();
        }
        offset
    }

    fn find_node_at<'a>(&self, node: &'a AstNode, offset: usize) -> Option<&'a AstNode> {
        if offset < node.span.start || offset > node.span.end {
            return None;
        }

        // Children first (most specific)
        match &node.kind {
            AstKind::Program(decls) => {
                for d in decls {
                    if let Some(found) = self.find_node_at(d, offset) {
                        return Some(found);
                    }
                }
            }
            AstKind::FnDecl { params, body, .. } => {
                for p in params {
                    // Params are just members of FnDecl in our AST, they don't have their own AstNode wrapping usually
                    // but we can check their Spans if they have them.
                    // For now, check body.
                }
                if let Some(found) = self.find_node_at(body, offset) {
                    return Some(found);
                }
            }
            AstKind::Block(stmts) => {
                for s in stmts {
                    if let Some(found) = self.find_node_at(s, offset) {
                        return Some(found);
                    }
                }
            }
            AstKind::LetDecl { init, .. } => {
                if let Some(i) = init {
                    if let Some(found) = self.find_node_at(i, offset) {
                        return Some(found);
                    }
                }
            }
            AstKind::IfStmt { condition, then_branch, else_branch } => {
                if let Some(found) = self.find_node_at(condition, offset) { return Some(found); }
                if let Some(found) = self.find_node_at(then_branch, offset) { return Some(found); }
                if let Some(eb) = else_branch {
                    if let Some(found) = self.find_node_at(eb, offset) { return Some(found); }
                }
            }
            AstKind::BinaryExpr { left, right, .. } => {
                if let Some(found) = self.find_node_at(left, offset) { return Some(found); }
                if let Some(found) = self.find_node_at(right, offset) { return Some(found); }
            }
            AstKind::Call { callee, args } => {
                if let Some(found) = self.find_node_at(callee, offset) { return Some(found); }
                for a in args {
                    if let Some(found) = self.find_node_at(a, offset) { return Some(found); }
                }
            }
            // Add more recursive cases as needed
            _ => {}
        }

        // If no child matched but this node contains offset, this is it.
        Some(node)
    }
}
