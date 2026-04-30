@echo off
chcp 65001 >nul
echo ============================================
echo   RS-Claw v0.3.0 - Windows GUI Build
echo ============================================
echo.
echo Prerequisites:
echo   1. Install Rust from https://rustup.rs
echo   2. Then run: cargo install tauri-cli
echo.
echo Starting build...
echo.

cd /d "%~dp0"

echo [1/3] Checking Rust...
rustup --version >nul 2>&1
if %errorlevel% neq 0 (
    echo Rust not found - install from https://rustup.rs
    pause
    exit /b 1
)
echo Rust OK

echo [2/3] Checking tauri-cli...
cargo tauri --version >nul 2>&1
if %errorlevel% neq 0 (
    echo Installing tauri-cli...
    cargo install tauri-cli --version "^2"
)
echo tauri-cli OK

echo [3/3] Building GUI...
cd src-tauri
cargo tauri build --no-bundle

if %errorlevel% equ 0 (
    echo.
    echo ============================================
    echo   SUCCESS!
    echo   Output: target\release\rs-claw-gui.exe
    echo   Double-click to run.
    echo ============================================
) else (
    echo.
    echo BUILD FAILED - check errors above
)

pause
