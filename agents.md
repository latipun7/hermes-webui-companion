# hermes-webui-companion

Desktop companion renderer for Hermes Agent — a native Windows pet window
that reacts to Hermes state (idle, running, approval, clarify) in real-time.

## Architecture

```
┌─ WSL ───────────────────────────────────────────────┐
│  hermes-webui-companion-sidecar (:17888)            │
│  → serves pet config + spritesheets                 │
│  → reads ~/.hermes/config.yaml & ~/.hermes/pets/    │
│                                                     │
│  Hermes WebUI (:8787)                               │
│  → companion-adapter.js POSTs snapshots to :17787   │
└─────────────────────────────────────────────────────┘
          │                        │
    HTTP fetch              HTTP POST
          │                        │
┌─ Windows (Tauri v2) ─────────────────────────────────┐
│  hermes-webui-companion-gui                          │
│  ├─ Bridge server (:17787, tiny_http) — receives     │
│  │   snapshots from companion-adapter.js             │
│  ├─ Animation engine — centralized in Rust, returns  │
│  │   resolved_animation field to frontend            │
│  ├─ SidecarClient — fetches pet data via HTTP        │
│  ├─ Health check — SidecarClient::check_health()     │
│  ├─ CSS sprite renderer — zero-blink, zero-canvas    │
│  └─ State polling — 1s interval from bridge          │
│                                                      │
│  Window: 172×186, transparent, always-on-top         │
│  Drag: CSS -webkit-app-region + Tauri IPC fallback   │
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
│       │   ├── lib.rs         # library crate root
│       │   ├── main.rs        # stub binary (version)
│       │   ├── gui.rs         # thin orchestrator (~92 lines)
│       │   ├── commands.rs    # 6 Tauri IPC command handlers
│       │   ├── health.rs      # sidecar health check logic
│       │   ├── bubble.rs      # bubble positioning + visibility
│       │   ├── debug.rs       # debug! macro + ASPECT_RATIO
│       │   ├── sprite.rs      # spritesheet parser (5 tests)
│       │   ├── animation.rs   # state machine (15 tests)
│       │   ├── bridge.rs      # snapshot parser (7 tests)
│       │   ├── bridge_server.rs # HTTP bridge + 14 tests
│       │   └── sidecar_client.rs # HTTP client + 8 tests
│       ├── gui/               # Tauri frontend
│       │   ├── index.html     # CSS sprite window
│       │   ├── pet.js         # animation + state polling
│       │   ├── bubbles.html   # bubble card window
│       │   └── bubbles.js     # bubble content + polling
│       ├── tauri.conf.json    # Tauri v2 config
│       ├── capabilities/
│       │   └── default.json   # IPC permissions
│       └── icons/icon.ico
```

## Tests

```
cargo test --workspace    # 55 tests
  sidecar: 6 integration  (health, config, spritesheet serving)
  renderer: 49 unit       (sprite 5, animation 15, bridge 7,
                           bridge_server 14, client 8)
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
$env:HERMES_COMPANION_DEBUG = "1"   # optional: enable debug logs
$env:CARGO_TARGET_DIR = "C:\tmp\companion-target"
cd crates\renderer
cargo tauri dev --features gui
```

### Tests only (WSL)

```bash
cargo test --workspace
```

## Key Design Decisions

- **CSS background-position over Canvas**: avoids clearRect blink and after-image artifacts.
- **Feature-gated Tauri**: `gui` feature keeps Tauri deps optional — library tests run without webkit2gtk in WSL.
- **Bridge + Sidecar split**: sidecar (:17888) serves static pet data; bridge (:17787) handles real-time state. Keeps concerns separated.
- **`tiny_http` for bridge server**: synchronous, zero mandatory async deps, fits Tauri's sync model. Handler functions are pure and testable without HTTP server. Chosen over `axum` (used by sidecar) because the renderer doesn't need a tokio runtime.
- **Centralized animation priority**: `resolve_animation_state()` in Rust with `sidecar_healthy` AtomicBool flag. Frontends read `resolved_animation` field — no priority logic duplication in JavaScript.
- **`SidecarClient::check_health()`**: application-level health check (hits `GET /health`, verifies `{"ok":true}`) instead of raw TCP connect. Catches HTTP-layer failures that raw TCP misses.
- **Tauri v2 `withGlobalTauri: true`**: required to inject `__TAURI__` as a global. Without this, `window.__TAURI__` is `undefined` — a breaking change from Tauri v1.
- **CSP disabled** (`csp: null`): desktop-only app, process-level security suffices.
- **Per-state frame counts**: matches actual petdex spritesheet layouts (idle=6, running=6, waving=4, etc.).

## Companion State Mapping

| WebUI State            | Animation | Trigger                  |
| ---------------------- | --------- | ------------------------ |
| `idle`                 | idle      | No activity              |
| `running`              | running   | Agent processing         |
| `approval` (attention) | waiting   | Pending approval         |
| `clarify` (attention)  | review    | Pending clarification    |
| `ready`                | waving    | Session completed        |
| sidecar down           | failed    | Health check unreachable |

## Debug Logging

Set `HERMES_COMPANION_DEBUG=1` for verbose logs with scoped prefixes:

```
[companion:bridge]  — HTTP bridge requests, snapshot processing
[companion:cmd]     — Tauri IPC command invocations
[companion:health]  — Sidecar health check state transitions
[companion:bubble]  — Bubble window visibility changes
```

## Dependencies

- **Rust**: stable, edition 2024
- **Tauri**: v2 (gui feature only, Windows)
- **Sidecar**: axum, tokio, serde_yaml_ng
- **Renderer**: ureq (with json feature), image, tiny_http, serde
- **Frontend**: vanilla HTML/CSS/JS (no npm)
