use std::io::{self, Read, Write};
use std::collections::HashMap;

/// Simple JSON value enum for LSP communication.
#[derive(Debug, Clone, PartialEq)]
pub enum JsonValue {
    Object(HashMap<String, JsonValue>),
    Array(Vec<JsonValue>),
    String(String),
    Number(f64),
    Bool(bool),
    Null,
}

impl JsonValue {
    pub fn as_str(&self) -> Option<&str> {
        if let JsonValue::String(s) = self { Some(s) } else { None }
    }

    pub fn get(&self, key: &str) -> Option<&JsonValue> {
        if let JsonValue::Object(map) = self { map.get(key) } else { None }
    }
}

/// Lightweight JSON parser with escape character support.
pub struct JsonParser<'a> {
    input: &'a [u8],
    pos: usize,
}

impl<'a> JsonParser<'a> {
    pub fn new(input: &'a [u8]) -> Self {
        Self { input, pos: 0 }
    }

    pub fn parse(&mut self) -> Result<JsonValue, String> {
        self.skip_whitespace();
        let val = self.parse_value()?;
        self.skip_whitespace();
        Ok(val)
    }

    fn parse_value(&mut self) -> Result<JsonValue, String> {
        match self.peek() {
            Some(b'{') => self.parse_object(),
            Some(b'[') => self.parse_array(),
            Some(b'"') => self.parse_string(),
            Some(b't') | Some(b'f') => self.parse_bool(),
            Some(b'n') => self.parse_null(),
            Some(b'-') | Some(b'0'..=b'9') => self.parse_number(),
            Some(c) => Err(format!("Unexpected character: {}", c as char)),
            None => Err("Unexpected end of input".to_string()),
        }
    }

    fn parse_object(&mut self) -> Result<JsonValue, String> {
        self.consume(b'{')?;
        let mut map = HashMap::new();
        self.skip_whitespace();
        if self.peek() == Some(b'}') {
            self.consume(b'}')?;
            return Ok(JsonValue::Object(map));
        }

        loop {
            let key = match self.parse_string()? {
                JsonValue::String(s) => s,
                _ => return Err("Object key must be string".to_string()),
            };
            self.skip_whitespace();
            self.consume(b':')?;
            let val = self.parse_value()?;
            map.insert(key, val);
            self.skip_whitespace();
            match self.peek() {
                Some(b',') => {
                    self.consume(b',')?;
                    self.skip_whitespace();
                }
                Some(b'}') => {
                    self.consume(b'}')?;
                    break;
                }
                _ => return Err("Expected ',' or '}'".to_string()),
            }
        }
        Ok(JsonValue::Object(map))
    }

    fn parse_array(&mut self) -> Result<JsonValue, String> {
        self.consume(b'[')?;
        let mut arr = Vec::new();
        self.skip_whitespace();
        if self.peek() == Some(b']') {
            self.consume(b']')?;
            return Ok(JsonValue::Array(arr));
        }

        loop {
            arr.push(self.parse_value()?);
            self.skip_whitespace();
            match self.peek() {
                Some(b',') => {
                    self.consume(b',')?;
                    self.skip_whitespace();
                }
                Some(b']') => {
                    self.consume(b']')?;
                    break;
                }
                _ => return Err("Expected ',' or ']'".to_string()),
            }
        }
        Ok(JsonValue::Array(arr))
    }

    fn parse_string(&mut self) -> Result<JsonValue, String> {
        self.consume(b'"')?;
        let mut s = String::new();
        while let Some(c) = self.next() {
            match c {
                b'"' => return Ok(JsonValue::String(s)),
                b'\\' => {
                    match self.next() {
                        Some(b'"') => s.push('"'),
                        Some(b'\\') => s.push('\\'),
                        Some(b'/') => s.push('/'),
                        Some(b'b') => s.push('\x08'),
                        Some(b'f') => s.push('\x0c'),
                        Some(b'n') => s.push('\n'),
                        Some(b'r') => s.push('\r'),
                        Some(b't') => s.push('\t'),
                        Some(b'u') => {
                            // Minimal hex support
                            let mut hex = String::new();
                            for _ in 0..4 {
                                hex.push(self.next().ok_or("Unexpected end of hex")? as char);
                            }
                            let code = u32::from_str_radix(&hex, 16).map_err(|_| "Invalid hex")?;
                            s.push(std::char::from_u32(code).ok_or("Invalid unicode")?);
                        }
                        _ => return Err("Invalid escape sequence".to_string()),
                    }
                }
                _ => s.push(c as char),
            }
        }
        Err("Unterminated string".to_string())
    }

    fn parse_bool(&mut self) -> Result<JsonValue, String> {
        if self.input[self.pos..].starts_with(b"true") {
            self.pos += 4;
            Ok(JsonValue::Bool(true))
        } else if self.input[self.pos..].starts_with(b"false") {
            self.pos += 5;
            Ok(JsonValue::Bool(false) )
        } else {
            Err("Expected bool".to_string())
        }
    }

