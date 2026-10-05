//! DualBridge desktop app (Tauri 2).

mod commands;
mod demo;
mod elevate;
mod engine;
mod foreground;
mod settings;
mod update;

use std::sync::Arc;

use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, WindowEvent};

use crate::engine::Engine;

pub use crate::elevate::helper_main;

fn show_main_window(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
    }
}

fn setup_tray(app: &tauri::App) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "Show DualBridge", true, None::<&str>)?;
    let off = MenuItem::with_id(
        app,
        "power_off",
        "Switch off controllers",
        true,
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &off, &quit])?;
    let mut tray = TrayIconBuilder::with_id("main")
        .tooltip("DualBridge")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => show_main_window(app),
            "power_off" => {
                if let Some(engine) = app.try_state::<Arc<Engine>>() {
                    engine.power_off_all();
                }
            }
            "quit" => {
                if let Some(engine) = app.try_state::<Arc<Engine>>() {
                    engine.shutdown();
                }
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            show_main_window(app);
        }))
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--minimized"]),
        ))
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let demo = demo::enabled();
            let config_dir = app.path().app_config_dir()?;
            let hid: Box<dyn dualbridge_hid::HidBackend> = if demo {
                Box::new(demo::backend())
            } else {
                match dualbridge_hid::hidapi_backend::HidapiBackend::new() {
                    Ok(b) => Box::new(b),
                    // No HID access at all: run with no controllers rather than crash.
                    Err(_) => Box::new(dualbridge_hid::mock::MockBackend::default()),
                }
            };
            let virt: Box<dyn dualbridge_virtual::VirtualBackend> = if demo {
                Box::new(dualbridge_virtual::mock::MockVirtualBackend::default())
            } else {
                dualbridge_virtual::default_backend()
            };
            let engine = Engine::new(hid, virt, config_dir.join("settings.json"), demo);
            engine.spawn(app.handle().clone());

            let settings = engine.settings();
            // Start with the computer by default (once: after that the user's
            // choice in Settings is kept).
            if !settings.autostart_configured && !demo {
                use tauri_plugin_autostart::ManagerExt;
                if app.autolaunch().enable().is_ok() {
                    let _ = engine.update_settings(|s| {
                        s.autostart_configured = true;
                        Ok(())
                    });
                }
            }
            let minimized = std::env::args().any(|a| a == "--minimized")
                || (settings.start_minimized && settings.first_run_done);
            app.manage(engine);
            setup_tray(app)?;
            if !minimized {
                show_main_window(app.handle());
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let app = window.app_handle();
                let to_tray = app
                    .try_state::<Arc<Engine>>()
                    .is_some_and(|e| e.settings().minimize_to_tray);
                if to_tray {
                    api.prevent_close();
                    let _ = window.hide();
                } else if let Some(engine) = app.try_state::<Arc<Engine>>() {
                    engine.shutdown();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_overview,
            commands::save_profile,
            commands::delete_profile,
            commands::assign_profile,
            commands::rename_controller,
            commands::set_controller_lighting,
            commands::set_preferences,
            commands::hide_controllers_now,
            commands::swap_slots,
            commands::identify_controller,
            commands::power_off_controller,
            commands::power_off_all,
            commands::check_update,
            commands::install_update,
            commands::reset_latency,
            commands::preview_lighting,
            commands::driver_status,
            commands::install_driver,
        ])
        .run(tauri::generate_context!())
        .expect("error while running DualBridge");
}
