//! The I/O threads of one controller.
//!
//! **Input thread** (latency critical): blocking read, parse, hand the state to
//! the [`InputSink`] (which updates the virtual controller), record latency,
//! repeat. It never allocates, logs, or waits on a lock: the UI snapshot is
//! published with `try_lock`, so a busy UI can only make it skip one snapshot.
//!
//! **Writer thread**: waits for [`ControllerShared::update_output`] to change
//! the [`OutputState`] (lighting, rumble...) and writes the output report on
//! its own device handle, so output never delays input. If the platform
//! refuses a second handle, the input thread writes pending output between
//! reads instead.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use dualbridge_core::calibration::{self, Calibration};
use dualbridge_core::input::InputParser;
use dualbridge_core::output::{OutputBuilder, OutputState};
use dualbridge_core::switch::SwitchInit;
use dualbridge_core::{ControllerState, Transport};

use crate::latency::LatencyMeter;
use crate::{DeviceInfo, HidBackend, HidDevice, HidError, HidResult};

/// How long a read waits before checking whether the thread should stop.
const READ_TIMEOUT_MS: i32 = 100;
/// Largest input report we expect (padding included).
const READ_BUF_LEN: usize = 128;
/// Controllers stream reports continuously (about 250 per second), so this
/// long without one means something is wrong. A Bluetooth controller that is
/// switched off often keeps its handle "open" on Windows: reads just time out
/// instead of failing. After this silence we check whether it still answers.
const SILENCE_TIMEOUT: Duration = Duration::from_secs(2);
/// How long a liveness check waits for an input report.
const PROBE_READ_MS: i32 = 1000;
/// How long a Switch setup step waits for its reply, and how many times it is
/// sent before moving on.
const SWITCH_STEP_MS: u64 = 300;
const SWITCH_STEP_TRIES: u32 = 3;
/// The Switch Pro Controller stops vibrating on its own unless the rumble is
/// sent again regularly.
const SWITCH_RUMBLE_REFRESH: Duration = Duration::from_millis(30);
/// Writer wake-up interval when there is nothing to refresh.
const WRITER_IDLE: Duration = Duration::from_millis(500);

/// Receives every parsed input report, on the input thread.
///
/// Implementations must be fast and must not block, allocate or log: they run
/// between a report arriving and the next read.
pub trait InputSink: Send + 'static {
    fn on_input(&mut self, state: &ControllerState);
    /// Called once when the controller disconnects or is stopped.
    fn on_disconnect(&mut self) {}
}

impl<F: FnMut(&ControllerState) + Send + 'static> InputSink for F {
    fn on_input(&mut self, state: &ControllerState) {
        self(state)
    }
}

#[derive(Default)]
struct OutputSlot {
    state: OutputState,
    dirty: bool,
}

/// State shared between a controller's threads and the rest of the app.
pub struct ControllerShared {
    pub info: DeviceInfo,
    pub calibration: Calibration,
    pub latency: LatencyMeter,
    snapshot: Mutex<ControllerState>,
    output: Mutex<OutputSlot>,
    output_changed: Condvar,
    stop: AtomicBool,
    connected: AtomicBool,
}

impl ControllerShared {
    /// Latest input state (published by the input thread when it could take
    /// the lock without waiting).
    pub fn snapshot(&self) -> ControllerState {
        *self.snapshot.lock().unwrap()
    }

    /// Current output state (what was last requested, not necessarily sent).
    pub fn output(&self) -> OutputState {
        self.output.lock().unwrap().state
    }

    /// Changes the output state; the report is sent only if something changed.
    pub fn update_output(&self, f: impl FnOnce(&mut OutputState)) {
        let mut slot = self.output.lock().unwrap();
        let before = slot.state;
        f(&mut slot.state);
        if slot.state != before {
            slot.dirty = true;
            self.output_changed.notify_one();
        }
    }

    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::Acquire)
    }

    /// Takes the pending output state, if any, clearing the one-shot flags.
    fn take_output(slot: &mut OutputSlot) -> Option<OutputState> {
        if !slot.dirty {
            return None;
        }
        slot.dirty = false;
        let out = slot.state;
        slot.state.release_startup_light = false;
        slot.state.power_off = false;
        Some(out)
    }
}

/// A running controller. Dropping it stops its threads.
pub struct ControllerHandle {
    shared: Arc<ControllerShared>,
    threads: Vec<JoinHandle<()>>,
}

