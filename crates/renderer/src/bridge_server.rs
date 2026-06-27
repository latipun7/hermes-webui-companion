//! Tiny HTTP server that receives WebUI companion snapshots.
//!
//! Listens on 127.0.0.1:17787 for POST requests from the
//! WebUI companion-adapter.js, parses the JSON body, and
//! updates the shared companion state.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::thread;

use crate::animation::CompanionSnapshot;
use crate::bridge::parse_snapshot;

fn cors_response(status: u16, body: &str) -> String {
    format!(
        "HTTP/1.1 {status} OK\r\n\
         Content-Type: application/json\r\n\
         Content-Length: {len}\r\n\
         Access-Control-Allow-Origin: *\r\n\
         Access-Control-Allow-Methods: GET, POST, OPTIONS\r\n\
         Access-Control-Allow-Headers: Content-Type\r\n\
         Connection: close\r\n\
         \r\n\
         {body}",
        status = status,
        len = body.len(),
        body = body,
    )
}

/// Spawn a background HTTP server that accepts POST snapshots.
///
/// On each successful POST, the shared `state` is atomically
/// updated — the frontend picks up the change on its next poll.
pub fn spawn_bridge_server(state: Arc<Mutex<CompanionSnapshot>>) {
    let listener = match TcpListener::bind("127.0.0.1:17787") {
        Ok(l) => l,
        Err(e) => {
            eprintln!("[companion] bridge server bind failed: {e}");
            return;
        }
    };

    thread::spawn(move || {
        for stream in listener.incoming() {
            let mut stream = match stream {
                Ok(s) => s,
                Err(_) => continue,
            };

            let mut reader = BufReader::new(stream.try_clone().unwrap());

            // Read request line
            let mut request_line = String::new();
            if reader.read_line(&mut request_line).is_err() {
                continue;
            }

            let parts: Vec<&str> = request_line.split_whitespace().collect();
            if parts.len() < 2 {
                continue;
            }
            let method = parts[0];
            let path = parts.get(1).copied().unwrap_or("/");

            // Handle GET /health or /api/state
            if method == "GET" && (path == "/health" || path == "/health/") {
                let response = cors_response(200, "{\"ok\":true,\"service\":\"hermes-webui-companion\"}");
                let _ = stream.write_all(response.as_bytes());
                continue;
            }

            // Serve current companion state for frontend fallback
            if method == "GET" && (path == "/api/state" || path == "/api/state/") {
                let body = if let Ok(guard) = state.lock() {
                    serde_json::to_string(&*guard).unwrap_or_else(|_| "{}".into())
                } else {
                    "{\"state\":\"Idle\",\"attention\":[]}".to_string()
                };
                let response = cors_response(200, &body);
                let _ = stream.write_all(response.as_bytes());
                continue;
            }

            // Open WebUI in browser — called by bubble on click
            if method == "GET" && path == "/api/open-webui" {
                #[cfg(target_os = "windows")]
                { let _ = std::process::Command::new("explorer.exe").arg("http://localhost:8787").spawn(); }
                #[cfg(target_os = "macos")]
                { let _ = std::process::Command::new("open").arg("http://localhost:8787").spawn(); }
                #[cfg(target_os = "linux")]
                { let _ = std::process::Command::new("xdg-open").arg("http://localhost:8787").spawn(); }
                let response = cors_response(200, "{\"ok\":true}");
                let _ = stream.write_all(response.as_bytes());
                continue;
            }

            // Handle OPTIONS preflight
            if method == "OPTIONS" {
                let response = cors_response(204, "");
                let _ = stream.write_all(response.as_bytes());
                continue;
            }

            // Only POST beyond this point
            if method != "POST" {
                continue;
            }
            eprintln!("[companion] POST {path}");
            let mut content_length = 0usize;
            loop {
                let mut line = String::new();
                if reader.read_line(&mut line).is_err() {
                    break;
                }
                if line == "\r\n" || line == "\n" {
                    break;
                }
                if let Some(val) = line
                    .to_lowercase()
                    .strip_prefix("content-length:")
                    .map(|s| s.trim().parse().unwrap_or(0))
                {
                    content_length = val;
                }
            }

            if content_length == 0 {
                continue;
            }

            let mut body = vec![0u8; content_length];
            if reader.read_exact(&mut body).is_err() {
                continue;
            }

            // Parse and update shared state
            if let Ok(raw) = serde_json::from_slice(&body) {
                let snapshot = parse_snapshot(&raw);
                eprintln!(
                    "[companion] state={:?} attention={}",
                    snapshot.state,
                    snapshot.attention.len()
                );
                if let Ok(mut guard) = state.lock() {
                    *guard = snapshot;
                }
            } else {
                let body_str = String::from_utf8_lossy(&body);
                eprintln!("[companion] failed to parse POST body (first 200 chars): {}", &body_str[..body_str.len().min(200)]);
            }

            // Send acknowledgment
            let response = cors_response(200, "{\"ok\":true}");
            let _ = stream.write_all(response.as_bytes());
        }
    });
}
