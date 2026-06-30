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

let currentState = "idle";
let currentCol = 0;
// User manually hid the bubble — suppress animation updates until state changes.
let userMuted = false;
let mutedCompanionState = null;
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
    currentSlug = pet.slug;
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
  // resolved_animation returns the correct spritesheet state name directly.
  if (state && state !== currentState) {
    currentState = state;
    currentCol = 0;
  }
}

// ---------------------------------------------------------------------------
// Drag animation — running-right / running-left while dragging
// ---------------------------------------------------------------------------

let dragState = null;    // "running-right" | "running-left" | null

// ---------------------------------------------------------------------------
// State polling
// ---------------------------------------------------------------------------

async function pollCompanionState() {
  // Don't poll state while dragging
  if (dragState) return;

  try {
    // Check for accumulated drag delta from Rust (OS drag pauses JS,
    // so we detect direction post-drag via Moved events on Rust side).
    try {
      const dx = await invokeTauri("get_drag_dx");
      if (Math.abs(dx) > 10) {
        const dir = dx > 0 ? "running-right" : "running-left";
        dragState = dir;
        currentState = dir;
        currentCol = 0;
        // Drain any additional dx accumulated during the animation,
        // then revert to companion state on next poll.
        setTimeout(async () => {
          await invokeTauri("get_drag_dx").catch(() => {});
          dragState = null;
        }, 500);
        return;
      }
    } catch (_) {
      // IPC error — ignore, proceed with companion state
    }

    const state = await invokeTauri("get_companion_state");
    const resolved = state.resolved_animation || "idle";

    if (userMuted) {
      // On first poll after mute, capture the companion state
      if (mutedCompanionState === null) {
        mutedCompanionState = resolved;
      }
      // Stay idle until the companion state actually changes
      if (resolved !== mutedCompanionState) {
        userMuted = false;
        mutedCompanionState = null;
        setAnimationState(resolved);
      } else {
        // Ensure pet stays idle — drag animation may have changed currentState
        setAnimationState("idle");
      }
    } else {
      setAnimationState(resolved);
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

      // When hiding the bubble, pet goes idle and stays idle until
      // the companion state actually changes (not just re-polled).
      if (!bubblesVisible) {
        userMuted = true;
        // Save companion state at mute time so we know when it changes.
        // We read it on the next poll and compare against resolved_animation.
        mutedCompanionState = null; // will be set on next pollCompanionState
        setAnimationState("idle");
      } else {
        // Bubble shown again — release mute immediately
        userMuted = false;
        mutedCompanionState = null;
      }
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
// Right-click context menu (native, via Tauri)
// ---------------------------------------------------------------------------

let currentSlug = null;

function setupContextMenu() {
  document.addEventListener("contextmenu", (e) => {
    e.preventDefault();
    invokeTauri("show_context_menu")
      .then(() => reloadSpritesheetIfChanged())
      .catch(() => {});
  });
}

/// After context menu dismisses, check if active pet changed.
/// If so, reload the spritesheet in-place without restart.
async function reloadSpritesheetIfChanged() {
  try {
    const pet = await invokeTauri("get_active_pet");
    if (!currentSlug || pet.slug !== currentSlug) {
      await reloadSpritesheet(pet.slug);
    }
  } catch (_) {
    // Sidecar unreachable — keep current sprite
  }
}

async function reloadSpritesheet(slug) {
  try {
    const bytes = await invokeTauri("get_spritesheet", { slug });
    let binary = "";
    for (let i = 0; i < bytes.length; i++) {
      binary += String.fromCharCode(bytes[i]);
    }
    const url = "data:image/webp;base64," + btoa(binary);

    return new Promise((resolve, reject) => {
      const img = new Image();
      img.onload = () => {
        spriteDiv.style.backgroundImage = `url(${url})`;
        currentSlug = slug;
        currentState = "idle";
        currentCol = 0;
        resolve();
      };
      img.onerror = () => reject(new Error("failed to load spritesheet"));
      img.src = url;
    });
  } catch (_) {
    // Spritesheet load failed — keep current sprite, show error indicator
  }
}

// ---------------------------------------------------------------------------
// Bootstrap
// ---------------------------------------------------------------------------

let retryTimer = null;

async function main() {
  spriteDiv = document.getElementById(SPRITE_ID);
  setupDrag();
  setupBubbleToggle();
  setupContextMenu();

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
