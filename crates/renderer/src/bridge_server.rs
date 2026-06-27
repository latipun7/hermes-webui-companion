//! Tiny HTTP server that receives WebUI companion snapshots.
//!
//! Listens on 127.0.0.1:17899 for POST /api/webui/snapshot,
//! parses the JSON body into CompanionSnapshot, and emits
//! Tauri events so the frontend can react to state changes.

use std::io::{BufRead, BufReader, Read};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::thread;

use serde_json;
use tauri::Emitter;

use crate::bridge::parse_snapshot;
use crate::animation::CompanionSnapshot;

/// Spawn a background HTTP server that accepts POST snapshots
/// from the WebUI companion-adapter.js bridge.
///
/// Emits a `companion-state` Tauri event with the parsed snapshot
/// on every successful POST.
pub fn spawn_bridge_server(app_handle: tauri::AppHandle) {
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

            let mut reader = BufReader::new(stream.try_clone().unwrap_or_else(|_| {
                // If cloning fails, we can't read — skip
                panic!("stream clone failed")
            }));

            // Read request line + headers
            let mut request_line = String::new();
            if reader.read_line(&mut request_line).is_err() {
                continue;
            }

            // Only handle POST
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

            // Read body
            let mut body = vec![0u8; content_length];
            if reader.read_exact(&mut body).is_err() {
                continue;
            }

            // Parse and emit
            if let Ok(raw) = serde_json::from_slice(&body) {
                let snapshot = parse_snapshot(&raw);
                let _ = app_handle.emit("companion-state", serde_json::to_value(&snapshot).ok());
            }
        }
    });
}
