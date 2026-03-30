const { invoke } = window.__TAURI__.core;

// ── State ─────────────────────────────────────────────────────────────────────
let isStreaming  = false;
let isDisplayOn  = true;
let savedIp      = "";

// ── DOM refs ──────────────────────────────────────────────────────────────────
const btnStart    = document.getElementById("btn-start");
const startLabel  = document.getElementById("start-label");
const btnDisplay  = document.getElementById("btn-display");
const displayLabel = document.getElementById("display-label");
const btnVol      = document.getElementById("btn-vol");
const btnBri      = document.getElementById("btn-bri");
const ipInput     = document.getElementById("ip-input");
const ipHint      = document.getElementById("ip-hint");
const btnSaveIp   = document.getElementById("btn-save-ip");
const statusBadge = document.getElementById("status-badge");
const portalMsg   = document.getElementById("portal-msg");
const portal      = document.getElementById("mirror-portal");

// ── Init ──────────────────────────────────────────────────────────────────────
async function init() {
  try {
    const cfg = await invoke("load_config");
    if (cfg.device_ip) {
      savedIp = cfg.device_ip;
      ipInput.value = savedIp;
      setIpHint("saved");
    }
  } catch (e) {
    console.error("Failed to load config:", e);
  }
}

// ── IP hint ───────────────────────────────────────────────────────────────────
function setIpHint(state, text) {
  ipHint.className = "ip-hint " + state;
  if (text) {
    ipHint.textContent = text;
  } else if (state === "saved") {
    ipHint.textContent = "Remembered ✓";
  } else if (state === "error") {
    ipHint.textContent = text || "Enter an IP first";
  } else {
    ipHint.textContent = "Not saved yet";
  }
}

ipInput.addEventListener("input", () => {
  if (ipInput.value.trim() !== savedIp) setIpHint("unsaved");
});

// ── Save IP ───────────────────────────────────────────────────────────────────
btnSaveIp.addEventListener("click", async () => {
  const ip = ipInput.value.trim();
  if (!ip) { setIpHint("error", "Enter an IP first"); return; }
  try {
    await invoke("save_ip", { ip });
    savedIp = ip;
    setIpHint("saved");
  } catch (e) {
    setIpHint("error", "Save failed");
  }
});

// ── Start / stop mirror ───────────────────────────────────────────────────────
btnStart.addEventListener("click", async () => {
  if (isStreaming) {
    // Stop
    try {
      await invoke("stop_mirror");
    } catch (_) {}
    isStreaming = false;
    startLabel.textContent  = "Engine Start";
    btnStart.classList.remove("streaming");
    statusBadge.textContent = "READY";
    statusBadge.className   = "status-badge";
    portal.classList.remove("streaming");
    portalMsg.innerHTML = "Phone display area active.<br/>Launch the engine to begin.";
  } else {
    // Start
    try {
      await invoke("start_mirror");
      isStreaming = true;
      startLabel.textContent  = "Stop Engine";
      btnStart.classList.add("streaming");
      statusBadge.textContent = "STREAMING";
      statusBadge.className   = "status-badge streaming";
      portal.classList.add("streaming");
      const ip = ipInput.value.trim() || savedIp;
      portalMsg.innerHTML = ip
        ? `Streaming wirelessly from<br/><code style="color:var(--accent-cyan);font-size:13px">${ip}</code>`
        : "Streaming via USB…";
    } catch (e) {
      statusBadge.textContent = "ERROR";
      statusBadge.className   = "status-badge error";
      portalMsg.textContent   = "Failed to start scrcpy. Is it installed?";
    }
  }
});

// ── Toggle display ────────────────────────────────────────────────────────────
btnDisplay.addEventListener("click", async () => {
  try {
    const newVal = await invoke("toggle_display");
    isDisplayOn = newVal;
    displayLabel.textContent = `Physical Screen: ${isDisplayOn ? "ON" : "OFF"}`;
    // If streaming, restart with new display setting
    if (isStreaming) {
      await invoke("start_mirror");
    }
  } catch (e) {
    console.error("toggle_display failed:", e);
  }
});

// ── ADB shortcuts ─────────────────────────────────────────────────────────────
btnVol.addEventListener("click", async () => {
  try { await invoke("adb_volume_up"); } catch (_) {}
});

btnBri.addEventListener("click", async () => {
  try { await invoke("adb_brightness", { level: 200 }); } catch (_) {}
});

// ── Boot ──────────────────────────────────────────────────────────────────────
init();