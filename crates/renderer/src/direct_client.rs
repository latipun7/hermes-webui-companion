//! Direct filesystem client for pet data — no sidecar needed.
//!
//! Reads `~/.hermes/config.yaml` and `~/.hermes/pets/` directly from
//! the local filesystem via the shared `ConfigReader` from `hermes-webui-companion-common`.
//! Used in Direct Mode when the renderer and Hermes Agent share the same host.
//!
//! Implements `PetDataProvider` — same interface as `SidecarClient`.

use std::fs;
use std::path::PathBuf;

use hermes_webui_companion_common::{
    ActivePetConfig, ConfigReader, DataError, PetList, SelectPetResponse,
};

use crate::sidecar_client::PetDataProvider;

// ---------------------------------------------------------------------------
// DirectClient
// ---------------------------------------------------------------------------

/// Filesystem-based pet data provider.
pub struct DirectClient {
    config: ConfigReader,
}

impl DirectClient {
    /// Create a new `DirectClient` rooted at the given Hermes home directory.
    ///
    /// Pass `None` to use the platform default (`ConfigReader::resolve()`).
    pub fn new(hermes_home: Option<PathBuf>) -> Self {
        let config = match hermes_home {
            Some(path) => ConfigReader::new(path),
            None => ConfigReader::resolve(),
        };
        Self { config }
    }
}

// ---------------------------------------------------------------------------
// PetDataProvider implementation
// ---------------------------------------------------------------------------

impl PetDataProvider for DirectClient {
    fn fetch_active_pet(&self) -> Result<ActivePetConfig, DataError> {
        if !self.config.pet_enabled()? {
            return Err(DataError { error: "no_active_pet".into() });
        }

        let slug = self
            .config
            .resolve_active_slug()?
            .ok_or(DataError { error: "no_active_pet".into() })?;

        let display_name = self.config.read_display_name(&slug);
        let spritesheet_url = format!("/pets/{}/spritesheet.webp", slug);

        Ok(ActivePetConfig { slug, spritesheet_url, display_name })
    }

    fn fetch_spritesheet(&self, slug: &str) -> Result<Vec<u8>, DataError> {
        let path = self.config.pets_dir().join(slug).join("spritesheet.webp");
        fs::read(&path).map_err(|e| DataError { error: format!("cannot read spritesheet: {}", e) })
    }

    fn fetch_pets(&self) -> Result<PetList, DataError> {
        let pets = self.config.scan_installed_pets()?;
        let active = self.fetch_active_pet().map(|a| a.slug).unwrap_or_default();
        Ok(PetList { pets, active })
    }

    fn select_pet(
        &self,
        slug: &str,
    ) -> Result<hermes_webui_companion_common::SelectPetResponse, DataError> {
        // In direct mode, run hermes pets select via CLI subprocess.
        // First try hermes in PATH, then fall back to venv Python.
        let hermes_home = ConfigReader::default_hermes_home();
        let hermes_home_str = hermes_home.to_string_lossy();

        // Try hermes in PATH first
        let output = std::process::Command::new("hermes").args(["pets", "select", slug]).output();

        let result = match output {
            Ok(o) if o.status.success() => o,
            _ => {
                // Fallback: run via venv Python
                #[cfg(target_os = "windows")]
                let python =
                    format!("{}\\\\hermes-agent\\\\venv\\\\Scripts\\\\python.exe", hermes_home_str);
                #[cfg(not(target_os = "windows"))]
                let python = format!("{}/hermes-agent/venv/bin/python", hermes_home_str);

                std::process::Command::new(&python)
                    .args(["-m", "hermes_cli", "pets", "select", slug])
                    .output()
                    .map_err(|e| DataError { error: format!("hermes pets select failed: {}", e) })?
            },
        };

        if !result.status.success() {
            let stderr = String::from_utf8_lossy(&result.stderr);
            return Err(DataError {
                error: format!("hermes pets select failed: {}", stderr.trim()),
            });
        }

        let display_name = self.config.read_display_name(slug);

        Ok(SelectPetResponse { ok: true, slug: slug.to_string(), display_name })
    }

    fn is_available(&self) -> bool {
        // Check if the stored config is readable
        self.config.pet_enabled().is_ok()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Create a fake hermes home in a temp dir with config.yaml and pets/.
    fn setup_temp_hermes_home() -> (tempfile::TempDir, DirectClient) {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("config.yaml");
        let pets_dir = dir.path().join("pets");
        fs::create_dir_all(&pets_dir).unwrap();

        // Write a minimal config
        fs::write(&config_path, "display:\n  pet:\n    enabled: true\n    slug: boba\n").unwrap();

        // Create pet dir with pet.json and spritesheet
        let pet_dir = pets_dir.join("boba");
        fs::create_dir_all(&pet_dir).unwrap();
        fs::write(
            pet_dir.join("pet.json"),
            r#"{"id":"boba","displayName":"Boba","description":"A cute pet"}"#,
        )
        .unwrap();
        fs::write(pet_dir.join("spritesheet.webp"), b"fake-webp-data").unwrap();

        let client = DirectClient::new(Some(dir.path().to_path_buf()));
        (dir, client)
    }

    #[test]
    fn fetch_active_pet_success() {
        let (_dir, client) = setup_temp_hermes_home();
        let pet = client.fetch_active_pet().unwrap();
        assert_eq!(pet.slug, "boba");
        assert_eq!(pet.display_name, "Boba");
    }

    #[test]
    fn fetch_spritesheet_success() {
        let (_dir, client) = setup_temp_hermes_home();
        let bytes = client.fetch_spritesheet("boba").unwrap();
        assert_eq!(bytes, b"fake-webp-data");
    }

    #[test]
    fn fetch_spritesheet_not_found() {
        let (_dir, client) = setup_temp_hermes_home();
        let err = client.fetch_spritesheet("nonexistent").unwrap_err();
        assert!(err.error.contains("cannot read spritesheet"));
    }

    #[test]
    fn fetch_pets_success() {
        let (_dir, client) = setup_temp_hermes_home();
        let list = client.fetch_pets().unwrap();
        assert_eq!(list.pets.len(), 1);
        assert_eq!(list.pets[0].slug, "boba");
        assert_eq!(list.pets[0].display_name, "Boba");
        assert_eq!(list.active, "boba");
    }

    #[test]
    fn is_available_returns_true() {
        let (_dir, client) = setup_temp_hermes_home();
        assert!(client.is_available());
    }

    #[test]
    fn is_available_returns_false_when_no_config() {
        let dir = tempfile::tempdir().unwrap();
        let client = DirectClient::new(Some(dir.path().to_path_buf()));
        assert!(!client.is_available());
    }
}
