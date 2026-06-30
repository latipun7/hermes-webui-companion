//! Tauri GUI binary for the desktop companion renderer.
//!
//! Thin orchestrator — wires together the bridge server, sidecar health check,
//! bubble tracking, Tauri commands, and window event handlers. All heavy logic
//! lives in sibling modules.

#![windows_subsystem = "windows"]

mod bubble;
mod commands;
mod debug;
mod health;

use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::{Arc, Mutex};

use companion_renderer::animation::{CompanionSnapshot, CompanionState};
use companion_renderer::bridge_server::{self, BridgeState};
use tauri::Manager;

use crate::bubble::{reposition_bubble, spawn_bubble_visibility_poller};
use crate::debug::{ASPECT_RATIO, debug};
use crate::health::spawn_health_check;

fn main() {
    // ── Shared state ──────────────────────────────────────────
    let companion_state = Arc::new(Mutex::new(CompanionSnapshot {
        state: CompanionState::Idle,
        attention: Vec::new(),
    }));
    let nav_command: Arc<Mutex<Option<bridge_server::NavigationCommand>>> =
        Arc::new(Mutex::new(None));
    let bubbles_visible = Arc::new(AtomicBool::new(true));
    let sidecar_healthy = spawn_health_check();
    let drag_dx = Arc::new(AtomicI32::new(0));
    let last_pos: Arc<Mutex<(i32, i32)>> = Arc::new(Mutex::new((0, 0)));

    // ── Tauri builder ─────────────────────────────────────────
    tauri::Builder::default()
        .plugin(tauri_plugin_process::init())
        .manage(companion_state.clone())
        .manage(sidecar_healthy.clone())
        .manage(drag_dx.clone())
        .invoke_handler(tauri::generate_handler![
            commands::get_active_pet,
            commands::get_spritesheet,
            commands::start_dragging,
            commands::open_webui,
            commands::get_companion_state,
            commands::set_bubbles_visible,
            commands::get_drag_dx
        ])
        .setup(move |app| {
            // Bridge server — receives WebUI snapshots
            bridge_server::spawn_bridge_server(BridgeState {
                snapshot: companion_state,
                navigation: nav_command,
                bubbles_visible: bubbles_visible.clone(),
                sidecar_healthy: sidecar_healthy.clone(),
            });

            // Windows
            let window = app
                .get_webview_window("main")
                .expect("main window not found");
            let bubbles = app
                .get_webview_window("bubbles")
                .expect("bubbles window not found");
            let _ = bubbles.show();
            app.manage(bubbles.clone());

            reposition_bubble(&window, &bubbles);
            spawn_bubble_visibility_poller(bubbles_visible, bubbles.clone());

            // ── Window events ───────────────────────────────────
            let resizing = AtomicBool::new(false);
            let win2 = window.clone();
            let drag_dx2 = drag_dx.clone();
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
                    }
                    WindowEvent::Moved(pos) => {
                        if let Ok(mut lp) = last_pos2.lock() {
                            let dx = pos.x - lp.0;
                            if dx != 0 {
                                drag_dx2.fetch_add(dx, Ordering::SeqCst);
                            }
                            *lp = (pos.x, pos.y);
                        }
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
