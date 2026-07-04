//! hermes-webui-companion-common — shared types and configuration reader.
//!
//! Provides the single source of truth for pet configuration resolution,
//! pet directory scanning, and the shared data types used by both the
//! sidecar and the renderer.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Shared types
// ---------------------------------------------------------------------------

/// Error returned by any pet data operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DataError {
    pub error: String,
}

impl std::fmt::Display for DataError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.error)
    }
}

/// A single installed pet entry (slug + display name).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PetEntry {
    pub slug: String,
    pub display_name: String,
}

/// Response for the currently active pet.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ActivePetConfig {
    pub slug: String,
    pub spritesheet_url: String,
    pub display_name: String,
}

/// Response listing all installed pets.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PetList {
    pub pets: Vec<PetEntry>,
    pub active: String,
}

/// Response after selecting a new pet.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SelectPetResponse {
    pub ok: bool,
    pub slug: String,
    pub display_name: String,
}

// ---------------------------------------------------------------------------
// ConfigReader
// ---------------------------------------------------------------------------

/// Reads Hermes pet configuration from the local filesystem.
///
/// All methods are synchronous (`std::fs`). Async callers (e.g. the sidecar)
/// can wrap blocking calls in `tokio::task::spawn_blocking` if needed.
#[derive(Clone)]
pub struct ConfigReader {
    hermes_home: PathBuf,
}

impl ConfigReader {
    /// Resolve the default Hermes home path.
    ///
    /// Respects `HERMES_HOME` environment variable.
    /// Falls back to `~/.hermes` on Linux/macOS, `%LOCALAPPDATA%\hermes\` on Windows.
    pub fn resolve() -> Self {
        Self { hermes_home: Self::default_hermes_home() }
    }

    /// Create a reader rooted at an explicit path (useful for tests).
    pub fn new<P: Into<PathBuf>>(hermes_home: P) -> Self {
        Self { hermes_home: hermes_home.into() }
    }

    /// Platform-aware default Hermes home path.
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

    /// The pets directory path.
    pub fn pets_dir(&self) -> PathBuf {
        self.hermes_home.join("pets")
    }

    // ── Config file access ──────────────────────────────────────────

    /// Read and parse the full config.yaml as a YAML value.
    fn read_config(&self) -> Result<serde_yaml_ng::Value, DataError> {
        let yaml_str = std::fs::read_to_string(self.config_path())
            .map_err(|e| DataError { error: format!("cannot read config: {}", e) })?;
        serde_yaml_ng::from_str(&yaml_str)
            .map_err(|e| DataError { error: format!("invalid config: {}", e) })
    }

    /// Is `display.pet.enabled` set to `true` in config.yaml?
    pub fn pet_enabled(&self) -> Result<bool, DataError> {
        let config = self.read_config()?;
        Ok(config
            .get("display")
            .and_then(|d| d.get("pet"))
            .and_then(|p| p.get("enabled"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false))
    }

    /// The active pet slug from `display.pet.slug`.
    ///
    /// Returns `None` when the slug is missing or empty.
    pub fn active_pet_slug(&self) -> Result<Option<String>, DataError> {
        let config = self.read_config()?;
        Ok(config
            .get("display")
            .and_then(|d| d.get("pet"))
            .and_then(|p| p.get("slug"))
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(String::from))
    }

    // ── Pet directory scanning ──────────────────────────────────────

    /// Scan the `pets/` directory and return the first subdirectory found.
    ///
    /// Returns `None` when the directory is empty or doesn't exist.
    pub fn first_pet_dir(&self) -> Result<Option<String>, DataError> {
        let pets_dir = self.pets_dir();
        if !pets_dir.exists() {
            return Ok(None);
        }
        let entries = std::fs::read_dir(&pets_dir)
            .map_err(|e| DataError { error: format!("cannot read pets directory: {}", e) })?;
        for entry in entries.flatten() {
            if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                return Ok(Some(entry.file_name().to_string_lossy().into_owned()));
            }
        }
        Ok(None)
    }

