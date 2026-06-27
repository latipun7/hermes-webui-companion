// bubbles.js — Light-themed bubble card for Hermes Companion

const POLL_MS = 1000;
let lastState = null;

function init() {
  const card = document.getElementById("card");
  if (!card) return;

  card.addEventListener("click", () => {
    fetch("http://127.0.0.1:17787/api/open-webui").catch(() => {});
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
