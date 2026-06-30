//! HTTP bridge server — receives WebUI companion snapshots.
//!
//! Listens on 127.0.0.1:17787. Uses `tiny_http` for request parsing
//! and routing. Core logic is extracted into pure handler functions
//! so each endpoint can be tested without starting a real server.
//!
//! Endpoints:
//! - GET  /health              → health check
//! - GET  /api/state           → current companion state
//! - GET  /api/bubbles/visible → bubble visibility flag
//! - GET  /api/pet/navigation  → pending navigation command
//! - POST /api/pet/navigation_ack → acknowledge navigation
//! - POST /api/webui/snapshot  → receive WebUI snapshot
//! - POST /api/bubbles/visible → set bubble visibility
//! - POST /api/open-webui      → open/focus WebUI session

use std::sync::atomic::AtomicBool;
#[cfg(feature = "gui")]
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

use crate::animation::CompanionSnapshot;
#[cfg(feature = "gui")]
use crate::animation::StateResponse;
#[cfg(feature = "gui")]
use crate::bridge::parse_snapshot;

/// Log only when HERMES_COMPANION_DEBUG=1.
#[cfg(feature = "gui")]
macro_rules! debug {
    ($($arg:tt)*) => {
        if std::env::var("HERMES_COMPANION_DEBUG").unwrap_or_default() == "1" {
            eprintln!($($arg)*);
        }
    };
}

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// A pending navigation command, consumed by companion-adapter.js.
pub type NavigationCommand = serde_json::Value;

/// Shared state passed to the bridge server.
pub struct BridgeState {
    pub snapshot: Arc<Mutex<CompanionSnapshot>>,
    pub navigation: Arc<Mutex<Option<NavigationCommand>>>,
    pub bubbles_visible: Arc<AtomicBool>,
    pub sidecar_healthy: Arc<AtomicBool>,
}

/// Minimal HTTP response for handler testing.
#[derive(Debug, PartialEq)]
#[allow(dead_code)]
struct HttpResponse {
    status: u16,
    body: String,
}

#[allow(dead_code)]
impl HttpResponse {
    fn new(status: u16, body: String) -> Self {
        Self { status, body }
    }

    fn ok(body: String) -> Self {
        Self { status: 200, body }
    }

    fn not_found() -> Self {
        Self {
            status: 404,
            body: r#"{"error":"not_found"}"#.into(),
        }
    }
}

// ---------------------------------------------------------------------------
// Route handlers (pure functions — testable without HTTP server)
// ---------------------------------------------------------------------------

