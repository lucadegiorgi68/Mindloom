//! Tiny dependency free dev server: serves ./ui and exposes
//! POST /invoke/<command> with a JSON body, backed by a real Session.
//! Lets the UI be developed and tested in an ordinary browser.
use mindmap_api::Session;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;

fn main() {
    let port = std::env::args().nth(1).unwrap_or_else(|| "5173".into());
    let ui = std::env::args().nth(2).unwrap_or_else(|| "ui".into());
    let listener = TcpListener::bind(format!("127.0.0.1:{port}")).unwrap();
    let mut session = Session::new("Atom structure");
    eprintln!("dev server on http://127.0.0.1:{port}");
    for stream in listener.incoming().flatten() {
        let mut reader = BufReader::new(&stream);
        let mut first = String::new();
        if reader.read_line(&mut first).is_err() {
            continue;
        }
        let mut len = 0usize;
        loop {
            let mut h = String::new();
            if reader.read_line(&mut h).unwrap_or(0) <= 2 {
                break;
            }
            if let Some(v) = h.to_ascii_lowercase().strip_prefix("content-length:") {
                len = v.trim().parse().unwrap_or(0);
            }
        }
        let mut body = vec![0u8; len];
        let _ = reader.read_exact(&mut body);
        let mut parts = first.split_whitespace();
        let (method, path) = (parts.next().unwrap_or(""), parts.next().unwrap_or("/"));
        let (status, ctype, out): (&str, &str, Vec<u8>) =
            if method == "POST" && path.starts_with("/invoke/") {
                let args: serde_json::Value =
                    serde_json::from_slice(&body).unwrap_or(serde_json::Value::Null);
                match session.invoke(&path[8..], &args) {
                    Ok(v) => ("200 OK", "application/json", v.to_string().into_bytes()),
                    Err(e) => (
                        "400 Bad Request",
                        "application/json",
                        serde_json::json!({"error": e}).to_string().into_bytes(),
                    ),
                }
            } else {
                let rel = if path == "/" {
                    "index.html"
                } else {
                    path.trim_start_matches('/')
                };
                if rel.contains("..") {
                    ("403 Forbidden", "text/plain", b"no".to_vec())
                } else {
                    match std::fs::read(format!("{ui}/{rel}")) {
                        Ok(b) => {
                            let ct = if rel.ends_with(".js") {
                                "text/javascript"
                            } else if rel.ends_with(".css") {
                                "text/css"
                            } else {
                                "text/html"
                            };
                            ("200 OK", ct, b)
                        }
                        Err(_) => ("404 Not Found", "text/plain", b"not found".to_vec()),
                    }
                }
            };
        let mut s = &stream;
        let _ = write!(s, "HTTP/1.1 {status}\r\nContent-Type: {ctype}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", out.len());
        let _ = s.write_all(&out);
    }
}
