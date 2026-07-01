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
├── Cargo.toml                 # workspace root — lints, edition, deps
├── rust-toolchain.toml        # stable + rust-analyzer
├── rustfmt.toml               # formatting config (stable-only)
├── Makefile                   # convenience: fmt, clippy, test, ci, setup-hooks
├── .githooks/
│   └── pre-commit             # fmt --check + clippy -D warnings
├── .github/workflows/
│   └── ci.yml                 # fmt → clippy → test (all-targets, all-features, locked)
├── docs/
│   ├── prd.md                 # full PRD
│   ├── adr-switch-pet.md      # architecture decision: Switch pet
│   ├── glossary-switch-pet.md # glossary for Switch pet feature
│   ├── issues-switch-pet.md   # tracking issues for Switch pet
│   ├── adr-right-click-menu.md
│   ├── glossary-right-click-menu.md
│   ├── issues-right-click-menu.md
│   └── hermes-webui-companion-sidecar.service
├── crates/
│   ├── sidecar/               # hermes-webui-companion-sidecar
│   │   └── src/main.rs        # axum HTTP server, integration tests
│   └── renderer/              # hermes-webui-companion-renderer
│       ├── src/
│       │   ├── lib.rs         # library crate root
│       │   ├── main.rs        # stub binary (version)
│       │   ├── gui.rs         # thin orchestrator — Tauri setup + event wiring
│       │   ├── commands.rs    # Tauri IPC command handlers
│       │   ├── health.rs      # sidecar health check logic
│       │   ├── bubble.rs      # bubble positioning + visibility
│       │   ├── debug.rs       # debug! macro + ASPECT_RATIO
│       │   ├── sprite.rs      # spritesheet parser
│       │   ├── animation.rs   # state machine
│       │   ├── bridge.rs      # snapshot parser
│       │   ├── bridge_server.rs # HTTP bridge + tests
│       │   └── sidecar_client.rs # HTTP client + tests
│       ├── gui/               # Tauri frontend
│       │   ├── index.html     # CSS sprite window
│       │   ├── pet.js         # animation + state polling
│       │   ├── bubbles.html   # bubble card window
│       │   └── bubbles.js     # bubble content + polling
│       ├── tauri.conf.json    # Tauri v2 config
│       ├── capabilities/
│       │   └── default.json   # IPC permissions
│       └── icons/
│           ├── icon.ico
│           └── icon.png
```

## Tests

```bash
cargo test --locked --workspace --all-targets --all-features
```

## Code Quality Enforcement

### Workspace Lints (Cargo.toml)

```
[workspace.lints.rust]
unsafe_code              = deny
rust_2018_idioms         = deny
rust_2024_compatibility  = deny

[workspace.lints.clippy]
all                      = warn
```

All crates inherit via `[lints] workspace = true` in their Cargo.toml.

### Pre-commit Hook

`.githooks/pre-commit` — runs on every `git commit`:

```bash
cargo fmt --check --all
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
```

Install: `git config core.hooksPath .githooks`

### CI (GitHub Actions)

`.github/workflows/ci.yml` — runs on push/PR to `main`:

```
  code-quality:
    cargo fmt --check --all
    cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
    cargo test --locked --workspace --all-features
```

Uses: emoji steps, pinned action SHAs, cargo cache, concurrency cancel-in-progress.

### Makefile

```
make fmt       → cargo fmt --all
make clippy    → cargo clippy --locked ... --all-features -- -D warnings
make check     → fmt + clippy (clippy covers check internally)
make test      → cargo test --locked ... --all-features
make ci        → check + test
make setup-hooks → git config core.hooksPath .githooks
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
- **`[lints] workspace = true` per crate**: workspace lints are inherited at the crate level via TOML `[lints]` section, not `package.lints.workspace = true` (Cargo 1.96 quirks).
- **`edition.workspace = true` per crate**: edition is set once in `[workspace.package]` and inherited — no duplication.
- **`--locked` + `--all-features` everywhere**: all cargo commands in CI/hooks/Makefile pin dependency versions via `Cargo.lock` and compile with every feature enabled (gui, etc.).

## Companion State Mapping

| WebUI State            | Animation | Trigger                  |
| ---------------------- | --------- | ------------------------ |
| `idle`                 | idle      | No activity              |
| `running`              | running   | Agent processing         |
| `approval` (attention) | waiting   | Pending approval         |
| `clarify` (attention)  | review    | Pending clarification    |
| `ready`                | waving    | Session completed        |
| sidecar down           | failed    | Health check unreachable |

## Drag Animation

During OS-level window drag, the WebView JS thread is paused — animations cannot update in real-time.
Instead, `WindowEvent::Moved` in Rust accumulates the horizontal delta in an `AtomicI32`.
`pollCompanionState()` reads this on each 1s tick and triggers a 500ms running-right/left animation
as post-drag feedback. During the animation, companion state polling is suppressed to prevent flicker.

## Bubble Mute

Clicking the bubble toggle button hides the bubble and sets `userMuted = true`. The pet animation
switches to idle and stays idle until the companion state **actually changes** (not just re-polled
with the same value). This prevents the animation from flickering back to the companion state
immediately after hiding the bubble.

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
- **Windows**: `#![windows_subsystem = "windows"]` suppresses console window on release builds
- **Cross-platform**: `open_webui` uses `cmd /c start` (Windows), `open` (macOS), or `xdg-open` (Linux)
