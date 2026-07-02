# Tauri native context menu via `popup_menu`

**Status:** accepted
**Supersedes:** custom HTML/CSS context menu (reversed — native was achievable after all)

The pet window has `decorations: false` (frameless). Initially we assumed native OS menus were impossible without a title bar, so we built a custom HTML/CSS context menu. Later we discovered Tauri v2's `window.popup_menu()` works independently of window decorations — the menu is rendered natively by the OS at the cursor position, not attached to a menu bar.

We switched to `MenuBuilder` + `SubmenuBuilder` + `CheckMenuItemBuilder` in Rust, with the frontend intercepting `contextmenu` and calling `invokeTauri("show_context_menu")`. This gives us truly native OS context menus with platform accessibility, keyboard navigation, and zero custom rendering code.

**Considered:** custom HTML/CSS (built and discarded — worked but lacked accessibility, required manual dismiss logic, frosted glass styling was fragile across DPI changes).

**Consequence:** "Switch pet" submenu is built at startup from the sidecar's `GET /api/pets` response, then static — does not refresh if pets are installed while the renderer is running. A restart picks up new pets.
