// bubbles.js — Standalone bubble window for Hermes Companion

const POLL_MS = 1000;

let container = null;
let lastState = null;

function init() {
  // Click opens WebUI
  document.body.addEventListener("click", () => {
    const invoke = window.__TAURI__?.invoke || window.__TAURI__?.core?.invoke;
    if (invoke) invoke("open_webui").catch(() => {});
    fetch("http://127.0.0.1:17787/api/open-webui").catch(() => {});
  });
}

async function poll() {
  try {
    const res = await fetch("http://127.0.0.1:17787/api/state");
    if (!res.ok) { document.body.className = ""; return; }
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
      const cls = status === "approval" ? "status-approval"
        : status === "clarify" ? "status-clarify"
        : status === "running" ? "status-running"
        : status === "ready" ? "status-ready"
        : "";
      document.body.className = cls;
      document.getElementById("title-text").textContent =
        (item && item.title) || status;
      document.getElementById("text").textContent =
        (item && item.text) || "";
    } else {
      document.body.className = "empty";
    }
  } catch (e) {
    document.body.className = "empty";
  }
}

init();
setInterval(poll, POLL_MS);
poll();
