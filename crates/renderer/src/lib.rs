//! hermes-webui-companion-renderer — desktop companion renderer.
//!
//! Library crate exporting pure modules for testing and reuse.
//! The Tauri GUI binary lives in `gui.rs` behind the `gui` feature flag.

pub mod animation;
pub mod bridge;
pub mod bridge_server;
pub mod sidecar_client;
pub mod sprite;

// Re-export the PetDataProvider trait for crate consumers.
pub use sidecar_client::PetDataProvider;
