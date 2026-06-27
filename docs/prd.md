# PRD: Hermes Pet Desktop Renderer

**Status:** Draft  
**Date:** 2026-06-27  
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

- **`hermes-pet-sidecar`** — A tiny HTTP server running inside WSL that bridges the filesystem boundary. It serves Hermes pet configuration (`GET /api/pet/active`) and spritesheet files (`GET /pets/{slug}/spritesheet.webp`) to the Windows host via `localhost`.
- **`hermes-pet-renderer`** — A Tauri desktop application running natively on Windows that:
  - Fetches the active pet config and spritesheet from the WSL sidecar at startup
  - Caches spritesheets locally in `%APPDATA%\hermes-pet\cache\`
  - Renders an animated, transparent, always-on-top desktop pet using native 2D graphics
  - Listens for WebUI state snapshots (via the companion-adapter.js protocol) to drive animation states
  - Shows bubble notifications for session attention, completions, approvals, and clarify prompts

The spritesheet format is the Codex Pet standard — 192×208px frames in an 8×9 grid with 9 animation states (idle, running-right, running-left, waving, jumping, failed, waiting, running, review) — which is shared identically by petdex.dev, hermes-agent, and the Desktop Companion extension.

## Architecture

```
┌─ Windows Host ──────────────────────────────────┐
│                                                   │
│  🦀 hermes-pet-renderer (Tauri)                   │
│  ├─ Spritesheet cache (%APPDATA%/hermes-pet/)     │
│  ├─ Animation engine (state machine)              │
│  ├─ wgpu/pixels 2D renderer (transparent window)  │
│  ├─ Bubble overlay (notifications)                │
│  └─ HTTP listener (WebUI bridge, :17899)          │
│         │                                         │
│         │ HTTP (localhost)                         │
│         ▼                                         │
│  ┌─ WSL ─────────────────────────────────────┐   │
│  │                                            │   │
│  │  🦀 hermes-pet-sidecar (:17888)            │   │
│  │  ├─ GET /api/pet/active → {slug, url}      │   │
│  │  └─ GET /pets/{slug}/spritesheet.webp      │   │
│  │         │                                   │   │
│  │         ▼                                   │   │
│  │  ~/.hermes/config.yaml                      │   │
│  │  ~/.hermes/pets/<slug>/spritesheet.webp     │   │
│  │                                            │   │
│  │  Hermes WebUI (:8787)                      │   │
│  │  └─ companion-adapter.js                   │   │
│  │       → POST snapshots to :17899           │   │
│  └────────────────────────────────────────────┘   │
└───────────────────────────────────────────────────┘
```

## User Stories

1. As a Hermes user, I want to see my active Hermes pet rendered on my Windows desktop, so that I have a visual companion while working.
2. As a Hermes user, I want the pet to automatically match whatever pet I selected with `hermes pets select <slug>`, so that I don't need to configure the pet in two places.
3. As a Hermes user, I want the pet to animate differently based on what Hermes is doing (idle, running, reviewing, error), so that I can glance at it and know the agent's state.
4. As a Hermes user, I want the pet window to be transparent and always-on-top, so that it floats over my workspace without obstructing it.
5. As a Hermes user, I want to see notification bubbles from the pet when sessions need attention (completions, approvals, clarify prompts), so that I don't miss important Hermes moments.
6. As a Hermes user running WSL, I want the pet renderer to work without manual filesystem path hacks (`\\wsl$\`), so that setup is reliable across distros.
7. As a developer, I want the sidecar to auto-start with WSL as a systemd user service, so that the pet renderer always has a source to fetch from.
8. As a first-time user, I want to install the pet renderer once and have it auto-detect my Hermes setup, so that onboarding takes under 2 minutes.
9. As a user who switches pets frequently, I want the renderer to poll for config changes or accept a refresh signal, so that I don't need to restart the app after `hermes pets select`.

## Implementation Decisions

### Workspace Structure

Single Cargo workspace with two crates:
- `crates/sidecar/` — `hermes-pet-sidecar` binary (WSL)
- `crates/renderer/` — `hermes-pet-renderer` binary (Tauri on Windows)

### Sidecar API Contract

The sidecar exposes a minimal HTTP API on `127.0.0.1:17888`:

```
GET /api/pet/active
  → 200 { "slug": "boba", "spritesheet_url": "/pets/boba/spritesheet.webp", "display_name": "Boba" }
  → 404 { "error": "no_active_pet" }  (no pet selected or `display.pet.enabled: false`)

GET /pets/{slug}/spritesheet.webp
  → 200 binary/webp
  → 404  (pet not installed)

GET /health
  → 200 { "ok": true, "service": "hermes-pet-sidecar" }
