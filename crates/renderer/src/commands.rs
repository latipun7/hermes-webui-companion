//! Tauri command handlers for the desktop companion renderer.
//!
//! Each function is a `#[tauri::command]` invoked from the frontend
//! via `invokeTauri()` or direct HTTP fallback.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use companion_renderer::animation::{CompanionSnapshot, StateResponse};
use companion_renderer::sidecar_client::SidecarClient;
use tauri;

use crate::debug;

#[tauri::command]
pub fn get_active_pet() -> Result<serde_json::Value, String> {
    debug!("[companion:cmd] get_active_pet");
    let client = SidecarClient::new("http://127.0.0.1:17888".into());
    let pet = client.fetch_active_pet().map_err(|e| {
        debug!("[companion:cmd] get_active_pet failed: {}", e.error);
        e.error
    })?;
    debug!("[companion:cmd] active pet: slug={}", pet.slug);
    Ok(serde_json::json!({
        "slug": pet.slug,
        "spritesheet_url": pet.spritesheet_url,
        "display_name": pet.display_name,
    }))
}

#[tauri::command]
pub fn get_spritesheet(slug: String) -> Result<Vec<u8>, String> {
    debug!("[companion:cmd] get_spritesheet slug={}", slug);
    let client = SidecarClient::new("http://127.0.0.1:17888".into());
    client.fetch_spritesheet(&slug).map_err(|e| {
        debug!("[companion:cmd] get_spritesheet failed: {}", e.error);
        e.error
    })
}

#[tauri::command]
pub fn start_dragging(window: tauri::WebviewWindow) {
    let _ = window.start_dragging();
}

#[tauri::command]
pub fn open_webui() {
    let _ = std::process::Command::new("cmd")
        .args(["/c", "start", "", "http://localhost:8787"])
        .spawn();
}

#[tauri::command]
pub fn get_companion_state(
    state: tauri::State<Arc<Mutex<CompanionSnapshot>>>,
    sidecar_healthy: tauri::State<Arc<AtomicBool>>,
) -> Result<serde_json::Value, String> {
    let snap = state.lock().map_err(|e| e.to_string())?;
    let healthy = sidecar_healthy.load(Ordering::SeqCst);
    let resp = StateResponse::from_snapshot(&snap, healthy);
    serde_json::to_value(&resp).map_err(|e| e.to_string())
}

/// Show or hide the bubbles window.
/// When hidden, mouse events reach windows underneath.
#[tauri::command]
pub fn set_bubbles_visible(
    bubbles: tauri::State<tauri::WebviewWindow>,
    visible: bool,
) -> Result<(), String> {
    if visible {
        bubbles.show().map_err(|e| e.to_string())
    } else {
        bubbles.hide().map_err(|e| e.to_string())
    }
}
