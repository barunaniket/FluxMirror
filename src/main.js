const { invoke } = window.__TAURI__.core;

// ── State ─────────────────────────────────────────────────────────────────────
let isStreaming    = false;
let isDisplayOn    = true;
let activeConn     = null;   // { id, name, address, connection_type }
let pendingAddress = "";     // address waiting to be saved after connect
let pendingConnType = "wireless"; // connection type for pending save
let deleteTargetId = "";
let detectedIp     = "";     // IP from adb_get_ip, applied when panel-b is visible

// ── DOM: sidebar ──────────────────────────────────────────────────────────────
const btnStart       = document.getElementById("btn-start");
const startLabel     = document.getElementById("start-label");
const btnDisplay     = document.getElementById("btn-display");
const displayLabel   = document.getElementById("display-label");
const btnVol         = document.getElementById("btn-vol");
const btnBri         = document.getElementById("btn-bri");
const btnDisconnect  = document.getElementById("btn-disconnect");
const statusBadge    = document.getElementById("status-badge");
const activeConnEl   = document.getElementById("active-conn");
const activeConnAddr = document.getElementById("active-conn-addr");
const portalMsg      = document.getElementById("portal-msg");
const portal         = document.getElementById("mirror-portal");

// ── DOM: quality controls ─────────────────────────────────────────────────────
const selResolution = document.getElementById("sel-resolution");
const selBitrate    = document.getElementById("sel-bitrate");

// ── DOM: nav ──────────────────────────────────────────────────────────────────
const navBtns      = document.querySelectorAll(".nav-btn");
const pageHome     = document.getElementById("page-home");
const pageConns    = document.getElementById("page-connections");

// ── DOM: connections page ─────────────────────────────────────────────────────
const btnNewConn   = document.getElementById("btn-new-connection");
const cardsGrid    = document.getElementById("cards-grid");
const emptyState   = document.getElementById("empty-state");

// ── DOM: setup modal ──────────────────────────────────────────────────────────
const modalSetup     = document.getElementById("modal-setup");
const methodTabs     = document.querySelectorAll(".method-tab");
const panelA         = document.getElementById("panel-a");
const panelB         = document.getElementById("panel-b");
const panelC         = document.getElementById("panel-c");
const btnScanUsb     = document.getElementById("btn-scan-usb");
const usbScanStatus  = document.getElementById("usb-scan-status");
const usbDeviceList  = document.getElementById("usb-device-list");
const pairAddress    = document.getElementById("pair-address");
const pairCode       = document.getElementById("pair-code");
const btnPair        = document.getElementById("btn-pair");
const pairStatus     = document.getElementById("pair-status");
const connectAddrA   = document.getElementById("connect-address-a");
const btnConnectA    = document.getElementById("btn-connect-a");
const connectStatusA = document.getElementById("connect-status-a");
const btnTcpip       = document.getElementById("btn-tcpip");
const tcpipStatus    = document.getElementById("tcpip-status");
const connectAddrB   = document.getElementById("connect-address-b");
const btnConnectB    = document.getElementById("btn-connect-b");
const connectStatusB = document.getElementById("connect-status-b");

// ── DOM: save modal ───────────────────────────────────────────────────────────
const modalSave      = document.getElementById("modal-save");
const connNameInput  = document.getElementById("conn-name");
const saveAddrPreview= document.getElementById("save-addr-preview");
const btnSaveSkip    = document.getElementById("btn-save-skip");
const btnSaveConfirm = document.getElementById("btn-save-confirm");

// ── DOM: delete modal ─────────────────────────────────────────────────────────
const modalDelete    = document.getElementById("modal-delete");
const deleteDesc     = document.getElementById("delete-desc");
const btnDeleteConfirm = document.getElementById("btn-delete-confirm");

// ── Helpers ───────────────────────────────────────────────────────────────────
function setStatus(text, cls) {
  statusBadge.textContent = text;
  statusBadge.className   = "status-badge" + (cls ? " " + cls : "");
}

function setModalStatus(el, state, text) {
  el.className   = "modal-status " + state;
  el.textContent = text;
}

