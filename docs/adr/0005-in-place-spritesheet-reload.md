# In-place spritesheet reload over process restart after pet switch

**Status:** accepted

After selecting a new pet, the renderer must display the new spritesheet. We chose in-place reload (fetch new spritesheet, update CSS `background-image`, reset animation to idle) over a full Tauri process restart. Restart flickers, reconnects to sidecar, and resets all state — a worse UX. The `switch_pet` Tauri command wraps both the sidecar `select` call and spritesheet fetch into a single IPC round-trip.

**Considered:** process restart (simpler code, but heavy UX cost — flicker, reconnection delay, state loss).

**Consequence:** more complex error handling: if spritesheet fetch fails after successful `hermes pets select`, the renderer shows an error indicator with text until recovery or next successful switch.
