//! Input and output reports checked against the fixtures in `tests/fixtures/`.

use dualbridge_core::color::Rgb;
use dualbridge_core::crc;
use dualbridge_core::input::{InputParser, ParseError};
use dualbridge_core::output::{
    Flash, MicLed, OutputBuilder, OutputState, PlayerLedBrightness, PlayerLeds, TriggerEffect,
};
use dualbridge_core::state::{Buttons, Charging, ControllerState};
use dualbridge_core::{Model, Transport};

fn fixture(name: &str) -> Vec<u8> {
    let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
    text.lines()
        .map(|l| l.split('#').next().unwrap())
        .flat_map(|l| l.split_whitespace())
        .map(|b| u8::from_str_radix(b, 16).unwrap())
        .collect()
}

fn parse(model: Model, transport: Transport, report: &[u8]) -> ControllerState {
    let mut s = ControllerState::default();
    InputParser::new(model, transport)
        .parse(report, &mut s)
        .expect("fixture parses");
    s
}

fn check_ds4(s: &ControllerState) {
    assert!(s.full);
    assert_eq!((s.left_stick.x, s.left_stick.y), (0x10, 0x20));
    assert_eq!((s.right_stick.x, s.right_stick.y), (0xE0, 0xF0));
    assert_eq!(
        s.buttons,
        Buttons::CROSS | Buttons::DPAD_RIGHT | Buttons::L1 | Buttons::OPTIONS | Buttons::PS
    );
    assert_eq!((s.l2, s.r2), (0x40, 0xFF));
    assert_eq!(s.counter, 5);
    assert_eq!(s.motion.timestamp, 0x1234);
    assert_eq!(s.motion.gyro, [100, -200, 300]);
    assert_eq!(s.motion.accel, [-1, 8192, -8192]);
    assert_eq!(s.battery.percent, 50);
    assert_eq!(s.battery.charging, Charging::Charging);
    assert!(s.battery.cable);
    assert!(s.touch[0].active);
    assert_eq!((s.touch[0].id, s.touch[0].x, s.touch[0].y), (3, 100, 200));
    assert!(!s.touch[1].active);
    assert_eq!(s.trigger_feedback, [0, 0]);
}

#[test]
fn ds4_usb() {
    check_ds4(&parse(
        Model::DualShock4,
        Transport::Usb,
        &fixture("ds4_usb.hex"),
    ));
}

#[test]
fn ds4_bluetooth() {
    let report = fixture("ds4_bt.hex");
    check_ds4(&parse(Model::DualShock4, Transport::Bluetooth, &report));

    // A corrupted report is rejected and leaves the state untouched.
    let mut bad = report.clone();
    bad[10] ^= 0x01;
    let mut s = ControllerState::default();
    let p = InputParser::new(Model::DualShock4, Transport::Bluetooth);
    assert_eq!(p.parse(&bad, &mut s), Err(ParseError::BadCrc));
    assert_eq!(s, ControllerState::default());

    // Padded reports (as some platforms deliver them) are accepted.
    let mut padded = report;
    padded.resize(547, 0);
    check_ds4(&parse(Model::DualShock4, Transport::Bluetooth, &padded));
}

fn check_dualsense(s: &ControllerState, edge: bool) {
    assert!(s.full);
    assert_eq!((s.left_stick.x, s.left_stick.y), (0x00, 0xFF));
    assert_eq!((s.right_stick.x, s.right_stick.y), (0x80, 0x7F));
    assert_eq!((s.l2, s.r2), (0x11, 0x22));
    assert_eq!(s.counter, 0x42);
    let mut expected = Buttons::TRIANGLE
        | Buttons::SQUARE
        | Buttons::DPAD_UP
        | Buttons::DPAD_LEFT
        | Buttons::R1
        | Buttons::SHARE
        | Buttons::R3
        | Buttons::TOUCHPAD
        | Buttons::MUTE;
    if edge {
        expected = expected
            | Buttons::LEFT_FN
            | Buttons::RIGHT_FN
            | Buttons::LEFT_PADDLE
            | Buttons::RIGHT_PADDLE;
    }
    assert_eq!(s.buttons, expected);
    assert_eq!(s.motion.gyro, [-5, 6, -7]);
    assert_eq!(s.motion.accel, [10, -20, 8000]);
    assert_eq!(s.motion.timestamp, 0xDEAD_BEEF);
    assert_eq!(
        (s.touch[0].active, s.touch[0].id, s.touch[0].x, s.touch[0].y),
        (true, 1, 1919, 1079)
    );
    assert_eq!(
        (s.touch[1].active, s.touch[1].id, s.touch[1].x, s.touch[1].y),
        (true, 2, 0, 0)
    );
    assert_eq!(s.trigger_feedback, [0x12, 0x21]);
    assert_eq!(s.battery.percent, 75);
    assert_eq!(s.battery.charging, Charging::Charging);
    assert!(s.battery.cable);
    assert!(s.headphones);
}

#[test]
fn dualsense_usb() {
    let r = fixture("dualsense_usb.hex");
    check_dualsense(&parse(Model::DualSense, Transport::Usb, &r), false);
}

