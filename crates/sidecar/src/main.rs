//! hermes-pet-sidecar — Tiny HTTP bridge between WSL filesystem and Windows host.
//!
//! Serves Hermes pet configuration and spritesheet assets via localhost,
//! so the Tauri renderer on the Windows host can access them without
//! fragile `\\wsl$` path hacks.

use axum::{Json, Router, extract::Path, http::StatusCode, response::IntoResponse, routing::get};
use serde::Serialize;
use std::path::PathBuf;
use tower_http::cors::CorsLayer;
use tracing::info;

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

fn hermes_home() -> PathBuf {
    std::env::var("HERMES_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| dirs::home_dir().unwrap().join(".hermes"))
}

async fn health() -> Json<Health> {
    Json(Health {
        ok: true,
        service: "hermes-pet-sidecar",
    })
}

async fn get_active_pet() -> Result<Json<ActivePet>, (StatusCode, Json<ErrorBody>)> {
    let config_path = hermes_home().join("config.yaml");
    let config: serde_yaml::Value = serde_yaml::from_str(
        &tokio::fs::read_to_string(&config_path)
            .await
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorBody { error: "cannot read config".into() })))?,
    )
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorBody { error: "invalid config".into() })))?;

    let pet_enabled = config
        .get("display")
        .and_then(|d| d.get("pet"))
        .and_then(|p| p.get("enabled"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    if !pet_enabled {
        return Err((StatusCode::NOT_FOUND, Json(ErrorBody { error: "no_active_pet".into() })));
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
            // No slug set — pick the first installed pet
            let pets_dir = hermes_home().join("pets");
            let mut entries = tokio::fs::read_dir(&pets_dir)
                .await
                .map_err(|_| (StatusCode::NOT_FOUND, Json(ErrorBody { error: "no_active_pet".into() })))?;
            let mut found = None;
            while let Ok(Some(entry)) = entries.next_entry().await {
                if entry.file_type().await.map(|t| t.is_dir()).unwrap_or(false) {
                    found = Some(entry.file_name().to_string_lossy().into_owned());
                    break;
                }
            }
            found.ok_or((StatusCode::NOT_FOUND, Json(ErrorBody { error: "no_active_pet".into() })))?
        }
    };

    let spritesheet_path = hermes_home().join("pets").join(&slug).join("spritesheet.webp");
    if !spritesheet_path.exists() {
        return Err((StatusCode::NOT_FOUND, Json(ErrorBody { error: "spritesheet not found".into() })));
    }

    Ok(Json(ActivePet {
        display_name: slug.clone(),
        spritesheet_url: format!("/pets/{}/spritesheet.webp", slug),
        slug,
    }))
}

async fn serve_spritesheet(
    Path(slug): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorBody>)> {
    let spritesheet_path = hermes_home().join("pets").join(&slug).join("spritesheet.webp");

    let bytes = tokio::fs::read(&spritesheet_path)
        .await
        .map_err(|_| (StatusCode::NOT_FOUND, Json(ErrorBody { error: "spritesheet not found".into() })))?;

    Ok((
        [(axum::http::header::CONTENT_TYPE, "image/webp")],
        bytes,
    ))
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let app = Router::new()
        .route("/health", get(health))
        .route("/api/pet/active", get(get_active_pet))
        .route("/pets/{slug}/spritesheet.webp", get(serve_spritesheet))
        .layer(CorsLayer::permissive());

    let addr = "127.0.0.1:17888";
    info!("hermes-pet-sidecar listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
