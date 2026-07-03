# Direct filesystem access when Hermes and renderer share a host

**Status:** proposed

When Hermes Agent and the renderer run on the same host (no WSL boundary), the sidecar is unnecessary — the renderer can read `~/.hermes/pets/` and `~/.hermes/config.yaml` directly from the local filesystem. We introduce a `PetDataProvider` trait with two implementations: `DirectClient` (filesystem reads) and `SidecarClient` (HTTP to sidecar). At startup, the renderer auto-detects which mode to use: if `$HERMES_HOME/config.yaml` is readable, use direct mode; otherwise fall back to sidecar mode. Mode is static for the process lifetime. `HERMES_HOME` env var overrides the default path. Pet selection in direct mode runs `hermes pets select <slug>` via subprocess (try PATH first, fallback to venv).

**Considered:** always requiring the sidecar (forces WSL dependency even on native hosts); runtime mode switching (risk of flicker and edge cases); direct `config.yaml` writes for pet selection (violates ADR-0004 — Hermes CLI remains the sole writer).

**Consequence:** bridge server health check moves from sidecar to WebUI (`:8787`). Two flags drive `Failed` animation: `webui_healthy` + `pet_data_available`. New `HERMES_WEBUI_PORT` env var controls the health check target. `DirectClient` needs platform-aware default paths (`~/.hermes` on Linux/macOS, `%LOCALAPPDATA%\hermes\` on Windows).