impl ControllerHandle {
    /// Opens the device, reads its calibration (which also switches Bluetooth
    /// controllers to full reports), and starts its threads.
    pub fn start(
        backend: &mut dyn HidBackend,
        info: DeviceInfo,
        sink: Box<dyn InputSink>,
        initial_output: OutputState,
    ) -> HidResult<ControllerHandle> {
        let mut reader = backend.open(&info)?;
        let calibration = if info.model.is_switch() {
            switch_setup(reader.as_mut(), &info)?
        } else {
            read_calibration(reader.as_mut(), &info)
        };
        // A Bluetooth controller that was switched off can still be listed
        // (and opened) for a while: only take it if it answers.
        if info.transport == Transport::Bluetooth
            && calibration.is_none()
            && !answers_input(reader.as_mut())
        {
            return Err(HidError::Disconnected);
        }
        let calibration = calibration.unwrap_or_default();
        let writer = backend.open(&info).ok();

        let shared = Arc::new(ControllerShared {
            info,
            calibration,
            latency: LatencyMeter::default(),
            snapshot: Mutex::new(ControllerState::default()),
            output: Mutex::new(OutputSlot {
                state: initial_output,
                dirty: true,
            }),
            output_changed: Condvar::new(),
            stop: AtomicBool::new(false),
            connected: AtomicBool::new(true),
        });

        let mut threads = Vec::with_capacity(2);
        let inline_writes = writer.is_none();
        if let Some(writer) = writer {
            let s = shared.clone();
            threads.push(
                std::thread::Builder::new()
                    .name("dualbridge-output".into())
                    .spawn(move || writer_loop(s, writer))
                    .map_err(|e| HidError::Other(e.to_string()))?,
            );
        }
        let s = shared.clone();
        let input = std::thread::Builder::new()
            .name("dualbridge-input".into())
            .spawn(move || input_loop(s, reader, sink, inline_writes));
        match input {
            Ok(t) => threads.push(t),
            Err(e) => {
                // Don't leave the writer thread running on its own.
                shared.stop.store(true, Ordering::Release);
                shared.output_changed.notify_all();
                for t in threads {
                    let _ = t.join();
                }
                return Err(HidError::Other(e.to_string()));
            }
        }
        Ok(ControllerHandle { shared, threads })
    }

    pub fn shared(&self) -> &Arc<ControllerShared> {
        &self.shared
    }

    /// Stops the threads and waits for them to finish.
    pub fn stop(mut self) {
        self.stop_and_join();
    }

    fn stop_and_join(&mut self) {
        self.shared.stop.store(true, Ordering::Release);
        {
            // Take the lock so the writer can't miss the wake-up.
            let _slot = self.shared.output.lock().unwrap();
            self.shared.output_changed.notify_all();
        }
        for t in self.threads.drain(..) {
            let _ = t.join();
        }
    }
}

impl Drop for ControllerHandle {
    fn drop(&mut self) {
        self.stop_and_join();
    }
}

/// Reads the calibration feature report. `None` if the controller didn't
/// answer at all; `Some(default)` if it answered with unusable data.
fn read_calibration(dev: &mut dyn HidDevice, info: &DeviceInfo) -> Option<Calibration> {
    let mut buf = [0u8; 64];
    let len = calibration::feature_report_len(info.model, info.transport);
    buf[0] = calibration::feature_report_id(info.model, info.transport);
    let n = dev.get_feature_report(&mut buf[..len]).ok()?;
    Some(
        Calibration::from_feature_report(info.model, info.transport, &buf[..n]).unwrap_or_default(),
    )
}

/// Configures a Switch Pro Controller (see [`SwitchInit`]) and returns its
/// calibration, or `None` if it never answered.
fn switch_setup(dev: &mut dyn HidDevice, info: &DeviceInfo) -> HidResult<Option<Calibration>> {
    let mut init = SwitchInit::new(info.transport);
    let mut out = [0u8; 64];
    let mut buf = [0u8; READ_BUF_LEN];
    let mut tries = 0;
    while let Some(n) = init.request(&mut out) {
        if let Err(HidError::Disconnected) = dev.write(&out[..n]) {
            return Err(HidError::Disconnected);
        }
        if !init.expects_reply() {
            init.skip();
            continue;
        }
        let deadline = Instant::now() + Duration::from_millis(SWITCH_STEP_MS);
        let mut done = false;
        while let Some(left) = deadline.checked_duration_since(Instant::now()) {
            match dev.read_timeout(&mut buf, left.as_millis().max(1) as i32) {
                Ok(0) => {}
                Ok(n) => {
                    if init.on_report(&buf[..n]) {
                        done = true;
                        break;
                    }
                }
                Err(_) => return Err(HidError::Disconnected),
            }
        }
        if done {
            tries = 0;
        } else {
            tries += 1;
            if tries >= SWITCH_STEP_TRIES {
                init.skip();
                tries = 0;
            }
        }
    }
    Ok(init.answered().then(|| Calibration::switch(init.sticks)))
}

