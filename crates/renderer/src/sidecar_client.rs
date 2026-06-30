//! HTTP client for the companion sidecar.
//!
//! Fetches active pet configuration and spritesheet data from the
//! hermes-webui-companion-sidecar running at `http://127.0.0.1:17888`.

use serde::Deserialize;

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

/// Response from `GET /api/pet/active`.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct ActivePetResponse {
    pub slug: String,
    pub spritesheet_url: String,
    pub display_name: String,
}

/// Error from the sidecar (non-200 response).
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct SidecarError {
    pub error: String,
}

/// A single pet entry from GET /api/pets.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct PetEntry {
    pub slug: String,
    pub display_name: String,
}

/// Response from GET /api/pets.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct PetListResponse {
    pub pets: Vec<PetEntry>,
    pub active: String,
}

/// Response from POST /api/pet/select.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct SelectPetResponse {
    pub ok: bool,
    pub slug: String,
    pub display_name: String,
}

// ---------------------------------------------------------------------------
// Client
// ---------------------------------------------------------------------------

/// HTTP client for the companion sidecar service.
pub struct SidecarClient {
    base_url: String,
}

impl SidecarClient {
    /// Create a new client pointing at the sidecar URL.
    pub fn new(base_url: String) -> Self {
        Self { base_url }
    }

    /// Fetch the currently active pet configuration.
    ///
    /// Returns `Ok(ActivePetResponse)` on success, or `Err(SidecarError)`
    /// if the sidecar returned an error (pet disabled, not found, etc.).
    pub fn fetch_active_pet(&self) -> Result<ActivePetResponse, SidecarError> {
        let url = format!("{}/api/pet/active", self.base_url);
        let response = ureq::get(&url)
            .config()
            .http_status_as_error(false)
            .build()
            .call()
            .map_err(|e| SidecarError {
                error: format!("http error: {}", e),
            })?;

        let status = response.status();
        if status == 200 {
            response.into_body().read_json::<ActivePetResponse>().map_err(|e| {
                SidecarError {
                    error: format!("parse error: {}", e),
                }
            })
        } else {
            Err(response
                .into_body()
                .read_json::<SidecarError>()
                .unwrap_or(SidecarError {
                    error: format!("unexpected status: {}", status),
                }))
        }
    }

    /// Fetch the spritesheet bytes for a given pet slug.
    pub fn fetch_spritesheet(&self, slug: &str) -> Result<Vec<u8>, SidecarError> {
        let url = format!("{}/pets/{}/spritesheet.webp", self.base_url, slug);
        let response = ureq::get(&url)
            .config()
            .http_status_as_error(false)
            .build()
            .call()
            .map_err(|e| SidecarError {
                error: format!("http error: {}", e),
            })?;

        let status = response.status();
        if status == 200 {
            response.into_body().read_to_vec().map_err(|e| SidecarError {
                error: format!("read error: {}", e),
            })
        } else {
            Err(response
                .into_body()
                .read_json::<SidecarError>()
                .unwrap_or(SidecarError {
                    error: format!("unexpected status: {}", status),
                }))
        }
    }
    /// Check whether the sidecar is healthy (application-level).
    ///
    /// Hits `GET /health` and returns `true` only if the sidecar responds
    /// with HTTP 200 and `{"ok": true}`. Returns `false` on network errors,
    /// non-200 status, malformed JSON, or missing/mismatched `ok` field.
    ///
    /// Check whether the sidecar is healthy (application-level).
    /// ...
    pub fn check_health(&self) -> bool {
        let url = format!("{}/health", self.base_url);
        match ureq::get(&url)
            .config()
            .http_status_as_error(false)
            .build()
            .call()
        {
            Ok(response) if response.status() == 200 => response
                .into_body()
                .read_json::<serde_json::Value>()
                .ok()
                .and_then(|v| v.get("ok").and_then(|ok| ok.as_bool()))
                .unwrap_or(false),
            _ => false,
        }
    }

    /// Fetch the list of all installed pets with display names.
    pub fn fetch_pets(&self) -> Result<PetListResponse, SidecarError> {
        let url = format!("{}/api/pets", self.base_url);
        let response = ureq::get(&url)
            .config()
            .http_status_as_error(false)
            .build()
            .call()
            .map_err(|e| SidecarError {
                error: format!("http error: {}", e),
            })?;

        let status = response.status();
        if status == 200 {
            response.into_body().read_json::<PetListResponse>().map_err(|e| {
                SidecarError {
                    error: format!("parse error: {}", e),
                }
            })
        } else {
            Err(response
                .into_body()
                .read_json::<SidecarError>()
                .unwrap_or(SidecarError {
                    error: format!("unexpected status: {}", status),
                }))
        }
    }

