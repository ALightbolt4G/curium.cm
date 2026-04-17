use std::collections::HashMap;

use super::token::{Span, Token, TokenKind};

/// The Curium lexer. Converts source text into a stream of tokens.
pub struct Lexer {
    source: Vec<char>,
    pos: usize,
    line: usize,
    column: usize,
    keywords: HashMap<&'static str, TokenKind>,
}

impl Lexer {
    /// Create a new lexer for the given source string.
    pub fn new(source: &str) -> Self {
        Self {
            source: source.chars().collect(),
            pos: 0,
            line: 1,
            column: 1,
            keywords: Self::build_keyword_map(),
        }
    }

    /// Tokenize the entire source, returning all tokens (including EOF).
    pub fn tokenize(source: &str) -> Result<Vec<Token>, String> {
        let mut lexer = Lexer::new(source);
        let mut tokens = Vec::new();
        loop {
            let token = lexer.next_token()?;
            let is_eof = token.kind == TokenKind::Eof;
            tokens.push(token);
            if is_eof {
                break;
            }
        }
        Ok(tokens)
    }

    /// Read the next token from the source.
    pub fn next_token(&mut self) -> Result<Token, String> {
        self.skip_whitespace();

        if self.is_at_end() {
            return Ok(self.make_token(TokenKind::Eof, self.pos));
        }

        let start = self.pos;
        let ch = self.advance();

        match ch {
            // ── Single-character tokens ──
            '(' => Ok(self.make_token(TokenKind::LParen, start)),
            ')' => Ok(self.make_token(TokenKind::RParen, start)),
            '{' => Ok(self.make_token(TokenKind::LBrace, start)),
            '}' => Ok(self.make_token(TokenKind::RBrace, start)),
            '[' => Ok(self.make_token(TokenKind::LBracket, start)),
            ']' => Ok(self.make_token(TokenKind::RBracket, start)),
            ';' => Ok(self.make_token(TokenKind::Semi, start)),
            ',' => Ok(self.make_token(TokenKind::Comma, start)),
            '@' => Ok(self.make_token(TokenKind::At, start)),
            '$' => Ok(self.make_token(TokenKind::Dollar, start)),
            '~' => Ok(self.make_token(TokenKind::Tilde, start)),

            // ── Potentially multi-character tokens ──
            '#' => self.lex_hash(start),
            '.' => {
                if self.peek() == '.' {
                    self.advance();
                    Ok(self.make_token(TokenKind::DotDot, start))
                } else {
                    Ok(self.make_token(TokenKind::Dot, start))
                }
            }
            ':' => {
                if self.peek() == '=' {
                    self.advance();
                    Ok(self.make_token(TokenKind::ColonEqual, start))
                } else if self.peek() == ':' {
                    self.advance();
                    Ok(self.make_token(TokenKind::DoubleColon, start))
                } else {
                    Ok(self.make_token(TokenKind::Colon, start))
                }
            }
            '?' => {
                if self.peek() == '?' {
                    self.advance();
                    Ok(self.make_token(TokenKind::DoubleQuestion, start))
                } else {
                    Ok(self.make_token(TokenKind::Question, start))
                }
            }
            '!' => {
                if self.peek() == '=' {
                    self.advance();
                    Ok(self.make_token(TokenKind::BangEqual, start))
                } else {
                    Ok(self.make_token(TokenKind::Bang, start))
                }
            }
            '=' => {
                if self.peek() == '=' {
                    self.advance();
                    Ok(self.make_token(TokenKind::EqualEqual, start))
                } else if self.peek() == '>' {
                    self.advance();
                    Ok(self.make_token(TokenKind::FatArrow, start))
                } else {
                    Ok(self.make_token(TokenKind::Equal, start))
                }
            }
            '<' => {
                if self.peek() == '=' {
                    self.advance();
                    Ok(self.make_token(TokenKind::LtEqual, start))
                } else {
                    Ok(self.make_token(TokenKind::Lt, start))
                }
            }
            '>' => {
                if self.peek() == '=' {
                    self.advance();
                    Ok(self.make_token(TokenKind::GtEqual, start))
                } else {
                    Ok(self.make_token(TokenKind::Gt, start))
                }
            }
            '&' => {
                if self.peek() == '&' {
                    self.advance();
                    Ok(self.make_token(TokenKind::AndAnd, start))
                } else {
                    Ok(self.make_token(TokenKind::Ampersand, start))
                }
            }
            '|' => {
                if self.peek() == '|' {
                    self.advance();
                    Ok(self.make_token(TokenKind::PipePipe, start))
                } else {
                    Ok(self.make_token(TokenKind::Pipe, start))
                }
            }
            '^' => Ok(self.make_token(TokenKind::Caret, start)),
            '+' => {
                if self.peek() == '=' {
                    self.advance();
                    Ok(self.make_token(TokenKind::PlusEqual, start))
                } else {
                    Ok(self.make_token(TokenKind::Plus, start))
                }
            }
            '-' => {
                if self.peek() == '>' {
                    self.advance();
                    Ok(self.make_token(TokenKind::Arrow, start))
                } else if self.peek() == '=' {
                    self.advance();
                    Ok(self.make_token(TokenKind::MinusEqual, start))
                } else {
                    Ok(self.make_token(TokenKind::Minus, start))
                }
            }
            '*' => {
                if self.peek() == '=' {
                    self.advance();
                    Ok(self.make_token(TokenKind::StarEqual, start))
                } else {
                    Ok(self.make_token(TokenKind::Star, start))
                }
            }
            '/' => {
                if self.peek() == '/' {
                    self.lex_line_comment(start)
                } else if self.peek() == '*' {
                    self.lex_block_comment(start)
                } else if self.peek() == '=' {
                    self.advance();
                    Ok(self.make_token(TokenKind::SlashEqual, start))
                } else {
                    Ok(self.make_token(TokenKind::Slash, start))
                }
            }
            '%' => {
                if self.peek() == '=' {
                    self.advance();
                    Ok(self.make_token(TokenKind::PercentEqual, start))
                } else {
                    Ok(self.make_token(TokenKind::Percent, start))
                }
            }

            // ── String literals ──
            '"' => self.lex_string(start),

            // ── Char literals ──
            '\'' => self.lex_char(start),

            // ── Number literals ──
            c if c.is_ascii_digit() => self.lex_number(start),

            // ── Identifiers and keywords ──
            c if Self::is_ident_start(c) => self.lex_identifier(start),

            other => Err(format!(
                "{}:{}: Unexpected character '{}'",
                self.line, self.column - 1, other
            )),
        }
    }