/// `true` if the controller sends an input report within [`PROBE_READ_MS`].
fn answers_input(dev: &mut dyn HidDevice) -> bool {
    let mut buf = [0u8; READ_BUF_LEN];
    matches!(dev.read_timeout(&mut buf, PROBE_READ_MS), Ok(n) if n > 0)
}

/// Checks a silent controller: it is alive if it answers a feature report
/// request (which also switches Bluetooth controllers back to full reports)
/// or sends an input report.
fn still_alive(dev: &mut dyn HidDevice, info: &DeviceInfo) -> bool {
    // The Switch Pro Controller has no feature reports.
    (!info.model.is_switch() && read_calibration(dev, info).is_some()) || answers_input(dev)
}

fn input_loop(
    shared: Arc<ControllerShared>,
    mut dev: Box<dyn HidDevice>,
    mut sink: Box<dyn InputSink>,
    inline_writes: bool,
) {
    let _priority = crate::priority::boost_current_thread();
    let mut parser = InputParser::new(shared.info.model, shared.info.transport);
    parser.sticks = shared.calibration.sticks;
    let mut builder = OutputBuilder::new(shared.info.model, shared.info.transport);
    let mut state = ControllerState::default();
    let mut buf = [0u8; READ_BUF_LEN];
    let mut last_report = Instant::now();

    while !shared.stop.load(Ordering::Acquire) {
        if inline_writes {
            // Only reached when no separate writer handle could be opened.
            let pending = match shared.output.try_lock() {
                Ok(mut slot) => ControllerShared::take_output(&mut slot),
                Err(_) => None,
            };
            if let Some(out) = pending {
                if let Err(HidError::Disconnected) = dev.write(builder.build(&out)) {
                    break;
                }
            }
        }

        let n = match dev.read_timeout(&mut buf, READ_TIMEOUT_MS) {
            Ok(0) => {
                if last_report.elapsed() >= SILENCE_TIMEOUT {
                    if !still_alive(dev.as_mut(), &shared.info) {
                        break;
                    }
                    last_report = Instant::now();
                }
                continue;
            }
            Ok(n) => n,
            Err(_) => break,
        };
        let arrived = Instant::now();
        last_report = arrived;
        if parser.parse(&buf[..n], &mut state).is_err() {
            shared.latency.record_error();
            continue;
        }
        sink.on_input(&state);
        shared.latency.record(arrived.elapsed().as_nanos() as u64);
        if let Ok(mut snap) = shared.snapshot.try_lock() {
            *snap = state;
        }
    }

    sink.on_disconnect();
    shared.connected.store(false, Ordering::Release);
    // Wake the writer so it notices.
    shared.stop.store(true, Ordering::Release);
    let _slot = shared.output.lock().unwrap();
    shared.output_changed.notify_all();
}

fn writer_loop(shared: Arc<ControllerShared>, mut dev: Box<dyn HidDevice>) {
    let mut builder = OutputBuilder::new(shared.info.model, shared.info.transport);
    let refresh_rumble = shared.info.model.is_switch();
    let mut last: Option<OutputState> = None;
    loop {
        let rumbling =
            refresh_rumble && last.is_some_and(|o| o.rumble_strong != 0 || o.rumble_weak != 0);
        let wait = if rumbling {
            SWITCH_RUMBLE_REFRESH
        } else {
            WRITER_IDLE
        };
        let out = {
            let slot = shared.output.lock().unwrap();
            let mut slot = shared
                .output_changed
                .wait_timeout_while(slot, wait, |s| {
                    !s.dirty && !shared.stop.load(Ordering::Acquire)
                })
                .unwrap()
                .0;
            if shared.stop.load(Ordering::Acquire) {
                return;
            }
            match ControllerShared::take_output(&mut slot) {
                Some(out) => out,
                None if rumbling => match last {
                    Some(out) => out,
                    None => continue,
                },
                None => continue,
            }
        };
        last = Some(out);
        if let Err(HidError::Disconnected) = dev.write(builder.build(&out)) {
            return;
        }
    }
}
