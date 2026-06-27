# hermes-webui-companion

Desktop companion renderer for Hermes Agent — a native Windows pet window
that reacts to Hermes state (idle, running, approval, clarify) in real-time.

## Architecture

```
┌─ WSL ───────────────────────────────────────────────┐
│  hermes-webui-companion-sidecar (:17888)             │
│  → serves pet config + spritesheets                 │
│  → reads ~/.hermes/config.yaml & ~/.hermes/pets/    │
│                                                      │
│  Hermes WebUI (:8787)                                │
│  → companion-adapter.js POSTs snapshots to :17787    │
└──────────────────────────────────────────────────────┘
          │                        │
    HTTP fetch              HTTP POST
          │                        │
┌─ Windows (Tauri) ────────────────────────────────────┐
│  hermes-webui-companion-gui                          │
│  ├─ Bridge server (:17787) — receives snapshots      │
│  ├─ Animation engine — maps state to spritesheet     │
│  ├─ CSS sprite renderer — zero-blink, zero-canvas     │
│  └─ State polling — 1s interval from bridge          │
│                                                      │
│  Window: 172×186, transparent, always-on-top          │
│  Drag: CSS -webkit-app-region + Tauri IPC fallback    │
└──────────────────────────────────────────────────────┘
```

## Project Structure

```
webui-companion/
├── Cargo.toml                 # workspace root
├── rust-toolchain.toml        # stable + rust-analyzer
├── docs/
│   ├── prd.md                 # full PRD
│   └── hermes-webui-companion-sidecar.service
├── crates/
│   ├── sidecar/               # hermes-webui-companion-sidecar
│   │   └── src/main.rs        # axum HTTP server, 6 integration tests
│   └── renderer/              # hermes-webui-companion-renderer
│       ├── src/
│       │   ├── lib.rs         # library crate
│       │   ├── main.rs        # stub binary
│       │   ├── gui.rs         # Tauri binary (feature: gui)
│       │   ├── sprite.rs      # spritesheet parser (5 tests)
│       │   ├── animation.rs   # state machine (8 tests)
│       │   ├── bridge.rs      # snapshot parser (7 tests)
│       │   ├── bridge_server.rs # HTTP server for WebUI snapshots
│       │   └── sidecar_client.rs # HTTP client to sidecar (3 tests)
│       ├── gui/               # Tauri frontend
│       │   ├── index.html     # CSS sprite window
│       │   └── pet.js         # animation + state polling
│       ├── tauri.conf.json    # Tauri config
│       └── icons/icon.ico
```

## Tests

```
cargo test --workspace    # 29 tests
  sidecar: 6 integration  (health, config, spritesheet serving)
  renderer: 23 unit       (sprite 5, animation 8, bridge 7, client 3)
```

## Build & Run

### Sidecar (WSL)
```bash
cargo build -p hermes-webui-companion-sidecar --release
# Auto-starts via systemd:
systemctl --user status hermes-webui-companion-sidecar
```

### Renderer GUI (Windows host)
```powershell
$env:CARGO_TARGET_DIR = "C:\tmp\companion-target"
cd crates\renderer
cargo tauri dev --features gui
```

### Tests only (WSL)
```bash
cargo test --workspace
```

## Key Design Decisions

- **CSS background-position over Canvas**: avoids clearRect blink and after-image artifacts. Same approach as reference project.
- **Feature-gated Tauri**: `gui` feature keeps Tauri deps optional — tests run without webkit2gtk in WSL.
- **Bridge + Sidecar split**: sidecar (:17888) serves static pet data; bridge (:17787) handles real-time state. Keeps concerns separated.
- **HTTP fallback for IPC**: frontend `invokeTauri()` falls back to direct HTTP when `__TAURI__` unavailable, ensuring state polling works regardless of Tauri injection status.
- **Per-state frame counts**: matches actual petdex spritesheet layouts (idle=6, running=6, waving=4, etc.), not uniform 8.

## Companion State Mapping

| WebUI State | Animation | Trigger |
|-------------|-----------|---------|
| `idle` | idle | No activity |
| `running` | running | Agent processing |
| `approval` (attention) | waiting | Pending approval |
| `clarify` (attention) | review | Pending clarification |
| `ready` | idle | Session completed (bubbles handle notification) |

## Dependencies

- **Rust**: stable, edition 2024
- **Tauri**: v2 (gui feature only, Windows)
- **Sidecar**: axum, tokio, serde_yaml_ng (maintained serde_yaml fork)
- **Renderer**: ureq (with json feature), image, serde
- **Frontend**: vanilla HTML/CSS/JS (no npm)
