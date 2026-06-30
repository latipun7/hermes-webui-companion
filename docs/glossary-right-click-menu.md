# Glossary — Right-Click Context Menu

Domain vocabulary for the pet window context menu feature.

---

| Term | Definition | Applies When |
|------|-----------|-------------|
| **Context menu** | A popup menu that appears on right-click, offering actions related to the clicked element or window | User right-clicks on the pet window |
| **Heavy restart** | Killing the current Tauri process and spawning a fresh one from the same executable | User selects "Restart pet" — reloads spritesheet, reconnects to sidecar, resets all state |
| **Frosted glass** | Visual style using semi-transparent background + backdrop blur, mimicking iOS/macOS glass effect | Applied to context menu background (`rgba(18,18,22,0.92)` + `blur(12px)`) |
| **`contextmenu` event** | DOM event fired when the user right-clicks (or presses the context menu key) | Intercepted on `document` to prevent default OS/browser menus and show custom menu |
| **`preventDefault`** | DOM method that cancels the default browser behavior for an event | Called on `contextmenu` to block WebView2's built-in right-click menu |
| **Auto-flip** | Positioning logic that flips the menu to the opposite side when it would overflow the window boundary | Applied when cursor is near the right or bottom edge of the pet window |
| **`tauri-plugin-process`** | Official Tauri v2 plugin providing `restart()` API for clean process respawning | Used by "Restart pet" command |
| **`app_handle.exit(0)`** | Tauri API that terminates the entire application process with exit code 0 | Used by "Close pet" command — kills both main and bubble windows |
| **Bubble toggle** | Small circular button (▼/▲) at the top-right of the pet window that toggles notification visibility | Left-click behavior unaffected by context menu interception |
| **`-webkit-app-region`** | CSS property marking an element as draggable (`drag`) or interactive (`no-drag`) in frameless windows | Currently `drag` on body, `no-drag` on bubble toggle — context menu overrides apply to both |
| **Segoe UI** | Windows 11 system font used for native UI elements | Used as primary font in context menu styling |
| **WebView2** | Microsoft's Chromium-based webview engine used by Tauri on Windows | Renders the pet window HTML/CSS/JS; its default right-click menu is what we're replacing |
| **Switch skin** | Planned future feature to change the active pet from a list of available pets | Out of scope for MVP, will be implemented in a follow-up session |
| **Permission control** | Planned future feature to toggle pet window behaviors (always-on-top, click-through, etc.) | Out of scope for MVP, skipped per user decision |
