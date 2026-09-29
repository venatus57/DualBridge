//! Controller manager and I/O threads, driven by the mock backend.

use std::sync::mpsc;
use std::time::Duration;

use dualbridge_core::color::Rgb;
use dualbridge_core::crc;
use dualbridge_core::output::OutputState;
use dualbridge_core::state::Buttons;
use dualbridge_core::{ControllerState, Model, Transport};
use dualbridge_hid::manager::{ControllerManager, ManagerEvent, MAX_SLOTS};
use dualbridge_hid::mock::{MockBackend, MockController};
use dualbridge_hid::runtime::InputSink;
use dualbridge_hid::DeviceInfo;

const WAIT: Duration = Duration::from_secs(5);

/// A sink that forwards every state to a channel (fine for tests; the real
/// sink must not allocate).
struct ChannelSink(mpsc::Sender<(u8, ControllerState)>, u8);

impl InputSink for ChannelSink {
    fn on_input(&mut self, state: &ControllerState) {
        let _ = self.0.send((self.1, *state));
    }
}

fn manager(
    backend: &MockBackend,
) -> (
    ControllerManager<MockBackend>,
    mpsc::Receiver<(u8, ControllerState)>,
) {
    let (tx, rx) = mpsc::channel();
    let factory = move |_info: &DeviceInfo, slot: u8| {
        let sink: Box<dyn InputSink> = Box::new(ChannelSink(tx.clone(), slot));
        let output = OutputState {
            lightbar: Rgb::new(slot, 0, 0),
            release_startup_light: true,
            ..OutputState::default()
        };
        (sink, output)
    };
    (ControllerManager::new(backend.clone(), factory), rx)
}

fn usb_report(model: Model, cross: bool) -> Vec<u8> {
    let mut r = vec![0u8; 64];
    r[0] = 0x01;
    let face = if cross { 0x20 | 0x08 } else { 0x08 };
    if model.is_dualsense() {
        r[8] = face;
    } else {
        r[5] = face;
    }
    r
}

fn wait_until(mut f: impl FnMut() -> bool) {
    let start = std::time::Instant::now();
    while !f() {
        assert!(start.elapsed() < WAIT, "condition not met in time");
        std::thread::sleep(Duration::from_millis(1));
    }
}

fn connected_slots(events: &[ManagerEvent]) -> Vec<u8> {
    events
        .iter()
        .filter_map(|e| match e {
            ManagerEvent::Connected { slot, .. } => Some(*slot),
            _ => None,
        })
        .collect()
}

#[test]
fn input_reaches_sink_and_snapshot() {
    let backend = MockBackend::default();
    let pad = backend.add(Model::DualSense, Transport::Usb, "pad-1");
    let (mut m, rx) = manager(&backend);
    assert_eq!(connected_slots(&m.poll().unwrap()), vec![1]);

    pad.push_report(&usb_report(Model::DualSense, true));
    let (slot, state) = rx.recv_timeout(WAIT).unwrap();
    assert_eq!(slot, 1);
    assert!(state.buttons.contains(Buttons::CROSS));

    // Latency and the snapshot are recorded right after the sink returns.
    let shared = m.get(1).unwrap().clone();
    wait_until(|| shared.latency.stats().reports == 1);
    wait_until(|| shared.snapshot().buttons.contains(Buttons::CROSS));

    // Garbage is counted, not forwarded.
    pad.push_report(&[0x7F; 64]);
    pad.push_report(&usb_report(Model::DualSense, false));
    let (_, state) = rx.recv_timeout(WAIT).unwrap();
    assert!(!state.buttons.contains(Buttons::CROSS));
    wait_until(|| shared.latency.stats().errors == 1);
}

#[test]
fn output_is_written_on_change_only() {
    let backend = MockBackend::default();
    let pad = backend.add(Model::DualShock4, Transport::Bluetooth, "pad-1");
    let (mut m, _rx) = manager(&backend);
    m.poll().unwrap();
    // Reader + writer handles.
    assert_eq!(pad.open_count(), 2);

    let written = pad.wait_written(1, WAIT);
    assert_eq!(written[0][0], 0x11);
    assert_eq!(written[0][8], 1); // red = slot number
    assert!(crc::verify(crc::SEED_OUTPUT, &written[0]));

    let shared = m.get(1).unwrap().clone();
    shared.update_output(|o| o.lightbar = Rgb::new(1, 0, 0)); // unchanged
    shared.update_output(|o| o.rumble_strong = 200);
    let written = pad.wait_written(2, WAIT);
    assert_eq!(written.len(), 2);
    assert_eq!(written[1][7], 200);
    std::thread::sleep(Duration::from_millis(50));
    assert_eq!(pad.written().len(), 2);
}

