Write-Host "🚀 Starting FluxMirror Environment Setup for Windows..." -ForegroundColor Cyan

# 1. Check for winget
if (-not (Get-Command winget -ErrorAction SilentlyContinue)) {
    Write-Host "❌ winget is not installed. Please update Windows or install App Installer from the Microsoft Store." -ForegroundColor Red
    exit
}

# 2. Install Node.js
if (-not (Get-Command npm -ErrorAction SilentlyContinue)) {
    Write-Host "📦 Installing Node.js..."
    winget install -e --id OpenJS.NodeJS
    $env:Path = [System.Environment]::GetEnvironmentVariable("Path","Machine") + ";" + [System.Environment]::GetEnvironmentVariable("Path","User")
}

# 3. Install Rust
if (-not (Get-Command rustc -ErrorAction SilentlyContinue)) {
    Write-Host "🦀 Installing Rust..."
    winget install -e --id Rustlang.Rustup
    $env:Path = [System.Environment]::GetEnvironmentVariable("Path","Machine") + ";" + [System.Environment]::GetEnvironmentVariable("Path","User")
}

# 4. Install Visual Studio Build Tools (Required for Tauri on Windows)
Write-Host "🛠️ Ensuring Visual Studio C++ Build Tools are installed..."
winget install -e --id Microsoft.VisualStudio.2022.BuildTools --custom "--add Microsoft.VisualStudio.Workload.VCTools --includeRecommended --quiet --wait"

# 5. Install scrcpy and ADB
Write-Host "📱 Installing scrcpy and ADB..."
winget install -e --id Genymobile.scrcpy

# 6. Build the App
Write-Host "📦 Installing NPM dependencies..."
npm install

Write-Host "🔨 Building FluxMirror (.exe and .msi)..."
npm run tauri build

Write-Host "✅ Setup and Build Complete! Check 'src-tauri\target\release\bundle' for your installers." -ForegroundColor Green