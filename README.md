# FluxMirror

A desktop app for wirelessly (or wired) mirroring your Android phone screen to your PC. Built with [Tauri 2](https://tauri.app/) and [scrcpy](https://github.com/Genymobile/scrcpy).

---

## Features

- **Wireless mirroring** via ADB over Wi-Fi — no companion app needed on the phone
- **Wired mirroring** via USB — plug in and mirror directly over the cable
- **Two wireless connection methods:**
  - No USB (Android 11+) — pair entirely over Wi-Fi using Wireless Debugging
  - USB Once — use USB to enable TCP/IP, then go wireless
- **Saved connections** — name and save devices for instant one-click reconnect
- **Resolution control** — limit stream to 360p / 480p / 720p / 1080p or keep original
- **Bitrate control** — set video bitrate from 2 Mbps up to 24 Mbps
- **Physical screen toggle** — keep the phone screen off while mirroring
- **Volume & brightness controls** via ADB

---

## Requirements

- [scrcpy](https://github.com/Genymobile/scrcpy) installed and available in `PATH`
- [ADB](https://developer.android.com/tools/adb) installed and available in `PATH` (usually bundled with scrcpy)
- USB debugging enabled on your Android device

---

## Connection Modes

### Wireless — No USB (Android 11+)
1. On your phone: Settings → Developer Options → Wireless Debugging → **Pair device with pairing code**
2. In FluxMirror: Connections → New Connection → **No USB** tab
3. Enter the pairing IP:port and 6-digit code shown on your phone
4. After pairing, enter the main Wireless Debugging IP:port and connect

### Wireless — USB Once
1. Plug your phone in via USB and accept the debug prompt
2. In FluxMirror: Connections → New Connection → **USB Once** tab
3. Click **Enable TCP/IP over USB** — the app detects your phone's IP automatically
4. Unplug USB, enter the IP if needed, and connect

### Wired — USB
1. Plug your phone in via USB and accept the debug prompt
2. In FluxMirror: Connections → New Connection → **USB Wired** tab
3. Click **Scan for USB Devices** and connect to your device
4. The connection is saved by device model name for future use

---

## Quality Settings

The sidebar exposes two controls that apply to the next mirror session (or restart it immediately if already streaming):

| Setting | Options |
|---|---|
| Resolution | Original, 1080p, 720p, 480p, 360p |
| Bitrate | Default, 2 / 4 / 8 / 16 / 24 Mbps |

Settings are persisted to `~/.fluxmirror_config.json`.

---

## Tech Stack

| Layer | Technology |
|---|---|
| App shell | Tauri 2 (Rust backend) |
| Frontend | Vanilla HTML / CSS / JS |
| Mirroring | scrcpy |
| Device communication | ADB |

---

## Development

```bash
# Install dependencies
npm install

# Run in development mode
npm run tauri dev

# Build for production
npm run tauri build
```
