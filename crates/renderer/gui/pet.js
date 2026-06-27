// pet.js — CSS sprite-based companion renderer
//
// Uses CSS background-position on a <div> for zero-blink animation.
// Same approach as franksong2702/hermes-webui-desktop-companion.

const SPRITE_ID = "pet-sprite";
const COLS = 8;
const ROWS = 9;

// State → spritesheet row
const STATE_ROWS = {
  idle: 0,
  "running-right": 1,
  "running-left": 2,
  waving: 3,
  jumping: 4,
  failed: 5,
  waiting: 6,
  running: 7,
  review: 8,
};

// Companion state → animation state (matches reference project)
const COMPANION_STATE_MAP = {
  idle: "idle",
  running: "running",
  ready: "waving",       // session completed → wave
  approval: "waiting",
  clarify: "review",
  error: "failed",
};

let currentState = "idle";
let currentCol = 0;
// Frame counts per state (most petdex sprites use 6-8 frames)
const FRAMES_PER_STATE = {
  idle: 6,
  "running-right": 8,
  "running-left": 8,
  waving: 4,
  jumping: 5,
  failed: 8,
  waiting: 6,
  running: 6,
  review: 6,
};
let spriteDiv = null;
let animTimer = null;

// ---------------------------------------------------------------------------
// Tauri IPC
// ---------------------------------------------------------------------------

async function invokeTauri(cmd, args = {}) {
  if (window.__TAURI__) {
    const invoke = window.__TAURI__.invoke || window.__TAURI__?.core?.invoke;
    if (invoke) return invoke(cmd, args);
  }
  // Fallback: direct HTTP to sidecar or bridge
  const base = "http://127.0.0.1:17888";
  if (cmd === "get_active_pet") {
    const res = await fetch(`${base}/api/pet/active`);
    return res.json();
  }
  if (cmd === "get_spritesheet") {
    const res = await fetch(`${base}/pets/${args.slug}/spritesheet.webp`);
    const blob = await res.blob();
    return new Uint8Array(await blob.arrayBuffer());
  }
  // Bridge state lives on :17787 — fetch directly as fallback
  if (cmd === "get_companion_state") {
    const res = await fetch("http://127.0.0.1:17787/api/state");
    return res.json();
  }
  if (cmd === "start_dragging") {
    return null; // requires Tauri IPC
  }
  throw new Error(`Unknown command: ${cmd}`);
}

// ---------------------------------------------------------------------------
// Spritesheet loading
// ---------------------------------------------------------------------------

async function loadSpritesheet() {
  try {
    const pet = await invokeTauri("get_active_pet");
    const bytes = await invokeTauri("get_spritesheet", { slug: pet.slug });

    // Convert bytes to base64 data URL
    let binary = "";
    for (let i = 0; i < bytes.length; i++) {
      binary += String.fromCharCode(bytes[i]);
    }
    const url = "data:image/webp;base64," + btoa(binary);

    return new Promise((resolve, reject) => {
      const img = new Image();
      img.onload = () => resolve(url);
      img.onerror = () => reject(new Error("failed to load spritesheet"));
      img.src = url;
    });
  } catch (err) {
    console.error("Failed to load pet:", err);
    return null;
  }
}

// ---------------------------------------------------------------------------
// Animation — CSS background-position
// ---------------------------------------------------------------------------

function applyFrame(state, col) {
  if (!spriteDiv || !spriteDiv.style.backgroundImage) return;
  const row = STATE_ROWS[state] ?? 0;
  const x = col / (COLS - 1) * 100;
  const y = row / (ROWS - 1) * 100;
  spriteDiv.style.backgroundPosition = `${x}% ${y}%`;
}

function startAnimation() {
  if (animTimer) clearInterval(animTimer);
  const fps = 8;
  animTimer = setInterval(() => {
    const maxFrames = FRAMES_PER_STATE[currentState] || 6;
    currentCol = (currentCol + 1) % maxFrames;
    applyFrame(currentState, currentCol);
  }, 1000 / fps);
}

function setAnimationState(state) {
  const mapped = COMPANION_STATE_MAP[state] || "idle";
  if (mapped !== currentState) {
    currentState = mapped;
    currentCol = 0;
  }
}

// ---------------------------------------------------------------------------
// State polling
// ---------------------------------------------------------------------------

async function pollCompanionState() {
  try {
    const state = await invokeTauri("get_companion_state");
    const attention = state.attention || [];
    const hasApproval = attention.some((a) => a.status === "approval");
    const hasClarify = attention.some((a) => a.status === "clarify");

    if (hasApproval) {
      setAnimationState("approval");
    } else if (hasClarify) {
      setAnimationState("clarify");
    } else {
      setAnimationState(state.state || "idle");
    }
  } catch (e) {
    setAnimationState("idle");
  }
}

function startStatePolling() {
  pollCompanionState();
  setInterval(pollCompanionState, 1000);
}

// ---------------------------------------------------------------------------
// Window dragging
// ---------------------------------------------------------------------------

function setupDrag() {
  document.addEventListener("mousedown", () => {
    const t = window.__TAURI__;
    if (!t) return;
    if (typeof t.invoke === "function") {
      t.invoke("start_dragging").catch(() => {});
    } else if (t.core && typeof t.core.invoke === "function") {
      t.core.invoke("start_dragging").catch(() => {});
    } else {
      const ti = window.__TAURI_INTERNALS__;
      if (ti && typeof ti.invoke === "function") {
        ti.invoke("start_dragging").catch(() => {});
      }
    }
  });
}

// ---------------------------------------------------------------------------
// Bootstrap
// ---------------------------------------------------------------------------

async function main() {
  spriteDiv = document.getElementById(SPRITE_ID);
  setupDrag();

  const url = await loadSpritesheet();
  if (!url) {
    console.error("Cannot start — no spritesheet loaded");
    return;
  }

  spriteDiv.style.backgroundImage = `url(${url})`;
  startAnimation();
  startStatePolling();
}

main().catch(console.error);
