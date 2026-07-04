//! Tauri command handlers for the desktop companion renderer.
//!
//! Each function is a `#[tauri::command]` invoked from the frontend
//! via `invokeTauri()` over Tauri IPC.

use std::sync::Arc;
use std::sync::atomic::Ordering;

use companion_renderer::PetDataProvider;
use companion_renderer::animation::StateResponse;
use tauri::Manager;
use tauri::menu::{CheckMenuItemBuilder, MenuBuilder, MenuItemBuilder, SubmenuBuilder};

use crate::CompanionContext;
use crate::debug;

#[tauri::command]
pub fn get_active_pet(
    provider: tauri::State<'_, Arc<dyn PetDataProvider + Send + Sync>>,
) -> Result<serde_json::Value, String> {
    debug!("[companion:cmd] get_active_pet");
    let pet = provider.fetch_active_pet().map_err(|e| {
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
pub fn get_spritesheet(
    slug: String,
    provider: tauri::State<'_, Arc<dyn PetDataProvider + Send + Sync>>,
) -> Result<Vec<u8>, String> {
    debug!("[companion:cmd] get_spritesheet slug={}", slug);
    provider.fetch_spritesheet(&slug).map_err(|e| {
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
        let _ = std::process::Command::new("open").arg("http://localhost:8787").spawn();
    }
    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("xdg-open").arg("http://localhost:8787").spawn();
    }
}

#[tauri::command]
pub fn get_companion_state(
    ctx: tauri::State<'_, Arc<CompanionContext>>,
) -> Result<serde_json::Value, String> {
    let snap = ctx.snapshot.lock().map_err(|e| e.to_string())?;
    let healthy = ctx.all_healthy.load(Ordering::SeqCst);
    let resp = StateResponse::from_snapshot(&snap, healthy);
    serde_json::to_value(&resp).map_err(|e| e.to_string())
}

/// Show or hide the bubbles window.
/// When hidden, mouse events reach windows underneath.
#[tauri::command]
pub fn set_bubbles_visible(
    bubbles: tauri::State<'_, tauri::WebviewWindow>,
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
pub fn get_drag_dx(ctx: tauri::State<'_, Arc<CompanionContext>>) -> Result<i32, String> {
    Ok(ctx.drag_dx.swap(0, Ordering::SeqCst))
}

/// Quit the entire application — both pet and bubbles windows.
#[tauri::command]
pub fn close_pet(app: tauri::AppHandle) {
    app.exit(0);
}

/// Restart the Tauri process, reloading the pet and reconnecting.
#[tauri::command]
pub fn restart_pet(app: tauri::AppHandle) {
    app.restart();
}

/// Switch to a new pet — called from on_menu_event when user clicks a pet in the submenu.
/// This is NOT a #[tauri::command] (called internally from gui.rs menu handler).
pub fn switch_pet_inner(app: tauri::AppHandle, slug: String) -> Result<(), String> {
    let provider = app.state::<Arc<dyn PetDataProvider + Send + Sync>>();

    // 1. Select the new pet via provider (hermes CLI or sidecar)
    provider.select_pet(&slug).map_err(|e| e.error)?;

    // 2. Verify the spritesheet is fetchable (preload check)
    provider.fetch_spritesheet(&slug).map_err(|e| e.error)?;

    // Frontend will detect the slug change on next poll and reload in-place.
    // No app restart needed — see ADR-003.
    Ok(())
}

/// Build the native context menu with Switch pet submenu, Restart pet, and Close pet.
fn build_context_menu(
    app: &tauri::AppHandle,
) -> Result<tauri::menu::Menu<tauri::Wry>, tauri::Error> {
    let restart = MenuItemBuilder::with_id("restart", "Restart pet").build(app)?;
    let close = MenuItemBuilder::with_id("close", "Close pet").build(app)?;

    let provider = app.state::<Arc<dyn PetDataProvider + Send + Sync>>();
    let switch_submenu = build_switch_submenu(app, &**provider);

    let mut menu_builder = MenuBuilder::new(app);
    if let Ok(submenu) = switch_submenu {
        menu_builder = menu_builder.item(&submenu);
    } else {
        // Provider unreachable — show disabled placeholder
        let disabled = MenuItemBuilder::with_id("switch_unavailable", "Switch pet (unavailable)")
            .enabled(false)
            .build(app)?;
        menu_builder = menu_builder.item(&disabled);
    }
    menu_builder.separator().item(&restart).item(&close).build()
}

/// Build the Switch pet submenu by fetching installed pets from the provider.
fn build_switch_submenu(
    app: &tauri::AppHandle,
    provider: &(dyn PetDataProvider + Send + Sync),
) -> Result<tauri::menu::Submenu<tauri::Wry>, tauri::Error> {
    let pet_list = provider
        .fetch_pets()
        .map_err(|_| tauri::Error::from(std::io::Error::other("provider unreachable")))?;

    let mut submenu = SubmenuBuilder::new(app, "Switch pet");
    for pet in &pet_list.pets {
        let id = format!("switch:{}", pet.slug);
        let is_active = pet.slug == pet_list.active;
        let item =
            CheckMenuItemBuilder::with_id(id, &pet.display_name).checked(is_active).build(app)?;
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
    window.popup_menu(&menu).map_err(|e| e.to_string())
}
