//! hermes-webui-companion-sidecar — Tiny HTTP bridge between WSL filesystem and Windows host.
//!
//! Serves Hermes pet configuration and spritesheet assets via localhost,
//! so the Tauri renderer on the Windows host can access them without
//! fragile `\\wsl$` path hacks.

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tracing::info;

// ---------------------------------------------------------------------------
// Application state
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct AppState {
    hermes_home: PathBuf,
}

impl AppState {
    fn resolve() -> Self {
        let home = std::env::var("HERMES_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| dirs::home_dir().unwrap().join(".hermes"));
        Self { hermes_home: home }
    }

    fn pets_dir(&self) -> PathBuf {
        self.hermes_home.join("pets")
    }

    fn config_path(&self) -> PathBuf {
        self.hermes_home.join("config.yaml")
    }
}

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct ActivePet {
    slug: String,
    spritesheet_url: String,
    display_name: String,
}

#[derive(Serialize)]
struct Health {
    ok: bool,
    service: &'static str,
}

#[derive(Serialize)]
struct ErrorBody {
    error: String,
}

#[derive(Serialize)]
struct PetEntry {
    slug: String,
    display_name: String,
}

#[derive(Serialize)]
struct PetList {
    pets: Vec<PetEntry>,
    active: String,
}

#[derive(Deserialize)]
struct SelectPetRequest {
    slug: String,
}

#[derive(Serialize)]
struct SelectPetResponse {
    ok: bool,
    slug: String,
    display_name: String,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn health() -> Json<Health> {
    Json(Health {
        ok: true,
        service: "hermes-webui-companion-sidecar",
    })
}

async fn get_active_pet(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ActivePet>, (StatusCode, Json<ErrorBody>)> {
    let config: serde_yaml_ng::Value = serde_yaml_ng::from_str(
        &tokio::fs::read_to_string(state.config_path())
            .await
            .map_err(|_| {
                (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorBody {
                    error: "cannot read config".into(),
                }))
            })?,
    )
    .map_err(|_| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorBody {
            error: "invalid config".into(),
        }))
    })?;

    let pet_enabled = config
        .get("display")
        .and_then(|d| d.get("pet"))
        .and_then(|p| p.get("enabled"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    if !pet_enabled {
        return Err((StatusCode::NOT_FOUND, Json(ErrorBody {
            error: "no_active_pet".into(),
        })));
    }

    let slug = config
        .get("display")
        .and_then(|d| d.get("pet"))
        .and_then(|p| p.get("slug"))
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty());

    let slug = match slug {
        Some(s) => s.to_string(),
        None => {
            let mut entries = tokio::fs::read_dir(state.pets_dir())
                .await
                .map_err(|_| {
                    (StatusCode::NOT_FOUND, Json(ErrorBody {
                        error: "no_active_pet".into(),
                    }))
                })?;
            let mut found = None;
            while let Ok(Some(entry)) = entries.next_entry().await {
                if entry.file_type().await.map(|t| t.is_dir()).unwrap_or(false) {
                    found = Some(entry.file_name().to_string_lossy().into_owned());
                    break;
                }
            }
            found.ok_or((StatusCode::NOT_FOUND, Json(ErrorBody {
                error: "no_active_pet".into(),
            })))?
        }
    };

    let spritesheet_path = state.pets_dir().join(&slug).join("spritesheet.webp");
    if !spritesheet_path.exists() {
        return Err((StatusCode::NOT_FOUND, Json(ErrorBody {
            error: "spritesheet not found".into(),
        })));
    }

    Ok(Json(ActivePet {
        display_name: slug.clone(),
        spritesheet_url: format!("/pets/{}/spritesheet.webp", slug),
        slug,
    }))
}

async fn serve_spritesheet(
    State(state): State<Arc<AppState>>,
    Path(slug): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorBody>)> {
    let spritesheet_path = state.pets_dir().join(&slug).join("spritesheet.webp");
    let bytes = tokio::fs::read(&spritesheet_path)
        .await
        .map_err(|_| {
            (StatusCode::NOT_FOUND, Json(ErrorBody {
                error: "spritesheet not found".into(),
            }))
        })?;

    Ok(([(axum::http::header::CONTENT_TYPE, "image/webp")], bytes))
}

async fn list_pets(
    State(state): State<Arc<AppState>>,
) -> Result<Json<PetList>, (StatusCode, Json<ErrorBody>)> {
    let mut pets = Vec::new();
    let mut entries = tokio::fs::read_dir(state.pets_dir())
        .await
        .map_err(|_| {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorBody {
                error: "cannot read pets directory".into(),
            }))
        })?;

    while let Ok(Some(entry)) = entries.next_entry().await {
        if !entry.file_type().await.map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }

        let slug = entry.file_name().to_string_lossy().into_owned();

        // Read displayName from pet.json; skip dirs without one
        let pet_json_path = state.pets_dir().join(&slug).join("pet.json");
        if !pet_json_path.exists() {
            continue;
        }

        let display_name = match tokio::fs::read_to_string(&pet_json_path).await {
            Ok(contents) => {
                serde_json::from_str::<serde_json::Value>(&contents)
                    .ok()
                    .and_then(|v| {
                        v.get("displayName")
                            .and_then(|dn| dn.as_str())
                            .map(String::from)
                    })
                    .unwrap_or_else(|| slug.clone())
            }
            Err(_) => slug.clone(),
        };

        pets.push(PetEntry {
            slug,
            display_name,
        });
    }

    // Read active slug from config
    let active = get_active_slug(&state).await.unwrap_or_default();

    Ok(Json(PetList { pets, active }))
}

