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

// Companion state → animation state mapping.
// resolved_animation comes from the Rust backend — frontend just passes it through.
const COMPANION_STATE_MAP = {
  idle: "idle",
  running: "running",
  ready: "waving",
  approval: "waiting",
  clarify: "review",
  error: "failed",
  failed: "failed",
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
    const invoke = window.__TAURI__.invoke || window.__TAURI__.core?.invoke;
    if (typeof invoke === "function") return invoke(cmd, args);
  }
  throw new Error(`Tauri IPC unavailable — cannot invoke ${cmd}`);
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
// Drag animation — running-right / running-left while dragging
// ---------------------------------------------------------------------------

let dragState = null;    // "running-right" | "running-left" | null
let dragPrevX = null;

function setupDragAnimation() {
  document.addEventListener("mousedown", (e) => {
    dragPrevX = e.clientX;
  });
  document.addEventListener("mousemove", (e) => {
    if (dragPrevX === null) return;
    const dx = e.clientX - dragPrevX;
    if (Math.abs(dx) > 2) {
      dragState = dx > 0 ? "running-right" : "running-left";
      currentState = dragState;
      currentCol = 0;
    }
    dragPrevX = e.clientX;
  });
  document.addEventListener("mouseup", () => {
    dragPrevX = null;
    dragState = null;
    // Restore companion state on next poll
  });
}

// ---------------------------------------------------------------------------
// State polling
// ---------------------------------------------------------------------------

async function pollCompanionState() {
  // Don't poll state while dragging
  if (dragState) return;

  try {
    const state = await invokeTauri("get_companion_state");

    // Bridge signals sidecar failure → failed animation takes priority.
    // The resolved_animation field already encodes the full priority chain
    // (Failed > Approval > Clarify > agent state) computed by the Rust backend.
    if (state.resolved_animation) {
      setAnimationState(state.resolved_animation);
    } else if (state.state === "failed") {
      // Backward compat: older backend without resolved_animation
      setAnimationState("failed");
    } else {
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
    }
  } catch (e) {
    // Bridge unreachable — keep current state; don't override to idle
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
// Bubble toggle
// ---------------------------------------------------------------------------

let bubblesVisible = true;

function setupBubbleToggle() {
  const btn = document.getElementById("bubble-toggle");
  if (!btn) return;

  // Block mousedown from reaching the drag handler
  btn.addEventListener("mousedown", (e) => {
    e.stopPropagation();
  });

  btn.addEventListener("click", async (e) => {
    e.stopPropagation();
    e.preventDefault();

    bubblesVisible = !bubblesVisible;
    btn.className = bubblesVisible ? "on" : "";
    btn.textContent = bubblesVisible ? "\u25BC" : "\u25B2"; // ▼ : ▲

    // Debug flash — blue blink confirms click detected
    flashButton("rgba(137, 180, 250, 0.9)", 100);

    try {
      const res = await fetch("http://127.0.0.1:17787/api/bubbles/visible", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ visible: bubblesVisible }),
      });
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
    } catch (err) {
      console.error("bubbles visible fetch failed:", err);
      flashButton("rgba(255, 100, 100, 0.9)", 400); // red: HTTP error
      bubblesVisible = !bubblesVisible; // revert
      btn.className = bubblesVisible ? "on" : "";
      btn.textContent = bubblesVisible ? "\u25BC" : "\u25B2";
    }
  });
}

function flashButton(color, ms = 100) {
  const btn = document.getElementById("bubble-toggle");
  if (!btn) return;
  const prev = btn.style.color;
  btn.style.transition = "none";
  btn.style.color = color;
  setTimeout(() => {
    btn.style.transition = "color 0.15s, background 0.15s, border-color 0.15s";
    btn.style.color = prev;
  }, ms);
}

// ---------------------------------------------------------------------------
// Sync button with auto-hide state from bridge
// ---------------------------------------------------------------------------

async function syncBubbleVisibility() {
  try {
    const res = await fetch("http://127.0.0.1:17787/api/bubbles/visible");
    if (!res.ok) return;
    const data = await res.json();
    const visible = !!data.visible;
    if (visible !== bubblesVisible) {
      bubblesVisible = visible;
      const btn = document.getElementById("bubble-toggle");
      if (btn) {
        btn.className = visible ? "on" : "";
        btn.textContent = visible ? "\u25BC" : "\u25B2";
      }
    }
  } catch (_) {
    // bridge unreachable — keep current button state
  }
}

// ---------------------------------------------------------------------------
// Bootstrap
// ---------------------------------------------------------------------------

let retryTimer = null;

async function main() {
  spriteDiv = document.getElementById(SPRITE_ID);
  setupDrag();
  setupDragAnimation();
  setupBubbleToggle();

  // Poll bridge to keep button in sync with auto-hide
  setInterval(syncBubbleVisibility, 1000);
  syncBubbleVisibility();

  const url = await loadSpritesheet();
  if (!url) {
    // No spritesheet — show error indicator, keep polling for recovery
    spriteDiv.style.display = "flex";
    spriteDiv.style.alignItems = "center";
    spriteDiv.style.justifyContent = "center";
    spriteDiv.style.backgroundColor = "rgba(255, 0, 0, 0.12)";
    spriteDiv.style.color = "rgba(255, 100, 100, 0.9)";
    spriteDiv.style.fontSize = "52px";
    spriteDiv.style.fontWeight = "bold";
    spriteDiv.style.fontFamily = "monospace";
    spriteDiv.textContent = "!";

    // Still poll state — bridge may report failed, sidecar may recover
    startStatePolling();

    // Retry spritesheet load periodically
    retryTimer = setInterval(async () => {
      const retryUrl = await loadSpritesheet();
      if (retryUrl) {
        clearInterval(retryTimer);
        retryTimer = null;
        // Restore normal sprite rendering
        spriteDiv.style.display = "";
        spriteDiv.style.alignItems = "";
        spriteDiv.style.justifyContent = "";
        spriteDiv.style.backgroundColor = "";
        spriteDiv.style.color = "";
        spriteDiv.style.fontSize = "";
        spriteDiv.style.fontWeight = "";
        spriteDiv.style.fontFamily = "";
        spriteDiv.textContent = "";
        spriteDiv.style.backgroundImage = `url(${retryUrl})`;
        startAnimation();
      }
    }, 10000);
    return;
  }

  spriteDiv.style.backgroundImage = `url(${url})`;
  startAnimation();
  startStatePolling();
}

main().catch(console.error);
