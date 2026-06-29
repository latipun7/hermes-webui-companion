// bubbles.js — Light-themed bubble card for Hermes Companion
//
// Polls the bridge server for companion state and renders
// a card notification above the pet. Click opens the session
// in WebUI via POST /api/open-webui with session_id.
//
// Auto-hides the bubble window when idle (no content to display),
// and shows it when attention items appear.

const POLL_MS = 1000;
let lastState = null;
let opening = false; // true while waiting for browser focus
let cardHasContent = false; // tracks whether card is currently showing content

// ── Auto-hide via bridge ───────────────────────────────────

async function setBubbleVisible(visible) {
  try {
    await fetch("http://127.0.0.1:17787/api/bubbles/visible", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ visible }),
    });
    cardHasContent = visible;
  } catch (_) {
    // bridge unreachable — retry on next poll
  }
}

// ── Init ───────────────────────────────────────────────────

function init() {
  const card = document.getElementById("card");
  if (!card) return;

  card.addEventListener("click", async () => {
    if (opening) return;
    opening = true;
    card.classList.add("opening");

    try {
      // Get current attention item for session_id
      const res = await fetch("http://127.0.0.1:17787/api/state");
      const state = await res.json();
      const attention = state.attention || [];
      const first = attention[0];

      await fetch("http://127.0.0.1:17787/api/open-webui", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          session_id: (first && first.session_id) || "",
        }),
      });
    } catch (e) {
      // Fallback: open homepage
      await fetch("http://127.0.0.1:17787/api/open-webui", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ session_id: "" }),
      });
    }

    setTimeout(() => {
      opening = false;
      card.classList.remove("opening");
    }, 2000);
  });
}

// ── Poll ──────────────────────────────────────────────────

async function poll() {
  const card = document.getElementById("card");
  if (!card) return;

  try {
    const res = await fetch("http://127.0.0.1:17787/api/state");
    if (!res.ok) {
      card.className = "";
      setBubbleVisible(false);
      lastState = null;
      return;
    }
    const state = await res.json();

    // Only act on state changes
    const stateKey = JSON.stringify(state);
    if (stateKey === lastState) return;
    lastState = stateKey;

    const attention = state.attention || [];
    const resolved = state.resolved_animation || state.state || "idle";

    let status = null;
    let item = null;

    // Priority chain is resolved by the Rust backend — frontend just maps to card states.
    if (resolved === "waiting") {
      status = "approval";
      item = attention.find((a) => a.status === "approval");
    } else if (resolved === "review") {
      status = "clarify";
      item = attention.find((a) => a.status === "clarify");
    } else if (resolved === "running") {
      status = "running";
      item = attention[0];
    } else if (resolved === "waving") {
      status = "ready";
      item = attention[0];
    }
    // idle / failed → status stays null → card hides (fixes: failed state previously leaked)

    if (status) {
      card.className = `visible status-${status}`;
      document.getElementById("title").textContent =
        (item && item.title) || status;
      document.getElementById("text").textContent =
        (item && item.text) || "";
      setBubbleVisible(true);
    } else {
      card.className = "";
      setBubbleVisible(false);
    }
  } catch (e) {
    card.className = "";
    setBubbleVisible(false);
    lastState = null;
  }
}

init();
setInterval(poll, POLL_MS);
poll();
