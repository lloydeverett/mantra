# Tools: cargo install tauri-cli --version "^2" --locked; cargo install just --locked

set windows-shell := ["powershell.exe", "-NoLogo", "-NoProfile", "-Command"]

# Run the app
run:
    cargo run

# Debug build
build:
    cargo build

# Release build with installers
bundle:
    cargo tauri build

# Regenerate every icon size from icons/icon.svg (desktop only)
[unix]
icons:
    cargo tauri icon icons/icon.svg --output icons
    rm -rf icons/android icons/ios

# Regenerate every icon size from icons/icon.svg (desktop only)
[windows]
icons:
    cargo tauri icon icons/icon.svg --output icons
    Remove-Item -Recurse -Force icons/android, icons/ios

# Headless smoke screenshot to target/shot.png (Linux/WSL)
shot:
    python3 scripts/drive.py
