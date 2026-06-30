//! Tauri command handlers for the desktop companion renderer.
//!
//! Each function is a `#[tauri::command]` invoked from the frontend
//! via `invokeTauri()` over Tauri IPC.

use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::{Arc, Mutex};

use companion_renderer::animation::{CompanionSnapshot, StateResponse};
use companion_renderer::sidecar_client::SidecarClient;
use tauri;
use tauri::menu::{MenuBuilder, MenuItemBuilder, SubmenuBuilder};

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
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("cmd")
            .args(["/c", "start", "", "http://localhost:8787"])
            .spawn();
    }
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open")
            .arg("http://localhost:8787")
            .spawn();
    }
    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("xdg-open")
            .arg("http://localhost:8787")
            .spawn();
    }
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

/// Return and reset the drag delta X since last call.
/// Positive = dragging right, negative = dragging left.
#[tauri::command]
pub fn get_drag_dx(
    drag_dx: tauri::State<Arc<AtomicI32>>,
) -> Result<i32, String> {
    Ok(drag_dx.swap(0, Ordering::SeqCst))
}

/// Quit the entire application — both pet and bubbles windows.
#[tauri::command]
pub fn close_pet(app: tauri::AppHandle) {
    app.exit(0);
}

/// Restart the Tauri process, reloading the pet and reconnecting to sidecar.
#[tauri::command]
pub fn restart_pet(app: tauri::AppHandle) {
    app.restart();
}

/// Switch to a new pet — called from on_menu_event when user clicks a pet in the submenu.
/// This is NOT a #[tauri::command] (called internally from gui.rs menu handler).
pub fn switch_pet_inner(app: tauri::AppHandle, slug: String) -> Result<(), String> {
    let client = SidecarClient::new("http://127.0.0.1:17888".into());

    // 1. Select the new pet via sidecar (runs hermes pets select)
    client.select_pet(&slug).map_err(|e| e.error)?;

    // 2. Fetch the new spritesheet
    let _spritesheet = client.fetch_spritesheet(&slug).map_err(|e| e.error)?;

    // 3. Restart the app to reload spritesheet with new active pet
    app.restart();

    Ok(())
}

/// Build the native context menu with Switch pet submenu, Restart pet, and Close pet.
fn build_context_menu(
    app: &tauri::AppHandle,
) -> Result<tauri::menu::Menu<tauri::Wry>, tauri::Error> {
    let restart = MenuItemBuilder::with_id("restart", "Restart pet").build(app)?;
    let close = MenuItemBuilder::with_id("close", "Close pet").build(app)?;

    // Build Switch pet submenu from sidecar
    let switch_submenu = build_switch_submenu(app);

    let mut menu_builder = MenuBuilder::new(app);
    if let Ok(submenu) = switch_submenu {
        menu_builder = menu_builder.item(&submenu);
    } else {
        // Sidecar unreachable — show disabled placeholder
        let disabled = MenuItemBuilder::with_id("switch_unavailable", "Switch pet (unavailable)")
            .enabled(false)
            .build(app)?;
        menu_builder = menu_builder.item(&disabled);
    }
    menu_builder
        .separator()
        .item(&restart)
        .item(&close)
        .build()
}

/// Build the Switch pet submenu by fetching installed pets from the sidecar.
fn build_switch_submenu(
    app: &tauri::AppHandle,
) -> Result<tauri::menu::Submenu<tauri::Wry>, tauri::Error> {
    let client = SidecarClient::new("http://127.0.0.1:17888".into());
    let pet_list = client
        .fetch_pets()
        .map_err(|_| tauri::Error::from(std::io::Error::new(std::io::ErrorKind::Other, "sidecar unreachable")))?;

    let mut submenu = SubmenuBuilder::new(app, "Switch pet");
    for pet in &pet_list.pets {
        let label = if pet.slug == pet_list.active {
            format!("✓ {}", pet.display_name)
        } else {
            format!("  {}", pet.display_name)
        };
        let item = MenuItemBuilder::with_id(format!("switch:{}", pet.slug), label)
            .build(app)?;
        submenu = submenu.item(&item);
    }

    submenu.build()
}

/// Show the native context menu as a popup at the cursor position.
#[tauri::command]
pub fn show_context_menu(
    window: tauri::WebviewWindow,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let menu = build_context_menu(&app).map_err(|e| e.to_string())?;
    window
        .popup_menu(&menu)
        .map_err(|e| e.to_string())
}
