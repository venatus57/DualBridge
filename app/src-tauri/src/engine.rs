//! Glue between the controllers, the virtual pads, the lighting engine and
//! the UI.
//!
//! One background thread ("engine") runs at ~30 Hz: it polls for hotplug
//! once a second, renders lighting, detects the foreground game, and emits a
//! `controllers` event with a snapshot for the UI. The per-controller input
//! threads never touch any of this: they only see their own [`PadSink`].

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use dualbridge_core::calibration::CalibratedMotion;
use dualbridge_core::lighting::LightingContext;
use dualbridge_core::mapping::{Mapper, XInputState};
use dualbridge_core::output::{OutputState, TriggerEffect};
use dualbridge_core::profile::{Profile, VirtualKind};
use dualbridge_core::state::{Battery, Buttons, Touch};
use dualbridge_core::{ControllerState, Model, Rgb, Transport};
use dualbridge_hid::latency::LatencyStats;
use dualbridge_hid::manager::{ControllerManager, ManagerEvent};
use dualbridge_hid::runtime::{ControllerShared, InputSink};
use dualbridge_hid::{DeviceInfo, HidBackend};
use dualbridge_virtual::{hidhide, BackendStatus, VirtualBackend, VirtualPad};
use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::settings::Settings;

const TICK: Duration = Duration::from_millis(33);
const POLL_EVERY: Duration = Duration::from_secs(1);

/// The mapping used by an input thread, swappable without blocking it.
#[derive(Default)]
pub struct MapperCell {
    version: AtomicU64,
    mapper: Mutex<Mapper>,
}

impl MapperCell {
    fn set(&self, m: Mapper) {
        *self.mapper.lock().unwrap() = m;
        self.version.fetch_add(1, Ordering::Release);
    }
}

/// Routes game rumble (from the virtual pad) to the physical controller.
#[derive(Default)]
struct RumbleLink {
    controller: OnceLock<Arc<ControllerShared>>,
    enabled: AtomicBool,
    /// Strength in thousandths.
    strength: AtomicU32,
}

impl RumbleLink {
    fn set_profile(&self, p: &Profile) {
        self.enabled.store(p.rumble, Ordering::Relaxed);
        let s = (p.rumble_strength.clamp(0.0, 1.0) * 1000.0) as u32;
        self.strength.store(s, Ordering::Relaxed);
    }

    fn rumble(&self, large: u8, small: u8) {
        let Some(c) = self.controller.get() else {
            return;
        };
        let (large, small) = if self.enabled.load(Ordering::Relaxed) {
            let k = self.strength.load(Ordering::Relaxed);
            (
                (large as u32 * k / 1000) as u8,
                (small as u32 * k / 1000) as u8,
            )
        } else {
            (0, 0)
        };
        c.update_output(|o| {
            o.rumble_strong = large;
            o.rumble_weak = small;
        });
    }
}

/// Runs on the input thread: maps the state and updates the virtual pad.
struct PadSink {
    cell: Arc<MapperCell>,
    version: u64,
    mapper: Mapper,
    pad: Option<Box<dyn VirtualPad>>,
    last: Option<XInputState>,
}

impl InputSink for PadSink {
    fn on_input(&mut self, state: &ControllerState) {
        let v = self.cell.version.load(Ordering::Acquire);
        if v != self.version {
            if let Ok(m) = self.cell.mapper.try_lock() {
                self.mapper = *m;
                self.version = v;
            }
        }
        if let Some(pad) = self.pad.as_mut() {
            let x = self.mapper.map(state);
            if self.last != Some(x) {
                let _ = pad.update(&x);
                self.last = Some(x);
            }
        }
    }
}

/// Per-controller state owned by the engine thread.
struct PadLinks {
    mapper: Arc<MapperCell>,
    rumble: Arc<RumbleLink>,
    virtual_error: Option<String>,
    has_virtual: bool,
    mic_muted: bool,
    mute_held: bool,
    identify_until: Option<Instant>,
}

struct EngineShared {
    settings: Mutex<Settings>,
    pads: Mutex<HashMap<String, PadLinks>>,
    virtual_backend: Mutex<Box<dyn VirtualBackend>>,
    virtual_status: Mutex<BackendStatus>,
    /// Profile name selected by the foreground game.
    game_profile: Mutex<Option<String>>,
    /// Last HidHide failure, shown in the settings.
    hidhide_error: Mutex<Option<String>>,
}

pub struct Engine {
    manager: Mutex<ControllerManager<Box<dyn HidBackend>>>,
    shared: Arc<EngineShared>,
    settings_path: PathBuf,
    started: Instant,
    pub demo: bool,
}

