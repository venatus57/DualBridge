//! Controller manager and I/O threads, driven by the mock backend.

use std::sync::mpsc;
use std::time::Duration;

use dualbridge_core::color::Rgb;
use dualbridge_core::crc;
use dualbridge_core::output::{OutputState, PlayerLeds};
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
fn hotplug_and_packed_slots() {
    let backend = MockBackend::default();
    let a = backend.add(Model::DualSense, Transport::Usb, "a");
    let b = backend.add(Model::DualShock4, Transport::Usb, "b");
    let c = backend.add(Model::DualSenseEdge, Transport::Bluetooth, "c");
    let (mut m, _rx) = manager(&backend);
    assert_eq!(connected_slots(&m.poll().unwrap()), vec![1, 2, 3]);

    // Controller 2 leaves: controller 3 becomes 2.
    b.disconnect();
    let events = m.poll().unwrap();
    assert!(matches!(
        events[0],
        ManagerEvent::Disconnected { slot: 2, .. }
    ));
    assert_eq!(m.controllers().count(), 2);
    assert_eq!(m.get(2).unwrap().info.serial.as_deref(), Some("c"));
    assert!(m.get(3).is_none());

    // It comes back as the next number.
    b.reconnect();
    assert_eq!(connected_slots(&m.poll().unwrap()), vec![3]);

    assert!(m.swap_slots(1, 3));
    assert_eq!(m.get(1).unwrap().info.serial.as_deref(), Some("b"));
    assert_eq!(m.get(3).unwrap().info.serial.as_deref(), Some("a"));
    assert!(!m.swap_slots(0, 9));
    // Swapping with an empty slot doesn't leave a gap.
    assert!(m.swap_slots(1, 5));
    assert_eq!(
        m.controllers().map(|(slot, _)| slot).collect::<Vec<_>>(),
        vec![1, 2, 3]
    );
    drop((a, c));
}

#[test]
fn switched_off_bluetooth_controller_is_removed() {
    let backend = MockBackend::default();
    let a = backend.add(Model::DualSense, Transport::Bluetooth, "a");
    let _b = backend.add(Model::DualShock4, Transport::Bluetooth, "b");
    let (mut m, _rx) = manager(&backend);
    assert_eq!(connected_slots(&m.poll().unwrap()), vec![1, 2]);

    // Controller 1 is switched off but Windows still lists it.
    a.go_silent();
    let start = std::time::Instant::now();
    wait_until(|| !m.get(1).unwrap().is_connected());
    assert!(start.elapsed() >= Duration::from_secs(2));
    let events = m.poll().unwrap();
    assert!(matches!(
        events[0],
        ManagerEvent::Disconnected { slot: 1, .. }
    ));
    // The one left becomes controller 1, and the silent one is not re-added.
    assert_eq!(m.get(1).unwrap().info.serial.as_deref(), Some("b"));
    assert_eq!(m.controllers().count(), 1);
    assert!(connected_slots(&m.poll().unwrap()).is_empty());

    // Switched back on: it comes back as controller 2.
    a.reconnect();
    let mut slots = Vec::new();
    for _ in 0..10 {
        slots = connected_slots(&m.poll().unwrap());
        if !slots.is_empty() {
            break;
        }
    }
    assert_eq!(slots, vec![2]);
}

#[test]
fn idle_controller_that_still_answers_stays() {
    let backend = MockBackend::default();
    let pad = backend.add(Model::DualSense, Transport::Bluetooth, "a");
    let (mut m, _rx) = manager(&backend);
    m.poll().unwrap();
    // No input reports for longer than the silence timeout, but the
    // controller answers the liveness check.
    std::thread::sleep(Duration::from_millis(2600));
    assert!(m.get(1).unwrap().is_connected());
    assert!(m.poll().unwrap().is_empty());
    drop(pad);
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
    let bt = backend.add(Model::DualSense, Transport::Bluetooth, "dup");
    let events = m.poll().unwrap();
    assert_eq!(connected_slots(&events).len(), 1);
    // The unused connection is reported once, so it can be hidden too.
    let dups: Vec<&ManagerEvent> = events
        .iter()
        .filter(|e| matches!(e, ManagerEvent::Duplicate { .. }))
        .collect();
    assert_eq!(dups.len(), 1);
    assert_eq!(m.duplicate_paths().next(), Some(&bt.info().path));
    assert!(!m
        .poll()
        .unwrap()
        .iter()
        .any(|e| matches!(e, ManagerEvent::Duplicate { .. })));
}