    fn parse_null(&mut self) -> Result<JsonValue, String> {
        if self.input[self.pos..].starts_with(b"null") {
            self.pos += 4;
            Ok(JsonValue::Null)
        } else {
            Err("Expected null".to_string())
        }
    }

    fn parse_number(&mut self) -> Result<JsonValue, String> {
        let start = self.pos;
        if self.peek() == Some(b'-') { self.pos += 1; }
        while let Some(b'0'..=b'9' | b'.') = self.peek() { self.pos += 1; }
        let s = std::str::from_utf8(&self.input[start..self.pos]).map_err(|_| "Invalid UTF-8 in number")?;
        let n = s.parse::<f64>().map_err(|_| "Invalid number")?;
        Ok(JsonValue::Number(n))
    }

    fn skip_whitespace(&mut self) {
        while let Some(b' ' | b'\n' | b'\r' | b'\t') = self.peek() {
            self.pos += 1;
        }
    }

    fn peek(&self) -> Option<u8> {
        self.input.get(self.pos).cloned()
    }

    fn next(&mut self) -> Option<u8> {
        let res = self.peek();
        if res.is_some() { self.pos += 1; }
        res
    }

    fn consume(&mut self, expected: u8) -> Result<(), String> {
        if self.next() == Some(expected) {
            Ok(())
        } else {
            Err(format!("Expected '{}'", expected as char))
        }
    }
}

/// JSON-RPC message implementation.
#[derive(Debug, Clone)]
pub struct Message {
    pub jsonrpc: String,
    pub id: Option<JsonValue>,
    pub method: Option<String>,
    pub params: Option<JsonValue>,
    pub result: Option<JsonValue>,
    pub error: Option<JsonValue>,
}

impl Message {
    pub fn parse(input: &[u8]) -> Result<Self, String> {
        let mut parser = JsonParser::new(input);
        let val = parser.parse()?;
        if let JsonValue::Object(map) = val {
            Ok(Message {
                jsonrpc: map.get("jsonrpc").and_then(|v| v.as_str()).unwrap_or("2.0").to_string(),
                id: map.get("id").cloned(),
                method: map.get("method").and_then(|v| v.as_str()).map(|s| s.to_string()),
                params: map.get("params").cloned(),
                result: map.get("result").cloned(),
                error: map.get("error").cloned(),
            })
        } else {
            Err("Expected JSON object".to_string())
        }
    }

    pub fn to_json(&self) -> String {
        let mut parts = vec![format!("\"jsonrpc\":\"{}\"", self.jsonrpc)];
        if let Some(id) = &self.id {
            parts.push(format!("\"id\":{}", json_to_str(id)));
        }
        if let Some(method) = &self.method {
            parts.push(format!("\"method\":\"{}\"", method));
        }
        if let Some(params) = &self.params {
            parts.push(format!("\"params\":{}", json_to_str(params)));
        }
        if let Some(result) = &self.result {
            parts.push(format!("\"result\":{}", json_to_str(result)));
        }
        if let Some(error) = &self.error {
            parts.push(format!("\"error\":{}", json_to_str(error)));
        }
        format!("{{{}}}", parts.join(","))
    }
}

fn json_to_str(val: &JsonValue) -> String {
    match val {
        JsonValue::Object(map) => {
            let parts: Vec<String> = map.iter().map(|(k, v)| format!("\"{}\":{}", k, json_to_str(v))).collect();
            format!("{{{}}}", parts.join(","))
        }
        JsonValue::Array(arr) => {
            let parts: Vec<String> = arr.iter().map(json_to_str).collect();
            format!("[{}]", parts.join(","))
        }
        JsonValue::String(s) => format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n").replace('\r', "\\r").replace('\t', "\\t")),
        JsonValue::Number(n) => n.to_string(),
        JsonValue::Bool(b) => b.to_string(),
        JsonValue::Null => "null".to_string(),
    }
}

pub fn read_message<R: Read>(reader: &mut R) -> Result<Vec<u8>, String> {
    let mut header = String::new();
    let mut buf = [0u8; 1];
    
    // Read headers
    loop {
        reader.read_exact(&mut buf).map_err(|e| format!("IO: {}", e))?;
        header.push(buf[0] as char);
        if header.ends_with("\r\n\r\n") { break; }
    }

    let mut content_length = 0;
    for line in header.lines() {
        if line.to_lowercase().starts_with("content-length:") {
            content_length = line["content-length:".len()..].trim().parse::<usize>().unwrap_or(0);
        }
    }

    if content_length == 0 { return Err("No Content-Length found".to_string()); }

    let mut content = vec![0u8; content_length];
    reader.read_exact(&mut content).map_err(|e| format!("IO: {}", e))?;
    Ok(content)
}

pub fn write_message<W: Write>(writer: &mut W, msg_json: &str) -> io::Result<()> {
    write!(writer, "Content-Length: {}\r\n\r\n", msg_json.len())?;
    write!(writer, "{}", msg_json)?;
    writer.flush()
}