    /// Resolve the active slug: config value first, then fall back to
    /// the first installed pet directory.
    pub fn resolve_active_slug(&self) -> Result<Option<String>, DataError> {
        if let Some(slug) = self.active_pet_slug()?
            && !slug.is_empty()
        {
            return Ok(Some(slug));
        }
        self.first_pet_dir()
    }

    // ── pet.json access ─────────────────────────────────────────────

    /// Read the `displayName` field from `pets/{slug}/pet.json`.
    ///
    /// Falls back to the slug itself if the file is missing or malformed.
    pub fn read_display_name(&self, slug: &str) -> String {
        let pet_json_path = self.pets_dir().join(slug).join("pet.json");
        std::fs::read_to_string(&pet_json_path)
            .ok()
            .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
            .and_then(|v| v.get("displayName").and_then(|n| n.as_str()).map(String::from))
            .unwrap_or_else(|| slug.to_string())
    }

    /// Walk the `pets/` directory and return all installed pets with
    /// their display names.
    pub fn scan_installed_pets(&self) -> Result<Vec<PetEntry>, DataError> {
        let pets_dir = self.pets_dir();
        let entries = std::fs::read_dir(&pets_dir)
            .map_err(|e| DataError { error: format!("cannot read pets directory: {}", e) })?;

        let mut pets = Vec::new();
        for entry in entries.flatten() {
            if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                continue;
            }
            let slug = entry.file_name().to_string_lossy().into_owned();
            let display_name = self.read_display_name(&slug);
            pets.push(PetEntry { slug, display_name });
        }
        Ok(pets)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_temp_hermes_home(
        config_yaml: &str,
        pets: &[(&str, &str)], // (slug, display_name)
        spritesheets: &[&str], // slugs that have a spritesheet
    ) -> (tempfile::TempDir, ConfigReader) {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("config.yaml");
        std::fs::write(&config_path, config_yaml).unwrap();

        let pets_dir = dir.path().join("pets");
        std::fs::create_dir_all(&pets_dir).unwrap();

        for &(slug, display_name) in pets {
            let pet_dir = pets_dir.join(slug);
            std::fs::create_dir_all(&pet_dir).unwrap();
            std::fs::write(
                pet_dir.join("pet.json"),
                serde_json::json!({ "displayName": display_name }).to_string(),
            )
            .unwrap();

            if spritesheets.contains(&slug) {
                std::fs::write(pet_dir.join("spritesheet.webp"), b"fake-webp").unwrap();
            }
        }

        let reader = ConfigReader::new(dir.path().to_path_buf());
        (dir, reader)
    }

    #[test]
    fn pet_enabled_true() {
        let (_dir, reader) = setup_temp_hermes_home(
            "display:\n  pet:\n    enabled: true\n    slug: boba\n",
            &[("boba", "Boba")],
            &["boba"],
        );
        assert!(reader.pet_enabled().unwrap());
    }

    #[test]
    fn pet_enabled_false() {
        let (_dir, reader) = setup_temp_hermes_home(
            "display:\n  pet:\n    enabled: false\n    slug: boba\n",
            &[("boba", "Boba")],
            &["boba"],
        );
        assert!(!reader.pet_enabled().unwrap());
    }

    #[test]
    fn pet_enabled_defaults_false() {
        let (_dir, reader) = setup_temp_hermes_home("", &[], &[]);
        assert!(!reader.pet_enabled().unwrap());
    }

    #[test]
    fn active_pet_slug_present() {
        let (_dir, reader) = setup_temp_hermes_home(
            "display:\n  pet:\n    enabled: true\n    slug: nika\n",
            &[("nika", "Nika")],
            &["nika"],
        );
        assert_eq!(reader.active_pet_slug().unwrap(), Some("nika".into()));
    }

