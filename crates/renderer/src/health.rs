//! Sidecar health check — initial synchronous probe + background polling thread.
//!
//! Sets an `AtomicBool` flag consumed by `resolve_animation_state()` so
//! incoming WebUI snapshots cannot race the health check and cause a
//! flicker loop.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::debug;

/// Run an initial synchronous health check, then spawn a background thread
/// that probes the sidecar every 10 seconds. Returns the `sidecar_healthy` flag.
pub fn spawn_health_check() -> Arc<AtomicBool> {
    let initial_healthy = std::net::TcpStream::connect_timeout(
        &"127.0.0.1:17888".parse().unwrap(),
        std::time::Duration::from_secs(2),
    )
    .is_ok();

    if !initial_healthy {
        debug!("[companion:health] sidecar unreachable at startup → Failed");
    }

    let sidecar_healthy = Arc::new(AtomicBool::new(initial_healthy));

    {
        let sidecar_healthy = sidecar_healthy.clone();
        std::thread::spawn(move || loop {
            std::thread::sleep(std::time::Duration::from_secs(10));
            let healthy = std::net::TcpStream::connect_timeout(
                &"127.0.0.1:17888".parse().unwrap(),
                std::time::Duration::from_secs(2),
            )
            .is_ok();
            let was = sidecar_healthy.swap(healthy, Ordering::SeqCst);
            if !healthy && was {
                debug!("[companion:health] sidecar unreachable → Failed");
            } else if healthy && !was {
                debug!("[companion:health] sidecar recovered → Idle");
            }
        });
    }

    sidecar_healthy
}
