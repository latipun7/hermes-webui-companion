//! hermes-webui-companion-renderer — desktop companion renderer.
//!
//! Library crate exporting pure modules for testing and reuse.
//! The Tauri GUI binary lives in `gui.rs` behind the `gui` feature flag.

pub mod animation;
pub mod bridge;
pub mod bridge_server;
pub mod direct_client;
pub mod sidecar_client;
pub mod sprite;

// Re-exports from common for downstream consumers.
pub use hermes_webui_companion_common as common;
pub use sidecar_client::PetDataProvider;
