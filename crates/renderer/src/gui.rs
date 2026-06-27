//! Tauri GUI binary for the desktop companion renderer.
//!
//! Build with: `cargo build --features gui -p hermes-webui-companion-renderer`
//! (requires Windows with WebView2, or Linux with webkit2gtk-4.1)

use companion_renderer::sidecar_client::SidecarClient;

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

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![get_active_pet, get_spritesheet])
        .run(tauri::generate_context!())
        .expect("failed to run companion GUI");
}
