use gtk::prelude::*;
use gtk::{gdk, glib, Align, Application, ApplicationWindow, Button, CssProvider, Grid, Label, Orientation, Scale, Switch};
use ksni::TrayMethods;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::rc::Rc;
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread;
use std::time::Duration;

const APP_ID: &str = "io.github.ankurpal.NightLightController";
const DEFAULT_TEMPERATURE: u16 = 4200;
const MIN_TEMPERATURE: f64 = 2500.0;
const MAX_TEMPERATURE: f64 = 6500.0;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Settings {
    enabled: bool,
    temperature: u16,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            enabled: false,
            temperature: DEFAULT_TEMPERATURE,
        }
    }
}

impl Settings {
    fn path() -> PathBuf {
        let base = env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
            .unwrap_or_else(|| PathBuf::from("."));
        base.join("night-light-controller").join("settings.json")
    }

    fn load() -> Self {
        fs::read_to_string(Self::path())
            .ok()
            .and_then(|contents| serde_json::from_str::<Self>(&contents).ok())
            .map(|mut settings| {
                settings.temperature = settings.temperature.clamp(2500, 6500);
                settings
            })
            .unwrap_or_default()
    }

    fn save(&self) -> Result<(), String> {
        let path = Self::path();
        let parent = path
            .parent()
            .ok_or_else(|| "Could not determine the settings directory".to_string())?;
        fs::create_dir_all(parent).map_err(|error| format!("Cannot create settings directory: {error}"))?;
        let contents = serde_json::to_string_pretty(self)
            .map_err(|error| format!("Cannot encode settings: {error}"))?;
        fs::write(path, contents).map_err(|error| format!("Cannot save settings: {error}"))
    }
}

fn run_gammastep(args: &[&str]) -> Result<(), String> {
    let output = Command::new("gammastep")
        .args(args)
        .output()
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                "gammastep is not installed. Install it with: sudo pacman -S gammastep".to_string()
            } else {
                format!("Could not start gammastep: {error}")
            }
        })?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if stderr.is_empty() {
            Err(format!("gammastep exited with status {}", output.status))
        } else {
            Err(stderr)
        }
    }
}

fn apply_temperature(temperature: u16) -> Result<(), String> {
    let temperature = temperature.to_string();
    run_gammastep(&["-P", "-O", temperature.as_str()])
}

fn reset_temperature() -> Result<(), String> {
    run_gammastep(&["-x"])
}

#[derive(Debug, Clone, Copy)]
enum TrayCommand {
    Open,
    Quit,
}

struct NightLightTray {
    sender: Sender<TrayCommand>,
}

impl ksni::Tray for NightLightTray {
    fn id(&self) -> String {
        "night-light-controller".to_string()
    }

    fn title(&self) -> String {
        "Night Light Controller".to_string()
    }

    fn icon_name(&self) -> String {
        "night-light-controller".to_string()
    }

    fn activate(&mut self, _x: i32, _y: i32) {
        let _ = self.sender.send(TrayCommand::Open);
    }

    // Keep the tray service alive if XFCE's Status Notifier watcher starts
    // after this application (common during desktop login).
    fn watcher_offline(&self, _reason: ksni::OfflineReason) -> bool {
        true
    }

    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        use ksni::menu::StandardItem;

        let open = StandardItem {
            label: "Open Night Light Controller".into(),
            icon_name: "preferences-system".into(),
            activate: Box::new(|tray: &mut Self| {
                let _ = tray.sender.send(TrayCommand::Open);
            }),
            ..Default::default()
        };

        let quit = StandardItem {
            label: "Quit".into(),
            icon_name: "application-exit".into(),
            activate: Box::new(|tray: &mut Self| {
                let _ = tray.sender.send(TrayCommand::Quit);
            }),
            ..Default::default()
        };

        vec![open.into(), ksni::MenuItem::Separator, quit.into()]
    }
}

fn start_tray(sender: Sender<TrayCommand>) {
    thread::Builder::new()
        .name("night-light-tray".into())
        .spawn(move || {
            let runtime = match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                Ok(runtime) => runtime,
                Err(error) => {
                    eprintln!("Could not start the tray runtime: {error}");
                    return;
                }
            };

            runtime.block_on(async move {
                let fallback_sender = sender.clone();
                let tray = NightLightTray { sender };
                match tray.assume_sni_available(true).spawn().await {
                    // Keep the handle in scope for the entire tray lifetime.
                    // With assume_sni_available(true), ksni waits for XFCE's
                    // Status Notifier watcher if it has not started yet.
                    Ok(handle) => {
                        let _tray_handle = handle;
                        std::future::pending::<()>().await;
                    }
                    Err(error) => {
                        eprintln!("Could not register the system tray icon: {error}");
                        let _ = fallback_sender.send(TrayCommand::Open);
                    }
                }
            });
        })
        .expect("Failed to create the tray thread");
}