#[allow(dead_code)]
fn handle_health() -> HttpResponse {
    HttpResponse::ok(r#"{"ok":true,"service":"hermes-webui-companion"}"#.into())
}

#[cfg(feature = "gui")]
fn handle_get_state(snapshot: &Mutex<CompanionSnapshot>, sidecar_healthy: &AtomicBool) -> HttpResponse {
    let body = if let Ok(guard) = snapshot.lock() {
        let healthy = sidecar_healthy.load(Ordering::SeqCst);
        let resp = StateResponse::from_snapshot(&guard, healthy);
        serde_json::to_string(&resp).unwrap_or_else(|_| "{}".into())
    } else {
        r#"{"state":"Idle","attention":[],"resolved_animation":"idle"}"#.into()
    };
    HttpResponse::ok(body)
}

#[cfg(feature = "gui")]
fn handle_get_bubbles_visible(flag: &AtomicBool) -> HttpResponse {
    let v = flag.load(Ordering::SeqCst);
    HttpResponse::ok(format!(r#"{{"visible":{v}}}"#))
}

#[cfg(feature = "gui")]
fn handle_get_navigation(nav: &Mutex<Option<NavigationCommand>>) -> HttpResponse {
    let body = if let Ok(guard) = nav.lock() {
        if let Some(ref cmd) = *guard {
            let json = serde_json::json!({ "command": cmd });
            serde_json::to_string(&json).unwrap_or_else(|_| r#"{"command":null}"#.into())
        } else {
            r#"{"command":null}"#.into()
        }
    } else {
        r#"{"command":null}"#.into()
    };
    HttpResponse::ok(body)
}

#[cfg(feature = "gui")]
fn handle_post_navigation_ack(nav: &Mutex<Option<NavigationCommand>>) -> HttpResponse {
    if let Ok(mut guard) = nav.lock() {
        *guard = None;
    }
    HttpResponse::ok(r#"{"ok":true}"#.into())
}

#[cfg(feature = "gui")]
fn handle_post_snapshot(body: &[u8], snapshot: &Mutex<CompanionSnapshot>) -> HttpResponse {
    match serde_json::from_slice(body) {
        Ok(raw) => {
            let snap = parse_snapshot(&raw);
            debug!("[companion:bridge] state={:?} attention={}", snap.state, snap.attention.len());
            if let Ok(mut guard) = snapshot.lock() {
                *guard = snap;
            }
            HttpResponse::ok(r#"{"ok":true}"#.into())
        }
        Err(_) => {
            debug!("[companion:bridge] failed to parse POST body");
            HttpResponse::ok(r#"{"ok":false,"error":"invalid_json"}"#.into())
        }
    }
}

#[cfg(feature = "gui")]
fn handle_post_bubbles_visible(body: &[u8], flag: &AtomicBool) -> HttpResponse {
    if let Ok(json) = serde_json::from_slice::<serde_json::Value>(body) {
        if let Some(v) = json.get("visible").and_then(|v| v.as_bool()) {
            flag.store(v, Ordering::SeqCst);
        }
    }
    HttpResponse::ok(r#"{"ok":true}"#.into())
}

#[cfg(feature = "gui")]
fn handle_post_open_webui(body: &[u8], nav: &Mutex<Option<NavigationCommand>>) -> HttpResponse {
    if let Ok(json) = serde_json::from_slice::<serde_json::Value>(body) {
        let sid = json
            .get("session_id")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if !sid.is_empty() {
            let cmd = serde_json::json!({
                "id": format!("nav-{}", std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis()),
                "session_id": sid,
                "url": format!("/session/{}", sid),
            });
            if let Ok(mut guard) = nav.lock() {
                *guard = Some(cmd);
            }
        }
        // Focus existing browser window
        #[cfg(target_os = "windows")]
        {
            let focus_url = if sid.is_empty() {
                "http://localhost:8787".to_string()
            } else {
                format!("http://localhost:8787/session/{}", sid)
            };
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_secs(2));
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
    HttpResponse::ok(r#"{"ok":true}"#.into())
}

// ---------------------------------------------------------------------------
// Server (tiny_http) — only compiled with the "gui" feature
// ---------------------------------------------------------------------------

#[cfg(feature = "gui")]
pub fn spawn_bridge_server(state: BridgeState) {
    let server = match tiny_http::Server::http("127.0.0.1:17787") {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[companion] bridge server bind failed: {e}");
            return;
        }
    };

    let snapshot = state.snapshot;
    let navigation = state.navigation;
    let bubbles_visible = state.bubbles_visible;
    let sidecar_healthy = state.sidecar_healthy;

    std::thread::spawn(move || {
        debug!("[companion:bridge] server listening on 127.0.0.1:17787");
        for mut request in server.incoming_requests() {
            let response = route_request(&mut request, &snapshot, &navigation, &bubbles_visible, &sidecar_healthy);
            let _ = request.respond(response);
        }
    });
}

#[cfg(feature = "gui")]
fn route_request(
    req: &mut tiny_http::Request,
    snapshot: &Mutex<CompanionSnapshot>,
    navigation: &Mutex<Option<NavigationCommand>>,
    bubbles_visible: &AtomicBool,
    sidecar_healthy: &AtomicBool,
) -> tiny_http::Response<std::io::Cursor<Vec<u8>>> {
    use tiny_http::{Header, Method, Response, StatusCode};
    let method = req.method();
    let url = req.url().to_string();

    debug!("[companion:bridge] {} {}", method, url);

    let handler_result = match (method, url.as_str()) {
        (Method::Get, "/health") | (Method::Get, "/health/") => handle_health(),

        (Method::Get, "/api/state") | (Method::Get, "/api/state/") => {
            handle_get_state(snapshot, sidecar_healthy)
        }

        (Method::Get, "/api/bubbles/visible") => handle_get_bubbles_visible(bubbles_visible),

        (Method::Get, url) if url.starts_with("/api/pet/navigation") => {
            handle_get_navigation(navigation)
        }

        (Method::Post, "/api/pet/navigation_ack") => handle_post_navigation_ack(navigation),

        (Method::Post, "/api/webui/snapshot") => {
            let mut body = Vec::new();
            let _ = req.as_reader().read_to_end(&mut body);
            handle_post_snapshot(&body, snapshot)
        }

        (Method::Post, "/api/bubbles/visible") => {
            let mut body = Vec::new();
            let _ = req.as_reader().read_to_end(&mut body);
            handle_post_bubbles_visible(&body, bubbles_visible)
        }

        (Method::Post, "/api/open-webui") => {
            let mut body = Vec::new();
            let _ = req.as_reader().read_to_end(&mut body);
            handle_post_open_webui(&body, navigation)
        }

        // CORS preflight — respond with allow-all headers
        (Method::Options, _) => HttpResponse::new(204, String::new()),

        _ => HttpResponse::not_found(),
    };

    let status = StatusCode(handler_result.status);
    let body = handler_result.body;
    Response::from_string(body)
        .with_status_code(status)
        .with_header(
            Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap(),
        )
        .with_header(
            Header::from_bytes(&b"Access-Control-Allow-Origin"[..], &b"*"[..]).unwrap(),
        )
        .with_header(
            Header::from_bytes(&b"Access-Control-Allow-Methods"[..], &b"GET, POST, OPTIONS"[..])
                .unwrap(),
        )
        .with_header(
            Header::from_bytes(
                &b"Access-Control-Allow-Headers"[..],
                &b"Content-Type"[..],
            )
            .unwrap(),
        )
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(all(test, feature = "gui"))]
mod tests {
    use super::*;
    use crate::animation::{AttentionStatus, CompanionState};

    fn test_snapshot() -> Arc<Mutex<CompanionSnapshot>> {
        Arc::new(Mutex::new(CompanionSnapshot {
            state: CompanionState::Idle,
            attention: vec![],
        }))
    }

    fn test_navigation() -> Arc<Mutex<Option<NavigationCommand>>> {
        Arc::new(Mutex::new(None))
    }

    fn test_flag(initial: bool) -> Arc<AtomicBool> {
        Arc::new(AtomicBool::new(initial))
    }

    #[test]
    fn health_returns_ok() {
        let resp = handle_health();
        assert_eq!(resp.status, 200);
        let json: serde_json::Value = serde_json::from_str(&resp.body).unwrap();
        assert_eq!(json["ok"], true);
        assert_eq!(json["service"], "hermes-webui-companion");
    }

    #[test]
    fn get_state_returns_idle_when_empty() {
        let snap = test_snapshot();
        let healthy = test_flag(true);
        let resp = handle_get_state(&snap, &healthy);
        assert_eq!(resp.status, 200);
        let json: serde_json::Value = serde_json::from_str(&resp.body).unwrap();
        assert_eq!(json["state"], "idle");
        assert!(json["resolved_animation"].as_str().is_some());
    }

    #[test]
    fn get_state_unhealthy_sidecar_resolves_to_failed() {
        let snap = test_snapshot();
        {
            let mut g = snap.lock().unwrap();
            g.state = CompanionState::Running;
        }
        let healthy = test_flag(false);
        let resp = handle_get_state(&snap, &healthy);
        assert_eq!(resp.status, 200);
        let json: serde_json::Value = serde_json::from_str(&resp.body).unwrap();
        assert_eq!(json["resolved_animation"], "failed");
    }

    #[test]
    fn get_bubbles_visible_returns_flag() {
        let flag = test_flag(true);
        let resp = handle_get_bubbles_visible(&flag);
        assert_eq!(resp.status, 200);
        let json: serde_json::Value = serde_json::from_str(&resp.body).unwrap();
        assert_eq!(json["visible"], true);
    }

    #[test]
    fn get_navigation_returns_null_when_empty() {
        let nav = test_navigation();
        let resp = handle_get_navigation(&nav);
        assert_eq!(resp.status, 200);
        let json: serde_json::Value = serde_json::from_str(&resp.body).unwrap();
        assert_eq!(json["command"], serde_json::Value::Null);
    }

    #[test]
    fn get_navigation_returns_pending_command() {
        let nav = test_navigation();
        {
            let mut g = nav.lock().unwrap();
            *g = Some(serde_json::json!({"session_id": "abc123"}));
        }
        let resp = handle_get_navigation(&nav);
        assert_eq!(resp.status, 200);
        let json: serde_json::Value = serde_json::from_str(&resp.body).unwrap();
        assert_eq!(json["command"]["session_id"], "abc123");
    }

    #[test]
    fn post_navigation_ack_clears_command() {
        let nav = test_navigation();
        {
            let mut g = nav.lock().unwrap();
            *g = Some(serde_json::json!({"session_id": "abc"}));
        }
        let resp = handle_post_navigation_ack(&nav);
        assert_eq!(resp.status, 200);
        // After ack, command should be cleared
        assert!(nav.lock().unwrap().is_none());
    }

    #[test]
    fn post_snapshot_updates_state() {
        let snap = test_snapshot();
        let body = br#"{"companion":{"state":"running","attention":[]}}"#;
        let resp = handle_post_snapshot(body, &snap);
        assert_eq!(resp.status, 200);
        let guard = snap.lock().unwrap();
        assert_eq!(guard.state, CompanionState::Running);
    }

    #[test]
    fn post_snapshot_with_approval_attention() {
        let snap = test_snapshot();
        let body = br#"{"companion":{"state":"idle","attention":[{"status":"approval","session_id":"sesh1"}]}}"#;
        let resp = handle_post_snapshot(body, &snap);
        assert_eq!(resp.status, 200);
        let guard = snap.lock().unwrap();
        assert_eq!(guard.attention.len(), 1);
        assert_eq!(guard.attention[0].status, AttentionStatus::Approval);
    }

    #[test]
    fn post_snapshot_handles_invalid_json() {
        let snap = test_snapshot();
        let body = b"not json";
        let resp = handle_post_snapshot(body, &snap);
        assert_eq!(resp.status, 200);
        let json: serde_json::Value = serde_json::from_str(&resp.body).unwrap();
        assert_eq!(json["ok"], false);
    }

    #[test]
    fn post_bubbles_visible_sets_flag() {
        let flag = test_flag(true);
        let body = br#"{"visible":false}"#;
        let resp = handle_post_bubbles_visible(body, &flag);
        assert_eq!(resp.status, 200);
        assert!(!flag.load(Ordering::SeqCst));
    }

    #[test]
    fn post_bubbles_visible_sets_true() {
        let flag = test_flag(false);
        let body = br#"{"visible":true}"#;
        handle_post_bubbles_visible(body, &flag);
        assert!(flag.load(Ordering::SeqCst));
    }

    #[test]
    fn post_open_webui_queues_navigation() {
        let nav = test_navigation();
        let body = br#"{"session_id":"test-sid"}"#;
        let resp = handle_post_open_webui(body, &nav);
        assert_eq!(resp.status, 200);
        let guard = nav.lock().unwrap();
        let cmd = guard.as_ref().unwrap();
        assert_eq!(cmd["session_id"], "test-sid");
    }

    #[test]
    fn post_open_webui_empty_session_id_no_nav() {
        let nav = test_navigation();
        let body = br#"{"session_id":""}"#;
        let _ = handle_post_open_webui(body, &nav);
        assert!(nav.lock().unwrap().is_none());
    }
}
