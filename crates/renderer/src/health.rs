//! Health checks — sidecar + WebUI — initial synchronous probe + background polling.
//!
//! Sets an `AtomicBool` flag consumed by `resolve_animation_state()` so
//! incoming WebUI snapshots cannot race the health check and cause a
//! flicker loop. Both the sidecar (:17888) and WebUI (:8787 or
//! `HERMES_WEBUI_PORT`) must be healthy for the flag to be `true`.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use companion_renderer::sidecar_client::SidecarClient;

use crate::debug;

/// Returns the WebUI base URL from `HERMES_WEBUI_PORT` (default `8787`).
fn webui_base_url() -> String {
    let port = std::env::var("HERMES_WEBUI_PORT").unwrap_or_else(|_| "8787".into());
    format!("http://127.0.0.1:{}", port)
}

/// Check whether the WebUI is healthy by hitting `GET /health`.
///
/// WebUI returns `{"status": "ok", ...}` (not `{"ok": true}` like the sidecar).
pub(crate) fn check_webui_health() -> bool {
    let url = format!("{}/health", webui_base_url());
    match ureq::get(&url).config().http_status_as_error(false).build().call() {
        Ok(response) if response.status() == 200 => response
            .into_body()
            .read_json::<serde_json::Value>()
            .ok()
            .map(|v| {
                // WebUI returns {"status": "ok", ...}
                // Also accept {"ok": true} for compatibility
                v.get("status").and_then(|s| s.as_str()).map(|s| s == "ok").unwrap_or(false)
                    || v.get("ok").and_then(|ok| ok.as_bool()).unwrap_or(false)
            })
            .unwrap_or(false),
        _ => false,
    }
}

/// Run initial synchronous health checks for both sidecar and WebUI,
/// then spawn a background thread that probes both every 10 seconds.
/// Returns an `AtomicBool` that is `true` only when BOTH services are healthy.
pub fn spawn_health_check() -> Arc<AtomicBool> {
    let sidecar_client = SidecarClient::new("http://127.0.0.1:17888".into());

    let sidecar_ok = sidecar_client.check_health();
    let webui_ok = check_webui_health();
    let initial_healthy = sidecar_ok && webui_ok;

    if !sidecar_ok {
        debug!("[companion:health] sidecar unreachable at startup → Failed");
    }
    if !webui_ok {
        debug!("[companion:health] webui unreachable at startup ({}) → Failed", webui_base_url());
    }

    let all_healthy = Arc::new(AtomicBool::new(initial_healthy));

    {
        let all_healthy = all_healthy.clone();
        std::thread::spawn(move || {
            loop {
                std::thread::sleep(std::time::Duration::from_secs(10));

                let sidecar_ok = sidecar_client.check_health();
                let webui_ok = check_webui_health();
                let healthy = sidecar_ok && webui_ok;

                let was = all_healthy.swap(healthy, Ordering::SeqCst);
                if !healthy && was {
                    if !sidecar_ok {
                        debug!("[companion:health] sidecar unreachable → Failed");
                    }
                    if !webui_ok {
                        debug!(
                            "[companion:health] webui unreachable ({}) → Failed",
                            webui_base_url()
                        );
                    }
                } else if healthy && !was {
                    debug!("[companion:health] all services recovered → Idle");
                }
            }
        });
    }

    all_healthy
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::{Shutdown, TcpListener};
    use std::thread;

    /// Start a tiny HTTP server on a random port that responds once then shuts down.
    fn serve_once(status: u16, body: &str) -> u16 {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let body = body.to_string();

        thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let _ = stream.read(&mut [0; 4096]); // drain
            let response = format!(
                "HTTP/1.1 {status} OK\r\nContent-Type: application/json\r\nContent-Length: {len}\r\nConnection: close\r\n\r\n{body}",
                status = status,
                len = body.len(),
                body = body,
            );
            stream.write_all(response.as_bytes()).unwrap();
            let _ = stream.shutdown(Shutdown::Write);
        });

        port
    }

    #[allow(unsafe_code)]
    fn with_webui_port(port: u16) {
        unsafe {
            std::env::set_var("HERMES_WEBUI_PORT", port.to_string());
        }
    }

    #[test]
    fn webui_health_returns_true_on_status_ok() {
        let port = serve_once(200, r#"{"status":"ok","sessions":0}"#);
        with_webui_port(port);
        assert!(check_webui_health());
    }

    #[test]
    fn webui_health_accepts_ok_true_format() {
        let port = serve_once(200, r#"{"ok":true}"#);
        with_webui_port(port);
        assert!(check_webui_health());
    }

    #[test]
    fn webui_health_returns_false_on_status_not_ok() {
        let port = serve_once(200, r#"{"status":"error"}"#);
        with_webui_port(port);
        assert!(!check_webui_health());
    }

    #[test]
    fn webui_health_returns_false_on_404() {
        let port = serve_once(404, r#"{}"#);
        with_webui_port(port);
        assert!(!check_webui_health());
    }

    #[test]
    fn webui_health_returns_false_on_bad_json() {
        let port = serve_once(200, "not json");
        with_webui_port(port);
        assert!(!check_webui_health());
    }

    #[test]
    fn webui_health_returns_false_on_missing_status() {
        let port = serve_once(200, r#"{"uptime":123}"#);
        with_webui_port(port);
        assert!(!check_webui_health());
    }
}
