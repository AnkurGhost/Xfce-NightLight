#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

if ! command -v cargo >/dev/null 2>&1; then
  echo "Rust/Cargo is missing. Install it with: sudo pacman -S rust"
  exit 1
fi
if ! command -v gammastep >/dev/null 2>&1; then
  echo "gammastep is missing. Install it with: sudo pacman -S gammastep"
  exit 1
fi

cd "$ROOT_DIR"
cargo build --release

install -Dm755 "$ROOT_DIR/target/release/night-light-controller" \
  "$HOME/.local/bin/night-light-controller"
install -Dm644 "$ROOT_DIR/assets/night-light-controller.svg" \
  "$HOME/.local/share/icons/hicolor/scalable/apps/night-light-controller.svg"

mkdir -p "$HOME/.local/share/applications" "$HOME/.config/autostart"
cat > "$HOME/.local/share/applications/night-light-controller.desktop" <<DESKTOP
[Desktop Entry]
Type=Application
Name=Night Light Controller
Comment=Control screen colour temperature from the XFCE system tray
Exec=$HOME/.local/bin/night-light-controller
Icon=night-light-controller
Terminal=false
Categories=Utility;Settings;
StartupNotify=false
DESKTOP
cp "$HOME/.local/share/applications/night-light-controller.desktop" \
  "$HOME/.config/autostart/night-light-controller.desktop"

if command -v gtk-update-icon-cache >/dev/null 2>&1; then
  gtk-update-icon-cache -f -t "$HOME/.local/share/icons/hicolor" >/dev/null 2>&1 || true
fi

echo
echo "Installed Night Light Controller."
echo "It is configured to start automatically at every XFCE login."
echo "Starting it now in the system tray..."
nohup "$HOME/.local/bin/night-light-controller" >/dev/null 2>&1 </dev/null &
echo "The tray icon should remain visible until you choose Quit or shut down/log out."
echo "Open its control window by left-clicking the tray icon."