fn install_css() {
    let provider = CssProvider::new();
    provider.load_from_data(
        "window { background: #111827; color: #f3f4f6; }\
         .muted { color: #9ca3af; }\
         .card { background: #1b2433; border: 1px solid #303b4e; border-radius: 16px; padding: 16px; }\
         .temp-value { color: #fbbf24; font-size: 22px; font-weight: 700; }\
         button.preset { background: #283449; color: #e5e7eb; border: 1px solid #39465c; border-radius: 10px; padding: 9px 7px; }\
         button.preset:hover { background: #36445c; }\
         scale trough { min-height: 7px; border-radius: 8px; background: #374151; }\
         scale highlight { background: #f59e0b; border-radius: 8px; }\
         scale slider { background: #fbbf24; min-width: 18px; min-height: 18px; border-radius: 99px; }\
         switch:checked { background: #d97706; }\
         .status { color: #a7f3d0; font-size: 11px; }",
    );

    if let Some(display) = gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}

fn build_window(app: &Application, receiver: Receiver<TrayCommand>, show_on_start: bool) {
    install_css();

    let settings = Rc::new(RefCell::new(Settings::load()));
    let initial_settings = settings.borrow().clone();
    let window = ApplicationWindow::builder()
        .application(app)
        .title("Night Light Controller")
        .default_width(390)
        .default_height(500)
        .resizable(false)
        .build();
    window.set_icon_name(Some("night-light-controller"));

    let root = gtk::Box::new(Orientation::Vertical, 18);
    root.set_margin_top(22);
    root.set_margin_bottom(20);
    root.set_margin_start(22);
    root.set_margin_end(22);
    window.set_child(Some(&root));

    let heading_row = gtk::Box::new(Orientation::Horizontal, 12);
    heading_row.set_valign(Align::Center);
    let app_icon = gtk::Image::from_icon_name("night-light-controller");
    app_icon.set_pixel_size(34);
    heading_row.append(&app_icon);

    let heading_box = gtk::Box::new(Orientation::Vertical, 3);
    let heading = Label::new(Some("Night Light"));
    heading.set_xalign(0.0);
    heading.add_css_class("title-1");
    let subtitle = Label::new(Some("Make your display warmer and easier on the eyes"));
    subtitle.set_xalign(0.0);
    subtitle.add_css_class("muted");
    subtitle.set_wrap(true);
    heading_box.append(&heading);
    heading_box.append(&subtitle);
    heading_row.append(&heading_box);
    root.append(&heading_row);

    let power_card = gtk::Box::new(Orientation::Horizontal, 12);
    power_card.add_css_class("card");
    let power_text = gtk::Box::new(Orientation::Vertical, 4);
    power_text.set_hexpand(true);
    let power_title = Label::new(Some("Night light"));
    power_title.set_xalign(0.0);
    power_title.add_css_class("heading");
    let power_description = Label::new(Some("Apply a warm colour tint to your screen"));
    power_description.set_xalign(0.0);
    power_description.add_css_class("muted");
    power_description.set_wrap(true);
    power_text.append(&power_title);
    power_text.append(&power_description);
    power_card.append(&power_text);

    let power_switch = Switch::new();
    power_switch.set_valign(Align::Center);
    power_switch.set_active(initial_settings.enabled);
    power_card.append(&power_switch);
    root.append(&power_card);

    let temperature_card = gtk::Box::new(Orientation::Vertical, 13);
    temperature_card.add_css_class("card");

    let temperature_header = gtk::Box::new(Orientation::Horizontal, 8);
    let temperature_title = Label::new(Some("Colour temperature"));
    temperature_title.set_xalign(0.0);
    temperature_title.set_hexpand(true);
    temperature_title.add_css_class("heading");
    let temperature_value = Label::new(Some(&format!("{} K", initial_settings.temperature)));
    temperature_value.add_css_class("temp-value");
    temperature_header.append(&temperature_title);
    temperature_header.append(&temperature_value);
    temperature_card.append(&temperature_header);

    let temperature_scale = Scale::with_range(
        Orientation::Horizontal,
        MIN_TEMPERATURE,
        MAX_TEMPERATURE,
        100.0,
    );
    temperature_scale.set_draw_value(false);
    temperature_scale.set_hexpand(true);
    temperature_scale.set_digits(0);
    temperature_scale.set_value(initial_settings.temperature as f64);
    temperature_card.append(&temperature_scale);

    let scale_legend = gtk::Box::new(Orientation::Horizontal, 0);
    let warmer = Label::new(Some("Warmer"));
    warmer.add_css_class("muted");
    warmer.set_xalign(0.0);
    warmer.set_hexpand(true);
    let cooler = Label::new(Some("Cooler"));
    cooler.add_css_class("muted");
    cooler.set_xalign(1.0);
    scale_legend.append(&warmer);
    scale_legend.append(&cooler);
    temperature_card.append(&scale_legend);

    let presets_title = Label::new(Some("Quick presets"));
    presets_title.set_xalign(0.0);
    presets_title.add_css_class("heading");
    temperature_card.append(&presets_title);

    let presets = Grid::new();
    presets.set_column_spacing(8);
    presets.set_row_spacing(8);
    presets.set_column_homogeneous(true);
    let preset_values: [(&str, u16); 4] = [
        ("Extra warm · 3000K", 3000),
        ("Warm · 4000K", 4000),
        ("Balanced · 4500K", 4500),
        ("Soft · 5500K", 5500),
    ];
    for (index, (label, value)) in preset_values.into_iter().enumerate() {
        let button = Button::with_label(label);
        button.add_css_class("preset");
        let scale = temperature_scale.clone();
        // `value` is copied out of the array, so the GTK callback owns it
        // and does not borrow `preset_values` beyond this loop iteration.
        button.connect_clicked(move |_| scale.set_value(value as f64));
        presets.attach(&button, (index % 2) as i32, (index / 2) as i32, 1, 1);
    }
    temperature_card.append(&presets);
    root.append(&temperature_card);

    let status = Label::new(Some("Night light is off"));
    status.set_xalign(0.0);
    status.set_wrap(true);
    status.add_css_class("status");
    root.append(&status);

    let note = Label::new(Some("Uses gammastep · Settings are saved automatically"));
    note.set_xalign(0.0);
    note.add_css_class("muted");
    note.set_wrap(true);
    root.append(&note);

    // Restore the saved effect if the user had enabled it before logging out.
    if initial_settings.enabled {
        match apply_temperature(initial_settings.temperature) {
            Ok(()) => status.set_text("Night light is on"),
            Err(error) => status.set_text(&format!("Could not restore night light: {error}")),
        }
    }

    {
        let settings = Rc::clone(&settings);
        let status = status.clone();
        power_switch.connect_active_notify(move |switch| {
            let mut current = settings.borrow_mut();
            current.enabled = switch.is_active();
            let result = if current.enabled {
                apply_temperature(current.temperature)
            } else {
                reset_temperature()
            };
            let save_result = current.save();
            match (result, save_result) {
                (Ok(()), Ok(())) => status.set_text(if current.enabled {
                    "Night light is on"
                } else {
                    "Night light is off"
                }),
                (Err(error), _) => status.set_text(&format!("Could not change night light: {error}")),
                (Ok(()), Err(error)) => status.set_text(&format!("Applied, but {error}")),
            }
        });
    }

    {
        let settings = Rc::clone(&settings);
        let value_label = temperature_value.clone();
        let status = status.clone();
        temperature_scale.connect_value_changed(move |scale| {
            let temperature = (scale.value().round() as u16).clamp(2500, 6500);
            value_label.set_text(&format!("{temperature} K"));
            let mut current = settings.borrow_mut();
            current.temperature = temperature;
            let apply_result = if current.enabled {
                apply_temperature(temperature)
            } else {
                Ok(())
            };
            let save_result = current.save();
            match (apply_result, save_result) {
                (Ok(()), Ok(())) => {
                    if current.enabled {
                        status.set_text(&format!("Night light is on · {temperature} K"));
                    } else {
                        status.set_text(&format!("Saved · {temperature} K will be used when enabled"));
                    }
                }
                (Err(error), _) => status.set_text(&format!("Could not apply temperature: {error}")),
                (Ok(()), Err(error)) => status.set_text(&format!("{error}")),
            }
        });
    }

    // Closing the window hides it; the tray icon remains available.
    window.connect_close_request(|window| {
        window.hide();
        glib::Propagation::Stop
    });

    let window_for_commands = window.clone();
    glib::timeout_add_local(Duration::from_millis(120), move || {
        loop {
            match receiver.try_recv() {
                Ok(TrayCommand::Open) => window_for_commands.present(),
                Ok(TrayCommand::Quit) => {
                    // The tray service runs on its own worker thread. Exit the
                    // process so both GTK and the tray worker are stopped.
                    std::process::exit(0);
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => return glib::ControlFlow::Break,
            }
        }
        glib::ControlFlow::Continue
    });

    if show_on_start {
        window.present();
    } else {
        window.hide();
    }
}

fn main() {
    let show_on_start = env::args().any(|argument| argument == "--window");
    let (sender, receiver) = mpsc::channel::<TrayCommand>();
    start_tray(sender);

    let application = Application::builder()
        .application_id(APP_ID)
        .build();
    let receiver = Rc::new(RefCell::new(Some(receiver)));
    application.connect_activate(move |app| {
        if let Some(receiver) = receiver.borrow_mut().take() {
            build_window(app, receiver, show_on_start);
        }
    });

    // We parse application-specific switches (such as --window) above.
    // Do not let GTK parse the original process arguments again: GTK treats
    // unknown application switches as errors. Pass only argv[0] to GTK.
    let program = env::args()
        .next()
        .unwrap_or_else(|| "night-light-controller".to_string());
    application.run_with_args(&[program]);
}
