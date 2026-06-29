//! Tauri GUI binary for the desktop companion renderer.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use companion_renderer::animation::{CompanionSnapshot, CompanionState, StateResponse};
use companion_renderer::bridge_server::{self, BridgeState};
use companion_renderer::sidecar_client::SidecarClient;
use tauri::Manager;

const ASPECT_RATIO: f64 = 192.0 / 208.0;

/// Log only when HERMES_COMPANION_DEBUG=1.
macro_rules! debug {
    ($($arg:tt)*) => {
        if std::env::var("HERMES_COMPANION_DEBUG").unwrap_or_default() == "1" {
            eprintln!($($arg)*);
        }
    };
}

fn reposition_bubble(pet: &tauri::WebviewWindow, bubble: &tauri::WebviewWindow) {
    let Ok(pet_pos) = pet.outer_position() else { return };
    let Ok(pet_size) = pet.outer_size() else { return };

    let bubble_w = 320i32;
    let bubble_h = 84i32;
    let mut x = pet_pos.x + (pet_size.width as i32 - bubble_w) / 2;
    let mut y = pet_pos.y.saturating_sub(bubble_h);

    if let Ok(Some(monitor)) = pet.current_monitor() {
        let m = monitor.position();
        let ms = monitor.size();
        if y < m.y {
            y = pet_pos.y + pet_size.height as i32;
        }
        let right_edge = x + bubble_w;
        if right_edge > m.x + ms.width as i32 {
            x = m.x + ms.width as i32 - bubble_w;
        }
        if x < m.x {
            x = m.x;
        }
    }

    let _ = bubble.set_position(tauri::Position::Physical(tauri::PhysicalPosition::new(x, y)));
}

#[tauri::command]
fn get_active_pet() -> Result<serde_json::Value, String> {
    let client = SidecarClient::new("http://127.0.0.1:17888".into());
    let pet = client.fetch_active_pet().map_err(|e| e.error)?;
    Ok(serde_json::json!({
        "slug": pet.slug,
        "spritesheet_url": pet.spritesheet_url,
        "display_name": pet.display_name,
    }))
}

#[tauri::command]
fn get_spritesheet(slug: String) -> Result<Vec<u8>, String> {
    let client = SidecarClient::new("http://127.0.0.1:17888".into());
    client.fetch_spritesheet(&slug).map_err(|e| e.error)
}

#[tauri::command]
fn start_dragging(window: tauri::WebviewWindow) {
    let _ = window.start_dragging();
}

#[tauri::command]
fn open_webui() {
    let _ = std::process::Command::new("cmd")
        .args(["/c", "start", "", "http://localhost:8787"])
        .spawn();
}

#[tauri::command]
fn get_companion_state(
    state: tauri::State<Arc<Mutex<CompanionSnapshot>>>,
) -> Result<serde_json::Value, String> {
    let snap = state.lock().map_err(|e| e.to_string())?;
    let resp = StateResponse::from(&*snap);
    serde_json::to_value(&resp).map_err(|e| e.to_string())
}

/// Show or hide the bubbles window.
/// When hidden, mouse events reach windows underneath.
#[tauri::command]
fn set_bubbles_visible(
    bubbles: tauri::State<tauri::WebviewWindow>,
    visible: bool,
) -> Result<(), String> {
    if visible {
        bubbles.show().map_err(|e| e.to_string())
    } else {
        bubbles.hide().map_err(|e| e.to_string())
    }
}

fn main() {
    let companion_state = Arc::new(Mutex::new(CompanionSnapshot {
        state: CompanionState::Idle,
        attention: Vec::new(),
    }));
    let nav_command: Arc<Mutex<Option<bridge_server::NavigationCommand>>> =
        Arc::new(Mutex::new(None));
    let bubbles_visible = Arc::new(AtomicBool::new(true));

    // ── Sidecar health check thread ──────────────────────────────
    // Periodically probes the sidecar (:17888). If unreachable,
    // sets companion state to Failed so the pet shows the failed animation.
    {
        let state_for_health = companion_state.clone();
        std::thread::spawn(move || loop {
            std::thread::sleep(std::time::Duration::from_secs(10));
            let healthy = std::net::TcpStream::connect_timeout(
                &"127.0.0.1:17888".parse().unwrap(),
                std::time::Duration::from_secs(2),
            )
            .is_ok();
            if let Ok(mut guard) = state_for_health.lock() {
                if !healthy && guard.state != CompanionState::Failed {
                    debug!("[companion] sidecar unreachable → Failed");
                    guard.state = CompanionState::Failed;
                } else if healthy && guard.state == CompanionState::Failed {
                    debug!("[companion] sidecar recovered → Idle");
                    guard.state = CompanionState::Idle;
                }
            }
        });
    }

    tauri::Builder::default()
        .manage(companion_state.clone())
        .invoke_handler(tauri::generate_handler![
            get_active_pet,
            get_spritesheet,
            start_dragging,
            open_webui,
            get_companion_state,
            set_bubbles_visible
        ])
        .setup(move |app| {
            bridge_server::spawn_bridge_server(BridgeState {
                snapshot: companion_state,
                navigation: nav_command,
                bubbles_visible: bubbles_visible.clone(),
            });

            let window = app
                .get_webview_window("main")
                .expect("main window not found");
            let _win = window.clone();
            let bubbles = app
                .get_webview_window("bubbles")
                .expect("bubbles window not found");
            let _ = bubbles.show();
            app.manage(bubbles.clone());
            // bubbles.js will auto-hide on first poll if no content

            reposition_bubble(&window, &bubbles);

            // ── Bubble visibility polling thread ──────────────────
            // Reads the flag set by POST /api/bubbles/visible and
            // shows/hides the bubbles window accordingly.
            {
                let bv = bubbles_visible.clone();
                let bw = bubbles.clone();
                let mut was_visible = true;
                std::thread::spawn(move || loop {
                    std::thread::sleep(std::time::Duration::from_millis(250));
                    let want = bv.load(Ordering::SeqCst);
                    if want != was_visible {
                        was_visible = want;
                        if want {
                            let _ = bw.show();
                        } else {
                            let _ = bw.hide();
                        }
                        debug!("[companion] bubbles window visible = {want}");
                    }
                });
            }

            let resizing = AtomicBool::new(false);
            let win2 = window.clone();

            window.on_window_event(move |event| {
                use tauri::WindowEvent;
                match event {
                    WindowEvent::Resized(size) => {
                        if resizing.swap(true, Ordering::SeqCst) {
                            return;
                        }
                        let new_height = (size.width as f64 / ASPECT_RATIO).round() as u32;
                        if (new_height as f64 - size.height as f64).abs() > 1.0 {
                            let _ = win2.set_size(tauri::Size::Physical(
                                tauri::PhysicalSize::new(size.width, new_height),
                            ));
                        }
                        reposition_bubble(&win2, &bubbles);
                        resizing.store(false, Ordering::SeqCst);
                    }
                    WindowEvent::Moved(_pos) => {
                        reposition_bubble(&win2, &bubbles);
                    }
                    _ => {}
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("failed to run companion GUI");
}
