use std::process::{Command, Stdio};
use std::io::{Write, Read};

fn main() {
    let mut child = Command::new("cargo")
        .args(["run", "--", "lsp"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to start cm lsp");

    let mut stdin = child.stdin.take().expect("Failed to open stdin");
    let mut stdout = child.stdout.take().expect("Failed to open stdout");

    // 1. Initialize
    let init_msg = "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{}}";
    write!(stdin, "Content-Length: {}\r\n\r\n{}", init_msg.len(), init_msg).unwrap();
    stdin.flush().unwrap();

    let mut buf = [0u8; 1024];
    let n = stdout.read(&mut buf).unwrap();
    println!("Init response: {}", String::from_utf8_lossy(&buf[..n]));

    // 2. didOpen
    let source = "fn main() { let x = 5; }";
    let open_msg = format!(
        "{{\"jsonrpc\":\"2.0\",\"method\":\"textDocument/didOpen\",\"params\":{{\"textDocument\":{{\"text\":\"{}\"}}}}}}",
        source
    );
    write!(stdin, "Content-Length: {}\r\n\r\n{}", open_msg.len(), open_msg).unwrap();
    stdin.flush().unwrap();

    // 3. Hover
    let hover_msg = "{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"textDocument/hover\",\"params\":{\"position\":{\"line\":0,\"character\":16}}}";
    write!(stdin, "Content-Length: {}\r\n\r\n{}", hover_msg.len(), hover_msg).unwrap();
    stdin.flush().unwrap();

    let n = stdout.read(&mut buf).unwrap();
    println!("Hover response: {}", String::from_utf8_lossy(&buf[..n]));

    child.kill().unwrap();
}
