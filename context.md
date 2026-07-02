# CONTEXT.md — hermes-webui-companion

Domain glossary for the Hermes Pet Desktop Renderer project.
This is the single source of truth for project terminology.
Feature-specific glossaries in `docs/glossary-*.md` are derived snapshots — this file is canonical.

---

## Core Concepts

| Term | Definition |
|------|-----------|
| **Pet** | An animated desktop companion character. Installed as a spritesheet in `~/.hermes/pets/<slug>/`. Defined by the petdex standard (8×9 grid, 192×208px frames). |
| **Slug** | The unique folder name identifying a pet (e.g., `boba`, `doraemon`). Used as the internal identifier in config, file paths, and API routes. |
| **Display Name** | Human-readable pet name from `pet.json` (`displayName` field). Shown in UI menus. Distinct from the technical slug. |
| **Spritesheet** | A single WebP/PNG image containing all animation frames in an 8-column × 9-row grid. Each row corresponds to one animation state. |
| **Sprite** | The rendered visual element on screen — a `<div>` with CSS `background-position` showing one frame from the spritesheet at a time. |
| **Petdex** | The open pet ecosystem standard (petdex.dev). Defines spritesheet layout, `pet.json` format, and the 9 standard animation states. |

## Architecture Components

| Term | Definition |
|------|-----------|
| **Sidecar** | The `hermes-webui-companion-sidecar` binary running inside WSL. An HTTP server (axum, `:17888`) that bridges the WSL filesystem boundary — reads `~/.hermes/pets/` and `~/.hermes/config.yaml`, serves pet data and spritesheets to the renderer via `localhost`. |
| **Renderer** | The `hermes-webui-companion-renderer` binary. A cross-platform Tauri v2 desktop app. Fetches pet data from the sidecar, renders the animated sprite window, listens for WebUI state snapshots, and shows bubble notifications. |
| **Bridge Server** | The HTTP server inside the renderer (`tiny_http`, `:17787`). Receives state snapshots from the WebUI companion-adapter.js. Sometimes called just "bridge" — same thing. |
| **Bridge Protocol** | The JSON-over-HTTP contract between the WebUI companion-adapter.js and the renderer's bridge server. Defines the `WebuiSnapshot` shape with `companion.state` and `companion.attention[]`. |
| **SidecarClient** | The HTTP client inside the renderer (`ureq`). All communication with the sidecar flows through this module. Handles health checks, fetching pet data, and pet selection. |
| **Companion Adapter** | The `companion-adapter.js` script inside Hermes WebUI (`:8787`). Monitors WebUI session state and POSTs snapshots to the renderer's bridge server. NOT part of this project — it's an external dependency. |

## State Machine

| Term | Definition |
|------|-----------|
| **Companion State** | The overall WebUI agent activity level as reported by the companion adapter: `idle`, `running`, `ready`, `failed`. Note: `Failed` in CompanionState means "sidecar unreachable" — distinct from the WebUI snapshot's own state. |
| **Attention Item** | A single session that needs user attention. Has a `status` (`Approval`, `Clarify`, `Running`, `Ready`), optional `text` (message preview), and optional `session_id` for linking back to WebUI. |
| **Attention Status** | The specific type of attention: `Approval` (pending user approval), `Clarify` (pending user clarification), `Running` (session actively processing), `Ready` (session completed). |
| **Animation State** | The resolved animation playing on the pet sprite. One of: `Idle`, `Running`, `RunningRight`, `RunningLeft`, `Waving`, `Jumping`, `Failed`, `Waiting`, `Review`. This is the OUTPUT of the priority resolver — frontends receive `resolved_animation` and never re-compute priority. |
| **Resolved Animation** | The `resolved_animation` string field in `StateResponse`. Computed by `resolve_animation_state()` with this priority chain: **sidecar down → Approval → Clarify → agent state**. Centralized in Rust so no priority logic lives in JavaScript. |
| **Sidecar Healthy** | An `AtomicBool` flag tracking whether the sidecar's `/health` endpoint returns `{"ok":true}`. When `false`, all animation resolves to `Failed` regardless of incoming snapshots — prevents race-condition flicker. |