function openModal(el)  { el.classList.add("open"); }
function closeModal(el) { el.classList.remove("open"); }

function formatLastConnected(ts) {
  if (!ts) return "Never connected";
  const secs = parseInt(ts, 10);
  if (isNaN(secs)) return "";
  const d = new Date(secs * 1000);
  return "Last: " + d.toLocaleDateString() + " " + d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
}

function setActiveConnection(conn) {
  activeConn = conn;
  if (conn) {
    activeConnEl.querySelector(".active-conn-label").textContent = conn.name;
    activeConnAddr.textContent = conn.address;
    btnDisconnect.classList.remove("hidden");
  } else {
    activeConnEl.querySelector(".active-conn-label").textContent = "No device connected";
    activeConnAddr.textContent = "";
    btnDisconnect.classList.add("hidden");
  }
  renderCards();
}

// Disconnect button
btnDisconnect.addEventListener("click", async () => {
  // Stop mirror if running
  if (isStreaming) {
    try { await invoke("stop_mirror"); } catch (_) {}
    isStreaming = false;
    startLabel.textContent = "Engine Start";
    btnStart.classList.remove("streaming");
    portal.classList.remove("streaming");
  }
  // Explicitly clear IP from config on disk
  try { await invoke("disconnect_device"); } catch (_) {}
  setActiveConnection(null);
  setStatus("READY");
  portalMsg.innerHTML = "Phone display area active.<br/>Launch the engine to begin.";
});

// ── Nav ───────────────────────────────────────────────────────────────────────
navBtns.forEach(btn => {
  btn.addEventListener("click", () => {
    navBtns.forEach(b => b.classList.remove("active"));
    btn.classList.add("active");
    const page = btn.dataset.page;
    pageHome.classList.toggle("hidden", page !== "home");
    pageConns.classList.toggle("hidden", page !== "connections");
    if (page === "connections") renderCards();
  });
});

// ── Init ──────────────────────────────────────────────────────────────────────
async function init() {
  try {
    const cfg = await invoke("load_config");
    if (cfg.device_ip && cfg.connections) {
      const match = cfg.connections.find(c => c.address === cfg.device_ip);
      if (match) setActiveConnection(match);
    }
    // Restore quality settings
    if (cfg.max_size) selResolution.value = String(cfg.max_size);
    if (cfg.video_bitrate) selBitrate.value = cfg.video_bitrate;
    renderCards();
  } catch (e) { console.error("init failed:", e); }
}

// ── Quality controls ──────────────────────────────────────────────────────────

async function applyQualitySettings() {
  const maxSize = selResolution.value ? parseInt(selResolution.value, 10) : null;
  const videoBitrate = selBitrate.value || null;
  try {
    await invoke("save_mirror_settings", { maxSize, videoBitrate });
    // If currently streaming, restart to apply
    if (isStreaming) {
      await invoke("stop_mirror");
      await invoke("start_mirror");
    }
  } catch (e) { console.error("save_mirror_settings failed:", e); }
}

selResolution.addEventListener("change", applyQualitySettings);
selBitrate.addEventListener("change", applyQualitySettings);

