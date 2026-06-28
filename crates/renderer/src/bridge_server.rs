//! Tiny HTTP server that receives WebUI companion snapshots.
//!
//! Listens on 127.0.0.1:17787 for HTTP requests from the
//! WebUI companion-adapter.js. Handles:
//! - POST /api/webui/snapshot → update companion state
//! - GET  /api/state → return current state
//! - GET  /health → health check
//! - GET  /api/pet/navigation?since= → pending navigation command
//! - POST /api/pet/navigation_ack → acknowledge navigation
//! - POST /api/open-webui → open WebUI (legacy, bubble fallback)

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

use crate::animation::CompanionSnapshot;
use crate::bridge::parse_snapshot;

/// Log only when HERMES_COMPANION_DEBUG=1.
macro_rules! debug {
    ($($arg:tt)*) => {
        if std::env::var("HERMES_COMPANION_DEBUG").unwrap_or_default() == "1" {
            eprintln!($($arg)*);
        }
    };
}

/// A pending navigation command, consumed by companion-adapter.js.
pub type NavigationCommand = serde_json::Value;

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

/// Shared state passed to the bridge server.
pub struct BridgeState {
    pub snapshot: Arc<Mutex<CompanionSnapshot>>,
    pub navigation: Arc<Mutex<Option<NavigationCommand>>>,
    pub bubbles_visible: Arc<AtomicBool>,
}