```

Config resolution: read `~/.hermes/config.yaml` → `display.pet.slug` and `display.pet.enabled`. If slug is empty, pick the first installed pet from `~/.hermes/pets/*/`. Respect `display.pet.enabled: false` by returning 404.

### Renderer ↔ WebUI Bridge Protocol

The renderer listens on `127.0.0.1:17899` for HTTP POST snapshots from the WebUI companion-adapter.js. The protocol mirrors the existing Desktop Companion snapshot format:

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
        "status": "running" | "ready" | "approval" | "clarify",
        "text": "last message excerpt"
      }
    ]
  }
}
```

Animation state mapping:
| Snapshot `companion.state` | Pet animation state |
|---------------------------|-------------------|
| `idle` | `idle` |
| `running` | `running` (with tool activity) |
| `ready` | `waving` (session complete) |
| `attention[].status == "approval"` | `waiting` |
| `attention[].status == "clarify"` | `review` |
| Error / disconnected | `failed` |

### Spritesheet Parsing

Use the `image` crate to decode WebP/PNG spritesheets. The standard layout:
- 8 columns × 9 rows = 72 frames
- Each frame: 192×208 pixels
- State-to-row mapping: 0=idle, 1=running-right, 2=running-left, 3=waving, 4=jumping, 5=failed, 6=waiting, 7=running, 8=review

Cache spritesheets in `%APPDATA%\hermes-pet\cache\<slug>\` to avoid re-fetching on every launch. Check on startup if the active slug changed — if so, fetch the new one.

### Rendering Approach

First choice: `pixels` crate with `tiny-skia` for frame compositing. Simpler API than raw `wgpu` for 2D sprites. Fallback: `softbuffer` + `tiny-skia` if `pixels` integration with Tauri is problematic.

Window properties via Tauri:
- `transparent: true`
- `decorations: false`
- `always_on_top: true`
- `skip_taskbar: true` (where supported)
- Resizable by user (drag corners)

### Sidecar as systemd Service

The sidecar runs as a systemd user service in WSL:
- Unit file: `~/.config/systemd/user/hermes-pet-sidecar.service`
- Auto-starts with WSL
- Restarts on failure
- Binds to `127.0.0.1:17888`

### Language and Tone

- Renderer UI strings: English (matching Hermes default)
- Future: respect WebUI locale for bubble notifications

## Testing Decisions

### What Makes a Good Test

- Test external behavior, not implementation details
- Spritesheet parsing: verify correct frame extraction from known test fixtures
- Animation state machine: verify state transitions given snapshot inputs
- Sidecar API: HTTP integration tests against a temp Hermes home directory
- No tests that depend on live WSL paths or running WebUI

### Modules Under Test

1. **Spritesheet parser** — `hermes-pet-renderer::sprite` — unit tests with a minimal 1-frame WebP fixture
2. **Animation state machine** — `hermes-pet-renderer::animation` — unit tests for all 6 state transitions
3. **Sidecar API** — `hermes-pet-sidecar` — integration tests with temp `HERMES_HOME`
4. **Config reader** — `hermes-pet-sidecar::config` — unit tests with fixture YAML files

### Prior Art

- The [petdex pet format](https://github.com/crafter-station/petdex) defines the canonical spritesheet layout
- The [Desktop Companion adapter](https://github.com/hermes-webui/hermes-webui-extensions/blob/main/extensions/desktop-companion/assets/companion-adapter.js) defines the snapshot protocol we mirror
- Hermes petdex skill (`~/.hermes/skills/productivity/petdex/SKILL.md`) documents the Hermes-side pet management

## Out of Scope

- Pet rendering inside the browser (WebUI). The Desktop Companion extension already handles the bridge side.
- Mobile/tablet support — desktop only (Windows first, macOS/Linux later).
- Pet skin creation or management — `hermes pets` CLI handles this.
- Replacing the Desktop Companion extension — we reuse its bridge adapter.
- Packaging/deployment/signing — MVP is dev-mode only.
- Direct interaction with the pet (click-to-dismiss, drag-to-move) — basic window dragging is Tauri default, no custom interaction in MVP.
- Multi-monitor aware positioning — pet starts at default position, user drags it.
- Pet animations beyond the 9 standard states (no custom looping, no idle variants).

## Further Notes

- The petdex ecosystem is an open standard — 3278+ pets available, community-maintained. By consuming the standard format directly, we avoid vendor lock-in.
- The WSL sidecar pattern (`localhost` port forwarding) is how WSL exposes Linux services to Windows. It's automatic — no firewall rules needed for localhost.
- The Tauri app should detect if the sidecar is unreachable at startup and show a clear "waiting for Hermes" state (pet sleeping or a status indicator).
- Future: if Hermes Agent or WebUI ever runs natively on Windows, the sidecar becomes unnecessary — the renderer can read `~/.hermes/pets/` directly.