// ── Cards ─────────────────────────────────────────────────────────────────────
async function renderCards() {
  try {
    const cfg = await invoke("load_config");
    const conns = cfg.connections || [];
    cardsGrid.innerHTML = "";
    if (conns.length === 0) {
      emptyState.classList.remove("hidden");
      return;
    }
    emptyState.classList.add("hidden");
    conns.forEach(conn => {
      const isActive = activeConn && activeConn.id === conn.id;
      const card = document.createElement("div");
      card.className = "conn-card" + (isActive ? " active-card" : "");
      const addrDisplay = conn.connection_type === "wired"
        ? `USB · ${conn.address}`
        : conn.address;
      card.innerHTML = `
        <div class="conn-card-icon">${conn.connection_type === "wired" ? "🔗" : "📱"}</div>
        <div class="conn-card-name">${conn.name}</div>
        <div class="conn-card-addr">${addrDisplay}</div>
        <div class="conn-card-meta">${formatLastConnected(conn.last_connected)}</div>
        <div class="conn-card-actions">
          <button class="btn conn-card-btn btn-connect-card" data-id="${conn.id}">
            ${isActive ? "✓ Connected" : "Connect"}
          </button>
          <button class="btn conn-card-btn btn-delete-card" data-id="${conn.id}" data-name="${conn.name}">
            Remove
          </button>
        </div>
      `;
      cardsGrid.appendChild(card);
    });

    // Connect buttons
    cardsGrid.querySelectorAll(".btn-connect-card").forEach(btn => {
      btn.addEventListener("click", async (e) => {
        e.stopPropagation();
        const id = btn.dataset.id;
        await connectById(id);
      });
    });

    // Delete buttons
    cardsGrid.querySelectorAll(".btn-delete-card").forEach(btn => {
      btn.addEventListener("click", (e) => {
        e.stopPropagation();
        deleteTargetId = btn.dataset.id;
        deleteDesc.textContent = `Remove "${btn.dataset.name}" from saved connections?`;
        openModal(modalDelete);
      });
    });

  } catch (e) { console.error("renderCards failed:", e); }
}

async function connectById(id) {
  try {
    const conn = await invoke("activate_connection", { id });
    setStatus("CONNECTING…", "");
    if (conn.connection_type !== "wired") {
      await invoke("adb_connect", { address: conn.address });
    }
    setActiveConnection(conn);
    setStatus("CONNECTED", "streaming");
    const addrLabel = conn.connection_type === "wired"
      ? `<code style="color:var(--accent-cyan);font-size:12px">USB · ${conn.address}</code>`
      : `<code style="color:var(--accent-cyan);font-size:12px">${conn.address}</code>`;
    portalMsg.innerHTML = `Connected to <strong>${conn.name}</strong><br/>${addrLabel}`;
    // Switch to home
    navBtns.forEach(b => b.classList.remove("active"));
    document.querySelector('[data-page="home"]').classList.add("active");
    pageConns.classList.add("hidden");
    pageHome.classList.remove("hidden");
  } catch (e) {
    setStatus("ERROR", "error");
    portalMsg.textContent = String(e);
  }
}

// ── Save connection flow ───────────────────────────────────────────────────────
function triggerSaveFlow(address, connType = "wireless") {
  pendingAddress = address;
  pendingConnType = connType;
  connNameInput.value = "";
  saveAddrPreview.textContent = address;
  closeModal(modalSetup);
  openModal(modalSave);
}

btnSaveSkip.addEventListener("click", () => {
  // Still set as active IP without a name
  invoke("save_ip", { ip: pendingAddress }).catch(() => {});
  closeModal(modalSave);
  pendingAddress = "";
});

btnSaveConfirm.addEventListener("click", async () => {
  const name = connNameInput.value.trim() || pendingAddress;
  try {
    const conn = await invoke("save_connection", { name, address: pendingAddress, connectionType: pendingConnType });
    setActiveConnection(conn);
    closeModal(modalSave);
    pendingAddress = "";
    pendingConnType = "wireless";
    renderCards();
  } catch (e) {
    console.error("save_connection failed:", e);
  }
});

// ── Delete connection ─────────────────────────────────────────────────────────
btnDeleteConfirm.addEventListener("click", async () => {
  try {
    await invoke("delete_connection", { id: deleteTargetId });
    if (activeConn && activeConn.id === deleteTargetId) setActiveConnection(null);
    closeModal(modalDelete);
    renderCards();
  } catch (e) { console.error("delete failed:", e); }
});

// ── Setup modal ───────────────────────────────────────────────────────────────
btnNewConn.addEventListener("click", () => openModal(modalSetup));

