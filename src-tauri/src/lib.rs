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
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SavedConnection {
    pub id: String,
    pub name: String,
    pub address: String,
    pub last_connected: Option<String>,
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
    // Simple timestamp without external deps
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
        load_config_from_disk, now_iso, save_config_to_disk, simple_id, AppState, Config,
        SavedConnection,
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

    // Save a named connection to the connections list
    #[tauri::command]
    pub fn save_connection(name: String, address: String) -> Result<SavedConnection, String> {
        let mut cfg = load_config_from_disk();
        // Update existing if same address, otherwise add new
        let existing = cfg.connections.iter().position(|c| c.address == address.trim());
        let conn = SavedConnection {
            id: existing
                .map(|i| cfg.connections[i].id.clone())
                .unwrap_or_else(simple_id),
            name: name.trim().to_string(),
            address: address.trim().to_string(),
            last_connected: Some(now_iso()),
        };
        if let Some(i) = existing {
            cfg.connections[i] = conn.clone();
        } else {
            cfg.connections.push(conn.clone());
        }
        // Also set as active IP
        cfg.device_ip = Some(address.trim().to_string());
        save_config_to_disk(&cfg)?;
        Ok(conn)
    }

    // Delete a saved connection by id
    #[tauri::command]
    pub fn delete_connection(id: String) -> Result<(), String> {
        let mut cfg = load_config_from_disk();
        cfg.connections.retain(|c| c.id != id);
        save_config_to_disk(&cfg)
    }

    // Activate a saved connection — sets it as the active IP and updates last_connected
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

        if let Some(ip) = &cfg.device_ip {
            if !ip.trim().is_empty() {
                args.push(format!("--tcpip={}", ip.trim()));
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
            .args(["tcpip", "5555"])
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

    /// Read the phone's WiFi IP directly over USB via `adb shell ip route`
    /// Returns just the IP string, e.g. "192.168.1.42"
    #[tauri::command]
    pub fn adb_get_ip() -> Result<String, String> {
        let output = std::process::Command::new("adb")
            .args(["shell", "ip", "route"])
            .output()
            .map_err(|e| format!("adb not found: {}", e))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();

        // `ip route` output looks like:
        //   192.168.1.0/24 dev wlan0 proto kernel scope link src 192.168.1.42
        // We want the IP after "src"
        for line in stdout.lines() {
            if line.contains("wlan") || line.contains("src") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if let Some(src_pos) = parts.iter().position(|&p| p == "src") {
                    if let Some(ip) = parts.get(src_pos + 1) {
                        return Ok(ip.to_string());
                    }
                }
            }
        }

        Err("Could not detect phone IP. Make sure WiFi is on and USB debugging is active.".to_string())
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running FluxMirror");
}