#[test]
fn single_handle_fallback_writes_from_input_thread() {
    let backend = MockBackend::default();
    let pad = backend.add(Model::DualSense, Transport::Usb, "pad-1");
    pad.set_max_opens(1);
    let (mut m, _rx) = manager(&backend);
    m.poll().unwrap();
    let written = pad.wait_written(1, WAIT);
    assert_eq!(written[0][0], 0x02);
    // The startup light release is only sent once.
    assert_eq!(written[0][1 + 41], 0x02);
    m.get(1).unwrap().update_output(|o| o.rumble_weak = 9);
    let written = pad.wait_written(2, WAIT);
    assert_eq!(written[1][1 + 41], 0x00);
    assert_eq!(written[1][1 + 2], 9);
}

#[test]
fn hotplug_and_stable_slots() {
    let backend = MockBackend::default();
    let a = backend.add(Model::DualSense, Transport::Usb, "a");
    let b = backend.add(Model::DualShock4, Transport::Usb, "b");
    let (mut m, _rx) = manager(&backend);
    assert_eq!(connected_slots(&m.poll().unwrap()), vec![1, 2]);

    a.disconnect();
    let events = m.poll().unwrap();
    assert!(matches!(
        events[0],
        ManagerEvent::Disconnected { slot: 1, .. }
    ));
    assert_eq!(m.controllers().count(), 1);

    // A new controller does not take a's remembered slot...
    let c = backend.add(Model::DualSenseEdge, Transport::Bluetooth, "c");
    assert_eq!(connected_slots(&m.poll().unwrap()), vec![3]);
    // ...so a gets slot 1 back.
    a.reconnect();
    assert_eq!(connected_slots(&m.poll().unwrap()), vec![1]);

    assert!(m.swap_slots(1, 3));
    assert_eq!(m.get(1).unwrap().info.serial.as_deref(), Some("c"));
    assert_eq!(m.get(3).unwrap().info.serial.as_deref(), Some("a"));
    assert!(!m.swap_slots(0, 9));

    // After a swap, reconnecting keeps the new slot.
    a.disconnect();
    m.poll().unwrap();
    a.reconnect();
    assert_eq!(connected_slots(&m.poll().unwrap()), vec![3]);
    drop((b, c));
}

#[test]
fn eight_slots_max_and_duplicates() {
    let backend = MockBackend::default();
    let pads: Vec<MockController> = (0..MAX_SLOTS + 1)
        .map(|i| backend.add(Model::DualSense, Transport::Usb, &format!("pad-{i}")))
        .collect();
    let (mut m, _rx) = manager(&backend);
    let events = m.poll().unwrap();
    assert_eq!(connected_slots(&events), (1..=8).collect::<Vec<u8>>());
    assert!(matches!(
        events.last(),
        Some(ManagerEvent::NoFreeSlot { .. })
    ));

    // The same controller over Bluetooth while it is on USB is ignored.
    pads[0].disconnect();
    pads[MAX_SLOTS].disconnect();
    m.poll().unwrap();
    let _usb = backend.add(Model::DualSense, Transport::Usb, "dup");
    let _bt = backend.add(Model::DualSense, Transport::Bluetooth, "dup");
    let events = m.poll().unwrap();
    assert_eq!(connected_slots(&events).len(), 1);
}

#[test]
fn bluetooth_reads_calibration_and_parses_full_reports() {
    let backend = MockBackend::default();
    let pad = backend.add(Model::DualSense, Transport::Bluetooth, "bt");
    let mut cal = vec![0u8; 41];
    cal[0] = 0x05;
    // Gyro: bias 0, +/-1000 at +/-1000 deg/s => 1 count per deg/s on pitch.
    for (i, v) in [
        0i16, 0, 0, 1000, -1000, 1000, -1000, 1000, -1000, 1000, 1000,
    ]
    .iter()
    .enumerate()
    {
        cal[1 + 2 * i..3 + 2 * i].copy_from_slice(&v.to_le_bytes());
    }
    crc::sign(crc::SEED_FEATURE, &mut cal);
    pad.set_feature_report(&cal);

    let (mut m, rx) = manager(&backend);
    m.poll().unwrap();
    let shared = m.get(1).unwrap().clone();
    assert!((shared.calibration.gyro[0].scale - 1.0).abs() < 1e-6);

    let mut r = vec![0u8; 78];
    r[0] = 0x31;
    r[2 + 8] = 0x20 | 0x08; // Options, D-pad neutral
    crc::sign(crc::SEED_INPUT, &mut r);
    pad.push_report(&r);
    let (_, state) = rx.recv_timeout(WAIT).unwrap();
    assert!(state.full);
    assert!(state.buttons.contains(Buttons::OPTIONS));
}