// Method tabs
methodTabs.forEach(tab => {
  tab.addEventListener("click", () => {
    methodTabs.forEach(t => t.classList.remove("active"));
    tab.classList.add("active");
    panelA.classList.toggle("hidden", tab.dataset.method !== "a");
    panelB.classList.toggle("hidden", tab.dataset.method !== "b");
    panelC.classList.toggle("hidden", tab.dataset.method !== "c");
    // Apply any detected IP now that panel-b is visible
    if (tab.dataset.method === "b" && detectedIp) {
      document.getElementById("connect-address-b").value = detectedIp;
    }
  });
});

// ── USB wired scan ────────────────────────────────────────────────────────────

btnScanUsb.addEventListener("click", async () => {
  setModalStatus(usbScanStatus, "pending", "Scanning…");
  usbDeviceList.innerHTML = "";
  try {
    const devices = await invoke("adb_list_usb_devices");
    if (devices.length === 0) {
      setModalStatus(usbScanStatus, "error", "No USB devices found. Check USB debugging is enabled.");
      return;
    }
    setModalStatus(usbScanStatus, "success", `${devices.length} device${devices.length > 1 ? "s" : ""} found`);
    devices.forEach(dev => {
      const item = document.createElement("div");
      item.className = "usb-device-item";
      item.innerHTML = `
        <div class="usb-device-info">
          <div class="usb-device-model">${dev.model}</div>
          <div class="usb-device-serial">${dev.serial}</div>
        </div>
        <button class="btn btn-connect-usb">Connect</button>
      `;
      item.querySelector(".btn-connect-usb").addEventListener("click", async () => {
        setModalStatus(usbScanStatus, "pending", "Saving connection…");
        try {
          const conn = await invoke("save_connection", {
            name: dev.model,
            address: dev.serial,
            connectionType: "wired",
          });
          setActiveConnection(conn);
          closeModal(modalSetup);
          setStatus("CONNECTED", "streaming");
          portalMsg.innerHTML = `Connected to <strong>${conn.name}</strong><br/><code style="color:var(--accent-cyan);font-size:12px">USB · ${conn.address}</code>`;
          navBtns.forEach(b => b.classList.remove("active"));
          document.querySelector('[data-page="home"]').classList.add("active");
          pageConns.classList.add("hidden");
          pageHome.classList.remove("hidden");
        } catch (e) {
          setModalStatus(usbScanStatus, "error", String(e));
        }
      });
      usbDeviceList.appendChild(item);
    });
  } catch (e) {
    setModalStatus(usbScanStatus, "error", String(e));
  }
});

// Generic close buttons
document.querySelectorAll("[data-close]").forEach(btn => {
  btn.addEventListener("click", () => {
    const target = document.getElementById(btn.dataset.close);
    if (target) closeModal(target);
  });
});
modalSetup.addEventListener("click", e => { if (e.target === modalSetup) closeModal(modalSetup); });
modalSave.addEventListener("click",  e => { if (e.target === modalSave)  closeModal(modalSave); });
modalDelete.addEventListener("click",e => { if (e.target === modalDelete) closeModal(modalDelete); });

// ── Method A: Pair ────────────────────────────────────────────────────────────
btnPair.addEventListener("click", async () => {
  const addr = pairAddress.value.trim();
  const code = pairCode.value.trim();
  if (!addr || !code) { setModalStatus(pairStatus, "error", "Enter IP:port and code"); return; }
  setModalStatus(pairStatus, "pending", "Pairing…");
  try {
    await invoke("adb_pair", { pairAddress: addr, code });
    setModalStatus(pairStatus, "success", "Paired successfully ✓ — now enter connect address below");
  } catch (e) {
    setModalStatus(pairStatus, "error", String(e));
  }
});

// ── Method A: Connect ─────────────────────────────────────────────────────────
btnConnectA.addEventListener("click", async () => {
  const addr = connectAddrA.value.trim();
  if (!addr) { setModalStatus(connectStatusA, "error", "Enter connect IP:port"); return; }
  setModalStatus(connectStatusA, "pending", "Connecting…");
  try {
    await invoke("adb_connect", { address: addr });
    setModalStatus(connectStatusA, "success", "Connected ✓");
    triggerSaveFlow(addr);
  } catch (e) {
    setModalStatus(connectStatusA, "error", String(e));
  }
});