## Window & UI

| Term | Definition |
|------|-----------|
| **Pet Window** | The main Tauri window (`172×186px`, transparent, always-on-top, frameless). Renders the animated pet sprite via CSS `background-position`. Referred to internally as the `main` window. |
| **Bubble Window** | A separate Tauri window showing notification overlay cards. Positioned relative to the pet window. Contains session attention messages and action buttons. Can be muted/hidden via the bubble toggle. |
| **Bubble Toggle** | A small circular button at the top-right of the pet window (▼/▲). Left-click toggles bubble visibility. Right-click is consumed by the context menu handler. |
| **Context Menu** | The native OS right-click popup on the pet window, built with Tauri's `MenuBuilder` + `SubmenuBuilder` and shown via `window.popup_menu()`. Items: Switch pet (submenu), Restart pet, Close pet. Platform-native — gets accessibility, keyboard navigation, and OS-consistent styling for free. |
| **Frosted Glass** | Visual style: semi-transparent dark background (`rgba(18,18,22,0.92)`) with `backdrop-filter: blur(12px)`. Used on bubble window cards. |
| **Drag Animation** | Post-drag feedback animation. During OS-level window drag, the WebView JS thread is paused. The renderer accumulates horizontal delta in an `AtomicI32` and plays a 500ms `RunningRight`/`RunningLeft` animation after drag ends. |

## Actions

| Term | Definition |
|------|-----------|
| **Switch Pet** | Changing the active pet to a different installed one. Triggers `POST /api/pet/select` on the sidecar (which runs `hermes pets select <slug>`), then fetches the new spritesheet and reloads in-place without restarting the Tauri process. |
| **Heavy Restart** | Killing and re-launching the entire Tauri process via `tauri-plugin-process::restart()`. Used by "Restart pet" menu item. Resets all state: reconnects to sidecar, re-fetches pet data, restarts animations. |
| **Close** | Terminating the Tauri application via `app_handle.exit(0)`. Kills both pet window and bubble window atomically. |
| **Bubble Mute** | Hiding the bubble and suppressing state-driven animation changes. Set via the bubble toggle. Animation stays idle until the companion state actually changes (not just re-polled with same value). |

## Spritesheet Layout

| Term | Definition |
|------|-----------|
| **Frame** | A single 192×208px image within the spritesheet grid. Indexed by (row, col). |
| **Row** | A spritesheet row (0-8), each corresponding to one animation state. Row mapping: 0=idle, 1=running-right, 2=running-left, 3=waving, 4=jumping, 5=failed, 6=waiting, 7=running, 8=review. |
| **Frame Count** | Number of frames per animation row. Varies per state: idle=6, running-right=6, running-left=6, waving=4, jumping=0 (unused), failed=2, waiting=4, running=6, review=4. |

## API Endpoints

| Endpoint | Owner | Purpose |
|----------|-------|---------|
| `GET /health` | Sidecar (`:17888`) | Health check — returns `{"ok":true}` if sidecar is operational |
| `GET /api/pet/active` | Sidecar | Returns the currently active pet's slug, display name, and spritesheet URL |
| `GET /api/pets` | Sidecar | Lists all installed pets with display names + marks the active one |
| `POST /api/pet/select` | Sidecar | Runs `hermes pets select <slug>` to change the active pet |
| `GET /pets/{slug}/spritesheet.webp` | Sidecar | Serves the spritesheet file for a given pet slug |
| `POST /` | Bridge Server (`:17787`) | Receives WebUI companion snapshots from the companion-adapter.js |
| `GET /api/state` | Bridge Server | Returns the current resolved animation state to the frontend |
