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
  resizeCanvas();
  window.addEventListener("resize", resizeCanvas);
}

function resizeCanvas() {
  canvas.width = canvas.clientWidth * window.devicePixelRatio;
  canvas.height = canvas.clientHeight * window.devicePixelRatio;
}

function drawFrame(state, col) {
  if (!spritesheet) return;
  const row = STATE_ROWS[state] ?? 0;
  const sx = col * FRAME_W;
  const sy = row * FRAME_H;

  ctx.clearRect(0, 0, canvas.width, canvas.height);
  ctx.drawImage(
    spritesheet,
    sx, sy, FRAME_W, FRAME_H,
    0, 0, canvas.width, canvas.height,
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
// State polling (placeholder — real state comes from WebUI bridge later)
// ---------------------------------------------------------------------------

function startStatePolling() {
  // For now, cycle through states every 5 seconds as a demo
  const states = ["idle", "running", "ready", "waving"];
  let i = 0;
  setInterval(() => {
    setAnimationState(states[i % states.length]);
    i++;
  }, 5000);
}

// ---------------------------------------------------------------------------
// Bootstrap
// ---------------------------------------------------------------------------

async function main() {
  initCanvas();
  spritesheet = await loadSpritesheet();
  if (!spritesheet) {
    console.error("Cannot start — no spritesheet loaded");
    return;
  }
  startAnimation();
  startStatePolling();
}

main().catch(console.error);