/// Spawn a background HTTP server that accepts POST snapshots
/// and serves companion state + navigation commands.
pub fn spawn_bridge_server(state: BridgeState) {
    let listener = match TcpListener::bind("127.0.0.1:17787") {
        Ok(l) => l,
        Err(e) => {
            eprintln!("[companion] bridge server bind failed: {e}");
            return;
        }
    };

    let snapshot = state.snapshot;
    let navigation = state.navigation;
    let bubbles_visible = state.bubbles_visible;

    thread::spawn(move || {
        for conn in listener.incoming() {
            let mut stream = match conn {
                Ok(s) => s,
                Err(_) => continue,
            };

            let mut reader = BufReader::new(stream.try_clone().unwrap());

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

            // ── GET /health ──────────────────────────────────────────
            if method == "GET" && (path == "/health" || path == "/health/") {
                let response = cors_response(200, "{\"ok\":true,\"service\":\"hermes-webui-companion\"}");
                let _ = stream.write_all(response.as_bytes());
                continue;
            }

            // ── GET /api/state ─────────────────────────────────────
            if method == "GET" && (path == "/api/state" || path == "/api/state/") {
                let body = if let Ok(guard) = snapshot.lock() {
                    serde_json::to_string(&*guard).unwrap_or_else(|_| "{}".into())
                } else {
                    "{\"state\":\"Idle\",\"attention\":[]}".to_string()
                };
                let response = cors_response(200, &body);
                let _ = stream.write_all(response.as_bytes());
                continue;
            }

            // ── GET /api/bubbles/visible ─────────────────────────────
            // Pet window polls this to sync toggle button state.
            if method == "GET" && path == "/api/bubbles/visible" {
                let v = bubbles_visible.load(Ordering::SeqCst);
                let body = format!("{{\"visible\":{v}}}");
                let response = cors_response(200, &body);
                let _ = stream.write_all(response.as_bytes());
                continue;
            }

            // ── GET /api/pet/navigation ─────────────────────────────
            // Adapter polls this to pick up pending navigation commands.
            // Returns { command: { id, session_id, url, ... } } or { command: null }.
            if method == "GET" && path.starts_with("/api/pet/navigation") {
                let body = if let Ok(guard) = navigation.lock() {
                    if let Some(ref cmd) = *guard {
                        debug!("[companion] nav poll: returning command id={}", cmd.get("id").and_then(|v| v.as_str()).unwrap_or("?"));
                        let json = serde_json::json!({ "command": cmd });
                        serde_json::to_string(&json).unwrap_or_else(|_| "{\"command\":null}".into())
                    } else {
                        "{\"command\":null}".to_string()
                    }
                } else {
                    "{\"command\":null}".to_string()
                };
                let response = cors_response(200, &body);
                let _ = stream.write_all(response.as_bytes());
                continue;
            }

            // ── POST /api/pet/navigation_ack ─────────────────────────
            // Adapter acks after applying a navigation command.
            if method == "POST" && path == "/api/pet/navigation_ack" {
                if let Ok(mut guard) = navigation.lock() {
                    *guard = None;
                }
                let response = cors_response(200, "{\"ok\":true}");
                let _ = stream.write_all(response.as_bytes());
                continue;
            }

            // ── Read headers (shared by POST handlers) ────────────
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
                // ── OPTIONS preflight ─────────────────────
                if method == "OPTIONS" {
                    let response = cors_response(204, "");
                    let _ = stream.write_all(response.as_bytes());
                }
                continue;
            }

            // ── POST /api/webui/snapshot ────────────────────────────
            if method == "POST" && path == "/api/webui/snapshot" {
                let mut body = vec![0u8; content_length];
                if reader.read_exact(&mut body).is_ok() {
                    if let Ok(raw) = serde_json::from_slice(&body) {
                        let snap = parse_snapshot(&raw);
                        debug!("[companion] state={:?} attention={}", snap.state, snap.attention.len());
                        if let Ok(mut guard) = snapshot.lock() {
                            *guard = snap;
                        }
                    } else {
                        let body_str = String::from_utf8_lossy(&body);
                        debug!("[companion] failed to parse POST body: {}", &body_str[..body_str.len().min(200)]);
                    }
                }
                let response = cors_response(200, "{\"ok\":true}");
                let _ = stream.write_all(response.as_bytes());
                continue;
            }

            // ── POST /api/bubbles/visible ─────────────────────────────
            // Pet window toggle button sets bubble visibility via HTTP.
            if method == "POST" && path == "/api/bubbles/visible" {
                let mut body = vec![0u8; content_length];
                if reader.read_exact(&mut body).is_ok() {
                    if let Ok(json) = serde_json::from_slice::<serde_json::Value>(&body) {
                        if let Some(v) = json.get("visible").and_then(|v| v.as_bool()) {
                            bubbles_visible.store(v, Ordering::SeqCst);
                            debug!("[companion] bubbles visible = {v}");
                        }
                    }
                }
                let response = cors_response(200, "{\"ok\":true}");
                let _ = stream.write_all(response.as_bytes());
                continue;
            }

            // ── POST /api/open-webui (legacy) ────────────────────────
            if method == "POST" && path == "/api/open-webui" {
                let mut body = vec![0u8; content_length];
                if reader.read_exact(&mut body).is_ok() {
                    if let Ok(json) = serde_json::from_slice::<serde_json::Value>(&body) {
                        let sid = json.get("session_id").and_then(|v| v.as_str()).unwrap_or("");
                        // Queue navigation command for adapter (navigates inside the tab)
                        if !sid.is_empty() {
                            let cmd = serde_json::json!({
                                "id": format!("nav-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis()),
                                "session_id": sid,
                                "url": format!("/session/{}", sid),
                            });
                            if let Ok(mut guard) = navigation.lock() {
                                *guard = Some(cmd);
                            }
                        }
                        // After adapter navigates, focus existing browser window
                        #[cfg(target_os = "windows")]
                        {
                            let focus_url = if sid.is_empty() {
                                "http://localhost:8787".to_string()
                            } else {
                                format!("http://localhost:8787/session/{}", sid)
                            };
                            std::thread::spawn(move || {
                                std::thread::sleep(std::time::Duration::from_secs(2));
                                // Three-stage browser focus:
                                //  1) Title match → WebUI tab is active, just focus it
                                //  2) Browser found → focus existing window
                                //     (companion-adapter.js handles session switch in-tab)
                                //  3) No browser → open default browser
                                let ps = format!(
                                    "$w=(New-Object -ComObject WScript.Shell); \
                                     $found=$w.AppActivate('localhost:8787'); \
                                     if(-not $found){{$found=$w.AppActivate('WebUI')}}; \
                                     if(-not $found){{ \
                                       $browsers=@('zen','msedge','chrome','firefox','brave','opera','vivaldi','chromium','arc'); \
                                       foreach($b in $browsers){{ \
                                         $p=Get-Process -Name $b -ErrorAction SilentlyContinue|Where-Object{{$_.MainWindowHandle -ne 0}}|Select-Object -First 1; \
                                         if($p){{$w.AppActivate($p.Id)|Out-Null;$found=$true;break}} \
                                       }} \
                                     }}; \
                                     if(-not $found){{Start-Process '{url}'}}",
                                    url = focus_url
                                );
                                let _ = std::process::Command::new("powershell")
                                    .args(["-NoProfile", "-Command", &ps])
                                    .spawn();
                            });
                        }
                    }
                }
                let response = cors_response(200, "{\"ok\":true}");
                let _ = stream.write_all(response.as_bytes());
                continue;
            }

            // ── Unknown ────────────────────────────────────────────
            let response = cors_response(404, "{\"error\":\"not_found\"}");
            let _ = stream.write_all(response.as_bytes());
        }
    });
}
