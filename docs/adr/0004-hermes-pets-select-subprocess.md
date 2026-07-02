# Pet selection via `hermes pets select` subprocess

**Status:** accepted

When the user picks a new pet from the context menu, the selection must persist across renderer restarts. The canonical persistence mechanism is `hermes pets select <slug>`, which writes `display.pet.slug` and `display.pet.enabled` to `~/.hermes/config.yaml`. We chose to have the sidecar shell out to this CLI command rather than write `config.yaml` directly — the CLI is the sole writer, avoiding config format assumptions and race conditions.

**Considered:** direct YAML write from sidecar (couples sidecar to config schema, risks format drift with future Hermes versions).

**Consequence:** `hermes` must be on PATH inside WSL. Subprocess overhead (~50ms) is negligible for a menu click. Error surface: CLI not installed, invalid slug, config locked.
