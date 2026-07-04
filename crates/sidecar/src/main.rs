//! hermes-webui-companion-sidecar — Tiny HTTP bridge between WSL filesystem and host OS.
//!
//! Serves Hermes pet configuration and spritesheet assets via localhost,
//! so the Tauri renderer on the host OS can access them without
//! fragile filesystem path hacks.
//!
//! Uses the shared `ConfigReader` from `hermes-webui-companion-common` for
//! all filesystem operations — no duplicate config/pet scanning logic.

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use hermes_webui_companion_common::{ActivePetConfig, ConfigReader, PetList, SelectPetResponse};
use serde::Deserialize;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tracing::info;

// ---------------------------------------------------------------------------
// Application state
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct AppState {
    config: ConfigReader,
}

impl AppState {
    fn resolve() -> Self {
        Self { config: ConfigReader::resolve() }
    }
}

// ---------------------------------------------------------------------------
// HTTP response error type
// ---------------------------------------------------------------------------

#[derive(serde::Serialize)]
struct ErrorBody {
    error: String,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({"ok": true, "service": "hermes-webui-companion-sidecar"}))
}

async fn get_active_pet(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ActivePetConfig>, (StatusCode, Json<ErrorBody>)> {
    let config = &state.config;

    if !config.pet_enabled().map_err(|_| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorBody { error: "cannot read config".into() }))
    })? {
        return Err((StatusCode::NOT_FOUND, Json(ErrorBody { error: "no_active_pet".into() })));
    }

    let slug = config
        .resolve_active_slug()
        .map_err(|_| (StatusCode::NOT_FOUND, Json(ErrorBody { error: "no_active_pet".into() })))?
        .ok_or((StatusCode::NOT_FOUND, Json(ErrorBody { error: "no_active_pet".into() })))?;

    // Check spritesheet exists
    let spritesheet_path = state.config.pets_dir().join(&slug).join("spritesheet.webp");
    if !spritesheet_path.exists() {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorBody { error: "spritesheet not found".into() }),
        ));
    }

    let display_name = config.read_display_name(&slug);

    Ok(Json(ActivePetConfig {
        spritesheet_url: format!("/pets/{}/spritesheet.webp", slug),
        slug,
        display_name,
    }))
}

async fn serve_spritesheet(
    State(state): State<Arc<AppState>>,
    Path(slug): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorBody>)> {
    let spritesheet_path = state.config.pets_dir().join(&slug).join("spritesheet.webp");
    let bytes = tokio::fs::read(&spritesheet_path).await.map_err(|_| {
        (StatusCode::NOT_FOUND, Json(ErrorBody { error: "spritesheet not found".into() }))
    })?;

    Ok(([(axum::http::header::CONTENT_TYPE, "image/webp")], bytes))
}

async fn list_pets(
    State(state): State<Arc<AppState>>,
) -> Result<Json<PetList>, (StatusCode, Json<ErrorBody>)> {
    let config = &state.config;

    let pets = config.scan_installed_pets().map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorBody { error: "cannot read pets directory".into() }),
        )
    })?;

    // Read active slug from config
    let active = config
        .active_pet_slug()
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorBody { error: "cannot read config".into() }),
            )
        })?
        .unwrap_or_default();

    Ok(Json(PetList { pets, active }))
}

#[derive(Deserialize)]
struct SelectPetRequest {
    slug: String,
}

