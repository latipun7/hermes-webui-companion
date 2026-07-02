# `tauri-plugin-process` for pet restart

**Status:** accepted

"Restart pet" kills and re-launches the Tauri process. We chose the official `tauri-plugin-process` crate (`restart()` API) over manually spawning `current_exe` + `exit(0)`. The plugin handles platform-specific edge cases (process group cleanup on Windows) that a raw `Command::new(current_exe).spawn()` + `std::process::exit(0)` risks racing on.

**Considered:** manual spawn+exit (zero deps but potential race between spawn and exit on Windows).

**Consequence:** ~200KB binary size increase. Plugin API may drift with Tauri v2 updates — currently stable in v2.x.