/// Live view of one controller, sent to the UI.
#[derive(Debug, Clone, Serialize)]
pub struct ControllerView {
    pub slot: u8,
    pub identity: String,
    pub name: String,
    pub model: Model,
    pub model_name: &'static str,
    pub transport: Transport,
    pub battery: Battery,
    pub lightbar: String,
    pub profile: String,
    pub input: InputView,
    pub latency: LatencyStats,
    pub has_virtual: bool,
    pub virtual_error: Option<String>,
    pub mic_muted: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct InputView {
    pub buttons: Vec<&'static str>,
    pub lx: u8,
    pub ly: u8,
    pub rx: u8,
    pub ry: u8,
    pub l2: u8,
    pub r2: u8,
    pub touch: [Touch; 2],
    pub motion: CalibratedMotion,
    pub full: bool,
}

impl InputView {
    fn new(s: &ControllerState, motion: CalibratedMotion) -> InputView {
        InputView {
            buttons: Buttons::ALL
                .iter()
                .filter(|(b, _)| s.buttons.contains(*b))
                .map(|(_, n)| *n)
                .collect(),
            lx: s.left_stick.x,
            ly: s.left_stick.y,
            rx: s.right_stick.x,
            ry: s.right_stick.y,
            l2: s.l2,
            r2: s.r2,
            touch: s.touch,
            motion,
            full: s.full,
        }
    }
}

fn output_for(profile: &Profile, model: Model) -> OutputState {
    let (left, right) = if model.is_dualsense() {
        (profile.left_trigger, profile.right_trigger)
    } else {
        (TriggerEffect::Off, TriggerEffect::Off)
    };
    OutputState {
        left_trigger: left,
        right_trigger: right,
        release_startup_light: true,
        ..OutputState::default()
    }
}

impl Engine {
    pub fn new(
        backend: Box<dyn HidBackend>,
        virtual_backend: Box<dyn VirtualBackend>,
        settings_path: PathBuf,
        demo: bool,
    ) -> Arc<Engine> {
        let settings = Settings::load(&settings_path);
        let shared = Arc::new(EngineShared {
            settings: Mutex::new(settings),
            pads: Mutex::new(HashMap::new()),
            virtual_backend: Mutex::new(virtual_backend),
            virtual_status: Mutex::new(BackendStatus::Unsupported),
            game_profile: Mutex::new(None),
            hidhide_error: Mutex::new(None),
        });
        let status = shared.virtual_backend.lock().unwrap().status();
        *shared.virtual_status.lock().unwrap() = status;

        let factory_shared = shared.clone();
        let factory = move |info: &DeviceInfo, _slot: u8| {
            let (sink, output) = factory_shared.create_controller(info);
            (sink, output)
        };
        Arc::new(Engine {
            manager: Mutex::new(ControllerManager::new(backend, factory)),
            shared,
            settings_path,
            started: Instant::now(),
            demo,
        })
    }

    /// Starts the engine thread.
    pub fn spawn(self: &Arc<Self>, app: AppHandle) {
        let engine = self.clone();
        std::thread::Builder::new()
            .name("dualbridge-engine".into())
            .spawn(move || engine.run(app))
            .expect("spawn engine thread");
    }

    fn run(&self, app: AppHandle) {
        let mut last_poll: Option<Instant> = None;
        loop {
            let now = Instant::now();
            if last_poll.is_none_or(|t| now - t >= POLL_EVERY) {
                last_poll = Some(now);
                self.poll_devices(&app);
                self.check_foreground();
            }
            self.render_lighting(now);
            let _ = app.emit("controllers", self.controllers());
            std::thread::sleep(TICK.saturating_sub(now.elapsed()));
        }
    }

    fn poll_devices(&self, app: &AppHandle) {
        let events = match self.manager.lock().unwrap().poll() {
            Ok(e) => e,
            Err(_) => return,
        };
        if events.is_empty() {
            return;
        }
        let exclusive = self.shared.settings.lock().unwrap().exclusive_mode;
        let manager = self.manager.lock().unwrap();
        for event in &events {
            match event {
                ManagerEvent::Connected { slot, info } => {
                    if let (Some(c), Some(links)) = (
                        manager.get(*slot),
                        self.shared.pads.lock().unwrap().get(info.identity()),
                    ) {
                        let _ = links.rumble.controller.set(c.clone());
                    }
                    if exclusive {
                        self.set_hidden(vec![info.path.clone()], true);
                    }
                }
                ManagerEvent::Disconnected { info, .. } => {
                    self.shared.pads.lock().unwrap().remove(info.identity());
                }
                _ => {}
            }
        }
        drop(manager);
        let _ = app.emit("device-events", &events);
    }

