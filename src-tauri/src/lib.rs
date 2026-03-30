use std::fs;
use std::path::PathBuf;
use std::process::Child;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

// ── Config ────────────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Config {
    pub device_ip: Option<String>,
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

// ── Commands (isolated module prevents Tauri macro name collisions) ───────────

mod commands {
    use super::{load_config_from_disk, save_config_to_disk, AppState, Config};
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
        let new_val = *display_on;
        Ok(new_val)
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
            commands::start_mirror,
            commands::stop_mirror,
            commands::toggle_display,
            commands::adb_volume_up,
            commands::adb_brightness,
        ])
        .run(tauri::generate_context!())
        .expect("error while running FluxMirror");
}