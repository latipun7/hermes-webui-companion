# Glossary — Switch Pet Context Menu

Domain vocabulary for the switch pet feature.

---

| Term                            | Definition                                                                                   | Applies When                                                                         |
| ------------------------------- | -------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------ |
| **Switch pet**                  | Changing the active desktop companion from one installed pet to another                      | User right-clicks → submenu → selects a different pet                                |
| **Submenu**                     | A nested menu that appears when hovering or clicking a parent menu item with a `▸` indicator | "Switch pet" expands to show a list of installed pets                                |
| **Slug**                        | The unique folder name for a pet in `~/.hermes/pets/` (e.g., `doraemon`, `nika`)             | Used as the internal identifier; visible in URLs and file paths                      |
| **`displayName`**               | Human-readable pet name from `pet.json` (e.g., "Doraemon", "Nika")                           | Shown in the submenu; distinct from the technical slug                               |
| **`pet.json`**                  | Metadata file inside each pet folder: `{id, displayName, description, spritesheetPath}`      | Read by sidecar to build the pet list for the submenu                                |
| **`hermes pets select <slug>`** | Hermes CLI command that sets the active pet by writing `display.pet.*` to `config.yaml`      | Called by sidecar via subprocess when user selects a new pet                         |
| **In-place reload**             | Updating the displayed spritesheet without restarting the Tauri process                      | After switch, new spritesheet bytes are sent to the frontend and applied immediately |
| **Checkmark**                   | A visual indicator (✓ or radio dot) next to the currently active pet in the submenu          | Helps user see which pet is active without opening a separate settings panel         |
| **Error indicator**             | Red "!" with error text shown when spritesheet fails to load after a switch                  | Pet window shows the error until recovery or next successful switch                  |
| **`POST /api/pet/select`**      | New sidecar endpoint that runs `hermes pets select <slug>` and returns the new pet info      | Called by renderer when user clicks a pet in the submenu                             |
| **`GET /api/pets`**             | New sidecar endpoint that lists all installed pets with display names + marks the active one | Called at menu build time to populate the submenu                                    |
| **Sidecar subprocess**          | The sidecar running an external command (`hermes pets select`) via `std::process::Command`   | Used instead of writing `config.yaml` directly, to avoid config format coupling      |
