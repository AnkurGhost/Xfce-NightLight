# Night Light Controller for XFCE

A small Rust + GTK4 application that stays in the XFCE Status Tray / Status Notifier area for the whole desktop session. It starts hidden when you log in, left-click the tray icon to open the controller, and close the window to hide it again. The tray icon stays present even when Night Light is switched off. Choose **Quit** from the tray menu only when you want to exit the application.

## Features

- Turn night light on or off.
- Adjust colour temperature from 2500 K to 6500 K.
- Quick presets: 3000 K, 4000 K, 4500 K, and 5500 K.
- Save the enabled state and temperature in `~/.config/night-light-controller/settings.json`.
- Restore the saved night-light state when the app starts.
- Start automatically at every XFCE login when installed with `install.sh`.
- Keep the tray item alive if XFCE starts its Status Notifier watcher a little later than the app.
- Stay running when the controller window is closed; exit only through the tray menu’s **Quit** item or when the desktop session shuts down.

## Requirements on Manjaro XFCE

This first version uses `gammastep` to set the display gamma ramps. It is intended for an X11 XFCE session. A Wayland session may require a compositor-specific backend.

Install the build and runtime dependencies:

```bash
sudo pacman -S --needed base-devel rust gtk4 gammastep
```

XFCE must have its **Status Tray / Status Notifier** area available on the panel. Modern XFCE versions include Status Notifier support in the Systray; add the relevant tray item from **Panel → Add New Items** if it is not already present.

## Build and install

Extract this folder, open a terminal inside it, then run:

```bash
./install.sh
```

The script builds the release binary and installs it into `~/.local/bin`. It also creates an application-menu entry and an XDG autostart entry.

Start it immediately with the controller window open:

```bash
~/.local/bin/night-light-controller --window
```

The app handles `--window` itself and passes only the executable name to GTK,
so GTK does not reject the app-specific option.

Or start it hidden in the tray:

```bash
night-light-controller
```

## Uninstall

From this project folder:

```bash
./uninstall.sh
```

Your saved settings are intentionally left in `~/.config/night-light-controller/`.

## Notes

- The app calls `gammastep -P -O TEMPERATURE` when enabling or changing the temperature, and `gammastep -x` when disabling.
- The app does not change the physical monitor's hardware settings; it applies a software colour-temperature adjustment.
- If the tray icon does not appear, confirm that the XFCE panel has a Status Tray / Status Notifier host and start the app from a terminal to see any D-Bus registration errors.
