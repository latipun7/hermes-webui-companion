// bubbles.js — Standalone bubble window for Hermes Companion
//
// Polls the bridge server (127.0.0.1:17787) for companion state
// and renders a speech-bubble notification above the pet.

const POLL_MS = 1000;

const STATUS_CLASSES = {
  approval: "status-approval",
  clarify: "status-clarify",
  running: "status-running",
  ready: "status-ready",
};

const STATUS_LABELS = {
  approval: "Approval",
  clarify: "Question",
  running: "Processing",
  ready: "Complete",
};

let container = null;
let lastState = null;

function init() {
  container = document.getElementById("container");
  const clickTarget = document.getElementById("click-target");
  if (!clickTarget) return;

  // Click opens WebUI
  clickTarget.addEventListener("click", () => {
    // Visual feedback
    clickTarget.style.background = "rgba(51, 65, 85, 0.5)";
    setTimeout(() => { clickTarget.style.background = ""; }, 200);

    // DEBUG: test if navigation works at all
    // window.location.href = "http://localhost:8787"; // uncomment to test

    // Try Tauri IPC
    const invoke = window.__TAURI__?.invoke || window.__TAURI__?.core?.invoke;
    if (invoke) {
      invoke("open_webui").catch(() => {
        container.style.background = "rgba(255,0,0,0.5)"; // red = error
      });
      invoke("plugin:shell|open", { path: "http://localhost:8787" }).catch(() => {});
    }
  });
}

async function poll() {
  try {
    const res = await fetch("http://127.0.0.1:17787/api/state");
    const state = await res.json();

    if (JSON.stringify(state) === lastState) return;
    lastState = JSON.stringify(state);

    const attention = state.attention || [];
    const companionState = (state.state || "idle").toLowerCase();

    // Priority: approval > clarify > running/ready
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
      container.className = `visible ${STATUS_CLASSES[status] || ""}`;
      document.getElementById("title-text").textContent =
        (item && item.title) || STATUS_LABELS[status] || status;
      document.getElementById("text").textContent =
        (item && item.text) || "";
    } else {
      container.className = "";
    }
  } catch (e) {
    // bridge not reachable — hide bubble
    container.className = "";
  }
}

init();
setInterval(poll, POLL_MS);
poll();
