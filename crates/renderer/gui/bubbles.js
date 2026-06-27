// bubbles.js — Light-themed bubble card for Hermes Companion
//
// Polls the bridge server for companion state and renders
// a card notification above the pet. Click opens the session
// in WebUI via POST /api/open-webui with session_id.

const POLL_MS = 1000;
let lastState = null;
let opening = false; // true while waiting for browser focus

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

async function poll() {
  const card = document.getElementById("card");
  if (!card) return;

  try {
    const res = await fetch("http://127.0.0.1:17787/api/state");
    if (!res.ok) { card.className = ""; return; }
    const state = await res.json();

    if (JSON.stringify(state) === lastState) return;
    lastState = JSON.stringify(state);

    const attention = state.attention || [];
    const companionState = (state.state || "idle").toLowerCase();

    let status = null;
    let item = null;

    if (attention.some((a) => a.status === "approval")) {
      status = "approval";
      item = attention.find((a) => a.status === "approval");
    } else if (attention.some((a) => a.status === "clarify")) {
      status = "clarify";
      item = attention.find((a) => a.status === "clarify");
    } else if (companionState === "running") {
      status = "running";
      item = attention[0];
    } else if (companionState === "ready") {
      status = "ready";
      item = attention[0];
    }

    if (status) {
      card.className = `visible status-${status}`;
      document.getElementById("title").textContent =
        (item && item.title) || status;
      document.getElementById("text").textContent =
        (item && item.text) || "";
    } else {
      card.className = "";
    }
  } catch (e) {
    card.className = "";
  }
}

init();
setInterval(poll, POLL_MS);
poll();
