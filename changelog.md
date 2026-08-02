# Changelog

All notable changes to this project will be documented in this file. See [conventional commits](https://www.conventionalcommits.org/) for commit guidelines.

---

## [v0.2.0](https://github.com/latipun7/hermes-webui-companion/compare/e5693e9355a7b22ab1e0902c7d5508edf4ffd2f4..v0.2.0) - 2026-08-02

#### ✨ Features

- (**common**) extract shared config/pet types into crates/common/ (#38) - ([e5693e9](https://github.com/latipun7/hermes-webui-companion/commit/e5693e9355a7b22ab1e0902c7d5508edf4ffd2f4)) - [@latipun7](https://github.com/latipun7)
- enforce prettier & markdown-lint - ([06ae753](https://github.com/latipun7/hermes-webui-companion/commit/06ae7537972c9c574c1f2e42b59473dd946c4cab)) - [@latipun7](https://github.com/latipun7)

#### 📚 Documentation

- (**readme**) fix emoji anchor link targets - ([293e074](https://github.com/latipun7/hermes-webui-companion/commit/293e07406ab1759d843e33cb6e0e8002bcf1a6ae)) - [@latipun7](https://github.com/latipun7)
- add common crate to context.md, agents.md, and README.md - ([2704432](https://github.com/latipun7/hermes-webui-companion/commit/2704432199d5e7c085497e09c923913c1ac9ff31)) - [@latipun7](https://github.com/latipun7)

#### 🏗️ Build System

- (**cog**) update release hooks for tauri gui - ([953447d](https://github.com/latipun7/hermes-webui-companion/commit/953447d27a76702866f74f730b404a3884d829e9)) - [@latipun7](https://github.com/latipun7)
- (**deps**) pin dependencies (#20) - ([53bb211](https://github.com/latipun7/hermes-webui-companion/commit/53bb2118fed905f440ff9faab136ac6bec566ce2)) - renovate[bot], renovate[bot]
- use cargo tauri build for release and dev - ([f7a14f1](https://github.com/latipun7/hermes-webui-companion/commit/f7a14f11a9a90c359726d7e536c0e19fe810a337)) - [@latipun7](https://github.com/latipun7)
- replace Makefile with mise and add cocogitto - ([84c8033](https://github.com/latipun7/hermes-webui-companion/commit/84c80331c081ba64d5f9179d0f20b9d33cd82bdb)) - [@latipun7](https://github.com/latipun7)

#### 👷 Continuous Integration

- (**actions**) update `softprops/action-gh-release` action to v3.0.2 (#40) - ([1f111ae](https://github.com/latipun7/hermes-webui-companion/commit/1f111aec283ede774ed332408f2aabd4997ea752)) - renovate[bot], renovate[bot]
- pin Rust toolchain version to 1.97.1 (stable) across workflows - ([06cbf33](https://github.com/latipun7/hermes-webui-companion/commit/06cbf33c8730f94239aba385e1f93417f72ef009)) - [@latipun7](https://github.com/latipun7)

#### ♻️ Refactoring

- (**renderer**) group shared state into CompanionContext (#39) - ([a8f5cf8](https://github.com/latipun7/hermes-webui-companion/commit/a8f5cf86e80770978c961fbbfb8a61ee26b326ac)) - [@latipun7](https://github.com/latipun7)

---

## [v0.1.0](https://github.com/latipun7/hermes-webui-companion/compare/8942c6bf5a402fa6747f04b7640c4705348ecb9c..v0.1.0) - 2026-07-03

#### ✨ Features

- (**bubbles**) sync toggle button with auto-hide state - ([d5249c3](https://github.com/latipun7/hermes-webui-companion/commit/d5249c34fbf25cb65b7efc4c7ab4313d0642d38f)) - [@latipun7](https://github.com/latipun7)
- (**bubbles**) auto-hide window when idle - ([f4c3daf](https://github.com/latipun7/hermes-webui-companion/commit/f4c3dafbde849b6ecb54835badff69b7939f0635)) - [@latipun7](https://github.com/latipun7)
- (**bubbles**) auto-hide window when no content to display - ([3da6f3e](https://github.com/latipun7/hermes-webui-companion/commit/3da6f3ed76f5ae41b1d5d97463f53d4ec9d19d40)) - [@latipun7](https://github.com/latipun7)
- (**companion**) manual bubble toggle via HTTP bridge - ([bacea55](https://github.com/latipun7/hermes-webui-companion/commit/bacea55a8baebc1ca3a441ac67ff8bd4edf70dbf)) - [@latipun7](https://github.com/latipun7)
- (**renderer**) direct filesystem mode — renderer reads ~/.hermes/ without sidecar (#36) - ([f416376](https://github.com/latipun7/hermes-webui-companion/commit/f416376be9adc506dd45bc9c41217e91b7ee49d2)) - [@latipun7](https://github.com/latipun7)
- (**renderer**) add Switch pet submenu to right-click context menu (#19) - ([53535cf](https://github.com/latipun7/hermes-webui-companion/commit/53535cfa2a819eeb0c2d3293c4fbd016ed3c373b)) - [@latipun7](https://github.com/latipun7)
- (**renderer**) add right-click native context menu to pet window - ([6e9f62a](https://github.com/latipun7/hermes-webui-companion/commit/6e9f62a03f4fc26f9e1f717b746735102ac435a1)) - [@latipun7](https://github.com/latipun7)
- (**renderer**) track drag direction via native window events - ([6dc160c](https://github.com/latipun7/hermes-webui-companion/commit/6dc160ca770c249bc528059cb3e52c3becf75464)) - [@latipun7](https://github.com/latipun7)
- (**renderer**) support cross-platform execution and hide console - ([827d892](https://github.com/latipun7/hermes-webui-companion/commit/827d892b185d931c7be146132967de222df3cc28)) - [@latipun7](https://github.com/latipun7)
- (**renderer**) pet goes idle when bubble manually hidden - ([fb8d8ed](https://github.com/latipun7/hermes-webui-companion/commit/fb8d8ed44fbb883adaac315f40c886a69e0cc231)) - [@latipun7](https://github.com/latipun7)
- (**renderer**) add WebUI health check alongside sidecar - ([2c0c9bc](https://github.com/latipun7/hermes-webui-companion/commit/2c0c9bc57f26eb51e4485b62bb103b20bac42d4a)) - [@latipun7](https://github.com/latipun7)
- (**renderer**) action_required parser + drag animation - ([e61d559](https://github.com/latipun7/hermes-webui-companion/commit/e61d559bc86bf61f955d4c31064271bae216cd7a)) - [@latipun7](https://github.com/latipun7)
- (**renderer**) align pet states to reference project - ([423787f](https://github.com/latipun7/hermes-webui-companion/commit/423787f09ce45f64ec10d83175fd49423125b443)) - [@latipun7](https://github.com/latipun7)
- (**renderer**) delayed browser focus after adapter navigation - ([2b5ea1e](https://github.com/latipun7/hermes-webui-companion/commit/2b5ea1e435d1c6b130916665331c91d0d61c0ec8)) - [@latipun7](https://github.com/latipun7)
- (**renderer**) adapter-driven session navigation via navigation commands - ([ecafdfd](https://github.com/latipun7/hermes-webui-companion/commit/ecafdfd2e99dd1b5a1595b78e335eeb810e4ebf6)) - [@latipun7](https://github.com/latipun7)
- (**renderer**) session-specific bubble click with loading state - ([a539e7e](https://github.com/latipun7/hermes-webui-companion/commit/a539e7e8b5f6c2e63f4361587cf0857d3b186bb2)) - [@latipun7](https://github.com/latipun7)
- (**renderer**) open_webui Tauri command + bubble click via IPC - ([aa3cec1](https://github.com/latipun7/hermes-webui-companion/commit/aa3cec17feb0bc0b1aa6ceb381be355a254dbfc7)) - [@latipun7](https://github.com/latipun7)
- (**renderer**) bubble hover cursor + click to WebUI - ([1808296](https://github.com/latipun7/hermes-webui-companion/commit/180829600d4f2a68c6bbd9babcf536981fd77a1e)) - [@latipun7](https://github.com/latipun7)
- (**renderer**) monitor-aware bubble positioning - ([dea5875](https://github.com/latipun7/hermes-webui-companion/commit/dea58752252115cc337e003fbb468616f5af9939)) - [@latipun7](https://github.com/latipun7)
- (**renderer**) separate bubbles window above pet - ([7214eea](https://github.com/latipun7/hermes-webui-companion/commit/7214eea06afd7cf79fafb80cb2dc1e57c3f59b5d)) - [@latipun7](https://github.com/latipun7)
- (**renderer**) bubble text preview + click-to-webui + lowercase serialization - ([e1e914d](https://github.com/latipun7/hermes-webui-companion/commit/e1e914dbe5f67e67fb1daac7eb3883b282ef36ee)) - [@latipun7](https://github.com/latipun7)
- (**renderer**) notification bubbles overlay - ([17936d5](https://github.com/latipun7/hermes-webui-companion/commit/17936d5e9670b242ac739a9a98c18d68fc8a2e0c)) - [@latipun7](https://github.com/latipun7)
- (**renderer**) WebUI state bridge — real-time companion state - ([abba1a3](https://github.com/latipun7/hermes-webui-companion/commit/abba1a322ec5b7169bb8015c1fd7b68c01416b2a)) - [@latipun7](https://github.com/latipun7)
- (**renderer**) lock window resize to spritesheet aspect ratio - ([b8f1f07](https://github.com/latipun7/hermes-webui-companion/commit/b8f1f0725e1ed6b3f52c6163fc8aa31f737ff5d7)) - [@latipun7](https://github.com/latipun7)
- (**renderer**) Tauri GUI integration with feature gate - ([f5c98d4](https://github.com/latipun7/hermes-webui-companion/commit/f5c98d42994f96795b4d1b1313d462cd9234ab98)) - [@latipun7](https://github.com/latipun7)
- (**renderer**) sidecar HTTP client with 3 TDD tests - ([c1c947a](https://github.com/latipun7/hermes-webui-companion/commit/c1c947a82d628982f006117493ea896ecf0531f8)) - [@latipun7](https://github.com/latipun7)
- (**renderer**) animation state machine with 8 TDD tests - ([0231ccd](https://github.com/latipun7/hermes-webui-companion/commit/0231ccd3fcbf7724e2d874fd4607db22ca7f90be)) - [@latipun7](https://github.com/latipun7)
- (**renderer**) sprite module with 5 TDD tests - ([c34e90b](https://github.com/latipun7/hermes-webui-companion/commit/c34e90b5730265d4dae810d9556c0bdfddb6a1fc)) - [@latipun7](https://github.com/latipun7)
- enforce clippy & rustfmt across workspace - ([08bfe97](https://github.com/latipun7/hermes-webui-companion/commit/08bfe9743da8716570282292f3df12dd09dcfb9e)) - [@latipun7](https://github.com/latipun7)
- add Failed state for sidecar unreachable + bubble focus reuse - ([86b2a08](https://github.com/latipun7/hermes-webui-companion/commit/86b2a0889d4f3f64146b2ddebe5b44f409a16d95)) - [@latipun7](https://github.com/latipun7)
- systemd user service for companion sidecar - ([6dfd969](https://github.com/latipun7/hermes-webui-companion/commit/6dfd96968423ea9d64f7c1b961c982f61d543b46)) - [@latipun7](https://github.com/latipun7)
- init hermes-webui-companion monorepo - ([8942c6b](https://github.com/latipun7/hermes-webui-companion/commit/8942c6bf5a402fa6747f04b7640c4705348ecb9c)) - [@latipun7](https://github.com/latipun7)

#### 🐛 Bug Fixes

- (**bridge**) stop opening new tab when browser already running - ([fa41aaf](https://github.com/latipun7/hermes-webui-companion/commit/fa41aafa4a1c0c36a5a92b3f1f15b13a3ae8052f)) - [@latipun7](https://github.com/latipun7)
- (**cache**) unique key per job, fallback to CI broader cache - ([01beec8](https://github.com/latipun7/hermes-webui-companion/commit/01beec83ee08dde2cbd583a0b9ae74819c00146a)) - [@latipun7](https://github.com/latipun7)
- (**release**) add contents:write permission for release job - ([88defd9](https://github.com/latipun7/hermes-webui-companion/commit/88defd983d4111b2f2d03e0b0ff6b4498f07817e)) - [@latipun7](https://github.com/latipun7)
- (**renderer**) fix drag animation loop and mute state tracking - ([02d0e10](https://github.com/latipun7/hermes-webui-companion/commit/02d0e106425aa1a1a000db95d84941d649924d3e)) - [@latipun7](https://github.com/latipun7)
- (**renderer**) use SidecarClient::check_health instead of raw TCP - ([49a109e](https://github.com/latipun7/hermes-webui-companion/commit/49a109e52fec268bd2400e6891464a3ef27b699b)) - [@latipun7](https://github.com/latipun7)
- (**renderer**) enable Tauri v2 IPC, remove JS HTTP fallback - ([3e396c4](https://github.com/latipun7/hermes-webui-companion/commit/3e396c46437892507724a6206ec7b08dfa414b99)) - [@latipun7](https://github.com/latipun7)
- (**renderer**) prevent sidecar health race with snapshot updates - ([8712f33](https://github.com/latipun7/hermes-webui-companion/commit/8712f3356211a205bac09266e589643d1aaaf63f)) - [@latipun7](https://github.com/latipun7)
- (**renderer**) center bubble horizontally above pet - ([3fb5280](https://github.com/latipun7/hermes-webui-companion/commit/3fb52807adfd526f3327e53800c9545b3229585e)) - [@latipun7](https://github.com/latipun7)
- (**renderer**) use session URL for delayed browser focus - ([3673aeb](https://github.com/latipun7/hermes-webui-companion/commit/3673aeb2a124cc89830d123bf1a507867fc5aefa)) - [@latipun7](https://github.com/latipun7)
- (**renderer**) remove browser spawn fallback — adapter path confirmed working - ([158e4fe](https://github.com/latipun7/hermes-webui-companion/commit/158e4fe5b34a8027d05e7c49d123abe523417687)) - [@latipun7](https://github.com/latipun7)
- (**renderer**) dual-path navigation — adapter + cmd fallback - ([b4088f0](https://github.com/latipun7/hermes-webui-companion/commit/b4088f049f9f70954d4f0641ccb020a551d586b3)) - [@latipun7](https://github.com/latipun7)
- (**renderer**) remove unnecessary mut + add nav poll debug log - ([a0c5677](https://github.com/latipun7/hermes-webui-companion/commit/a0c56774ea0d6cae1913bb77c92166ddd3873d07)) - [@latipun7](https://github.com/latipun7)
- (**renderer**) remove explorer.exe fallback, keep only adapter navigation - ([ff2f552](https://github.com/latipun7/hermes-webui-companion/commit/ff2f5528e4ffc144c0295c83fab87ceeadbeeaf8)) - [@latipun7](https://github.com/latipun7)
- (**renderer**) use explorer.exe instead of cmd start - ([f892f8e](https://github.com/latipun7/hermes-webui-companion/commit/f892f8ee90dcd580727b8e2e9c571b120ca6e21b)) - [@latipun7](https://github.com/latipun7)
- (**renderer**) body always visible, empty class hides text - ([263aef0](https://github.com/latipun7/hermes-webui-companion/commit/263aef06228085c5bc4d37c2cdec1e235207a5c4)) - [@latipun7](https://github.com/latipun7)
- (**renderer**) solid dark bubble + Tauri IPC + HTTP fallback - ([0903a0e](https://github.com/latipun7/hermes-webui-companion/commit/0903a0efbcb710323736bb439ee35a4ba17434ed)) - [@latipun7](https://github.com/latipun7)
- (**renderer**) transparent bubble + click-target overlay + cmd start - ([4f8866c](https://github.com/latipun7/hermes-webui-companion/commit/4f8866c9b9b56addf418757157c5967618c07bc8)) - [@latipun7](https://github.com/latipun7)
- (**renderer**) transparent window + click-target overlay + red error flash - ([d3e291a](https://github.com/latipun7/hermes-webui-companion/commit/d3e291add921d8e65153b5c11b2bd896ee8dcc29)) - [@latipun7](https://github.com/latipun7)
- (**renderer**) use cmd start for URL, rgba body bg for visual - ([4ece38b](https://github.com/latipun7/hermes-webui-companion/commit/4ece38b2692aa1250977cd7300091b739884c0a8)) - [@latipun7](https://github.com/latipun7)
- (**renderer**) bubble window non-transparent for click handling - ([1d05bc6](https://github.com/latipun7/hermes-webui-companion/commit/1d05bc69d899f7aa5ec8e0e6b0acd97d4cb19c0f)) - [@latipun7](https://github.com/latipun7)
- (**renderer**) move permissions to capabilities/default.json - ([067b34b](https://github.com/latipun7/hermes-webui-companion/commit/067b34ba98445be22776478bc83a336454860c61)) - [@latipun7](https://github.com/latipun7)
- (**renderer**) multi-strategy bubble click + shell:allow-open permission - ([04e0e2c](https://github.com/latipun7/hermes-webui-companion/commit/04e0e2c2691220445ea18c879c35ca43f7c3c183)) - [@latipun7](https://github.com/latipun7)
- (**renderer**) bubble position — exact height offset + initial sync - ([5cd45bd](https://github.com/latipun7/hermes-webui-companion/commit/5cd45bd1ad01382da0555b39c075671336af358a)) - [@latipun7](https://github.com/latipun7)
- (**renderer**) separate Resized/Moved match arms for bubble positioning - ([d7b814f](https://github.com/latipun7/hermes-webui-companion/commit/d7b814f14aaba7e39590ff847c977811b3ed8074)) - [@latipun7](https://github.com/latipun7)
- (**renderer**) restore html selector for body height inheritance - ([e1fe0a0](https://github.com/latipun7/hermes-webui-companion/commit/e1fe0a0266ec5fd772f46e1eccc6c9a83e4f792c)) - [@latipun7](https://github.com/latipun7)
- (**renderer**) bubble positioning — inside window, not outside - ([738c7c4](https://github.com/latipun7/hermes-webui-companion/commit/738c7c45d44ce7ff02775b612d718b1f06ab4724)) - [@latipun7](https://github.com/latipun7)
- (**renderer**) map companion Ready to idle instead of waving - ([ed2ee00](https://github.com/latipun7/hermes-webui-companion/commit/ed2ee00121b46c496e50b14293477dede1c42481)) - [@latipun7](https://github.com/latipun7)
- (**renderer**) add /api/state endpoint + frontend HTTP fallback - ([4842fdf](https://github.com/latipun7/hermes-webui-companion/commit/4842fdfd32fade74cce5109c4ab313a8bd3d03d7)) - [@latipun7](https://github.com/latipun7)
- (**renderer**) add CORS headers to bridge HTTP responses - ([32daa0f](https://github.com/latipun7/hermes-webui-companion/commit/32daa0fc5659f216939b926b75db98d005261e8e)) - [@latipun7](https://github.com/latipun7)
- (**renderer**) add mut to stream for write_all HTTP response - ([50470f8](https://github.com/latipun7/hermes-webui-companion/commit/50470f8746bf3ace62d3676d3348c89d6f55f2b2)) - [@latipun7](https://github.com/latipun7)
- (**renderer**) add HTTP responses + /health endpoint to bridge - ([76f3de1](https://github.com/latipun7/hermes-webui-companion/commit/76f3de175eb2985f54b8f7ee602789f28ba63890)) - [@latipun7](https://github.com/latipun7)
- (**renderer**) per-state frame counts to avoid empty spritesheet cells - ([a516fd0](https://github.com/latipun7/hermes-webui-companion/commit/a516fd047a8ef4aa786a24f0f40cb1f5c60fed40)) - [@latipun7](https://github.com/latipun7)
- (**renderer**) CSS sprite rendering + state bridge fix - ([ae5c60d](https://github.com/latipun7/hermes-webui-companion/commit/ae5c60d67646fe47a1038e499b1993cf4a17e5c0)) - [@latipun7](https://github.com/latipun7)
- (**renderer**) remove clearRect to eliminate canvas blink - ([aa0c99d](https://github.com/latipun7/hermes-webui-companion/commit/aa0c99d04000209c2d249c67c363ba518a544f56)) - [@latipun7](https://github.com/latipun7)
- (**renderer**) use base64 data URL instead of blob to fix CSP blink - ([2463d57](https://github.com/latipun7/hermes-webui-companion/commit/2463d574df7d056935d0bce03fa53b1aa533f900)) - [@latipun7](https://github.com/latipun7)
- (**renderer**) stop canvas resize on every window event - ([0facd1d](https://github.com/latipun7/hermes-webui-companion/commit/0facd1d9bfbc6e7b313a08e00bd30ea797b3c76d)) - [@latipun7](https://github.com/latipun7)
- (**renderer**) remove unnecessary stream clone in bridge server - ([f2d3497](https://github.com/latipun7/hermes-webui-companion/commit/f2d34973b5dfa49656c0eec1d282af98bc1c7905)) - [@latipun7](https://github.com/latipun7)
- (**renderer**) add Serialize derives for companion state types - ([5658e99](https://github.com/latipun7/hermes-webui-companion/commit/5658e99e5c95c638cb392f76c78cea6ea8745656)) - [@latipun7](https://github.com/latipun7)
- (**renderer**) use port 17787 for bridge server - ([37bb36a](https://github.com/latipun7/hermes-webui-companion/commit/37bb36aa6653042faa27367a3e50f99b5e0788b1)) - [@latipun7](https://github.com/latipun7)
- (**renderer**) clone window before move into resize closure - ([ff46a3e](https://github.com/latipun7/hermes-webui-companion/commit/ff46a3eaef545b61db1e76ada50840e26bfb1a00)) - [@latipun7](https://github.com/latipun7)
- (**renderer**) import tauri::Manager for get_webview_window - ([1b5ad69](https://github.com/latipun7/hermes-webui-companion/commit/1b5ad694399115055a0997b9eabd4e8f7c4f1ffe)) - [@latipun7](https://github.com/latipun7)
- (**renderer**) combine CSS drag + all JS invoke paths + dragDropEnabled - ([7954b4a](https://github.com/latipun7/hermes-webui-companion/commit/7954b4a5a5e2ae08d84e9994b8ddb5d5bd6ae73b)) - [@latipun7](https://github.com/latipun7)
- (**renderer**) use WebviewWindow + add drag debug logs - ([8d594f7](https://github.com/latipun7/hermes-webui-companion/commit/8d594f7839c936acdbbf702a7880f7e12d337278)) - [@latipun7](https://github.com/latipun7)
- (**renderer**) native window drag via Tauri Rust command - ([e35af82](https://github.com/latipun7/hermes-webui-companion/commit/e35af82a22e7582254881c14a48296ade57e7ce0)) - [@latipun7](https://github.com/latipun7)
- (**renderer**) use CSS -webkit-app-region for window drag - ([f2db23c](https://github.com/latipun7/hermes-webui-companion/commit/f2db23cdfef6de6143a9412a2e74e9261783b10a)) - [@latipun7](https://github.com/latipun7)
- (**renderer**) enable drag-to-move and remove window border - ([d979ab3](https://github.com/latipun7/hermes-webui-companion/commit/d979ab3ec3077afcca730a7feb2d4f81da032ec0)) - [@latipun7](https://github.com/latipun7)
- (**renderer**) add icon.ico and bundle config for Windows build - ([148ad1e](https://github.com/latipun7/hermes-webui-companion/commit/148ad1e599ffaeb658a1f5bcf85d29fbef408405)) - [@latipun7](https://github.com/latipun7)
- (**renderer**) add default-run for Tauri binary - ([95e5019](https://github.com/latipun7/hermes-webui-companion/commit/95e501953ecf5c31181c86744446f573803c6828)) - [@latipun7](https://github.com/latipun7)
- (**renderer**) remove devUrl so Tauri serves gui/ directly - ([58f3495](https://github.com/latipun7/hermes-webui-companion/commit/58f3495efae72ee7c4a25afe67f91bb18ac4a07a)) - [@latipun7](https://github.com/latipun7)
- (**test**) drain TCP recv before write, use cmd /c on Windows - ([536894e](https://github.com/latipun7/hermes-webui-companion/commit/536894e08d2c961a87f10f553beadd7e2812eb0b)) - [@latipun7](https://github.com/latipun7)
- (**windows**) use PowerShell-safe npx invocation - ([2f6aacf](https://github.com/latipun7/hermes-webui-companion/commit/2f6aacffaedef3d49e014eaec2680fc09dbb3fab)) - [@latipun7](https://github.com/latipun7)
- use CI cache keys and robust bundle copies in release - ([0aaf035](https://github.com/latipun7/hermes-webui-companion/commit/0aaf035ad6270c678201b274f0f797f9876afdf2)) - [@latipun7](https://github.com/latipun7)
- use npx tauri-cli (pre-built binary) instead of cargo install - ([afdc4ec](https://github.com/latipun7/hermes-webui-companion/commit/afdc4ec7e02420a47b04b5bfc5b158a9ba8de484)) - [@latipun7](https://github.com/latipun7)
- install tauri-cli before cargo tauri build - ([86f90cc](https://github.com/latipun7/hermes-webui-companion/commit/86f90cca33f15ee1d1109433b3aa3407f189009a)) - [@latipun7](https://github.com/latipun7)
- use correct SHAs for upload-artifact & download-artifact - ([917b8c7](https://github.com/latipun7/hermes-webui-companion/commit/917b8c7310c7924ab2be1114675a9eb3026d6411)) - [@latipun7](https://github.com/latipun7)
- serialise health tests to prevent HERMES_WEBUI_PORT race - ([0eec1da](https://github.com/latipun7/hermes-webui-companion/commit/0eec1da1ff7489c3ce2435c5e29ce1eae17b46a0)) - [@latipun7](https://github.com/latipun7)
- gate bridge handlers behind gui feature, drop dup var - ([1a4d7ee](https://github.com/latipun7/hermes-webui-companion/commit/1a4d7ee8cf58e465fb2144036ec442a6cae1f0d1)) - [@latipun7](https://github.com/latipun7)
- multi-stage browser focus on bubble click - ([93f87d4](https://github.com/latipun7/hermes-webui-companion/commit/93f87d49807bec9f5eb64f276f175cdc6e4c7558)) - [@latipun7](https://github.com/latipun7)

#### 📚 Documentation

- (**readme**) clarify sidecar is optional, highlight direct mode - ([cebf253](https://github.com/latipun7/hermes-webui-companion/commit/cebf2534c9b8766b4c8a9a9990401db54ec73228)) - [@latipun7](https://github.com/latipun7)
- apply domain modeling - consolidate glossary, restructure ADRs, trim fat - ([71b74a3](https://github.com/latipun7/hermes-webui-companion/commit/71b74a3dd7e3db91d2d465d6604729ed5c91324f)) - [@latipun7](https://github.com/latipun7)
- update agents.md & prd.md for code quality tooling - ([75cae5f](https://github.com/latipun7/hermes-webui-companion/commit/75cae5fc94c81aea86339a5e2da43fd9778c36ee)) - [@latipun7](https://github.com/latipun7)
- mark switch pet menu as complete - ([0c2ffed](https://github.com/latipun7/hermes-webui-companion/commit/0c2ffed6be8819b186cab3ada03b6aa13d3ec04c)) - [@latipun7](https://github.com/latipun7)
- add ADR, glossary, and tracking issues for switch pet feature - ([a6b13ee](https://github.com/latipun7/hermes-webui-companion/commit/a6b13eea8a217c9dce5c9ea3bf9f9611c238508c)) - [@latipun7](https://github.com/latipun7)
- mark right-click menu issues as complete (PR #11) - ([80e95fc](https://github.com/latipun7/hermes-webui-companion/commit/80e95fc88752e19af14e842a3294575df3940359)) - [@latipun7](https://github.com/latipun7)
- add right-click context menu ADR, glossary, and issues - ([ffd78a8](https://github.com/latipun7/hermes-webui-companion/commit/ffd78a85367540b964632aff3ad973fe14759198)) - [@latipun7](https://github.com/latipun7)
- sync documentation with recent architecture changes - ([7a025f7](https://github.com/latipun7/hermes-webui-companion/commit/7a025f722ea09b558e8ed318bae326ff764f83a7)) - [@latipun7](https://github.com/latipun7)
- add agents.md with project architecture and build guide - ([edbdddc](https://github.com/latipun7/hermes-webui-companion/commit/edbdddcd72dae7ef0e7598985750e7a4ecf2e747)) - [@latipun7](https://github.com/latipun7)

#### 👷 Continuous Integration

- (**actions**) update `softprops/action-gh-release` action to v3 (#28) - ([fb2552e](https://github.com/latipun7/hermes-webui-companion/commit/fb2552e1525c463cbc5b642fae41d89cea10b2df)) - renovate[bot]
- (**actions**) update `actions/cache` action to v6 (#23) - ([12c32d4](https://github.com/latipun7/hermes-webui-companion/commit/12c32d47743512c37a80f0196ccc1f51fee31042)) - renovate[bot], renovate[bot]
- (**actions**) update `actions/checkout` action to v7 (#24) - ([923a50b](https://github.com/latipun7/hermes-webui-companion/commit/923a50b187b647668c9cd5a936f44e71ecc0161d)) - renovate[bot], renovate[bot]
- add reusable release workflow with cross-platform renderer - ([b094230](https://github.com/latipun7/hermes-webui-companion/commit/b09423007f07982a4a3ed0d8cd6fd909c03f3193)) - [@latipun7](https://github.com/latipun7)
- install Tauri system deps before clippy --all-features - ([d4362ca](https://github.com/latipun7/hermes-webui-companion/commit/d4362cacfffa447be03471ce4456a87c9fbc2e12)) - [@latipun7](https://github.com/latipun7)

#### ♻️ Refactoring

- (**renderer**) remove dead backward-compat code - ([ef407a6](https://github.com/latipun7/hermes-webui-companion/commit/ef407a6c344f94798f1e98304ce355e0dbcf84c9)) - [@latipun7](https://github.com/latipun7)
- (**renderer**) migrate bridge server to tiny_http & decouple handlers - ([e2766a2](https://github.com/latipun7/hermes-webui-companion/commit/e2766a299381abb2fb0fab8f4603c7bf75ff6bcc)) - [@latipun7](https://github.com/latipun7)
- (**renderer**) extract gui.rs into focused modules - ([3bc7cf3](https://github.com/latipun7/hermes-webui-companion/commit/3bc7cf372f5ff7c2bce75735cf69e6e6b4ee9d74)) - [@latipun7](https://github.com/latipun7)
- (**renderer**) centralize animation priority in Rust backend - ([5b13c20](https://github.com/latipun7/hermes-webui-companion/commit/5b13c20630c771d10ecf43d8debb7ced686d23b9)) - [@latipun7](https://github.com/latipun7)
- (**renderer**) bubble click via bridge HTTP, cross-platform URL open - ([c9e1b82](https://github.com/latipun7/hermes-webui-companion/commit/c9e1b82819383d7bce30f5af8461a9f697abfeb9)) - [@latipun7](https://github.com/latipun7)

---

Changelog generated by [cocogitto](https://github.com/cocogitto/cocogitto).
