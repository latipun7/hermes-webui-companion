//! Tauri GUI binary for the desktop companion renderer.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use companion_renderer::animation::CompanionSnapshot;
use companion_renderer::bridge_server;
use companion_renderer::sidecar_client::SidecarClient;
use tauri::Manager;
/// Spritesheet frame aspect ratio: 192 / 208.
const ASPECT_RATIO: f64 = 192.0 / 208.0;

/// Position the bubble window above the pet, clamped to monitor edges.
fn reposition_bubble(pet: &tauri::WebviewWindow, bubble: &tauri::WebviewWindow) {
    let Ok(pet_pos) = pet.outer_position() else { return };
    let Ok(pet_size) = pet.outer_size() else { return };

    // Ideal position: centered above pet, offset left by 10px
    let bubble_w = 320i32;
    let bubble_h = 44i32;
    let mut x = pet_pos.x + 10;
    let mut y = pet_pos.y.saturating_sub(bubble_h);

    // Clamp to current monitor
    if let Ok(Some(monitor)) = pet.current_monitor() {
        let m = monitor.position();
        let ms = monitor.size();

        // If bubble would go above monitor, flip below pet
        if y < m.y {
            y = pet_pos.y + pet_size.height as i32;
        }

        // Clamp horizontally — keep within monitor edges
        let right_edge = x + bubble_w;
        if right_edge > m.x + ms.width as i32 {
            x = m.x + ms.width as i32 - bubble_w;
        }
        if x < m.x {
            x = m.x;
        }
    }

    let _ = bubble.set_position(tauri::Position::Physical(
        tauri::PhysicalPosition::new(x, y),
    ));
}

// ---------------------------------------------------------------------------
// Tauri commands
// ---------------------------------------------------------------------------

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
    let _ = open::that("http://localhost:8787");
}

#[tauri::command]
fn get_companion_state(
    state: tauri::State<Arc<Mutex<CompanionSnapshot>>>,
) -> Result<serde_json::Value, String> {
    let snap = state.lock().map_err(|e| e.to_string())?;
    serde_json::to_value(&*snap).map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// Window setup
// ---------------------------------------------------------------------------

fn main() {
    let companion_state = Arc::new(Mutex::new(CompanionSnapshot {
        state: companion_renderer::animation::CompanionState::Idle,
        attention: vec![],
    }));

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(companion_state.clone())
        .invoke_handler(tauri::generate_handler![
            get_active_pet,
            get_spritesheet,
            start_dragging,
            open_webui,
            get_companion_state
        ])
        .setup(move |app| {
            // Spawn bridge HTTP server — updates companion state on POST
            bridge_server::spawn_bridge_server(companion_state);

            // Aspect-ratio-locked window
            let window = app
                .get_webview_window("main")
                .expect("main window not found");
            let _win = window.clone();
            let bubbles = app
                .get_webview_window("bubbles")
                .expect("bubbles window not found");
            let _ = bubbles.show();

            // Initial position sync
            reposition_bubble(&window, &bubbles);
            let resizing = AtomicBool::new(false);

            // Position bubbles above pet window on move/resize
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
                        // Reposition bubbles on resize too
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