// ── Method B: TCP/IP ──────────────────────────────────────────────────────────
btnTcpip.addEventListener("click", async () => {
  setModalStatus(tcpipStatus, "pending", "Enabling TCP/IP…");
  try {
    await invoke("adb_tcpip");
    setModalStatus(tcpipStatus, "pending", "Detecting phone IP…");
    try {
      let ip = await invoke("adb_get_ip");
      let displayIp = ip;
      let hint = `TCP/IP enabled · IP detected: ${ip} ✓ — unplug USB now`;

      if (ip.startsWith("__hotspot__")) {
        displayIp = ip.replace("__hotspot__", "");
        hint = `TCP/IP enabled ✓ — Hotspot detected, using gateway IP: ${displayIp}. Unplug USB now`;
      }

      detectedIp = displayIp;
      const ipField = document.getElementById("connect-address-b");
      if (ipField) ipField.value = displayIp;
      setModalStatus(tcpipStatus, "success", hint);
    } catch (_) {
      detectedIp = "";
      setModalStatus(tcpipStatus, "success", "TCP/IP enabled ✓ — unplug USB, then enter your phone's IP below");
    }
  } catch (e) {
    setModalStatus(tcpipStatus, "error", String(e));
  }
});

// ── Method B: Connect ─────────────────────────────────────────────────────────
btnConnectB.addEventListener("click", async () => {
  const ipField = document.getElementById("connect-address-b");
  let addr = ipField ? ipField.value.trim() : "";
  if (!addr) { setModalStatus(connectStatusB, "error", "Enter phone IP"); return; }
  if (!addr.includes(":")) addr += ":5555";
  setModalStatus(connectStatusB, "pending", "Connecting…");
  try {
    await invoke("adb_connect", { address: addr });
    setModalStatus(connectStatusB, "success", "Connected ✓");
    triggerSaveFlow(addr);
  } catch (e) {
    setModalStatus(connectStatusB, "error", String(e));
  }
});

// ── Mirror controls ───────────────────────────────────────────────────────────
btnStart.addEventListener("click", async () => {
  if (isStreaming) {
    try { await invoke("stop_mirror"); } catch (_) {}
    isStreaming = false;
    startLabel.textContent = "Engine Start";
    btnStart.classList.remove("streaming");
    setStatus("READY");
    portal.classList.remove("streaming");
    portalMsg.innerHTML = "Phone display area active.<br/>Launch the engine to begin.";
  } else {
    if (!activeConn) {
      portalMsg.innerHTML = "No device connected.<br/>Go to <strong>Connections</strong> to connect a device first.";
      setStatus("NO DEVICE", "error");
      return;
    }
    try {
      await invoke("start_mirror");
      isStreaming = true;
      startLabel.textContent = "Stop Engine";
      btnStart.classList.add("streaming");
      setStatus("STREAMING", "streaming");
      portal.classList.add("streaming");
      const name = activeConn ? activeConn.name : (activeConn?.address || "device");
      const addr = activeConn?.address || "";
      portalMsg.innerHTML = `Streaming from <strong>${name}</strong>${addr ? `<br/><code style="color:var(--accent-cyan);font-size:12px">${addr}</code>` : ""}`;
    } catch (e) {
      setStatus("ERROR", "error");
      portalMsg.textContent = "Failed to start scrcpy. Is it installed?";
    }
  }
});

btnDisplay.addEventListener("click", async () => {
  try {
    const newVal = await invoke("toggle_display");
    isDisplayOn = newVal;
    displayLabel.textContent = `Physical Screen: ${isDisplayOn ? "ON" : "OFF"}`;
    if (isStreaming) await invoke("start_mirror");
  } catch (e) { console.error(e); }
});

btnVol.addEventListener("click", async () => { try { await invoke("adb_volume_up"); } catch (_) {} });
btnBri.addEventListener("click", async () => { try { await invoke("adb_brightness", { level: 200 }); } catch (_) {} });

// ── Boot ──────────────────────────────────────────────────────────────────────
init();