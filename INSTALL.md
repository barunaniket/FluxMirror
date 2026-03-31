# FluxMirror Installation & Build Guide

FluxMirror uses Tauri (Rust + Web Technologies) to provide a lightweight, native desktop experience. To make building the app from source as painless as possible, we've included automated setup scripts for Windows, macOS, and Linux.

These scripts will automatically install the required build tools (Node.js, Rust, C++ Build Tools / WebKit) as well as the runtime dependencies required for screen mirroring (`scrcpy` and `adb`), and then build the final executable for your system.

---

## Step 1: Clone the Repository

First, download the source code to your local machine:

```bash
git clone [https://github.com/yourusername/fluxmirror.git](https://github.com/yourusername/fluxmirror.git)
cd fluxmirror
```

---

## Step 2: Run the Automated Setup

Choose the instructions for your operating system below.

### 🪟 Windows

The Windows setup uses `winget` (Windows Package Manager) to install Node.js, Rust, Visual Studio C++ Build Tools, and scrcpy/ADB.

1. Open **PowerShell as an Administrator**.
2. Navigate to the cloned `fluxmirror` directory.
3. Run the setup script. You will need to temporarily bypass the execution policy to run the local script:

```powershell
powershell -ExecutionPolicy Bypass -File setup.ps1
```

*Note: The Visual Studio Build Tools installation may take several minutes and might require a system restart once finished.*

### 🍎 macOS & 🐧 Linux

The Unix setup script automatically detects your OS/Distribution and uses the native package manager (`brew`, `apt`, `pacman`, or `dnf`) to install all required dependencies, including `scrcpy` and Android Platform Tools.

1. Open your **Terminal**.
2. Navigate to the cloned `fluxmirror` directory.
3. Make the script executable and run it:

```bash
chmod +x setup.sh
./setup.sh
```

*Supported Linux Distributions out-of-the-box: Arch Linux/Manjaro, Fedora, and Debian/Ubuntu-based systems.*

---

## Step 3: Locate Your Built Application

Once the script finishes successfully, it will compile the frontend and run the Tauri bundler. You can find your ready-to-use application installers and executables in the following directory:

```text
src-tauri/target/release/bundle/
```

* **Windows:** Look in the `msi` or `nsis` folders for your installer, or `exe` for the standalone executable.
* **macOS:** Look in the `dmg` folder for your disk image, or `macos` for the `.app` bundle.
* **Linux:** Look in the `appimage` folder for the standalone `.AppImage`, or `deb` for the Debian package.

---

## 🛠️ Manual Development Setup

If you prefer not to use the automated scripts and want to develop FluxMirror manually, ensure you have the following installed on your system:

1. **Node.js** (v18+)
2. **Rust** (`rustup`)
3. **OS-Specific Build Tools** (Visual Studio C++ Build Tools for Windows, Xcode Command Line Tools for Mac, `webkit2gtk-4.1` for Linux)
4. **scrcpy** and **adb** (must be available in your system's PATH)

Then, run the following commands to start the development server with Hot Module Replacement (HMR):

```bash
npm install
npm run tauri dev
```

or if it doesn't work when use the below command
```bash
WEBKIT_DISABLE_DMABUF_RENDERER=1 npm run tauri dev
```