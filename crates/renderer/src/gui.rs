//! Tauri GUI binary for the desktop companion renderer.
//!
//! Build with: `cargo build --features gui -p hermes-webui-companion-renderer`
//! (requires Windows with WebView2, or Linux with webkit2gtk-4.1)

use std::sync::atomic::{AtomicBool, Ordering};

use companion_renderer::sidecar_client::SidecarClient;

/// Spritesheet frame aspect ratio: 192 / 208.
const ASPECT_RATIO: f64 = 192.0 / 208.0;

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

// ---------------------------------------------------------------------------
// Window setup
// ---------------------------------------------------------------------------

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            get_active_pet,
            get_spritesheet,
            start_dragging
        ])
        .setup(|app| {
            let window = app.get_webview_window("main").expect("main window not found");
            let resizing = AtomicBool::new(false);

            window.on_window_event(move |event| {
                use tauri::WindowEvent;
                if let WindowEvent::Resized(size) = event {
                    // Guard against recursive resize loops
                    if resizing.swap(true, Ordering::SeqCst) {
                        return;
                    }

                    // Lock to spritesheet aspect ratio: width drives height
                    let new_height = (size.width as f64 / ASPECT_RATIO).round() as u32;
                    if (new_height as f64 - size.height as f64).abs() > 1.0 {
                        let _ = window.set_size(tauri::Size::Physical(
                            tauri::PhysicalSize::new(size.width, new_height),
                        ));
                    }

                    resizing.store(false, Ordering::SeqCst);
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("failed to run companion GUI");
}