    #[test]
    fn active_pet_slug_empty_string_is_none() {
        let (_dir, reader) = setup_temp_hermes_home(
            "display:\n  pet:\n    enabled: true\n    slug: \n",
            &[("boba", "Boba")],
            &["boba"],
        );
        assert_eq!(reader.active_pet_slug().unwrap(), None);
    }

    #[test]
    fn active_pet_slug_missing_is_none() {
        let (_dir, reader) = setup_temp_hermes_home(
            "display:\n  pet:\n    enabled: true\n",
            &[("boba", "Boba")],
            &["boba"],
        );
        assert_eq!(reader.active_pet_slug().unwrap(), None);
    }

    #[test]
    fn first_pet_dir_finds_first() {
        let (_dir, reader) = setup_temp_hermes_home("", &[("boba", "Boba"), ("nika", "Nika")], &[]);
        let found = reader.first_pet_dir().unwrap();
        // Directory order is OS-dependent — either is valid
        assert!(found == Some("boba".into()) || found == Some("nika".into()));
    }

    #[test]
    fn first_pet_dir_empty_returns_none() {
        let (_dir, reader) = setup_temp_hermes_home("", &[], &[]);
        assert_eq!(reader.first_pet_dir().unwrap(), None);
    }

    #[test]
    fn resolve_active_slug_uses_config_first() {
        let (_dir, reader) = setup_temp_hermes_home(
            "display:\n  pet:\n    enabled: true\n    slug: nika\n",
            &[("boba", "Boba"), ("nika", "Nika")],
            &["boba", "nika"],
        );
        assert_eq!(reader.resolve_active_slug().unwrap(), Some("nika".into()));
    }

    #[test]
    fn resolve_active_slug_falls_back_to_first_dir() {
        let (_dir, reader) = setup_temp_hermes_home(
            "display:\n  pet:\n    enabled: true\n    slug: \n",
            &[("boba", "Boba"), ("nika", "Nika")],
            &[],
        );
        let found = reader.resolve_active_slug().unwrap();
        assert!(found == Some("boba".into()) || found == Some("nika".into()));
    }

    #[test]
    fn resolve_active_slug_returns_none_when_no_pets() {
        let (_dir, reader) = setup_temp_hermes_home("", &[], &[]);
        assert_eq!(reader.resolve_active_slug().unwrap(), None);
    }

    #[test]
    fn read_display_name_from_pet_json() {
        let (_dir, reader) = setup_temp_hermes_home("", &[("boba", "Boba Tea")], &[]);
        assert_eq!(reader.read_display_name("boba"), "Boba Tea");
    }

    #[test]
    fn read_display_name_falls_back_to_slug() {
        let (_dir, reader) = setup_temp_hermes_home("", &[], &[]);
        assert_eq!(reader.read_display_name("nope"), "nope");
    }

    #[test]
    fn scan_installed_pets_returns_all() {
        let (_dir, reader) = setup_temp_hermes_home(
            "",
            &[("boba", "Boba"), ("doraemon", "Doraemon"), ("nika", "Nika")],
            &[],
        );
        let pets = reader.scan_installed_pets().unwrap();
        assert_eq!(pets.len(), 3);
        let slugs: Vec<&str> = pets.iter().map(|p| p.slug.as_str()).collect();
        assert!(slugs.contains(&"boba"));
        assert!(slugs.contains(&"doraemon"));
        assert!(slugs.contains(&"nika"));
        // Display names are read from pet.json
        let boba = pets.iter().find(|p| p.slug == "boba").unwrap();
        assert_eq!(boba.display_name, "Boba");
    }

    #[test]
    fn scan_installed_pets_empty_dir() {
        let (_dir, reader) = setup_temp_hermes_home("", &[], &[]);
        let pets = reader.scan_installed_pets().unwrap();
        assert!(pets.is_empty());
    }

    #[test]
    fn default_hermes_home_ends_with_dot_hermes() {
        let path = ConfigReader::default_hermes_home();
        assert!(path.ends_with(".hermes"));
    }
}
