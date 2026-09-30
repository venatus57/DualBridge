//! Demo mode: simulated controllers, for trying the app (and developing the
//! UI) without hardware. Start the app with `--demo` or `DUALBRIDGE_DEMO=1`.

use std::f32::consts::TAU;
use std::time::{Duration, Instant};

use dualbridge_core::crc;
use dualbridge_core::{Model, Transport};
use dualbridge_hid::mock::{MockBackend, MockController};

pub fn enabled() -> bool {
    std::env::args().any(|a| a == "--demo")
        || std::env::var("DUALBRIDGE_DEMO").is_ok_and(|v| v != "0" && !v.is_empty())
}

/// Creates a backend with three simulated controllers and starts feeding them.
pub fn backend() -> MockBackend {
    let backend = MockBackend::default();
    let dualsense = backend.add(Model::DualSense, Transport::Usb, "DEMO-DUALSENSE");
    let ds4 = backend.add(Model::DualShock4, Transport::Bluetooth, "DEMO-DS4");
    let pro = backend.add(Model::SwitchPro, Transport::Bluetooth, "DEMO-SWITCH");
    std::thread::Builder::new()
        .name("dualbridge-demo".into())
        .spawn(move || simulate(dualsense, ds4, pro))
        .expect("spawn demo thread");
    backend
}

fn simulate(dualsense: MockController, ds4: MockController, pro: MockController) {
    let start = Instant::now();
    let mut counter = 0u8;
    loop {
        let t = start.elapsed().as_secs_f32();
        counter = counter.wrapping_add(1);
        dualsense.push_report(&dualsense_usb(t, counter));
        ds4.push_report(&ds4_bt(t + 1.7, counter));
        pro.push_report(&switch_bt(t + 3.1, counter));
        // ~250 Hz like a real controller over USB.
        std::thread::sleep(Duration::from_millis(4));
    }
}

fn axis(v: f32) -> u8 {
    (128.0 + v.clamp(-1.0, 1.0) * 127.0) as u8
}

struct Sim {
    lx: u8,
    ly: u8,
    rx: u8,
    ry: u8,
    l2: u8,
    r2: u8,
    face: u8,
    shoulders: u8,
    touch: [u8; 4],
}

fn sim(t: f32) -> Sim {
    // One face/shoulder button at a time, D-pad walking around.
    let step = (t / 0.4) as u32;
    let face_bits = [0x10u8, 0x20, 0x40, 0x80, 0, 0, 0, 0];
    let hat = if (step / 8).is_multiple_of(2) {
        (step % 8) as u8
    } else {
        8
    };
    let shoulders = [0x01u8, 0x02, 0, 0, 0x10, 0x20, 0, 0][(step % 8) as usize];
    let tx = (960.0 + 800.0 * (t * 0.7).sin()) as u16;
    let ty = (470.0 + 300.0 * (t * 1.3).cos()) as u16;
    let touching = (t % 4.0) < 2.5;
    Sim {
        lx: axis((t * 1.2).cos()),
        ly: axis((t * 1.2).sin()),
        rx: axis((t * 0.8).sin()),
        ry: axis((t * 1.6).sin() * 0.6),
        l2: (127.5 + 127.5 * (t * TAU * 0.25).sin()) as u8,
        r2: if (t % 3.0) < 0.6 { 255 } else { 0 },
        face: face_bits[(step % 8) as usize] | hat,
        shoulders,
        touch: [
            if touching { 7 } else { 0x80 | 7 },
            tx as u8,
            ((tx >> 8) as u8 & 0x0F) | ((ty as u8 & 0x0F) << 4),
            (ty >> 4) as u8,
        ],
    }
}