    /// Hides the controllers from games with HidHide (or shows them again),
    /// so games don't see both the PlayStation controller and the virtual
    /// Xbox one. Runs in the background because the CLI takes a moment.
    fn set_hidden(&self, paths: Vec<String>, hidden: bool) {
        if self.demo || paths.is_empty() || !hidhide::is_installed() {
            return;
        }
        let shared = self.shared.clone();
        std::thread::spawn(move || {
            let result = if hidden {
                hidhide::hide_devices(&paths)
            } else {
                hidhide::unhide_devices(&paths)
            };
            *shared.hidhide_error.lock().unwrap() = result.err();
        });
    }

    pub fn hidhide_error(&self) -> Option<String> {
        self.shared.hidhide_error.lock().unwrap().clone()
    }

    fn check_foreground(&self) {
        let auto = self.shared.settings.lock().unwrap().auto_profile_switch;
        let exe = if auto {
            crate::foreground::foreground_executable()
        } else {
            None
        };
        let matched = exe.and_then(|exe| {
            let settings = self.shared.settings.lock().unwrap();
            dualbridge_core::profile::profile_for_game(&settings.profiles, &exe)
                .map(|p| p.name.clone())
        });
        let changed = {
            let mut game = self.shared.game_profile.lock().unwrap();
            let changed = *game != matched;
            *game = matched;
            changed
        };
        if changed {
            self.apply_profiles();
        }
    }

    fn render_lighting(&self, now: Instant) {
        let time_ms = (now - self.started).as_millis() as u64;
        let manager = self.manager.lock().unwrap();
        let settings = self.shared.settings.lock().unwrap();
        let game = self.shared.game_profile.lock().unwrap().clone();
        let mut pads = self.shared.pads.lock().unwrap();
        for (slot, c) in manager.controllers() {
            let snapshot = c.snapshot();
            let Some(links) = pads.get_mut(c.info.identity()) else {
                continue;
            };
            // The mute button toggles the mic LED state.
            let mute = snapshot.buttons.contains(Buttons::MUTE);
            if mute && !links.mute_held {
                links.mic_muted = !links.mic_muted;
            }
            links.mute_held = mute;

            let profile = settings.profile_for(c.info.identity(), game.as_deref());
            // Until the first full report the battery level is unknown:
            // don't show a low-battery warning for it.
            let battery = if snapshot.full {
                snapshot.battery
            } else {
                Battery {
                    percent: 100,
                    ..snapshot.battery
                }
            };
            let ctx = LightingContext {
                time_ms,
                slot,
                battery,
                mic_muted: links.mic_muted,
            };
            let mut frame = profile.lighting.render(&ctx);
            let identifying = links.identify_until.is_some_and(|t| now < t);
            if identifying {
                frame.lightbar = if (time_ms / 120).is_multiple_of(2) {
                    Rgb::WHITE
                } else {
                    Rgb::BLACK
                };
            } else if links.identify_until.take().is_some() {
                c.update_output(|o| {
                    o.rumble_strong = 0;
                    o.rumble_weak = 0;
                });
            }
            let dualsense = c.info.model.is_dualsense();
            c.update_output(|o| {
                o.lightbar = frame.lightbar;
                o.player_leds = frame.player_leds;
                o.player_led_brightness = frame.player_led_brightness;
                o.mic_led = frame.mic_led;
                if dualsense {
                    o.left_trigger = profile.left_trigger;
                    o.right_trigger = profile.right_trigger;
                }
            });
        }
    }

    /// Pushes the current profiles to every connected controller.
    pub fn apply_profiles(&self) {
        let settings = self.shared.settings.lock().unwrap();
        let game = self.shared.game_profile.lock().unwrap().clone();
        let pads = self.shared.pads.lock().unwrap();
        for (identity, links) in pads.iter() {
            let p = settings.profile_for(identity, game.as_deref());
            links.mapper.set(p.mapping.compile());
            links.rumble.set_profile(p);
        }
    }

