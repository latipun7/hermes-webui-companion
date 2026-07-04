//! Tauri GUI binary for the desktop companion renderer.
//!
//! Thin orchestrator — wires together the bridge server, pet data provider,
//! health check, bubble tracking, Tauri commands, and window event handlers.
//! All heavy logic lives in sibling modules.

#![windows_subsystem = "windows"]

mod bubble;
mod commands;
mod debug;
mod health;

use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::{Arc, Mutex};

use companion_renderer::PetDataProvider;
use companion_renderer::animation::{CompanionSnapshot, CompanionState};
use companion_renderer::bridge_server::{self, BridgeState};
use companion_renderer::common::ConfigReader;
use companion_renderer::direct_client::DirectClient;
use companion_renderer::sidecar_client::SidecarClient;
use tauri::Manager;

use crate::bubble::{reposition_bubble, spawn_bubble_visibility_poller};
use crate::debug::{ASPECT_RATIO, debug};

// ---------------------------------------------------------------------------
// CompanionContext — single managed state for the Tauri app
// ---------------------------------------------------------------------------

/// Groups the shared state previously scattered across individual
/// `app.manage()` calls. One struct, one managed type.
pub struct CompanionContext {
    pub snapshot: Arc<Mutex<CompanionSnapshot>>,
    pub nav_command: Arc<Mutex<Option<bridge_server::NavigationCommand>>>,
    pub all_healthy: Arc<AtomicBool>,
    pub drag_dx: Arc<AtomicI32>,
    pub bubbles_visible: Arc<AtomicBool>,
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Auto-detect which pet data source to use: DirectClient if HERMES_HOME is
/// readable, otherwise SidecarClient (WSL).
fn detect_provider() -> Arc<dyn PetDataProvider + Send + Sync> {
    let direct = DirectClient::new(None);
    if direct.is_available() {
        debug!("[companion] direct mode — reading from {:?}", ConfigReader::default_hermes_home());
        Arc::new(direct)
    } else {
        debug!("[companion] sidecar mode — connecting to :17888");
        Arc::new(SidecarClient::new("http://127.0.0.1:17888".into()))
    }
}

fn main() {
    // ── Shared state ──────────────────────────────────────────
    let ctx = Arc::new(CompanionContext {
        snapshot: Arc::new(Mutex::new(CompanionSnapshot {
            state: CompanionState::Idle,
            attention: Vec::new(),
        })),
        nav_command: Arc::new(Mutex::new(None)),
        bubbles_visible: Arc::new(AtomicBool::new(true)),
        all_healthy: Arc::new(AtomicBool::new(false)),
        drag_dx: Arc::new(AtomicI32::new(0)),
    });

    // Auto-detect mode and instantiate the pet data provider
    let provider = detect_provider();

    // Start health checking — updates ctx.all_healthy directly
    health::spawn_health_check(provider.clone(), ctx.all_healthy.clone());

    let last_pos: Arc<Mutex<(i32, i32)>> = Arc::new(Mutex::new((0, 0)));

    // ── Tauri builder ─────────────────────────────────────────
    tauri::Builder::default()
        .plugin(tauri_plugin_process::init())
        .manage(ctx.clone())
        .manage(provider.clone())
        .invoke_handler(tauri::generate_handler![
            commands::get_active_pet,
            commands::get_spritesheet,
            commands::start_dragging,
            commands::open_webui,
            commands::get_companion_state,
            commands::set_bubbles_visible,
            commands::get_drag_dx,
            commands::close_pet,
            commands::restart_pet,
            commands::show_context_menu
        ])
        .on_menu_event(|app, event| match event.id().as_ref() {
            "restart" => app.restart(),
            "close" => app.exit(0),
            id if id.starts_with("switch:") => {
                let slug = id.strip_prefix("switch:").unwrap_or("");
                if let Err(e) = commands::switch_pet_inner(app.clone(), slug.to_string()) {
                    eprintln!("[companion] switch pet failed: {}", e);
                }
            },
            _ => {},
        })
        .setup(move |app| {
            // Bridge server — receives WebUI snapshots
            bridge_server::spawn_bridge_server(BridgeState {
                snapshot: ctx.snapshot.clone(),
                navigation: ctx.nav_command.clone(),
                bubbles_visible: ctx.bubbles_visible.clone(),
                all_healthy: ctx.all_healthy.clone(),
            });

            // Windows
            let window = app.get_webview_window("main").expect("main window not found");
            let bubbles = app.get_webview_window("bubbles").expect("bubbles window not found");
            let _ = bubbles.show();
            app.manage(bubbles.clone());

            reposition_bubble(&window, &bubbles);
            spawn_bubble_visibility_poller(ctx.bubbles_visible.clone(), bubbles.clone());

            // ── Window events ───────────────────────────────────
            let resizing = AtomicBool::new(false);
            let win2 = window.clone();
            let drag_dx2 = ctx.drag_dx.clone();
            let last_pos2 = last_pos.clone();

            window.on_window_event(move |event| {
                use tauri::WindowEvent;
                match event {
                    WindowEvent::Resized(size) => {
                        if resizing.swap(true, Ordering::SeqCst) {
                            return;
                        }
                        let new_height = (size.width as f64 / ASPECT_RATIO).round() as u32;
                        if (new_height as f64 - size.height as f64).abs() > 1.0 {
                            let _ = win2.set_size(tauri::Size::Physical(tauri::PhysicalSize::new(
                                size.width, new_height,
                            )));
                        }
                        reposition_bubble(&win2, &bubbles);
                        resizing.store(false, Ordering::SeqCst);
                    },
                    WindowEvent::Moved(pos) => {
                        if let Ok(mut lp) = last_pos2.lock() {
                            let dx = pos.x - lp.0;
                            if dx != 0 {
                                drag_dx2.fetch_add(dx, Ordering::SeqCst);
                            }
                            *lp = (pos.x, pos.y);
                        }
                        reposition_bubble(&win2, &bubbles);
                    },
                    _ => {},
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("failed to run companion GUI");
}
