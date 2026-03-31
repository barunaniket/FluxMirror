use std::fs;
use std::path::PathBuf;
use std::process::Child;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

// ── Config ────────────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Config {
    pub device_ip: Option<String>,
    pub connections: Vec<SavedConnection>,
    pub max_size: Option<u32>,       // scrcpy --max-size, None = original
    pub video_bitrate: Option<String>, // scrcpy --video-bit-rate, None = default
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SavedConnection {
    pub id: String,
    pub name: String,
    pub address: String,
    pub last_connected: Option<String>,
    pub connection_type: Option<String>, // "wireless" | "wired"
}

#[derive(Serialize, Clone)]
pub struct UsbDevice {
    pub serial: String,
    pub model: String,
}

fn config_path() -> PathBuf {
    let mut path = dirs_next::home_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push(".fluxmirror_config.json");
    path
}

fn load_config_from_disk() -> Config {
    fs::read_to_string(config_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_config_to_disk(cfg: &Config) -> Result<(), String> {
    let json = serde_json::to_string_pretty(cfg).map_err(|e| e.to_string())?;
    fs::write(config_path(), json).map_err(|e| e.to_string())
}

fn simple_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let t = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    format!("{:x}", t)
}

fn now_iso() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("{}", secs)
}

// ── App state ─────────────────────────────────────────────────────────────────

pub struct AppState {
    pub scrcpy_process: Mutex<Option<Child>>,
    pub is_display_on: Mutex<bool>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            scrcpy_process: Mutex::new(None),
            is_display_on: Mutex::new(true),
        }
    }
}

// ── Commands ──────────────────────────────────────────────────────────────────

mod commands {
    use super::{
        load_config_from_disk, now_iso, save_config_to_disk,
        simple_id, AppState, Config, SavedConnection, UsbDevice,
    };
    use tauri::State;

    #[tauri::command]
    pub fn load_config() -> Config {
        load_config_from_disk()
    }

    #[tauri::command]
    pub fn save_ip(ip: String) -> Result<(), String> {
        let mut cfg = load_config_from_disk();
        cfg.device_ip = if ip.trim().is_empty() {
            None
        } else {
            Some(ip.trim().to_string())
        };
        save_config_to_disk(&cfg)
    }

    #[tauri::command]
    pub fn disconnect_device(_state: State<'_, AppState>) -> Result<(), String> {
        let mut cfg = load_config_from_disk();
        cfg.device_ip = None;
        save_config_to_disk(&cfg)
    }

    #[tauri::command]
    pub fn save_connection(name: String, address: String, connection_type: Option<String>) -> Result<SavedConnection, String> {
        let mut cfg = load_config_from_disk();
        let existing = cfg.connections.iter().position(|c| c.address == address.trim());
        let conn = SavedConnection {
            id: existing
                .map(|i| cfg.connections[i].id.clone())
                .unwrap_or_else(simple_id),
            name: name.trim().to_string(),
            address: address.trim().to_string(),
            last_connected: Some(now_iso()),
            connection_type,
        };
        if let Some(i) = existing {
            cfg.connections[i] = conn.clone();
        } else {
            cfg.connections.push(conn.clone());
        }
        cfg.device_ip = Some(address.trim().to_string());
        save_config_to_disk(&cfg)?;
        Ok(conn)
    }

    #[tauri::command]
    pub fn delete_connection(id: String) -> Result<(), String> {
        let mut cfg = load_config_from_disk();
        cfg.connections.retain(|c| c.id != id);
        save_config_to_disk(&cfg)
    }

    #[tauri::command]
    pub fn activate_connection(id: String) -> Result<SavedConnection, String> {
        let mut cfg = load_config_from_disk();
        let conn = cfg
            .connections
            .iter_mut()
            .find(|c| c.id == id)
            .ok_or("Connection not found")?;
        conn.last_connected = Some(now_iso());
        let conn_clone = conn.clone();
        cfg.device_ip = Some(conn_clone.address.clone());
        save_config_to_disk(&cfg)?;
        Ok(conn_clone)
    }

    #[tauri::command]
    pub fn start_mirror(state: State<AppState>) -> Result<String, String> {
        let mut guard = state.scrcpy_process.lock().unwrap();
        if let Some(mut child) = guard.take() {
            let _ = child.kill();
            let _ = child.wait();
        }

        let cfg = load_config_from_disk();
        let display_on = *state.is_display_on.lock().unwrap();

        let mut args: Vec<String> = vec![
            "--window-title".into(),
            "FluxMirror_Video".into(),
            "--always-on-top".into(),
        ];

        // Determine if the active connection is wired (USB) or wireless
        let is_wired = cfg.connections.iter()
            .find(|c| cfg.device_ip.as_deref() == Some(c.address.as_str()))
            .and_then(|c| c.connection_type.as_deref())
            .map(|t| t == "wired")
            .unwrap_or(false);

        if is_wired {
            if let Some(serial) = &cfg.device_ip {
                args.push("-s".into());
                args.push(serial.trim().to_string());
            }
        } else if let Some(ip) = &cfg.device_ip {
            if !ip.trim().is_empty() {
                args.push(format!("--tcpip={}", ip.trim()));
            }
        }

        if let Some(size) = cfg.max_size {
            if size > 0 {
                args.push("--max-size".into());
                args.push(size.to_string());
            }
        }

        if let Some(ref bitrate) = cfg.video_bitrate {
            if !bitrate.is_empty() {
                args.push("--video-bit-rate".into());
                args.push(bitrate.clone());
            }
        }

        if !display_on {
            args.push("--turn-screen-off".into());
            args.push("--stay-awake".into());
        }

        let child = std::process::Command::new("scrcpy")
            .args(&args)
            .spawn()
            .map_err(|e| format!("Failed to start scrcpy: {}", e))?;

        *guard = Some(child);
        Ok("STREAMING".into())
    }

