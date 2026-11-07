#!/usr/bin/env bash
set -euo pipefail
rm -f "$HOME/.local/bin/night-light-controller"
rm -f "$HOME/.local/share/applications/night-light-controller.desktop"
rm -f "$HOME/.config/autostart/night-light-controller.desktop"
rm -f "$HOME/.local/share/icons/hicolor/scalable/apps/night-light-controller.svg"
echo "Night Light Controller has been removed. Your settings file was kept at:"
echo "$HOME/.config/night-light-controller/settings.json"