    pub fn controllers(&self) -> Vec<ControllerView> {
        let manager = self.manager.lock().unwrap();
        let settings = self.shared.settings.lock().unwrap();
        let game = self.shared.game_profile.lock().unwrap().clone();
        let pads = self.shared.pads.lock().unwrap();
        manager
            .controllers()
            .map(|(slot, c)| {
                let s = c.snapshot();
                let id = c.info.identity().to_string();
                let links = pads.get(&id);
                ControllerView {
                    slot,
                    name: settings
                        .controller_names
                        .get(&id)
                        .cloned()
                        .unwrap_or_else(|| c.info.model.display_name().to_string()),
                    model: c.info.model,
                    model_name: c.info.model.display_name(),
                    transport: c.info.transport,
                    battery: s.battery,
                    lightbar: c.output().lightbar.to_hex(),
                    profile: settings.profile_for(&id, game.as_deref()).name.clone(),
                    input: InputView::new(&s, c.calibration.apply(&s.motion)),
                    latency: c.latency.stats(),
                    has_virtual: links.is_some_and(|l| l.has_virtual),
                    virtual_error: links.and_then(|l| l.virtual_error.clone()),
                    mic_muted: links.is_some_and(|l| l.mic_muted),
                    identity: id,
                }
            })
            .collect()
    }

    pub fn settings(&self) -> Settings {
        self.shared.settings.lock().unwrap().clone()
    }

    /// Edits the settings, saves them, and applies the result.
    pub fn update_settings<T>(
        &self,
        f: impl FnOnce(&mut Settings) -> Result<T, String>,
    ) -> Result<T, String> {
        let (result, exclusive_changed, exclusive) = {
            let mut s = self.shared.settings.lock().unwrap();
            let before = s.exclusive_mode;
            let r = f(&mut s)?;
            s.normalize();
            s.save(&self.settings_path)
                .map_err(|e| format!("could not save settings: {e}"))?;
            (r, before != s.exclusive_mode, s.exclusive_mode)
        };
        self.apply_profiles();
        if exclusive_changed {
            let paths: Vec<String> = self
                .manager
                .lock()
                .unwrap()
                .controllers()
                .map(|(_, c)| c.info.path.clone())
                .collect();
            self.set_hidden(paths, exclusive);
        }
        Ok(result)
    }

    pub fn active_game_profile(&self) -> Option<String> {
        self.shared.game_profile.lock().unwrap().clone()
    }

    pub fn virtual_status(&self) -> BackendStatus {
        let status = self.shared.virtual_backend.lock().unwrap().status();
        *self.shared.virtual_status.lock().unwrap() = status;
        status
    }

    pub fn swap_slots(&self, a: u8, b: u8) -> bool {
        self.manager.lock().unwrap().swap_slots(a, b)
    }

    /// Blinks the lightbar and rumbles briefly so the user can tell which
    /// physical controller is in a slot.
    pub fn identify(&self, slot: u8) -> bool {
        let manager = self.manager.lock().unwrap();
        let Some(c) = manager.get(slot) else {
            return false;
        };
        c.update_output(|o| {
            o.rumble_strong = 120;
            o.rumble_weak = 120;
        });
        if let Some(links) = self.shared.pads.lock().unwrap().get_mut(c.info.identity()) {
            links.identify_until = Some(Instant::now() + Duration::from_millis(1200));
        }
        true
    }

    pub fn reset_latency(&self, slot: u8) {
        if let Some(c) = self.manager.lock().unwrap().get(slot) {
            c.latency.reset_max();
        }
    }

