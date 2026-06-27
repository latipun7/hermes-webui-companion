// pet.js — Canvas-based companion renderer for Hermes WebUI Companion
//
// Renders animated pet spritesheets fetched from the companion sidecar
// via Tauri IPC commands.

const CANVAS_ID = "pet-canvas";
const FRAME_W = 192;
const FRAME_H = 208;
const COLS = 8;
const ROWS = 9;

// Animation state → spritesheet row mapping (matches sprite::AnimationState)
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

// Which animation state to use for each companion state
const COMPANION_STATE_MAP = {
  idle: "idle",
  running: "running",
  ready: "waving",
  approval: "waiting",
  clarify: "review",
  error: "failed",
};

let currentState = "idle";
let currentFrame = 0;
let spritesheet = null;
let canvas = null;
let ctx = null;
let animTimer = null;

// ---------------------------------------------------------------------------
// Tauri IPC helpers
// ---------------------------------------------------------------------------

async function invokeTauri(cmd, args = {}) {
  // When running inside Tauri, use the IPC bridge
  if (window.__TAURI__) {
    const { invoke } = window.__TAURI__.core;
    return invoke(cmd, args);
  }
  // Fallback: direct HTTP to sidecar (for dev without Tauri)
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
  throw new Error(`Unknown command: ${cmd}`);
}

// ---------------------------------------------------------------------------
// Spritesheet loading
// ---------------------------------------------------------------------------

async function loadSpritesheet() {
  try {
    const pet = await invokeTauri("get_active_pet");
    const bytes = await invokeTauri("get_spritesheet", { slug: pet.slug });
    const blob = new Blob([bytes], { type: "image/webp" });
    const url = URL.createObjectURL(blob);

    return new Promise((resolve, reject) => {
      const img = new Image();
      img.onload = () => {
        URL.revokeObjectURL(url);
        resolve(img);
      };
      img.onerror = () => reject(new Error("failed to load spritesheet"));
      img.src = url;
    });
  } catch (err) {
    console.error("Failed to load pet:", err);
    return null;
  }
}

// ---------------------------------------------------------------------------
// Rendering
// ---------------------------------------------------------------------------

function initCanvas() {
  canvas = document.getElementById(CANVAS_ID);
  ctx = canvas.getContext("2d");
  // Set internal resolution once — CSS handles visual scaling
  canvas.width = FRAME_W;
  canvas.height = FRAME_H;
}

function drawFrame(state, col) {
  if (!spritesheet) return;
  const row = STATE_ROWS[state] ?? 0;
  const sx = col * FRAME_W;
  const sy = row * FRAME_H;

  ctx.clearRect(0, 0, FRAME_W, FRAME_H);
  ctx.drawImage(
    spritesheet,
    sx, sy, FRAME_W, FRAME_H,
    0, 0, FRAME_W, FRAME_H,
  );
}

function startAnimation() {
  if (animTimer) clearInterval(animTimer);
  const fps = 8; // frames per second
  const colsPerState = COLS;
  animTimer = setInterval(() => {
    currentFrame = (currentFrame + 1) % colsPerState;
    drawFrame(currentState, currentFrame);
  }, 1000 / fps);
}

function setAnimationState(state) {
  const mapped = COMPANION_STATE_MAP[state] || "idle";
  if (mapped !== currentState) {
    currentState = mapped;
    currentFrame = 0;
  }
}

// ---------------------------------------------------------------------------
// State polling — real WebUI bridge state via Tauri command
// ---------------------------------------------------------------------------

async function pollCompanionState() {
  try {
    const state = await invokeTauri("get_companion_state");
    // Apply same priority logic as animation.rs::resolve_animation_state:
    // Approval > Clarify > agent state
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
    // Sidecar or bridge not running — default to idle
    setAnimationState("idle");
  }
}

function startStatePolling() {
  // Poll immediately, then every 1 second
  pollCompanionState();
  setInterval(pollCompanionState, 1000);
}

// ---------------------------------------------------------------------------
// Window dragging — via Tauri command (works without @tauri-apps/api npm)
// ---------------------------------------------------------------------------

function setupDrag() {
  // Tauri v2 injects __TAURI__ even without npm, but the API shape varies.
  // Try every known invocation path.
  document.addEventListener("mousedown", () => {
    const t = window.__TAURI__;
    if (!t) return;

    // Try direct invoke on __TAURI__ (v2)
    if (typeof t.invoke === "function") {
      t.invoke("start_dragging").catch(() => {});
      return;
    }
    // Try core.invoke (v2 with @tauri-apps/api loaded)
    if (t.core && typeof t.core.invoke === "function") {
      t.core.invoke("start_dragging").catch(() => {});
      return;
    }
    // Try __TAURI_INTERNALS__ (older injection pattern)
    const ti = window.__TAURI_INTERNALS__;
    if (ti && typeof ti.invoke === "function") {
      ti.invoke("start_dragging").catch(() => {});
    }
  });
}

// ---------------------------------------------------------------------------
// Bootstrap
// ---------------------------------------------------------------------------

async function main() {
  initCanvas();
  setupDrag();
  spritesheet = await loadSpritesheet();
  if (!spritesheet) {
    console.error("Cannot start — no spritesheet loaded");
    return;
  }
  startAnimation();
  startStatePolling();
}

main().catch(console.error);