    /// Select a new active pet via hermes pets select.
    pub fn select_pet(&self, slug: &str) -> Result<SelectPetResponse, SidecarError> {
        let url = format!("{}/api/pet/select", self.base_url);
        let body = serde_json::json!({"slug": slug});
        let response = ureq::post(&url)
            .config()
            .http_status_as_error(false)
            .build()
            .send_json(&body)
            .map_err(|e| SidecarError {
                error: format!("http error: {}", e),
            })?;

        let status = response.status();
        if status == 200 {
            response
                .into_body()
                .read_json::<SelectPetResponse>()
                .map_err(|e| SidecarError {
                    error: format!("parse error: {}", e),
                })
        } else {
            Err(response
                .into_body()
                .read_json::<SidecarError>()
                .unwrap_or(SidecarError {
                    error: format!("unexpected status: {}", status),
                }))
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::{Shutdown, TcpListener};
    use std::thread;

    /// Start a tiny HTTP server on a random port that responds with `body`
    /// and `status` to the next request, then shuts down.
    fn serve_once(status: u16, body: &str) -> u16 {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let body = body.to_string();

        thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            // Drain receive buffer, then flush with Shutdown::Write.
            let _ = stream.read(&mut [0; 4096]);
            let response = format!(
                "HTTP/1.1 {status} OK\r\nContent-Type: application/json\r\nContent-Length: {len}\r\nConnection: close\r\n\r\n{body}",
                status = status,
                len = body.len(),
                body = body,
            );
            stream.write_all(response.as_bytes()).unwrap();
            let _ = stream.shutdown(Shutdown::Write);
        });

        port
    }

    #[test]
    fn fetch_active_pet_success() {
        let port = serve_once(
            200,
            r#"{"slug":"boba","spritesheet_url":"/pets/boba/spritesheet.webp","display_name":"Boba"}"#,
        );
        let client = SidecarClient::new(format!("http://127.0.0.1:{}", port));

        let result = client.fetch_active_pet().unwrap();
        assert_eq!(result.slug, "boba");
        assert_eq!(result.spritesheet_url, "/pets/boba/spritesheet.webp");
        assert_eq!(result.display_name, "Boba");
    }

    #[test]
    fn fetch_active_pet_not_found() {
        let port = serve_once(404, r#"{"error":"no_active_pet"}"#);
        let client = SidecarClient::new(format!("http://127.0.0.1:{}", port));

        let err = client.fetch_active_pet().unwrap_err();
        assert_eq!(err.error, "no_active_pet");
    }

    #[test]
    fn fetch_spritesheet_success() {
        let port = {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let port = listener.local_addr().unwrap().port();
            let body = b"fake-webp-data".to_vec();

            thread::spawn(move || {
                let (mut stream, _) = listener.accept().unwrap();
                let _ = stream.read(&mut [0; 4096]); // drain
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: image/webp\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                stream.write_all(response.as_bytes()).unwrap();
                stream.write_all(&body).unwrap();
                let _ = stream.shutdown(Shutdown::Write);
            });

            port
        };

        let client = SidecarClient::new(format!("http://127.0.0.1:{}", port));
        let bytes = client.fetch_spritesheet("boba").unwrap();
        assert_eq!(bytes, b"fake-webp-data");
    }

    #[test]
    fn check_health_returns_true() {
        let port = serve_once(200, r#"{"ok":true,"service":"hermes-webui-companion-sidecar"}"#);
        let client = SidecarClient::new(format!("http://127.0.0.1:{}", port));
        assert!(client.check_health());
    }

    #[test]
    fn check_health_returns_false_on_404() {
        let port = serve_once(404, r#"{"error":"not_found"}"#);
        let client = SidecarClient::new(format!("http://127.0.0.1:{}", port));
        assert!(!client.check_health());
    }

    #[test]
    fn check_health_returns_false_on_bad_json() {
        let port = serve_once(200, "not json");
        let client = SidecarClient::new(format!("http://127.0.0.1:{}", port));
        assert!(!client.check_health());
    }

    #[test]
    fn check_health_returns_false_on_ok_false() {
        let port = serve_once(200, r#"{"ok":false}"#);
        let client = SidecarClient::new(format!("http://127.0.0.1:{}", port));
        assert!(!client.check_health());
    }

    #[test]
    fn check_health_returns_false_on_missing_ok() {
        let port = serve_once(200, r#"{"service":"test"}"#);
        let client = SidecarClient::new(format!("http://127.0.0.1:{}", port));
        assert!(!client.check_health());
    }

    #[test]
    fn fetch_pets_success() {
        let port = serve_once(
            200,
            r#"{"pets":[{"slug":"doraemon","display_name":"Doraemon"},{"slug":"nika","display_name":"Nika"}],"active":"nika"}"#,
        );
        let client = SidecarClient::new(format!("http://127.0.0.1:{}", port));
        let result = client.fetch_pets().unwrap();
        assert_eq!(result.pets.len(), 2);
        assert_eq!(result.pets[0].slug, "doraemon");
        assert_eq!(result.pets[0].display_name, "Doraemon");
        assert_eq!(result.active, "nika");
    }

    #[test]
    fn select_pet_success() {
        let port = serve_once(
            200,
            r#"{"ok":true,"slug":"nika","display_name":"Nika"}"#,
        );
        let client = SidecarClient::new(format!("http://127.0.0.1:{}", port));
        let result = client.select_pet("nika").unwrap();
        assert!(result.ok);
        assert_eq!(result.slug, "nika");
        assert_eq!(result.display_name, "Nika");
    }
}