    #[tauri::command]
    pub fn stop_mirror(state: State<AppState>) -> Result<(), String> {
        let mut guard = state.scrcpy_process.lock().unwrap();
        if let Some(mut child) = guard.take() {
            child.kill().map_err(|e| e.to_string())?;
            let _ = child.wait();
        }
        Ok(())
    }

    #[tauri::command]
    pub fn toggle_display(state: State<AppState>) -> Result<bool, String> {
        let mut display_on = state.is_display_on.lock().unwrap();
        *display_on = !*display_on;
        Ok(*display_on)
    }

    #[tauri::command]
    pub fn adb_volume_up() -> Result<(), String> {
        std::process::Command::new("adb")
            .args(["shell", "input", "keyevent", "24"])
            .spawn()
            .map_err(|e| format!("adb error: {}", e))?;
        Ok(())
    }

    #[tauri::command]
    pub fn adb_brightness(level: u32) -> Result<(), String> {
        let level = level.min(255).to_string();
        std::process::Command::new("adb")
            .args(["shell", "cmd", "display", "set-brightness", &level])
            .spawn()
            .map_err(|e| format!("adb error: {}", e))?;
        Ok(())
    }

    #[tauri::command]
    pub fn adb_pair(pair_address: String, code: String) -> Result<String, String> {
        let output = std::process::Command::new("adb")
            .args(["pair", pair_address.trim(), code.trim()])
            .output()
            .map_err(|e| format!("adb not found: {}", e))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let combined = format!("{}{}", stdout, stderr);

        if combined.contains("Successfully paired") {
            Ok(combined.trim().to_string())
        } else {
            Err(format!("Pairing failed: {}", combined.trim()))
        }
    }

    #[tauri::command]
    pub fn adb_connect(address: String) -> Result<String, String> {
        let output = std::process::Command::new("adb")
            .args(["connect", address.trim()])
            .output()
            .map_err(|e| format!("adb not found: {}", e))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let combined = format!("{}{}", stdout, stderr);

        if combined.contains("connected") || combined.contains("already connected") {
            Ok(combined.trim().to_string())
        } else {
            Err(format!("Connect failed: {}", combined.trim()))
        }
    }

    #[tauri::command]
    pub fn adb_tcpip() -> Result<String, String> {
        let output = std::process::Command::new("adb")
            .args(["-d", "tcpip", "5555"])
            .output()
            .map_err(|e| format!("adb not found: {}", e))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let combined = format!("{}{}", stdout, stderr);

        if output.status.success() {
            Ok(combined.trim().to_string())
        } else {
            Err(format!("tcpip failed: {}", combined.trim()))
        }
    }

    #[tauri::command]
    pub fn save_mirror_settings(max_size: Option<u32>, video_bitrate: Option<String>) -> Result<(), String> {
        let mut cfg = load_config_from_disk();
        cfg.max_size = max_size;
        cfg.video_bitrate = video_bitrate;
        save_config_to_disk(&cfg)
    }

    #[tauri::command]
    pub fn adb_list_usb_devices() -> Result<Vec<UsbDevice>, String> {
        let output = std::process::Command::new("adb")
            .args(["devices", "-l"])
            .output()
            .map_err(|e| format!("adb error: {}", e))?;
        let text = String::from_utf8_lossy(&output.stdout).to_string();
        let mut devices = Vec::new();
        for line in text.lines().skip(1) {
            let line = line.trim();
            if line.is_empty() { continue; }
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 2 { continue; }
            let serial = parts[0];
            let status = parts[1];
            if status != "device" { continue; }
            if serial.contains(':') { continue; } // skip wireless devices
            let model = parts.iter()
                .find(|p| p.starts_with("model:"))
                .map(|p| p.trim_start_matches("model:").replace('_', " "))
                .unwrap_or_else(|| serial.to_string());
            devices.push(UsbDevice { serial: serial.to_string(), model });
        }
        Ok(devices)
    }

    #[tauri::command]
    pub fn adb_get_ip() -> Result<String, String> {
        if let Ok(ip) = adb_shell_ip_route() { return Ok(ip); }
        if let Ok(ip) = adb_shell_ip_addr()  { return Ok(ip); }
        if let Ok(ip) = adb_shell_ifconfig() { return Ok(ip); }
        if let Ok(ip) = pc_default_gateway() {
            return Ok(format!("__hotspot__{}", ip));
        }
        Err("Could not detect phone IP. Enter it manually.".to_string())
    }

