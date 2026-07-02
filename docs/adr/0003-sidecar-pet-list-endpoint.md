# `GET /api/pets` sidecar endpoint for pet listing

**Status:** accepted

The renderer needs a list of all installed pets to populate the "Switch pet" submenu. We added a `GET /api/pets` endpoint on the sidecar that reads `~/.hermes/pets/`, parses each `pet.json` for `displayName`, and returns slugs + display names plus the currently active pet slug. The sidecar is already the filesystem bridge — keeping it the single source of truth avoids the renderer needing direct WSL filesystem access.

**Considered:** renderer reading `~/.hermes/pets/` directly via `\\wsl.localhost\` (fragile, slow, requires WSL path translation); hardcoded pet list (stale immediately).

**Consequence:** new API surface (tests, error handling, CORS). Malformed `pet.json` in one directory must not break the whole list — graceful skip required.
