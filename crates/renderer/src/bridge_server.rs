//! Tiny HTTP server that receives WebUI companion snapshots.
//!
//! Listens on 127.0.0.1:17787 for POST requests from the
//! WebUI companion-adapter.js, parses the JSON body, and
//! updates the shared companion state.

use std::io::{BufRead, BufReader, Read};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::thread;

use crate::animation::CompanionSnapshot;
use crate::bridge::parse_snapshot;

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
            let stream = match stream {
                Ok(s) => s,
                Err(_) => continue,
            };

            let mut reader = BufReader::new(stream);

            // Read request line
            let mut request_line = String::new();
            if reader.read_line(&mut request_line).is_err() {
                continue;
            }

            let parts: Vec<&str> = request_line.split_whitespace().collect();
            if parts.len() < 2 || parts[0] != "POST" {
                continue;
            }

            // Read headers to find Content-Length
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
                if let Ok(mut guard) = state.lock() {
                    *guard = snapshot;
                }
            }
        }
    });
}