/// Helper: read the active pet slug from config.yaml.
async fn get_active_slug(state: &AppState) -> Result<String, (StatusCode, Json<ErrorBody>)> {
    let config: serde_yaml_ng::Value = serde_yaml_ng::from_str(
        &tokio::fs::read_to_string(state.config_path())
            .await
            .map_err(|_| {
                (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorBody {
                    error: "cannot read config".into(),
                }))
            })?,
    )
    .map_err(|_| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorBody {
            error: "invalid config".into(),
        }))
    })?;

    Ok(config
        .get("display")
        .and_then(|d| d.get("pet"))
        .and_then(|p| p.get("slug"))
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(String::from)
        .unwrap_or_default())
}

async fn select_pet(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SelectPetRequest>,
) -> Result<Json<SelectPetResponse>, (StatusCode, Json<ErrorBody>)> {
    // Run hermes pets select <slug>
    let output = std::process::Command::new("hermes")
        .args(["pets", "select", &req.slug])
        .output()
        .map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorBody {
                error: format!("hermes pets select failed: {}", e),
            }))
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorBody {
            error: format!("hermes pets select failed: {}", stderr.trim()),
        })));
    }

    // Read display_name from pet.json
    let pet_json_path = state.pets_dir().join(&req.slug).join("pet.json");
    let display_name = if pet_json_path.exists() {
        tokio::fs::read_to_string(&pet_json_path)
            .await
            .ok()
            .and_then(|contents| {
                serde_json::from_str::<serde_json::Value>(&contents).ok()
            })
            .and_then(|v| {
                v.get("displayName")
                    .and_then(|dn| dn.as_str())
                    .map(String::from)
            })
            .unwrap_or_else(|| req.slug.clone())
    } else {
        req.slug.clone()
    };

    Ok(Json(SelectPetResponse {
        ok: true,
        slug: req.slug,
        display_name,
    }))
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
        let state = AppState {
            hermes_home: dir.path().to_path_buf(),
        };
        (dir, state)
    }

    #[tokio::test]
    async fn health_returns_200() {
        let (_home, state) = setup_state();
        let router = build_router(state);

        let response = router
            .oneshot(
                axum::http::Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = BodyExt::collect(response.into_body())
            .await
            .unwrap()
            .to_bytes();
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
                axum::http::Request::builder()
                    .uri("/api/pet/active")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = BodyExt::collect(response.into_body())
            .await
            .unwrap()
            .to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["slug"], "boba");
        assert_eq!(json["spritesheet_url"], "/pets/boba/spritesheet.webp");
        assert_eq!(json["display_name"], "boba");
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
                axum::http::Request::builder()
                    .uri("/api/pet/active")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let body = BodyExt::collect(response.into_body())
            .await
            .unwrap()
            .to_bytes();
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
                axum::http::Request::builder()
                    .uri("/api/pet/active")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = BodyExt::collect(response.into_body())
            .await
            .unwrap()
            .to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        // Should pick the first directory found (directory order is OS-dependent,
        // but both "boba" and "nyan" are valid — we just assert a slug was returned)
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
        let content_type = response
            .headers()
            .get(axum::http::header::CONTENT_TYPE)
            .unwrap()
            .to_str()
            .unwrap();
        assert_eq!(content_type, "image/webp");
        let body = BodyExt::collect(response.into_body())
            .await
            .unwrap()
            .to_bytes();
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
        let body = BodyExt::collect(response.into_body())
            .await
            .unwrap()
            .to_bytes();
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
            .oneshot(
                axum::http::Request::builder()
                    .uri("/api/pets")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = BodyExt::collect(response.into_body())
            .await
            .unwrap()
            .to_bytes();
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
        // One valid pet, one dir without pet.json (should be skipped)
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
            .oneshot(
                axum::http::Request::builder()
                    .uri("/api/pets")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = BodyExt::collect(response.into_body())
            .await
            .unwrap()
            .to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

        let pets = json["pets"].as_array().unwrap();
        // Only valid pet, no_json dir should be skipped
        assert_eq!(pets.len(), 1);
        assert_eq!(pets[0]["slug"], "valid");
        assert_eq!(pets[0]["display_name"], "Valid");
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
        let hermes_script = if cfg!(windows) {
            format!("@echo off\r\nexit /b 0\r\n")
        } else {
            format!("#!/bin/sh\nexit 0\n")
        };
        let script_path = fake_bin.join(if cfg!(windows) { "hermes.bat" } else { "hermes" });
        std::fs::write(&script_path, &hermes_script).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o755))
                .unwrap();
        }
        // Prepend fake-bin to PATH
        let old_path = std::env::var("PATH").unwrap_or_default();
        unsafe {
            std::env::set_var(
                "PATH",
                format!("{}:{}", fake_bin.display(), old_path),
            );
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
        let body = BodyExt::collect(response.into_body())
            .await
            .unwrap()
            .to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["ok"], true);
        assert_eq!(json["slug"], "nika");
        assert_eq!(json["display_name"], "Nika");

        // Restore PATH
        unsafe { std::env::set_var("PATH", old_path); }
    }
}