    fn pc_default_gateway() -> Result<String, String> {
        let out = std::process::Command::new("ip")
            .args(["route"])
            .output().map_err(|e| e.to_string())?;
        let text = String::from_utf8_lossy(&out.stdout).to_string();
        for line in text.lines() {
            if !line.starts_with("default") { continue; }
            let parts: Vec<&str> = line.split_whitespace().collect();
            if let Some(i) = parts.iter().position(|&p| p == "via") {
                if let Some(ip) = parts.get(i + 1) {
                    if is_private_ip(ip) { return Ok(ip.to_string()); }
                }
            }
        }
        Err("no gateway found".into())
    }

    fn adb_shell_ip_route() -> Result<String, String> {
        let out = std::process::Command::new("adb")
            .args(["-d", "shell", "ip", "route"])
            .output().map_err(|e| e.to_string())?;
        let text = String::from_utf8_lossy(&out.stdout).to_string();
        let mut best: Option<(u8, String)> = None;
        for line in text.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if let Some(i) = parts.iter().position(|&p| p == "src") {
                if let Some(ip) = parts.get(i + 1) {
                    if !is_private_ip(ip) { continue; }
                    let priority: u8 = if line.contains("wlan") { 0 }
                        else if line.contains("ap") { 1 }
                        else { 2 };
                    if best.as_ref().map_or(true, |(p, _)| priority < *p) {
                        best = Some((priority, ip.to_string()));
                    }
                }
            }
        }
        best.map(|(_, ip)| ip).ok_or_else(|| "not found".into())
    }

    fn adb_shell_ip_addr() -> Result<String, String> {
        let out = std::process::Command::new("adb")
            .args(["-d", "shell", "ip", "addr"])
            .output().map_err(|e| e.to_string())?;
        let text = String::from_utf8_lossy(&out.stdout).to_string();
        let mut current_iface = String::new();
        let mut fallback: Option<String> = None;
        for line in text.lines() {
            if !line.starts_with(' ') && !line.starts_with('\t') {
                current_iface = line.to_lowercase();
            }
            let line = line.trim();
            if line.starts_with("inet ") && !line.starts_with("inet6") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if let Some(addr) = parts.get(1) {
                    let ip = addr.split('/').next().unwrap_or("");
                    if is_private_ip(ip) {
                        if current_iface.contains("wlan") { return Ok(ip.to_string()); }
                        fallback = Some(ip.to_string());
                    }
                }
            }
        }
        fallback.ok_or_else(|| "not found".into())
    }

    fn adb_shell_ifconfig() -> Result<String, String> {
        let out = std::process::Command::new("adb")
            .args(["-d", "shell", "ifconfig"])
            .output().map_err(|e| e.to_string())?;
        let text = String::from_utf8_lossy(&out.stdout).to_string();
        let mut current_iface = String::new();
        let mut fallback: Option<String> = None;
        for line in text.lines() {
            if !line.starts_with(' ') && !line.starts_with('\t') {
                current_iface = line.to_lowercase();
            }
            let line = line.trim();
            if line.contains("inet addr:") {
                if let Some(start) = line.find("inet addr:") {
                    let ip = line[start + 10..].split_whitespace().next().unwrap_or("");
                    if is_private_ip(ip) {
                        if current_iface.contains("wlan") { return Ok(ip.to_string()); }
                        fallback = Some(ip.to_string());
                    }
                }
            } else if line.starts_with("inet ") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if let Some(ip) = parts.get(1) {
                    if is_private_ip(ip) {
                        if current_iface.contains("wlan") { return Ok(ip.to_string()); }
                        fallback = Some(ip.to_string());
                    }
                }
            }
        }
        fallback.ok_or_else(|| "not found".into())
    }

    fn is_private_ip(ip: &str) -> bool {
        let parts: Vec<&str> = ip.split('.').collect();
        if parts.len() != 4 { return false; }
        let nums: Vec<u8> = match parts.iter().map(|p| p.parse::<u8>()).collect::<Result<Vec<_>, _>>() {
            Ok(n) => n, Err(_) => return false,
        };
        if nums[0] == 127 { return false; }
        if nums[0] == 169 && nums[1] == 254 { return false; }
        matches!(nums[0], 10) ||
        (nums[0] == 172 && (16..=31).contains(&nums[1])) ||
        (nums[0] == 192 && nums[1] == 168)
    }

}

// ── Entry point ───────────────────────────────────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::load_config,
            commands::save_ip,
            commands::disconnect_device,
            commands::save_connection,
            commands::delete_connection,
            commands::activate_connection,
            commands::start_mirror,
            commands::stop_mirror,
            commands::toggle_display,
            commands::adb_volume_up,
            commands::adb_brightness,
            commands::adb_pair,
            commands::adb_connect,
            commands::adb_tcpip,
            commands::adb_get_ip,
            commands::adb_list_usb_devices,
            commands::save_mirror_settings,
        ])
        .run(tauri::generate_context!())
        .expect("error while running FluxMirror");
}
