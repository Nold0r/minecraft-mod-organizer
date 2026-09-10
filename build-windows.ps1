$ErrorActionPreference = 'Stop'

Write-Host '== Minecraft Mod Organizer: Windows production build ==' -ForegroundColor Cyan

function Require-Command($name, $hint) {
  if (-not (Get-Command $name -ErrorAction SilentlyContinue)) {
    throw "$name not found. $hint"
  }
}

Require-Command node 'Install Node.js LTS first.'
Require-Command npm 'npm should be installed together with Node.js.'
Require-Command rustc 'Install Rust from rustup.rs (stable MSVC toolchain).'
Require-Command cargo 'Install Rust from rustup.rs (stable MSVC toolchain).'

Write-Host "Node:  $(node --version)"
Write-Host "npm:   $(npm --version)"
Write-Host "Rust:  $(rustc --version)"
Write-Host "Cargo: $(cargo --version)"

Write-Host '\nInstalling JavaScript dependencies...' -ForegroundColor Yellow
npm install
if ($LASTEXITCODE -ne 0) { throw "npm install failed with exit code $LASTEXITCODE" }

Write-Host '\nBuilding Tauri application...' -ForegroundColor Yellow
npm run tauri build
if ($LASTEXITCODE -ne 0) { throw "Tauri build failed with exit code $LASTEXITCODE" }

$release = Join-Path $PSScriptRoot 'src-tauri\target\release'
Write-Host "\nBuild complete. Release directory:" -ForegroundColor Green
Write-Host $release

$exe = Get-ChildItem $release -Filter '*.exe' -File -ErrorAction SilentlyContinue | Select-Object -First 1
if ($exe) { Write-Host "Executable: $($exe.FullName)" -ForegroundColor Green }

$bundle = Join-Path $release 'bundle'
if (Test-Path $bundle) {
  Write-Host "Installers: $bundle" -ForegroundColor Green
}