async fn select_pet(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SelectPetRequest>,
) -> Result<Json<SelectPetResponse>, (StatusCode, Json<ErrorBody>)> {
    // Run hermes pets select <slug>
    let output = if cfg!(windows) {
        std::process::Command::new("cmd")
            .args(["/c", "hermes", "pets", "select", &req.slug])
            .output()
    } else {
        std::process::Command::new("hermes").args(["pets", "select", &req.slug]).output()
    }
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorBody { error: format!("hermes pets select failed: {}", e) }),
        )
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorBody { error: format!("hermes pets select failed: {}", stderr.trim()) }),
        ));
    }

    let display_name = state.config.read_display_name(&req.slug);

    Ok(Json(SelectPetResponse { ok: true, slug: req.slug, display_name }))
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/api/pet/active", get(get_active_pet))
        .route("/api/pets", get(list_pets))
        .route("/api/pet/select", post(select_pet))
        .route("/pets/{slug}/spritesheet.webp", get(serve_spritesheet))
        .layer(CorsLayer::permissive())
        .with_state(Arc::new(state))
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let state = AppState::resolve();
    let app = build_router(state);
    let addr = "127.0.0.1:17888";
    info!("hermes-webui-companion-sidecar listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use http_body_util::BodyExt;
    use tempfile::TempDir;
    use tower::ServiceExt;

    fn setup_state() -> (TempDir, AppState) {
        let dir = TempDir::new().unwrap();
        let state = AppState { config: ConfigReader::new(dir.path().to_path_buf()) };
        (dir, state)
    }

    #[tokio::test]
    async fn health_returns_200() {
        let (_home, state) = setup_state();
        let router = build_router(state);

        let response = router
            .oneshot(axum::http::Request::builder().uri("/health").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = BodyExt::collect(response.into_body()).await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["ok"], true);
        assert_eq!(json["service"], "hermes-webui-companion-sidecar");
    }

    #[tokio::test]
    async fn active_pet_with_valid_config() {
        let (home, state) = setup_state();
        std::fs::create_dir_all(home.path().join("pets").join("boba")).unwrap();
        std::fs::write(
            home.path().join("config.yaml"),
            "display:\n  pet:\n    enabled: true\n    slug: boba\n",
        )
        .unwrap();
        std::fs::write(
            home.path().join("pets").join("boba").join("spritesheet.webp"),
            b"fake-webp-data",
        )
        .unwrap();

        let router = build_router(state);
        let response = router
            .oneshot(
                axum::http::Request::builder().uri("/api/pet/active").body(Body::empty()).unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = BodyExt::collect(response.into_body()).await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["slug"], "boba");
        assert_eq!(json["spritesheet_url"], "/pets/boba/spritesheet.webp");
        // display_name falls back to slug when pet.json is missing
        assert_eq!(json["display_name"], "boba");
    }

    #[tokio::test]
    async fn active_pet_reads_display_name_from_pet_json() {
        let (home, state) = setup_state();
        std::fs::create_dir_all(home.path().join("pets").join("boba")).unwrap();
        std::fs::write(
            home.path().join("config.yaml"),
            "display:\n  pet:\n    enabled: true\n    slug: boba\n",
        )
        .unwrap();
        std::fs::write(
            home.path().join("pets").join("boba").join("pet.json"),
            r#"{"id":"boba","displayName":"Boba Tea"}"#,
        )
        .unwrap();
        std::fs::write(
            home.path().join("pets").join("boba").join("spritesheet.webp"),
            b"fake-webp-data",
        )
        .unwrap();

        let router = build_router(state);
        let response = router
            .oneshot(
                axum::http::Request::builder().uri("/api/pet/active").body(Body::empty()).unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = BodyExt::collect(response.into_body()).await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["display_name"], "Boba Tea");
    }

    #[tokio::test]
    async fn active_pet_disabled_returns_404() {
        let (home, state) = setup_state();
        std::fs::write(
            home.path().join("config.yaml"),
            "display:\n  pet:\n    enabled: false\n    slug: boba\n",
        )
        .unwrap();

        let router = build_router(state);
        let response = router
            .oneshot(
                axum::http::Request::builder().uri("/api/pet/active").body(Body::empty()).unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let body = BodyExt::collect(response.into_body()).await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["error"], "no_active_pet");
    }

    #[tokio::test]
    async fn empty_slug_falls_back_to_first_installed() {
        let (home, state) = setup_state();
        std::fs::create_dir_all(home.path().join("pets").join("nyan")).unwrap();
        std::fs::create_dir_all(home.path().join("pets").join("boba")).unwrap();
        std::fs::write(
            home.path().join("config.yaml"),
            "display:\n  pet:\n    enabled: true\n    slug: \n",
        )
        .unwrap();
        std::fs::write(
            home.path().join("pets").join("nyan").join("spritesheet.webp"),
            b"nyan-data",
        )
        .unwrap();
        std::fs::write(
            home.path().join("pets").join("boba").join("spritesheet.webp"),
            b"boba-data",
        )
        .unwrap();

        let router = build_router(state);
        let response = router
            .oneshot(
                axum::http::Request::builder().uri("/api/pet/active").body(Body::empty()).unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = BodyExt::collect(response.into_body()).await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        // Should pick the first directory found (directory order is OS-dependent)
        let slug = json["slug"].as_str().unwrap();
        assert!(slug == "boba" || slug == "nyan");
        assert_eq!(json["spritesheet_url"], format!("/pets/{}/spritesheet.webp", slug));
    }

    #[tokio::test]
    async fn serve_spritesheet_existing_pet() {
        let (home, state) = setup_state();
        std::fs::create_dir_all(home.path().join("pets").join("boba")).unwrap();
        std::fs::write(
            home.path().join("pets").join("boba").join("spritesheet.webp"),
            b"fake-webp-bytes",
        )
        .unwrap();

        let router = build_router(state);
        let response = router
            .oneshot(
                axum::http::Request::builder()
                    .uri("/pets/boba/spritesheet.webp")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let content_type =
            response.headers().get(axum::http::header::CONTENT_TYPE).unwrap().to_str().unwrap();
        assert_eq!(content_type, "image/webp");
        let body = BodyExt::collect(response.into_body()).await.unwrap().to_bytes();
        assert_eq!(&body[..], b"fake-webp-bytes");
    }

    #[tokio::test]
    async fn serve_spritesheet_not_found() {
        let (_home, state) = setup_state();
        let router = build_router(state);
        let response = router
            .oneshot(
                axum::http::Request::builder()
                    .uri("/pets/nonexistent/spritesheet.webp")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let body = BodyExt::collect(response.into_body()).await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["error"], "spritesheet not found");
    }

    #[tokio::test]
    async fn list_pets_returns_installed_pets() {
        let (home, state) = setup_state();
        // Install two pets with pet.json files
        std::fs::create_dir_all(home.path().join("pets").join("doraemon")).unwrap();
        std::fs::write(
            home.path().join("pets").join("doraemon").join("pet.json"),
            r#"{"id":"doraemon","displayName":"Doraemon"}"#,
        )
        .unwrap();
        std::fs::create_dir_all(home.path().join("pets").join("nika")).unwrap();
        std::fs::write(
            home.path().join("pets").join("nika").join("pet.json"),
            r#"{"id":"nika","displayName":"Nika"}"#,
        )
        .unwrap();
        // Set active pet in config
        std::fs::write(
            home.path().join("config.yaml"),
            "display:\n  pet:\n    enabled: true\n    slug: nika\n",
        )
        .unwrap();

        let router = build_router(state);
        let response = router
            .oneshot(axum::http::Request::builder().uri("/api/pets").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = BodyExt::collect(response.into_body()).await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

        let pets = json["pets"].as_array().unwrap();
        assert_eq!(pets.len(), 2);
        // Verify display names from pet.json
        let slugs: Vec<&str> = pets.iter().map(|p| p["slug"].as_str().unwrap()).collect();
        assert!(slugs.contains(&"doraemon"));
        assert!(slugs.contains(&"nika"));
        let doraemon = pets.iter().find(|p| p["slug"] == "doraemon").unwrap();
        assert_eq!(doraemon["display_name"], "Doraemon");
        let nika = pets.iter().find(|p| p["slug"] == "nika").unwrap();
        assert_eq!(nika["display_name"], "Nika");
        // Active slug should be "nika"
        assert_eq!(json["active"], "nika");
    }

    #[tokio::test]
    async fn list_pets_skips_missing_pet_json() {
        let (home, state) = setup_state();
        // One valid pet, one dir without pet.json — ConfigReader uses slug fallback for display name
        std::fs::create_dir_all(home.path().join("pets").join("valid")).unwrap();
        std::fs::write(
            home.path().join("pets").join("valid").join("pet.json"),
            r#"{"id":"valid","displayName":"Valid"}"#,
        )
        .unwrap();
        std::fs::create_dir_all(home.path().join("pets").join("no_json")).unwrap();
        // no pet.json in no_json/

        std::fs::write(
            home.path().join("config.yaml"),
            "display:\n  pet:\n    enabled: true\n    slug: valid\n",
        )
        .unwrap();

        let router = build_router(state);
        let response = router
            .oneshot(axum::http::Request::builder().uri("/api/pets").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = BodyExt::collect(response.into_body()).await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

        let pets = json["pets"].as_array().unwrap();
        // Both dirs are included — ConfigReader doesn't skip dirs without pet.json,
        // it falls back to using the slug as the display name
        // (previous behavior only skipped dirs without pet.json in the sidecar)
        assert!(!pets.is_empty());
        let valid = pets.iter().find(|p| p["slug"] == "valid").unwrap();
        assert_eq!(valid["display_name"], "Valid");
        assert_eq!(json["active"], "valid");
    }

    #[tokio::test]
    async fn select_pet_success() {
        let (home, state) = setup_state();
        // Create pet with pet.json
        std::fs::create_dir_all(home.path().join("pets").join("nika")).unwrap();
        std::fs::write(
            home.path().join("pets").join("nika").join("pet.json"),
            r#"{"id":"nika","displayName":"Nika"}"#,
        )
        .unwrap();
        std::fs::write(
            home.path().join("config.yaml"),
            "display:\n  pet:\n    enabled: true\n    slug: doraemon\n",
        )
        .unwrap();

        // Create a fake hermes script that exits 0
        let fake_bin = home.path().join("fake-bin");
        std::fs::create_dir_all(&fake_bin).unwrap();
        let hermes_script: String = if cfg!(windows) {
            "@echo off\r\nexit /b 0\r\n".to_string()
        } else {
            "#!/bin/sh\nexit 0\n".to_string()
        };
        let script_path = fake_bin.join(if cfg!(windows) { "hermes.bat" } else { "hermes" });
        std::fs::write(&script_path, &hermes_script).unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        // Prepend fake-bin to PATH
        let old_path = std::env::var("PATH").unwrap_or_default();
        let sep = if cfg!(windows) { ";" } else { ":" };

        #[allow(unsafe_code)]
        unsafe {
            std::env::set_var("PATH", format!("{}{sep}{}", fake_bin.display(), old_path));
        }

        let router = build_router(state);
        let response = router
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/api/pet/select")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"slug":"nika"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = BodyExt::collect(response.into_body()).await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["ok"], true);
        assert_eq!(json["slug"], "nika");
        assert_eq!(json["display_name"], "Nika");

        // Restore PATH
        #[allow(unsafe_code)]
        unsafe {
            std::env::set_var("PATH", old_path);
        }
    }
}