fn dualsense_usb(t: f32, counter: u8) -> [u8; 64] {
    let s = sim(t);
    let mut r = [0u8; 64];
    r[0] = 0x01;
    let d = &mut r[1..];
    d[0..4].copy_from_slice(&[s.lx, s.ly, s.rx, s.ry]);
    d[4] = s.l2;
    d[5] = s.r2;
    d[6] = counter;
    d[7] = s.face;
    d[8] = s.shoulders;
    // Press "mute" for a moment every 8 seconds.
    d[9] = if (t % 8.0) < 0.15 { 0x04 } else { 0 };
    let gyro = [(t.sin() * 300.0) as i16, 0, 0];
    for (i, g) in gyro.iter().enumerate() {
        d[15 + 2 * i..17 + 2 * i].copy_from_slice(&g.to_le_bytes());
    }
    d[25..27].copy_from_slice(&8192i16.to_le_bytes());
    d[32..36].copy_from_slice(&s.touch);
    d[36] = 0x80;
    d[52] = 0x10 | 7; // charging, ~75 %
    d[53] = 0x08;
    r
}

fn ds4_bt(t: f32, counter: u8) -> [u8; 78] {
    let s = sim(t);
    let mut r = [0u8; 78];
    r[0] = 0x11;
    r[1] = 0xC0;
    let d = &mut r[3..];
    d[0..4].copy_from_slice(&[s.lx, s.ly, s.rx, s.ry]);
    d[4] = s.face;
    d[5] = s.shoulders;
    d[6] = (counter & 0x3F) << 2;
    d[7] = s.l2;
    d[8] = s.r2;
    d[29] = 1; // ~15 %, on battery: shows the low-battery warning
    d[32] = 1;
    d[34..38].copy_from_slice(&s.touch);
    d[38] = 0x80;
    crc::sign(crc::SEED_INPUT, &mut r);
    r
}

/// Switch Pro Controller `0x30` report (sticks use the default calibration:
/// center 2048, reach 1400).
fn switch_bt(t: f32, counter: u8) -> [u8; 64] {
    let s = sim(t);
    let mut r = [0u8; 64];
    r[0] = 0x30;
    r[1] = counter;
    r[2] = 0x60; // ~70 %, on battery
                 // PlayStation face bits (square, cross, circle, triangle) to the Switch
                 // buttons in the same positions (Y, B, A, X).
    let face = s.face & 0xF0;
    r[3] = [(0x10, 0x01), (0x20, 0x04), (0x40, 0x08), (0x80, 0x02)]
        .iter()
        .filter(|(ps, _)| face & ps != 0)
        .fold(0, |acc, (_, sw)| acc | sw);
    if s.r2 > 0 {
        r[3] |= 0x80;
    }
    let stick = |x: u8, y: u8| {
        let raw = |v: u8| (2048 + (v as i32 - 128) * 1400 / 128).clamp(0, 4095) as u16;
        // The Switch reports y growing upward.
        let (x, y) = (raw(x), raw(255 - y));
        [
            (x & 0xFF) as u8,
            ((x >> 8) as u8 & 0x0F) | ((y as u8 & 0x0F) << 4),
            (y >> 4) as u8,
        ]
    };
    r[6..9].copy_from_slice(&stick(s.lx, s.ly));
    r[9..12].copy_from_slice(&stick(s.rx, s.ry));
    r
}

#[cfg(test)]
mod tests {
    use super::*;
    use dualbridge_core::input::InputParser;
    use dualbridge_core::ControllerState;

    #[test]
    fn simulated_reports_parse() {
        let mut s = ControllerState::default();
        InputParser::new(Model::DualSense, Transport::Usb)
            .parse(&dualsense_usb(1.0, 1), &mut s)
            .unwrap();
        assert_eq!(s.battery.percent, 75);
        InputParser::new(Model::DualShock4, Transport::Bluetooth)
            .parse(&ds4_bt(1.0, 1), &mut s)
            .unwrap();
        assert_eq!(s.battery.percent, 15);
        InputParser::new(Model::SwitchPro, Transport::Bluetooth)
            .parse(&switch_bt(1.0, 1), &mut s)
            .unwrap();
        assert_eq!(s.battery.percent, 70);
    }
}