    // ── Helpers ──────────────────────────────────────────────────────────

    fn build_keyword_map() -> HashMap<&'static str, TokenKind> {
        let mut m = HashMap::new();
        m.insert("fn", TokenKind::KwFn);
        m.insert("let", TokenKind::KwLet);
        m.insert("mut", TokenKind::KwMut);
        m.insert("return", TokenKind::KwReturn);
        m.insert("if", TokenKind::KwIf);
        m.insert("else", TokenKind::KwElse);
        m.insert("while", TokenKind::KwWhile);
        m.insert("for", TokenKind::KwFor);
        m.insert("loop", TokenKind::KwLoop);
        m.insert("break", TokenKind::KwBreak);
        m.insert("continue", TokenKind::KwContinue);
        m.insert("in", TokenKind::KwIn);
        m.insert("true", TokenKind::KwTrue);
        m.insert("false", TokenKind::KwFalse);
        m.insert("null", TokenKind::KwNull);
        m.insert("string", TokenKind::KwString);
        m.insert("void", TokenKind::KwVoid);
        m.insert("dyn", TokenKind::KwDyn);
        m.insert("i8", TokenKind::KwI8);
        m.insert("i16", TokenKind::KwI16);
        m.insert("i32", TokenKind::KwI32);
        m.insert("i64", TokenKind::KwI64);
        m.insert("u8", TokenKind::KwU8);
        m.insert("u16", TokenKind::KwU16);
        m.insert("u32", TokenKind::KwU32);
        m.insert("u64", TokenKind::KwU64);
        m.insert("f32", TokenKind::KwF32);
        m.insert("f64", TokenKind::KwF64);
        m.insert("usize", TokenKind::KwUsize);
        m.insert("bool", TokenKind::KwBool);
        m.insert("char", TokenKind::KwChar);
        m.insert("str", TokenKind::KwStr);
        m.insert("strnum", TokenKind::KwStrnum);
        m.insert("ptr", TokenKind::KwPtr);
        m.insert("struct", TokenKind::KwStruct);
        m.insert("enum", TokenKind::KwEnum);
        m.insert("union", TokenKind::KwUnion);
        m.insert("trait", TokenKind::KwTrait);
        m.insert("impl", TokenKind::KwImpl);
        m.insert("class", TokenKind::KwClass);
        m.insert("interface", TokenKind::KwInterface);
        m.insert("implements", TokenKind::KwImplements);
        m.insert("extends", TokenKind::KwExtends);
        m.insert("new", TokenKind::KwNew);
        m.insert("self", TokenKind::KwSelf_);
        m.insert("get", TokenKind::KwGet);
        m.insert("set", TokenKind::KwSet);
        m.insert("static", TokenKind::KwStatic);
        m.insert("pub", TokenKind::KwPub);
        m.insert("match", TokenKind::KwMatch);
        m.insert("import", TokenKind::KwImport);
        m.insert("module", TokenKind::KwModule);
        m.insert("package", TokenKind::KwPackage);
        m.insert("using", TokenKind::KwUsing);
        m.insert("namespace", TokenKind::KwNamespace);
        m.insert("from", TokenKind::KwFrom);
        m.insert("require", TokenKind::KwRequire);
        m.insert("try", TokenKind::KwTry);
        m.insert("catch", TokenKind::KwCatch);
        m.insert("throw", TokenKind::KwThrow);
        m.insert("finally", TokenKind::KwFinally);
        m.insert("async", TokenKind::KwAsync);
        m.insert("await", TokenKind::KwAwait);
        m.insert("task", TokenKind::KwTask);
        m.insert("spawn", TokenKind::KwSpawn);
        m.insert("call", TokenKind::KwCall);
        m.insert("reactor", TokenKind::KwReactor);
        m.insert("arena", TokenKind::KwArena);
        m.insert("manual", TokenKind::KwManual);
        m.insert("gc", TokenKind::KwGc);
        m.insert("gc_collect", TokenKind::KwGcCollect);
        m.insert("malloc", TokenKind::KwMalloc);
        m.insert("free", TokenKind::KwFree);
        m.insert("print", TokenKind::KwPrint);
        m.insert("println", TokenKind::KwPrintln);
        m
    }

    fn is_ident_start(c: char) -> bool {
        c.is_ascii_alphabetic() || c == '_'
    }

    fn is_ident_continue(c: char) -> bool {
        c.is_ascii_alphanumeric() || c == '_'
    }

    fn is_at_end(&self) -> bool {
        self.pos >= self.source.len()
    }

    fn peek(&self) -> char {
        if self.is_at_end() {
            '\0'
        } else {
            self.source[self.pos]
        }
    }

    fn peek_next(&self) -> char {
        if self.pos + 1 >= self.source.len() {
            '\0'
        } else {
            self.source[self.pos + 1]
        }
    }

    fn advance(&mut self) -> char {
        let ch = self.source[self.pos];
        self.pos += 1;
        if ch == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        ch
    }

    fn skip_whitespace(&mut self) {
        while !self.is_at_end() {
            match self.peek() {
                ' ' | '\t' | '\r' | '\n' => {
                    self.advance();
                }
                _ => break,
            }
        }
    }

    fn make_token(&self, kind: TokenKind, start: usize) -> Token {
        Token::new(
            kind,
            self.line,
            self.column,
            Span::new(start, self.pos),
        )
    }

    // ── Lexer sub-routines ──────────────────────────────────────────────

    fn lex_identifier(&mut self, start: usize) -> Result<Token, String> {
        while !self.is_at_end() && Self::is_ident_continue(self.peek()) {
            self.advance();
        }

        let text: String = self.source[start..self.pos].iter().collect();

        // Check for `c { ... }` and `cpp { ... }` polyglot blocks
        // Only trigger at statement positions — not when `c` is a variable
        // in expression context (e.g. `match c { ... }` should NOT be a CBlock).
        // Heuristic: polyglot blocks appear at the start of a statement, so the
        // preceding non-whitespace character should be a statement boundary
        // (`{`, `}`, `;`) or the identifier should be at column 1.
        if (text == "c" || text == "cpp") && self.peek_after_whitespace() == '{' {
            let is_stmt_start = if start == 0 {
                true  // Very beginning of file
            } else {
                // Walk backwards to find the preceding non-whitespace character
                let mut i = start;
                while i > 0 {
                    i -= 1;
                    let prev = self.source[i];
                    if !matches!(prev, ' ' | '\t' | '\r' | '\n') {
                        break;
                    }
                }
                if i == 0 && matches!(self.source[0], ' ' | '\t' | '\r' | '\n') {
                    true  // Only whitespace before
                } else {
                    matches!(self.source[i], '{' | '}' | ';')
                }
            };
            if is_stmt_start {
                return self.lex_polyglot_block(&text, start);
            }
        }

        // Check keyword table
        if let Some(kw) = self.keywords.get(text.as_str()) {
            Ok(self.make_token(kw.clone(), start))
        } else {
            Ok(self.make_token(TokenKind::Identifier(text), start))
        }
    }

    fn peek_after_whitespace(&self) -> char {
        let mut i = self.pos;
        while i < self.source.len() && matches!(self.source[i], ' ' | '\t' | '\r' | '\n') {
            i += 1;
        }
        if i < self.source.len() {
            self.source[i]
        } else {
            '\0'
        }
    }

    fn lex_polyglot_block(&mut self, lang: &str, start: usize) -> Result<Token, String> {
        // Skip whitespace to the opening brace
        self.skip_whitespace();
        if self.peek() != '{' {
            return Err(format!(
                "{}:{}: Expected '{{' after '{}' block",
                self.line, self.column, lang
            ));
        }
        self.advance(); // consume '{'

        let content_start = self.pos;
        let mut depth = 1;
        while !self.is_at_end() && depth > 0 {
            match self.peek() {
                '{' => {
                    depth += 1;
                    self.advance();
                }
                '}' => {
                    depth -= 1;
                    if depth > 0 {
                        self.advance();
                    }
                }
                _ => {
                    self.advance();
                }
            }
        }

        if depth != 0 {
            return Err(format!(
                "{}:{}: Unterminated {} block",
                self.line, self.column, lang
            ));
        }

        let content: String = self.source[content_start..self.pos].iter().collect();
        self.advance(); // consume closing '}'

        let kind = if lang == "c" {
            TokenKind::CBlock(content)
        } else {
            TokenKind::CppBlock(content)
        };

        Ok(self.make_token(kind, start))
    }

    fn lex_number(&mut self, start: usize) -> Result<Token, String> {
        // Integer part
        while !self.is_at_end() && self.peek().is_ascii_digit() {
            self.advance();
        }

        // Check for hex: 0x...
        if self.pos - start == 1
            && self.source[start] == '0'
            && (self.peek() == 'x' || self.peek() == 'X')
        {
            self.advance(); // consume 'x'
            while !self.is_at_end() && self.peek().is_ascii_hexdigit() {
                self.advance();
            }
            let text: String = self.source[start..self.pos].iter().collect();
            return Ok(self.make_token(TokenKind::NumberLiteral(text), start));
        }

        // Check for binary: 0b...
        if self.pos - start == 1
            && self.source[start] == '0'
            && (self.peek() == 'b' || self.peek() == 'B')
        {
            self.advance(); // consume 'b'
            while !self.is_at_end() && matches!(self.peek(), '0' | '1') {
                self.advance();
            }
            let text: String = self.source[start..self.pos].iter().collect();
            return Ok(self.make_token(TokenKind::NumberLiteral(text), start));
        }

        // Fractional part
        if self.peek() == '.' && self.peek_next().is_ascii_digit() {
            self.advance(); // consume '.'
            while !self.is_at_end() && self.peek().is_ascii_digit() {
                self.advance();
            }
        }

        // Exponent
        if self.peek() == 'e' || self.peek() == 'E' {
            self.advance();
            if self.peek() == '+' || self.peek() == '-' {
                self.advance();
            }
            while !self.is_at_end() && self.peek().is_ascii_digit() {
                self.advance();
            }
        }

        // Type suffix (e.g. 42i32, 3.14f64)
        if Self::is_ident_start(self.peek()) {
            let suffix_start = self.pos;
            while !self.is_at_end() && Self::is_ident_continue(self.peek()) {
                self.advance();
            }
            // Only accept valid type suffixes, else roll back
            let suffix: String = self.source[suffix_start..self.pos].iter().collect();
            match suffix.as_str() {
                "i8" | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" | "u64" | "f32" | "f64"
                | "usize" => {}
                _ => {
                    // Not a valid suffix — rewind
                    self.pos = suffix_start;
                }
            }
        }

        let text: String = self.source[start..self.pos].iter().collect();
        Ok(self.make_token(TokenKind::NumberLiteral(text), start))
    }

    fn lex_string(&mut self, start: usize) -> Result<Token, String> {
        let mut value = String::new();

        while !self.is_at_end() && self.peek() != '"' {
            if self.peek() == '\\' {
                self.advance(); // consume backslash
                if self.is_at_end() {
                    return Err(format!(
                        "{}:{}: Unterminated string escape",
                        self.line, self.column
                    ));
                }
                let escaped = self.advance();
                match escaped {
                    'n' => value.push('\n'),
                    't' => value.push('\t'),
                    'r' => value.push('\r'),
                    '\\' => value.push('\\'),
                    '"' => value.push('"'),
                    '0' => value.push('\0'),
                    '$' => value.push('$'),
                    other => {
                        value.push('\\');
                        value.push(other);
                    }
                }
            } else {
                value.push(self.advance());
            }
        }

        if self.is_at_end() {
            return Err(format!(
                "{}:{}: Unterminated string literal",
                self.line, self.column
            ));
        }

        self.advance(); // consume closing '"'
        Ok(self.make_token(TokenKind::StringLiteral(value), start))
    }

    fn lex_char(&mut self, start: usize) -> Result<Token, String> {
        if self.is_at_end() {
            return Err(format!(
                "{}:{}: Unterminated character literal",
                self.line, self.column
            ));
        }

        let ch = if self.peek() == '\\' {
            self.advance(); // backslash
            let escaped = self.advance();
            match escaped {
                'n' => '\n',
                't' => '\t',
                'r' => '\r',
                '\\' => '\\',
                '\'' => '\'',
                '0' => '\0',
                other => other,
            }
        } else {
            self.advance()
        };

        if self.peek() != '\'' {
            return Err(format!(
                "{}:{}: Expected closing quote for character literal",
                self.line, self.column
            ));
        }
        self.advance(); // consume closing '\''

        Ok(self.make_token(TokenKind::CharLiteral(ch), start))
    }

    fn lex_line_comment(&mut self, start: usize) -> Result<Token, String> {
        self.advance(); // consume second '/'
        let content_start = self.pos;
        while !self.is_at_end() && self.peek() != '\n' {
            self.advance();
        }
        let text: String = self.source[content_start..self.pos].iter().collect();
        Ok(self.make_token(TokenKind::Comment(text.trim().to_string()), start))
    }

    fn lex_block_comment(&mut self, start: usize) -> Result<Token, String> {
        self.advance(); // consume '*'
        let content_start = self.pos;
        let mut depth = 1;

        while !self.is_at_end() && depth > 0 {
            if self.peek() == '/' && self.peek_next() == '*' {
                depth += 1;
                self.advance();
                self.advance();
            } else if self.peek() == '*' && self.peek_next() == '/' {
                depth -= 1;
                self.advance();
                self.advance();
            } else {
                self.advance();
            }
        }

        if depth != 0 {
            return Err(format!(
                "{}:{}: Unterminated block comment",
                self.line, self.column
            ));
        }

        let text: String = self.source[content_start..self.pos - 2].iter().collect();
        Ok(self.make_token(
            TokenKind::Comment(text.trim().to_string()),
            start,
        ))
    }

    fn lex_hash(&mut self, start: usize) -> Result<Token, String> {
        if self.peek() == '[' {
            self.advance(); // consume '['
            let attr_start = self.pos;
            while !self.is_at_end() && self.peek() != ']' {
                self.advance();
            }
            if self.is_at_end() {
                return Err(format!(
                    "{}:{}: Unterminated attribute",
                    self.line, self.column
                ));
            }
            let attr: String = self.source[attr_start..self.pos].iter().collect();
            self.advance(); // consume ']'
            Ok(self.make_token(TokenKind::HashAttr(attr), start))
        } else {
            Ok(self.make_token(TokenKind::Hash, start))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_tokens() {
        let tokens = Lexer::tokenize("( ) { } ;").unwrap();
        assert_eq!(tokens[0].kind, TokenKind::LParen);
        assert_eq!(tokens[1].kind, TokenKind::RParen);
        assert_eq!(tokens[2].kind, TokenKind::LBrace);
        assert_eq!(tokens[3].kind, TokenKind::RBrace);
        assert_eq!(tokens[4].kind, TokenKind::Semi);
        assert_eq!(tokens[5].kind, TokenKind::Eof);
    }

    #[test]
    fn test_keywords() {
        let tokens = Lexer::tokenize("fn let mut return if else").unwrap();
        assert_eq!(tokens[0].kind, TokenKind::KwFn);
        assert_eq!(tokens[1].kind, TokenKind::KwLet);
        assert_eq!(tokens[2].kind, TokenKind::KwMut);
        assert_eq!(tokens[3].kind, TokenKind::KwReturn);
        assert_eq!(tokens[4].kind, TokenKind::KwIf);
        assert_eq!(tokens[5].kind, TokenKind::KwElse);
    }

    #[test]
    fn test_operators() {
        let tokens = Lexer::tokenize("== != <= >= && || -> =>").unwrap();
        assert_eq!(tokens[0].kind, TokenKind::EqualEqual);
        assert_eq!(tokens[1].kind, TokenKind::BangEqual);
        assert_eq!(tokens[2].kind, TokenKind::LtEqual);
        assert_eq!(tokens[3].kind, TokenKind::GtEqual);
        assert_eq!(tokens[4].kind, TokenKind::AndAnd);
        assert_eq!(tokens[5].kind, TokenKind::PipePipe);
        assert_eq!(tokens[6].kind, TokenKind::Arrow);
        assert_eq!(tokens[7].kind, TokenKind::FatArrow);
    }

    #[test]
    fn test_string_literal() {
        let tokens = Lexer::tokenize(r#""hello world""#).unwrap();
        assert_eq!(
            tokens[0].kind,
            TokenKind::StringLiteral("hello world".to_string())
        );
    }

    #[test]
    fn test_number_literals() {
        let tokens = Lexer::tokenize("42 3.14 0xFF 0b1010").unwrap();
        assert_eq!(
            tokens[0].kind,
            TokenKind::NumberLiteral("42".to_string())
        );
        assert_eq!(
            tokens[1].kind,
            TokenKind::NumberLiteral("3.14".to_string())
        );
        assert_eq!(
            tokens[2].kind,
            TokenKind::NumberLiteral("0xFF".to_string())
        );
        assert_eq!(
            tokens[3].kind,
            TokenKind::NumberLiteral("0b1010".to_string())
        );
    }

    #[test]
    fn test_c_block() {
        let tokens = Lexer::tokenize(r#"c { printf("hi"); }"#).unwrap();
        match &tokens[0].kind {
            TokenKind::CBlock(content) => {
                assert!(content.contains("printf"));
            }
            other => panic!("Expected CBlock, got {:?}", other),
        }
    }

    #[test]
    fn test_attribute() {
        let tokens = Lexer::tokenize("#[hot]").unwrap();
        assert_eq!(tokens[0].kind, TokenKind::HashAttr("hot".to_string()));
    }

    #[test]
    fn test_fn_declaration() {
        let tokens = Lexer::tokenize("fn main() -> i32 { return 0; }").unwrap();
        assert_eq!(tokens[0].kind, TokenKind::KwFn);
        assert_eq!(tokens[1].kind, TokenKind::Identifier("main".to_string()));
        assert_eq!(tokens[2].kind, TokenKind::LParen);
        assert_eq!(tokens[3].kind, TokenKind::RParen);
        assert_eq!(tokens[4].kind, TokenKind::Arrow);
        assert_eq!(tokens[5].kind, TokenKind::KwI32);
        assert_eq!(tokens[6].kind, TokenKind::LBrace);
        assert_eq!(tokens[7].kind, TokenKind::KwReturn);
        assert_eq!(tokens[8].kind, TokenKind::NumberLiteral("0".to_string()));
        assert_eq!(tokens[9].kind, TokenKind::Semi);
        assert_eq!(tokens[10].kind, TokenKind::RBrace);
    }
}
