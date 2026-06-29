# PRD: Hermes Pet Desktop Renderer

**Status:** In Progress  
**Date:** 2026-06-27 (updated 2026-06-29)  
**Author:** Latif + Hermes

---

## Problem Statement

Hermes Agent has a rich pet system (`hermes pets`) that installs animated companion spritesheets from [petdex.dev](https://petdex.dev) and renders them in the terminal (CLI/TUI) and the Hermes desktop (Electron) app. The [Desktop Companion extension](https://github.com/hermes-webui/hermes-webui-extensions/tree/main/extensions/desktop-companion) for Hermes WebUI is only a bridge — it monitors WebUI sessions and POSTs snapshots to a local loopback sidecar. It does NOT render pets by itself. To see a pet on the desktop, the user must also run a Node.js sidecar + a Tauri native shell from a separate repo (`franksong2702/hermes-webui-desktop-companion`). This three-tier architecture (bridge → Node.js sidecar → Tauri shell) is complex, has no direct integration with Hermes' own pet system, and requires the user to manage two separate pet skin directories.

The user runs Hermes Agent and Hermes WebUI inside WSL (Arch Linux), and wants a native Windows desktop pet renderer that:

1. Reads pets directly from the Hermes pet directory inside WSL (`~/.hermes/pets/<slug>/`)
2. Respects Hermes' active pet configuration (`display.pet.slug` in `~/.hermes/config.yaml`)
3. Renders the pet with transparency and always-on-top window behavior
4. Reacts to agent state (idle, running, reviewing, error, done) based on WebUI session snapshots
5. Is a single, self-contained native application — no Node.js dependency

## Solution

A Rust workspace with two binaries:

- **`hermes-webui-companion-sidecar`** — A tiny HTTP server running inside WSL that bridges the filesystem boundary. It serves Hermes pet configuration (`GET /api/pet/active`) and spritesheet files (`GET /pets/{slug}/spritesheet.webp`) to the Windows host via `localhost`.
- **`hermes-webui-companion-renderer`** — A Tauri v2 desktop application running natively on Windows that:
  - Fetches the active pet config and spritesheet from the WSL sidecar at startup
  - Caches spritesheets locally in `%APPDATA%\hermes-pet\cache\`
  - Renders an animated, transparent, always-on-top desktop pet using CSS sprites
  - Listens for WebUI state snapshots (via the companion-adapter.js protocol) to drive animation states
  - Shows bubble notifications for session attention, completions, approvals, and clarify prompts
  - Uses `tiny_http` for its internal bridge server (no async runtime needed)

The spritesheet format is the Codex Pet standard — 192×208px frames in an 8×9 grid with 9 animation states — which is shared identically by petdex.dev, hermes-agent, and the Desktop Companion extension.

## Architecture

```
┌─ Windows Host ────────────────────────────────────────────┐
│                                                           │
│  🦀 hermes-webui-companion-renderer (Tauri v2)            │
│  ├─ Spritesheet cache (%APPDATA%/hermes-webui-companion/) │
│  ├─ Animation state machine (centralized in Rust)         │
│  ├─ CSS sprite renderer (transparent window)              │
│  ├─ Bubble overlay (notifications, separate window)       │
│  ├─ Bridge server (tiny_http, :17787)                     │
│  └─ SidecarClient (ureq → sidecar :17888)                 │
│         │                 │                               │
│         │ HTTP            │ HTTP                          │
│         ▼                 ▼                               │
│  ┌─ WSL ───────────────────────────────────────┐          │
│  │                                             │          │
│  │  🦀 hermes-webui-companion-sidecar (:17888) │          │
│  │  ├─ GET /health → {"ok":true}               │          │
│  │  ├─ GET /api/pet/active → {slug, url}       │          │
│  │  └─ GET /pets/{slug}/spritesheet.webp       │          │
│  │         │                                   │          │
│  │         ▼                                   │          │
│  │  ~/.hermes/config.yaml                      │          │
│  │  ~/.hermes/pets/<slug>/spritesheet.webp     │          │
│  │                                             │          │
│  │  Hermes WebUI (:8787)                       │          │
│  │  └─ companion-adapter.js                    │          │
│  │       → POST snapshots to :17787            │          │
│  └─────────────────────────────────────────────┘          │
└───────────────────────────────────────────────────────────┘
```

## User Stories

1. As a Hermes user, I want to see my active Hermes pet rendered on my Windows desktop.
2. As a Hermes user, I want the pet to automatically match whatever pet I selected with `hermes pets select <slug>`.
3. As a Hermes user, I want the pet to animate differently based on what Hermes is doing (idle, running, reviewing, error).
4. As a Hermes user, I want the pet window to be transparent and always-on-top.
5. As a Hermes user, I want to see notification bubbles from the pet when sessions need attention.
6. As a Hermes user running WSL, I want the pet renderer to work without manual filesystem path hacks.
7. As a developer, I want the sidecar to auto-start with WSL as a systemd user service.
8. As a first-time user, I want to install the pet renderer once and have it auto-detect my Hermes setup.
9. As a user who switches pets frequently, I want the renderer to respect config changes without restarting.

## Implementation Decisions

### Workspace Structure

Single Cargo workspace with two crates:

- `crates/sidecar/` — `hermes-webui-companion-sidecar` binary (WSL, axum)
- `crates/renderer/` — `hermes-webui-companion-renderer` binary (Tauri v2 on Windows)

### Renderer Module Structure

The renderer binary (`gui.rs`) was extracted from a 231-line god-file into focused modules:

| Module        | Responsibility                                                                   |
| ------------- | -------------------------------------------------------------------------------- |
| `gui.rs`      | Thin orchestrator (~92 lines) — wires state, Tauri builder, window events        |
| `commands.rs` | 6 Tauri IPC command handlers                                                     |
| `health.rs`   | Sidecar health check (initial + 10s polling via `SidecarClient::check_health()`) |
| `bubble.rs`   | Bubble window positioning + visibility polling                                   |
| `debug.rs`    | Shared `debug!` macro (gated by `HERMES_COMPANION_DEBUG=1`) + constants          |

### Tauri v2 Configuration

- **`withGlobalTauri: true`** — Required in Tauri v2 to inject `__TAURI__` as a global. Without this, `window.__TAURI__` is `undefined` and Tauri IPC does not work. This is a breaking change from Tauri v1.
- **`csp: null`** — CSP disabled. Desktop-only app; process-level security suffices. Removes blockers for Tauri IPC injection and DevTools.
- **`devtools: true`** — Enabled on the main window for development debugging.

### Sidecar API Contract

The sidecar exposes a minimal HTTP API on `127.0.0.1:17888`:

```
GET /health
  → 200 {"ok": true, "service": "hermes-webui-companion-sidecar"}

GET /api/pet/active
  → 200 {"slug": "boba", "spritesheet_url": "/pets/boba/spritesheet.webp", "display_name": "Boba"}
  → 404 {"error": "no_active_pet"}

GET /pets/{slug}/spritesheet.webp
  → 200 binary/webp
  → 404
```

Config resolution: read `~/.hermes/config.yaml` → `display.pet.slug` and `display.pet.enabled`.

### SidecarClient & Health Check

All sidecar communication flows through `SidecarClient` (ureq sync HTTP). Health checking uses `check_health()` which hits `GET /health` and verifies `{"ok":true}`, catching HTTP-layer failures that a raw TCP connect would miss.

### Renderer ↔ WebUI Bridge Protocol

The renderer listens on `127.0.0.1:17787` using `tiny_http` (synchronous, no async runtime) for HTTP POST snapshots from the WebUI companion-adapter.js:

```json
{
  "source": "hermes-webui",
  "timestamp": "ISO8601",
  "companion": {
    "state": "running" | "ready" | "idle",
    "attention": [
      {
        "session_id": "...",
        "title": "...",
        "status": "running" | "ready" | "action_required",
        "text": "last message excerpt",
        "action_required_type": "approval" | "clarify"
      }
    ]
  }
}
```

CORS is handled with full headers (Allow-Origin, Allow-Methods, Allow-Headers) and OPTIONS preflight returns 204.

### Animation State Machine

The priority chain `sidecar down > Failed > Approval > Clarify > agent state` is centralized in `animation.rs::resolve_animation_state()`. The `GET /api/state` endpoint returns a `resolved_animation` field so frontends never re-implement priority logic. A `sidecar_healthy` AtomicBool flag prevents race conditions where incoming WebUI snapshots overwrite a health-check-triggered Failed state.

Animation state mapping — priority: Approval > Clarify > agent state:

| Input                              | Pet animation state |
| ---------------------------------- | ------------------- |
| sidecar unreachable                | `failed`            |
| `attention[].status == "approval"` | `waiting`           |
| `attention[].status == "clarify"`  | `review`            |
| `companion.state == "running"`     | `running`           |
| `companion.state == "ready"`       | `waving`            |
| `companion.state == "idle"`        | `idle`              |

### Spritesheet Parsing

Use the `image` crate to decode WebP/PNG spritesheets. Standard layout: 8 columns × 9 rows, 192×208px frames. State-to-row mapping: 0=idle, 1=running-right, 2=running-left, 3=waving, 4=jumping, 5=failed, 6=waiting, 7=running, 8=review.

### Rendering Approach

CSS `background-position` on a `<div>` — zero-blink, zero-canvas. Same approach as reference project.

Window properties via Tauri v2:

- `transparent: true`
- `decorations: false`
- `alwaysOnTop: true`
- `skipTaskbar: true`
- Resizable by user

### Sidecar as systemd Service

The sidecar runs as a systemd user service in WSL:

- Unit file: `~/.config/systemd/user/hermes-webui-companion-sidecar.service`
- Auto-starts with WSL, restarts on failure
- Binds to `127.0.0.1:17888`

### Debug Logging

Set `HERMES_COMPANION_DEBUG=1` for verbose logging. Scoped prefixes:

- `[companion:bridge]` — HTTP bridge server requests/responses
- `[companion:cmd]` — Tauri IPC command invocations
- `[companion:health]` — Sidecar health check state transitions
- `[companion:bubble]` — Bubble window visibility changes

## Testing Decisions

### What Makes a Good Test

- Test external behavior through public interfaces, not implementation details
- Bridge server handlers are pure functions returning `HttpResponse` — tested without HTTP server
- SidecarClient tested against real TCP listeners on random ports
- Animation state machine tested with all priority combinations

### Modules Under Test

1. **Spritesheet parser** — `sprite` — 5 unit tests with minimal PNG fixtures
2. **Animation state machine** — `animation` — 15 tests covering all state transitions + sidecar flag + StateResponse
3. **Snapshot parser** — `bridge` — 7 tests for all WebUI snapshot formats
4. **Sidecar client** — `sidecar_client` — 8 tests (active pet, spritesheet, health check with 5 edge cases)
5. **Bridge server handlers** — `bridge_server` — 14 tests covering all 8 endpoints
6. **Sidecar API** — 6 integration tests with temp `HERMES_HOME`

**Total: 55 tests** (all passing)

### Prior Art

- The [petdex pet format](https://github.com/crafter-station/petdex) defines the canonical spritesheet layout
- The [Desktop Companion adapter](https://github.com/hermes-webui/hermes-webui-extensions/blob/main/extensions/desktop-companion/assets/companion-adapter.js) defines the snapshot protocol
- `tiny_http` chosen over `axum` for the bridge server: synchronous, no async runtime needed in Tauri

## Out of Scope

- Pet rendering inside the browser (WebUI)
- Mobile/tablet support — desktop only (Windows first)
- Pet skin creation or management — `hermes pets` CLI handles this
- Replacing the Desktop Companion extension
- Packaging/deployment/signing — MVP is dev-mode only
- Direct interaction with the pet beyond window dragging
- Multi-monitor aware positioning
- Pet animations beyond the 9 standard states

## Further Notes

- The petdex ecosystem is an open standard — 3278+ pets available. By consuming the standard format directly, we avoid vendor lock-in.
- The WSL sidecar pattern (`localhost` port forwarding) is automatic — no firewall rules needed for localhost.
- Future: if Hermes Agent or WebUI ever runs natively on Windows, the sidecar becomes unnecessary — the renderer can read `~/.hermes/pets/` directly.
- Tauri v2 requires `withGlobalTauri: true` in `tauri.conf.json` to expose `__TAURI__` as a global. Without this, Tauri IPC silently fails and `invoke()` is unavailable to frontend JavaScript.