/// A Switch Pro Controller `0x30` report with B (bottom face button) held.
fn switch_report(pressed: bool) -> Vec<u8> {
    let mut r = vec![0u8; 64];
    r[0] = 0x30;
    r[2] = 0x80; // battery full
    r[3] = if pressed { 0x04 } else { 0 };
    // Both sticks centered (2048, 2048).
    for at in [6, 9] {
        r[at..at + 3].copy_from_slice(&[0x00, 0x08, 0x80]);
    }
    r
}

#[test]
fn switch_pro_controller_is_set_up_and_read() {
    for transport in [Transport::Bluetooth, Transport::Usb] {
        let backend = MockBackend::default();
        let pad = backend.add(Model::SwitchPro, transport, "sw");
        let (mut m, rx) = manager(&backend);
        assert_eq!(connected_slots(&m.poll().unwrap()), vec![1]);

        // Setup: USB handshake (USB only), then full reports, IMU,
        // vibration and the two stick calibration reads.
        let written = pad.written();
        let subcommands: Vec<u8> = written
            .iter()
            .filter(|w| w[0] == 0x01)
            .map(|w| w[10])
            .collect();
        assert_eq!(&subcommands[..5], &[0x03, 0x40, 0x48, 0x10, 0x10]);
        let usb_commands = written.iter().filter(|w| w[0] == 0x80).count();
        assert_eq!(
            usb_commands,
            if transport == Transport::Usb { 5 } else { 0 }
        );

        pad.push_report(&switch_report(true));
        let (_, state) = loop {
            let (slot, state) = rx.recv_timeout(WAIT).unwrap();
            // Skip the subcommand replies that are also parsed as input.
            if state.full {
                break (slot, state);
            }
        };
        assert!(state.buttons.contains(Buttons::CROSS));
        assert_eq!((state.left_stick.x, state.left_stick.y), (128, 128));
        assert_eq!(state.battery.percent, 100);

        // Player lights are sent as a subcommand when they change.
        let lights = |n: usize| {
            let written = pad.wait_written(n, WAIT);
            written
                .iter()
                .filter(|w| w[0] == 0x01 && w[10] == 0x30)
                .map(|w| w[11])
                .collect::<Vec<u8>>()
        };
        let before = pad.written().len();
        m.get(1)
            .unwrap()
            .update_output(|o| o.player_leds = PlayerLeds::for_player(1));
        let start = std::time::Instant::now();
        while lights(before + 1).last() != Some(&0b0001) {
            assert!(start.elapsed() < WAIT, "player lights not sent");
            std::thread::sleep(Duration::from_millis(5));
        }
    }
}

#[test]
fn switch_rumble_is_refreshed_while_active() {
    let backend = MockBackend::default();
    let pad = backend.add(Model::SwitchPro, Transport::Bluetooth, "sw");
    let (mut m, _rx) = manager(&backend);
    m.poll().unwrap();
    let shared = m.get(1).unwrap().clone();
    let before = pad.wait_written(6, WAIT).len();
    shared.update_output(|o| o.rumble_strong = 200);
    // Without any change, the rumble keeps being resent.
    let written = pad.wait_written(before + 5, WAIT);
    assert!(written[before..]
        .iter()
        .all(|w| w[2..6] != [0x00, 0x01, 0x40, 0x40]));
    shared.update_output(|o| o.rumble_strong = 0);
    std::thread::sleep(Duration::from_millis(100));
    let n = pad.written().len();
    std::thread::sleep(Duration::from_millis(150));
    assert_eq!(pad.written().len(), n);
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