    /// Stops every controller (unplugging the virtual pads). In exclusive
    /// mode the physical controllers are made visible again, so they keep
    /// working in games while DualBridge is not running.
    pub fn shutdown(&self) {
        let mut manager = self.manager.lock().unwrap();
        let paths: Vec<String> = manager
            .controllers()
            .map(|(_, c)| c.info.path.clone())
            .collect();
        manager.shutdown();
        drop(manager);
        let exclusive = self.shared.settings.lock().unwrap().exclusive_mode;
        if exclusive && !self.demo && !paths.is_empty() && hidhide::is_installed() {
            let _ = hidhide::unhide_devices(&paths);
        }
    }
}

impl EngineShared {
    /// Called by the manager (engine thread) when a controller connects.
    fn create_controller(&self, info: &DeviceInfo) -> (Box<dyn InputSink>, OutputState) {
        let (profile, output) = {
            let settings = self.settings.lock().unwrap();
            let game = self.game_profile.lock().unwrap().clone();
            let p = settings
                .profile_for(info.identity(), game.as_deref())
                .clone();
            let out = output_for(&p, info.model);
            (p, out)
        };
        let cell = Arc::new(MapperCell::default());
        cell.set(profile.mapping.compile());
        let rumble = Arc::new(RumbleLink::default());
        rumble.set_profile(&profile);

        let (pad, virtual_error) = match profile.virtual_kind {
            VirtualKind::None => (None, None),
            VirtualKind::Xbox360 => {
                let r = rumble.clone();
                let result = self
                    .virtual_backend
                    .lock()
                    .unwrap()
                    .create_xbox360(Box::new(move |l, s| r.rumble(l, s)));
                match result {
                    Ok(pad) => (Some(pad), None),
                    Err(e) => (None, Some(e.to_string())),
                }
            }
        };
        self.pads.lock().unwrap().insert(
            info.identity().to_string(),
            PadLinks {
                mapper: cell.clone(),
                rumble,
                virtual_error,
                has_virtual: pad.is_some(),
                mic_muted: false,
                mute_held: false,
                identify_until: None,
            },
        );
        let sink = PadSink {
            version: cell.version.load(Ordering::Acquire),
            mapper: profile.mapping.compile(),
            cell,
            pad,
            last: None,
        };
        (Box::new(sink), output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dualbridge_hid::mock::MockBackend;
    use dualbridge_virtual::mock::MockVirtualBackend;

    fn engine() -> (Arc<Engine>, MockBackend, MockVirtualBackend, PathBuf) {
        let dir = std::env::temp_dir().join(format!(
            "dualbridge-engine-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let hid = MockBackend::default();
        let virt = MockVirtualBackend::default();
        let e = Engine::new(
            Box::new(hid.clone()),
            Box::new(virt.clone()),
            dir.join("settings.json"),
            false,
        );
        (e, hid, virt, dir)
    }

    fn wait_until(mut f: impl FnMut() -> bool) {
        let start = Instant::now();
        while !f() {
            assert!(start.elapsed() < Duration::from_secs(5), "timed out");
            std::thread::sleep(Duration::from_millis(2));
        }
    }

    #[test]
    fn controller_drives_virtual_pad_and_rumble() {
        let (e, hid, virt, dir) = engine();
        let pad = hid.add(Model::DualSense, Transport::Usb, "p1");
        // Poll without a Tauri app: call the pieces directly.
        let events = e.manager.lock().unwrap().poll().unwrap();
        assert_eq!(events.len(), 1);
        {
            let manager = e.manager.lock().unwrap();
            let c = manager.get(1).unwrap().clone();
            let pads = e.shared.pads.lock().unwrap();
            let _ = pads["p1"].rumble.controller.set(c);
        }

        let mut r = [0u8; 64];
        r[0] = 0x01;
        r[1 + 7] = 0x20 | 0x08; // cross, D-pad neutral
        r[1 + 52] = 8; // battery ~85 %
        pad.push_report(&r);
        let vpads = virt.pads();
        assert_eq!(vpads.len(), 1);
        wait_until(|| vpads[0].last().is_some());
        assert_eq!(vpads[0].last().unwrap().buttons, 0x1000);

        // Game rumble reaches the controller's output reports.
        vpads[0].game_rumble(200, 100);
        let written = pad.wait_written(2, Duration::from_secs(5));
        let last = written.last().unwrap();
        assert_eq!((last[1 + 3], last[1 + 2]), (200, 100));

        // Lighting: the default profile shows slot 1's color.
        e.render_lighting(Instant::now());
        let views = e.controllers();
        assert_eq!(views[0].lightbar, "#0040FF");
        assert_eq!(views[0].profile, "Default");
        assert!(views[0].has_virtual);

        // Remapping takes effect without reconnecting.
        e.update_settings(|s| {
            s.profiles[0]
                .mapping
                .buttons
                .push(("cross".into(), dualbridge_core::mapping::ButtonTarget::B));
            Ok(())
        })
        .unwrap();
        r[1 + 7] = 0x08;
        pad.push_report(&r);
        r[1 + 7] = 0x20 | 0x08;
        pad.push_report(&r);
        wait_until(|| vpads[0].last().unwrap().buttons == 0x2000);
        e.shutdown();
        assert!(vpads[0].is_unplugged());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn missing_driver_is_reported() {
        let (e, hid, virt, dir) = engine();
        virt.set_status(BackendStatus::DriverMissing);
        hid.add(Model::DualShock4, Transport::Bluetooth, "p1");
        e.manager.lock().unwrap().poll().unwrap();
        let views = e.controllers();
        assert!(!views[0].has_virtual);
        assert!(views[0].virtual_error.is_some());
        assert_eq!(e.virtual_status(), BackendStatus::DriverMissing);
        e.shutdown();
        let _ = std::fs::remove_dir_all(dir);
    }
}