#[test]
fn dualsense_edge_usb() {
    let r = fixture("dualsense_edge_usb.hex");
    check_dualsense(&parse(Model::DualSenseEdge, Transport::Usb, &r), true);
    // A regular DualSense ignores the Edge-only bits.
    check_dualsense(&parse(Model::DualSense, Transport::Usb, &r), false);
}

#[test]
fn dualsense_bluetooth() {
    let r = fixture("dualsense_bt.hex");
    check_dualsense(&parse(Model::DualSense, Transport::Bluetooth, &r), false);
}

#[test]
fn bluetooth_simple_mode() {
    let r = fixture("bt_simple.hex");
    for model in [Model::DualShock4, Model::DualSense] {
        let mut s = ControllerState::default();
        s.motion.gyro = [1, 2, 3];
        InputParser::new(model, Transport::Bluetooth)
            .parse(&r, &mut s)
            .unwrap();
        assert!(!s.full);
        assert_eq!((s.left_stick.x, s.right_stick.y), (0x10, 0xF0));
        assert!(s.buttons.contains(Buttons::CROSS | Buttons::PS));
        assert_eq!((s.l2, s.r2), (0x40, 0xFF));
        // Motion data is not in simple reports: the previous value stays.
        assert_eq!(s.motion.gyro, [1, 2, 3]);
    }
}

#[test]
fn ds4_output_usb() {
    let mut b = OutputBuilder::new(Model::DualShock4, Transport::Usb);
    let r = b.build(&OutputState {
        rumble_strong: 0xAA,
        rumble_weak: 0x55,
        lightbar: Rgb::new(1, 2, 3),
        flash: Some(Flash { on: 10, off: 20 }),
        ..OutputState::default()
    });
    assert_eq!(r.len(), 32);
    assert_eq!(&r[..11], &[0x05, 0x07, 0, 0, 0x55, 0xAA, 1, 2, 3, 10, 20]);
    assert!(r[11..].iter().all(|&b| b == 0));
}

#[test]
fn ds4_output_bluetooth() {
    let mut b = OutputBuilder::new(Model::DualShock4, Transport::Bluetooth);
    let r = b.build(&OutputState {
        rumble_strong: 0xAA,
        rumble_weak: 0x55,
        lightbar: Rgb::new(1, 2, 3),
        ..OutputState::default()
    });
    assert_eq!(r.len(), 78);
    assert_eq!(
        &r[..13],
        &[0x11, 0xC0, 0, 0x07, 0, 0, 0x55, 0xAA, 1, 2, 3, 0, 0]
    );
    assert!(crc::verify(crc::SEED_OUTPUT, r));
}

fn dualsense_state() -> OutputState {
    OutputState {
        rumble_strong: 0x80,
        rumble_weak: 0x40,
        lightbar: Rgb::new(9, 8, 7),
        player_leds: PlayerLeds::for_player(2),
        player_led_brightness: PlayerLedBrightness::Low,
        mic_led: MicLed::Pulse,
        left_trigger: TriggerEffect::Feedback {
            position: 0,
            strength: 1,
        },
        right_trigger: TriggerEffect::Weapon {
            start: 2,
            end: 5,
            strength: 8,
        },
        ..OutputState::default()
    }
}

fn check_dualsense_payload(p: &[u8], release: bool) {
    assert_eq!(p[0], 0x0F);
    assert_eq!(p[1], 0x15);
    assert_eq!((p[2], p[3]), (0x40, 0x80));
    assert_eq!(p[8], 2);
    assert_eq!(&p[10..14], &[0x25, 0b0010_0100, 0, 7]);
    assert_eq!(&p[21..24], &[0x21, 0xFF, 0x03]);
    assert_eq!(p[38], if release { 0x03 } else { 0x01 });
    assert_eq!(p[41], if release { 0x02 } else { 0x00 });
    assert_eq!(p[42], 2);
    assert_eq!(p[43], 0b01010);
    assert_eq!(&p[44..47], &[9, 8, 7]);
}

#[test]
fn dualsense_output_usb() {
    let mut b = OutputBuilder::new(Model::DualSense, Transport::Usb);
    let r = b.build(&dualsense_state());
    assert_eq!(r.len(), 48);
    assert_eq!(r[0], 0x02);
    check_dualsense_payload(&r[1..], false);

    let r = b.build(&OutputState {
        release_startup_light: true,
        ..dualsense_state()
    });
    check_dualsense_payload(&r[1..], true);
}

#[test]
fn dualsense_output_bluetooth() {
    let mut b = OutputBuilder::new(Model::DualSenseEdge, Transport::Bluetooth);
    for seq in 0..20u8 {
        let r = b.build(&dualsense_state());
        assert_eq!(r.len(), 78);
        assert_eq!(r[0], 0x31);
        assert_eq!(r[1], (seq % 16) << 4);
        assert_eq!(r[2], 0x10);
        check_dualsense_payload(&r[3..], false);
        assert!(crc::verify(crc::SEED_OUTPUT, r));
    }
}
