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
    routing::get,
};
use serde::Serialize;
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

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/api/pet/active", get(get_active_pet))
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
}
