//! Commands the UI can invoke.

use std::sync::Arc;

use dualbridge_core::lighting::{LightingConfig, LightingContext};
use dualbridge_core::profile::Profile;
use dualbridge_core::state::{Battery, Charging};
use dualbridge_virtual::{hidhide, BackendStatus};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};

use crate::engine::{ControllerView, Engine};
use crate::settings::Settings;

type EngineState<'a> = State<'a, Arc<Engine>>;

#[derive(Serialize)]
pub struct Overview {
    pub settings: Settings,
    pub controllers: Vec<ControllerView>,
    pub virtual_status: BackendStatus,
    pub hidhide_installed: bool,
    pub hidhide_error: Option<String>,
    pub platform: &'static str,
    pub version: &'static str,
    pub demo: bool,
    pub active_game_profile: Option<String>,
}

fn platform() -> &'static str {
    if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else {
        "linux"
    }
}

#[tauri::command]
pub fn get_overview(engine: EngineState) -> Overview {
    Overview {
        settings: engine.settings(),
        controllers: engine.controllers(),
        virtual_status: engine.virtual_status(),
        hidhide_installed: hidhide::is_installed(),
        hidhide_error: engine.hidhide_error(),
        platform: platform(),
        version: env!("CARGO_PKG_VERSION"),
        demo: engine.demo,
        active_game_profile: engine.active_game_profile(),
    }
}

#[tauri::command]
pub fn save_profile(
    engine: EngineState,
    profile: Profile,
    previous_name: Option<String>,
) -> Result<Settings, String> {
    engine.update_settings(|s| s.upsert_profile(profile, previous_name.as_deref()))?;
    Ok(engine.settings())
}

#[tauri::command]
pub fn delete_profile(engine: EngineState, name: String) -> Result<Settings, String> {
    engine.update_settings(|s| s.delete_profile(&name))?;
    Ok(engine.settings())
}

#[tauri::command]
pub fn assign_profile(
    engine: EngineState,
    identity: String,
    profile: String,
) -> Result<Settings, String> {
    engine.update_settings(|s| {
        if s.profile(&profile).is_none() {
            return Err(format!("unknown profile \"{profile}\""));
        }
        s.assignments.insert(identity, profile);
        Ok(())
    })?;
    Ok(engine.settings())
}

#[tauri::command]
pub fn rename_controller(
    engine: EngineState,
    identity: String,
    name: String,
) -> Result<Settings, String> {
    engine.update_settings(|s| {
        let name = name.trim();
        if name.is_empty() {
            s.controller_names.remove(&identity);
        } else {
            s.controller_names.insert(identity, name.to_string());
        }
        Ok(())
    })?;
    Ok(engine.settings())
}

/// Plain app preferences. Omitted fields are left unchanged.
#[derive(Debug, Default, Deserialize)]
pub struct PreferencesPatch {
    pub language: Option<Option<String>>,
    pub first_run_done: Option<bool>,
    pub minimize_to_tray: Option<bool>,
    pub start_minimized: Option<bool>,
    pub exclusive_mode: Option<bool>,
    pub auto_profile_switch: Option<bool>,
}

#[tauri::command]
pub fn set_preferences(engine: EngineState, patch: PreferencesPatch) -> Result<Settings, String> {
    engine.update_settings(|s| {
        if let Some(v) = patch.language {
            s.language = v;
        }
        if let Some(v) = patch.first_run_done {
            s.first_run_done = v;
        }
        if let Some(v) = patch.minimize_to_tray {
            s.minimize_to_tray = v;
        }
        if let Some(v) = patch.start_minimized {
            s.start_minimized = v;
        }
        if let Some(v) = patch.exclusive_mode {
            s.exclusive_mode = v;
        }
        if let Some(v) = patch.auto_profile_switch {
            s.auto_profile_switch = v;
        }
        Ok(())
    })?;
    Ok(engine.settings())
}

#[tauri::command]
pub fn swap_slots(engine: EngineState, a: u8, b: u8) -> bool {
    engine.swap_slots(a, b)
}

#[tauri::command]
pub fn identify_controller(engine: EngineState, slot: u8) -> bool {
    engine.identify(slot)
}

#[tauri::command]
pub fn reset_latency(engine: EngineState, slot: u8) {
    engine.reset_latency(slot)
}

/// Renders a lighting configuration over time, for the live preview.
/// Returns one `#RRGGBB` color per frame.
#[tauri::command]
pub fn preview_lighting(
    lighting: LightingConfig,
    slot: u8,
    battery_percent: u8,
    duration_ms: u32,
    fps: u32,
) -> Vec<String> {
    let fps = fps.clamp(1, 60);
    let frames = (duration_ms.min(20_000) as u64 * fps as u64 / 1000).max(1);
    (0..frames)
        .map(|i| {
            let ctx = LightingContext {
                time_ms: i * 1000 / fps as u64,
                slot: slot.max(1),
                battery: Battery {
                    percent: battery_percent.min(100),
                    charging: Charging::Discharging,
                    cable: false,
                },
                mic_muted: false,
            };
            lighting.render(&ctx).lightbar.to_hex()
        })
        .collect()
}

#[derive(Serialize)]
pub struct DriverStatus {
    pub virtual_status: BackendStatus,
    pub hidhide_installed: bool,
}

#[tauri::command]
pub fn driver_status(engine: EngineState) -> DriverStatus {
    DriverStatus {
        virtual_status: engine.virtual_status(),
        hidhide_installed: hidhide::is_installed(),
    }
}

/// Starts a bundled driver installer (`vigembus` or `hidhide`) if present,
/// otherwise opens its download page.
#[tauri::command]
pub fn install_driver(app: AppHandle, driver: String) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    let (file, url) = match driver.as_str() {
        "vigembus" => (
            "ViGEmBus_Setup.exe",
            "https://github.com/nefarius/ViGEmBus/releases/latest",
        ),
        "hidhide" => (
            "HidHide_Setup.exe",
            "https://github.com/nefarius/HidHide/releases/latest",
        ),
        _ => return Err(format!("unknown driver \"{driver}\"")),
    };
    if let Ok(dir) = app.path().resource_dir() {
        let path = dir.join("drivers").join(file);
        if path.exists() {
            // Opening through the shell lets Windows show the UAC prompt.
            return app
                .opener()
                .open_path(path.to_string_lossy(), None::<&str>)
                .map_err(|e| e.to_string());
        }
    }
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use dualbridge_core::lighting::Effect;
    use dualbridge_core::Rgb;

    #[test]
    fn preview_frames() {
        let cfg = LightingConfig {
            effect: Effect::Strobe {
                color: Rgb::WHITE,
                on_ms: 500,
                off_ms: 500,
            },
            ..LightingConfig::default()
        };
        let frames = preview_lighting(cfg, 1, 80, 2000, 2);
        assert_eq!(frames, vec!["#FFFFFF", "#000000", "#FFFFFF", "#000000"]);
    }
}
