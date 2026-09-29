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
use dualbridge_core::ControllerState;

use crate::latency::LatencyMeter;
use crate::{DeviceInfo, HidBackend, HidDevice, HidError, HidResult};

/// How long a read waits before checking whether the thread should stop.
const READ_TIMEOUT_MS: i32 = 100;
/// Largest input report we expect (padding included).
const READ_BUF_LEN: usize = 128;

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
        let calibration = read_calibration(reader.as_mut(), &info);
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

fn read_calibration(dev: &mut dyn HidDevice, info: &DeviceInfo) -> Calibration {
    let mut buf = [0u8; 64];
    let len = calibration::feature_report_len(info.model, info.transport);
    buf[0] = calibration::feature_report_id(info.model, info.transport);
    match dev.get_feature_report(&mut buf[..len]) {
        Ok(n) => Calibration::from_feature_report(info.model, info.transport, &buf[..n])
            .unwrap_or_default(),
        Err(_) => Calibration::default(),
    }
}

fn input_loop(
    shared: Arc<ControllerShared>,
    mut dev: Box<dyn HidDevice>,
    mut sink: Box<dyn InputSink>,
    inline_writes: bool,
) {
    let _priority = crate::priority::boost_current_thread();
    let parser = InputParser::new(shared.info.model, shared.info.transport);
    let mut builder = OutputBuilder::new(shared.info.model, shared.info.transport);
    let mut state = ControllerState::default();
    let mut buf = [0u8; READ_BUF_LEN];

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
            Ok(0) => continue,
            Ok(n) => n,
            Err(_) => break,
        };
        let arrived = Instant::now();
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
    loop {
        let out = {
            let slot = shared.output.lock().unwrap();
            let mut slot = shared
                .output_changed
                .wait_timeout_while(slot, Duration::from_millis(500), |s| {
                    !s.dirty && !shared.stop.load(Ordering::Acquire)
                })
                .unwrap()
                .0;
            if shared.stop.load(Ordering::Acquire) {
                return;
            }
            match ControllerShared::take_output(&mut slot) {
                Some(out) => out,
                None => continue,
            }
        };
        if let Err(HidError::Disconnected) = dev.write(builder.build(&out)) {
            return;
        }
    }
}
