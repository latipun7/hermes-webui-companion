//! Direct filesystem client for pet data — no sidecar needed.
//!
//! Reads `~/.hermes/config.yaml` and `~/.hermes/pets/` directly from
//! the local filesystem. Used in Direct Mode when the renderer and
//! Hermes Agent share the same host.
//!
//! Implements `PetDataProvider` — same interface as `SidecarClient`.

use std::fs;
use std::path::PathBuf;

use crate::sidecar_client::{
    ActivePetResponse, PetDataProvider, PetEntry, PetListResponse, SelectPetResponse, SidecarError,
};

// ---------------------------------------------------------------------------
// DirectClient
// ---------------------------------------------------------------------------

/// Filesystem-based pet data provider.
pub struct DirectClient {
    hermes_home: PathBuf,
}

impl DirectClient {
    /// Create a new `DirectClient` rooted at the given Hermes home directory.
    ///
    /// Pass `None` to use the platform default (`Self::default_hermes_home()`).
    pub fn new(hermes_home: Option<PathBuf>) -> Self {
        Self { hermes_home: hermes_home.unwrap_or_else(Self::default_hermes_home) }
    }

    /// Platform-aware default Hermes home path.
    ///
    /// Respects `HERMES_HOME` environment variable.
    /// Falls back to `~/.hermes` on Linux/macOS, `%LOCALAPPDATA%\hermes\` on Windows.
    pub fn default_hermes_home() -> PathBuf {
        if let Ok(env_home) = std::env::var("HERMES_HOME") {
            return PathBuf::from(env_home);
        }

        #[cfg(target_os = "windows")]
        {
            if let Ok(local_appdata) = std::env::var("LOCALAPPDATA") {
                return PathBuf::from(local_appdata).join("hermes");
            }
            if let Ok(userprofile) = std::env::var("USERPROFILE") {
                return PathBuf::from(userprofile).join(".hermes");
            }
        }

        // Linux / macOS / fallback
        if let Ok(home) = std::env::var("HOME") {
            PathBuf::from(home).join(".hermes")
        } else {
            PathBuf::from(".hermes")
        }
    }

    fn config_path(&self) -> PathBuf {
        self.hermes_home.join("config.yaml")
    }

    fn pets_dir(&self) -> PathBuf {
        self.hermes_home.join("pets")
    }
}

// ---------------------------------------------------------------------------
// PetDataProvider implementation
// ---------------------------------------------------------------------------

impl PetDataProvider for DirectClient {
    fn fetch_active_pet(&self) -> Result<ActivePetResponse, SidecarError> {
        let config_path = self.config_path();
        let yaml_str = fs::read_to_string(&config_path)
            .map_err(|e| SidecarError { error: format!("cannot read config: {}", e) })?;

        let config: serde_yaml_ng::Value = serde_yaml_ng::from_str(&yaml_str)
            .map_err(|e| SidecarError { error: format!("invalid config: {}", e) })?;

        let pet_enabled = config
            .get("display")
            .and_then(|d| d.get("pet"))
            .and_then(|p| p.get("enabled"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        if !pet_enabled {
            return Err(SidecarError { error: "no_active_pet".into() });
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
                // Fallback: pick the first installed pet directory
                let pets_dir = self.pets_dir();
                let mut found = None;
                if let Ok(entries) = fs::read_dir(&pets_dir) {
                    for entry in entries.flatten() {
                        if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                            found = Some(entry.file_name().to_string_lossy().into_owned());
                            break;
                        }
                    }
                }
                found.ok_or(SidecarError { error: "no_active_pet".into() })?
            },
        };

        // Read display_name from pet.json
        let pet_json_path = self.pets_dir().join(&slug).join("pet.json");
        let display_name = fs::read_to_string(&pet_json_path)
            .ok()
            .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
            .and_then(|v| v.get("displayName").and_then(|n| n.as_str()).map(String::from))
            .unwrap_or_else(|| slug.clone());

        Ok(ActivePetResponse {
            spritesheet_url: format!("/pets/{}/spritesheet.webp", slug),
            slug,
            display_name,
        })
    }

    fn fetch_spritesheet(&self, slug: &str) -> Result<Vec<u8>, SidecarError> {
        let path = self.pets_dir().join(slug).join("spritesheet.webp");
        fs::read(&path)
            .map_err(|e| SidecarError { error: format!("cannot read spritesheet: {}", e) })
    }

    fn fetch_pets(&self) -> Result<PetListResponse, SidecarError> {
        let pets_dir = self.pets_dir();
        let entries = fs::read_dir(&pets_dir)
            .map_err(|e| SidecarError { error: format!("cannot read pets dir: {}", e) })?;

        let mut pets = Vec::new();
        for entry in entries.flatten() {
            if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                continue;
            }
            let slug = entry.file_name().to_string_lossy().into_owned();
            let pet_json_path = entry.path().join("pet.json");
            let display_name = fs::read_to_string(&pet_json_path)
                .ok()
                .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
                .and_then(|v| v.get("displayName").and_then(|n| n.as_str()).map(String::from))
                .unwrap_or_else(|| slug.clone());
            pets.push(PetEntry { slug, display_name });
        }

        // Determine active slug from config
        let active = self.fetch_active_pet().map(|a| a.slug).unwrap_or_default();

        Ok(PetListResponse { pets, active })
    }

    fn select_pet(&self, slug: &str) -> Result<SelectPetResponse, SidecarError> {
        // In direct mode, run hermes pets select via CLI subprocess.
        // First try hermes in PATH, then fall back to venv Python.
        let hermes_home = self.hermes_home.to_string_lossy();

        // Try hermes in PATH first
        let output = std::process::Command::new("hermes").args(["pets", "select", slug]).output();

        let result = match output {
            Ok(o) if o.status.success() => o,
            _ => {
                // Fallback: run via venv Python
                #[cfg(target_os = "windows")]
                let python = format!("{}\\hermes-agent\\venv\\Scripts\\python.exe", hermes_home);
                #[cfg(not(target_os = "windows"))]
                let python = format!("{}/hermes-agent/venv/bin/python", hermes_home);

                std::process::Command::new(&python)
                    .args(["-m", "hermes_cli", "pets", "select", slug])
                    .output()
                    .map_err(|e| SidecarError {
                        error: format!("hermes pets select failed: {}", e),
                    })?
            },
        };

        if !result.status.success() {
            let stderr = String::from_utf8_lossy(&result.stderr);
            return Err(SidecarError {
                error: format!("hermes pets select failed: {}", stderr.trim()),
            });
        }

        // Read display name from pet.json
        let pet_json_path = self.pets_dir().join(slug).join("pet.json");
        let display_name = fs::read_to_string(&pet_json_path)
            .ok()
            .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
            .and_then(|v| v.get("displayName").and_then(|n| n.as_str()).map(String::from))
            .unwrap_or_else(|| slug.to_string());

        Ok(SelectPetResponse { ok: true, slug: slug.to_string(), display_name })
    }

    fn is_available(&self) -> bool {
        self.config_path().exists() && fs::read_to_string(self.config_path()).is_ok()
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

    #[test]
    fn default_hermes_home_uses_home_fallback() {
        // On Linux/macOS, HOME is always set; this test just verifies
        // the path ends with .hermes. Full env var test skipped due to
        // unsafe_code=deny (cannot call set_var/remove_var).
        let path = DirectClient::default_hermes_home();
        assert!(path.ends_with(".hermes"));
    }
}
