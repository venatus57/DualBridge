//! Nintendo Switch Pro Controller protocol.
//!
//! Written from the community protocol notes (report layouts, subcommands,
//! SPI flash map and rumble encoding); no driver code was used.
//!
//! **Reports.** The controller sends:
//!
//! | ID     | What                                                        |
//! |--------|-------------------------------------------------------------|
//! | `0x30` | Full report: buttons, sticks, battery and 3 IMU samples      |
//! | `0x21` | Reply to a subcommand (same first 13 bytes as `0x30`)       |
//! | `0x81` | Reply to a USB-only `0x80` command                          |
//! | `0x3F` | "Simple" report sent on Bluetooth before it is configured   |
//!
//! and accepts `0x01` (rumble + subcommand), `0x10` (rumble only) and, on USB,
//! `0x80` commands. Common layout of `0x30` / `0x21`:
//!
//! | Byte    | Content                                                   |
//! |---------|-----------------------------------------------------------|
//! | 1       | Timer                                                     |
//! | 2       | Battery level (high nibble) and connection info           |
//! | 3, 4, 5 | Right, shared and left buttons                            |
//! | 6..9    | Left stick, two 12-bit values (x, then y growing upward)  |
//! | 9..12   | Right stick                                               |
//! | 13..49  | `0x30`: 3 IMU samples (accel xyz, gyro xyz, `i16` LE)     |
//! | 13, 14  | `0x21`: ACK byte and the subcommand it answers            |
//! | 15..    | `0x21`: subcommand reply data                             |
//!
//! **Setup** ([`SwitchInit`]): on USB the controller first needs a handshake
//! and "USB only" mode (`0x80` commands); then, on both transports, it is set
//! to full `0x30` reports with the IMU and rumble on, and the stick calibration
//! is read from its SPI flash. Sticks report raw 12-bit positions whose center
//! and range differ per controller, so that calibration is needed to map them
//! to the usual 0-255 range.
//!
//! Buttons are mapped by position: the bottom face button (B) acts as
//! PlayStation Cross / Xbox A, the right one (A) as Circle / Xbox B, and so on.
//!
//! Nothing in the input path allocates.

use serde::{Deserialize, Serialize};

use crate::device::Transport;
use crate::output::OutputState;
use crate::state::{Battery, Buttons, Charging, ControllerState};

pub const REPORT_FULL: u8 = 0x30;
pub const REPORT_REPLY: u8 = 0x21;
pub const REPORT_USB_REPLY: u8 = 0x81;
pub const OUT_SUBCOMMAND: u8 = 0x01;
pub const OUT_RUMBLE: u8 = 0x10;
pub const OUT_USB: u8 = 0x80;

/// Length of the reports we build (the Bluetooth output report size; USB
/// accepts shorter reports than its 64 bytes).
pub const OUTPUT_LEN: usize = 49;
/// Shortest `0x30` report that carries the IMU samples.
const FULL_LEN: usize = 49;
/// Bytes shared by `0x30` and `0x21` reports.
const STANDARD_LEN: usize = 13;

const SUB_SET_INPUT_MODE: u8 = 0x03;
/// Set HCI state; argument 0 disconnects and switches the controller off.
const SUB_HCI_STATE: u8 = 0x06;
const SUB_SPI_READ: u8 = 0x10;
const SUB_PLAYER_LIGHTS: u8 = 0x30;
const SUB_ENABLE_IMU: u8 = 0x40;
const SUB_ENABLE_VIBRATION: u8 = 0x48;

const USB_STATUS: u8 = 0x01;
const USB_HANDSHAKE: u8 = 0x02;
const USB_HIGH_SPEED: u8 = 0x03;
const USB_ONLY: u8 = 0x04;

/// Factory stick calibration: 9 bytes for the left stick, then 9 for the right.
const SPI_FACTORY_STICKS: u32 = 0x603D;
/// User stick calibration: magic `B2 A1` + 9 bytes, twice (left, right).
const SPI_USER_STICKS: u32 = 0x8010;
const USER_CAL_MAGIC: [u8; 2] = [0xB2, 0xA1];

/// Rumble data that keeps both actuators still.
pub const RUMBLE_NEUTRAL: [u8; 4] = [0x00, 0x01, 0x40, 0x40];

/// One stick's raw center and reach in each direction (12-bit units).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StickRange {
    pub center_x: u16,
    pub center_y: u16,
    /// How far the raw value goes below the center (left / down).
    pub below_x: u16,
    pub below_y: u16,
    /// How far it goes above the center (right / up).
    pub above_x: u16,
    pub above_y: u16,
}

impl Default for StickRange {
    fn default() -> Self {
        // Typical values; real ones are read from the controller.
        StickRange {
            center_x: 2048,
            center_y: 2048,
            below_x: 1400,
            below_y: 1400,
            above_x: 1400,
            above_y: 1400,
        }
    }
}

impl StickRange {
    /// Decodes the 9-byte flash block: six 12-bit values packed in pairs.
    /// The left stick stores (above, center, below), the right stick
    /// (center, below, above), each as an x/y pair.
    fn from_flash(b: &[u8], left: bool) -> Option<StickRange> {
        if b.len() < 9 || b[..9].iter().all(|&x| x == 0xFF) {
            return None;
        }
        let mut v = [0u16; 6];
        for i in 0..3 {
            v[2 * i] = (((b[3 * i + 1] & 0x0F) as u16) << 8) | b[3 * i] as u16;
            v[2 * i + 1] = ((b[3 * i + 2] as u16) << 4) | (b[3 * i + 1] >> 4) as u16;
        }
        let (above, center, below) = if left {
            ((v[0], v[1]), (v[2], v[3]), (v[4], v[5]))
        } else {
            ((v[4], v[5]), (v[0], v[1]), (v[2], v[3]))
        };
        let r = StickRange {
            center_x: center.0,
            center_y: center.1,
            below_x: below.0,
            below_y: below.1,
            above_x: above.0,
            above_y: above.1,
        };
        // Reject nonsense (zero reach would divide by zero).
        let sane = [r.below_x, r.below_y, r.above_x, r.above_y]
            .iter()
            .all(|&d| (200..=2047).contains(&d))
            && (500..=3600).contains(&r.center_x)
            && (500..=3600).contains(&r.center_y);
        sane.then_some(r)
    }

    /// Maps a raw position to 0-255 with 128 at the center, `y` growing
    /// downward like the PlayStation controllers.
    #[inline]
    fn map(&self, raw_x: u16, raw_y: u16) -> (u8, u8) {
        let x = axis(raw_x, self.center_x, self.below_x, self.above_x);
        let y = axis(raw_y, self.center_y, self.below_y, self.above_y);
        (to_byte(x), to_byte(-y))
    }
}

/// Signed position in -1024..=1024.
#[inline]
fn axis(raw: u16, center: u16, below: u16, above: u16) -> i32 {
    let d = raw as i32 - center as i32;
    let reach = if d >= 0 { above } else { below }.max(1) as i32;
    (d * 1024 / reach).clamp(-1024, 1024)
}

#[inline]
fn to_byte(v: i32) -> u8 {
    // -1024 -> 0, 0 -> 128, 1024 -> 255.
    let b = if v >= 0 {
        128 + v * 127 / 1024
    } else {
        128 + v * 128 / 1024
    };
    b as u8
}

/// Calibration of both sticks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct StickCalibration {
    pub left: StickRange,
    pub right: StickRange,
}

#[inline]
fn stick_raw(d: &[u8]) -> (u16, u16) {
    let x = d[0] as u16 | ((d[1] as u16 & 0x0F) << 8);
    let y = (d[1] as u16 >> 4) | ((d[2] as u16) << 4);
    (x, y)
}

#[inline]
fn i16_at(d: &[u8], i: usize) -> i16 {
    i16::from_le_bytes([d[i], d[i + 1]])
}

/// Parses the part shared by `0x30` and `0x21` reports (`r` starts with the
/// report ID and is at least 13 bytes long).
fn parse_standard(r: &[u8], sticks: &StickCalibration, s: &mut ControllerState) {
    s.counter = r[1];

    let status = r[2];
    let level = (status >> 4) & 0x0E; // 0, 2, 4, 6 or 8
    let charging = status & 0x10 != 0;
    let cable = status & 0x01 != 0;
    let percent = match level {
        8 => 100,
        6 => 70,
        4 => 40,
        2 => 15,
        _ => 0,
    };
    s.battery = Battery {
        percent,
        charging: if charging {
            Charging::Charging
        } else if cable && level == 8 {
            Charging::Full
        } else {
            Charging::Discharging
        },
        cable,
    };

    let (right, shared, left) = (r[3], r[4], r[5]);
    let b = &mut s.buttons;
    // Face buttons by position (see the module docs).
    b.set(Buttons::SQUARE, right & 0x01 != 0); // Y, left
    b.set(Buttons::TRIANGLE, right & 0x02 != 0); // X, top
    b.set(Buttons::CROSS, right & 0x04 != 0); // B, bottom
    b.set(Buttons::CIRCLE, right & 0x08 != 0); // A, right
    b.set(Buttons::R1, right & 0x40 != 0);
    b.set(Buttons::R2, right & 0x80 != 0);
    b.set(Buttons::SHARE, shared & 0x01 != 0); // Minus
    b.set(Buttons::OPTIONS, shared & 0x02 != 0); // Plus
    b.set(Buttons::R3, shared & 0x04 != 0);
    b.set(Buttons::L3, shared & 0x08 != 0);
    b.set(Buttons::PS, shared & 0x10 != 0); // Home
    b.set(Buttons::TOUCHPAD, shared & 0x20 != 0); // Capture
    b.set(Buttons::DPAD_DOWN, left & 0x01 != 0);
    b.set(Buttons::DPAD_UP, left & 0x02 != 0);
    b.set(Buttons::DPAD_RIGHT, left & 0x04 != 0);
    b.set(Buttons::DPAD_LEFT, left & 0x08 != 0);
    b.set(Buttons::L1, left & 0x40 != 0);
    b.set(Buttons::L2, left & 0x80 != 0);
    // ZL / ZR are digital.
    s.l2 = if left & 0x80 != 0 { 255 } else { 0 };
    s.r2 = if right & 0x80 != 0 { 255 } else { 0 };

    let (lx, ly) = stick_raw(&r[6..9]);
    let (rx, ry) = stick_raw(&r[9..12]);
    let (x, y) = sticks.left.map(lx, ly);
    s.left_stick.x = x;
    s.left_stick.y = y;
    let (x, y) = sticks.right.map(rx, ry);
    s.right_stick.x = x;
    s.right_stick.y = y;
}

/// Parses a `0x30` or `0x21` report. Returns `false` for other reports or
/// reports that are too short.
pub fn parse(report: &[u8], sticks: &StickCalibration, s: &mut ControllerState) -> bool {
    match report.first() {
        Some(&REPORT_FULL) if report.len() >= FULL_LEN => {
            parse_standard(report, sticks, s);
            // The last of the three IMU samples is the most recent.
            let imu = &report[37..49];
            s.motion.accel = [i16_at(imu, 0), i16_at(imu, 2), i16_at(imu, 4)];
            s.motion.gyro = [i16_at(imu, 6), i16_at(imu, 8), i16_at(imu, 10)];
            s.motion.timestamp = report[1] as u32;
            s.full = true;
            true
        }
        Some(&REPORT_REPLY) if report.len() >= STANDARD_LEN => {
            parse_standard(report, sticks, s);
            true
        }
        _ => false,
    }
}

// Rumble -------------------------------------------------------------------

/// Low band frequency used for the "strong" motor.
const LOW_BAND_HZ: f32 = 160.0;
/// High band frequency used for the "weak" motor.
const HIGH_BAND_HZ: f32 = 320.0;

/// Encoded amplitude step (0 = silent, 100 = the safe maximum) for an
/// amplitude in 0..=1.
fn amplitude_step(amp: f32) -> u16 {
    if amp <= 0.0 {
        return 0;
    }
    let amp = amp.min(1.0);
    let step = if amp > 0.23 {
        (amp * 8.7).log2() * 32.0
    } else {
        (amp * 17.0).log2() * 16.0
    };
    step.round().clamp(0.0, 100.0) as u16
}

/// Rumble data for one actuator: `low` drives the 160 Hz band and `high` the
/// 320 Hz band, both 0-255.
pub fn encode_rumble(low: u8, high: u8) -> [u8; 4] {
    if low == 0 && high == 0 {
        return RUMBLE_NEUTRAL;
    }
    let freq = |hz: f32| ((hz / 10.0).log2() * 32.0).round() as u16;
    let hf = (freq(HIGH_BAND_HZ) - 0x60) * 4; // 9 bits, in bytes 0-1
    let lf = freq(LOW_BAND_HZ) - 0x40; // 7 bits, in byte 2
    let hf_amp = amplitude_step(high as f32 / 255.0) * 2; // even, up to 0xC8
    let lf_step = amplitude_step(low as f32 / 255.0);
    // Low amplitude: step / 2 + 0x40, its lowest bit stored in byte 2 bit 7.
    let lf_amp = ((lf_step & 1) << 15) | ((lf_step >> 1) + 0x40);
    [
        (hf & 0xFF) as u8,
        (hf_amp + (hf >> 8)) as u8,
        (lf + (lf_amp >> 8)) as u8,
        (lf_amp & 0xFF) as u8,
    ]
}

/// The four player LEDs show as many lights as the (5-LED, PlayStation-style)
/// pattern has, which gives the usual Switch player numbering: player 1 lights
/// one LED, player 4 all four.
pub fn player_lights(pattern: u8) -> u8 {
    let n = (pattern & 0x1F).count_ones().min(4);
    ((1u16 << n) - 1) as u8
}

// Output -------------------------------------------------------------------

/// Builds output reports: rumble only, or rumble plus the player lights
/// subcommand when they changed.
#[derive(Debug, Clone, Default)]
pub struct SwitchOutput {
    counter: u8,
    lights: Option<u8>,
}

impl SwitchOutput {
    /// Writes the report for `state` into `buf` and returns its length.
    pub fn build(&mut self, state: &OutputState, buf: &mut [u8]) -> usize {
        if state.power_off {
            return subcommand(self.next_counter(), SUB_HCI_STATE, &[0x00], buf);
        }
        buf[..OUTPUT_LEN].fill(0);
        let rumble = encode_rumble(state.rumble_strong, state.rumble_weak);
        buf[1] = self.next_counter();
        buf[2..6].copy_from_slice(&rumble);
        buf[6..10].copy_from_slice(&rumble);
        let lights = player_lights(state.player_leds.0);
        if self.lights != Some(lights) {
            self.lights = Some(lights);
            buf[0] = OUT_SUBCOMMAND;
            buf[10] = SUB_PLAYER_LIGHTS;
            buf[11] = lights;
        } else {
            buf[0] = OUT_RUMBLE;
        }
        OUTPUT_LEN
    }

    fn next_counter(&mut self) -> u8 {
        let c = self.counter;
        self.counter = (self.counter + 1) & 0x0F;
        c
    }
}

/// Builds a subcommand report with neutral rumble.
fn subcommand(counter: u8, id: u8, args: &[u8], buf: &mut [u8]) -> usize {
    buf[..OUTPUT_LEN].fill(0);
    buf[0] = OUT_SUBCOMMAND;
    buf[1] = counter & 0x0F;
    buf[2..6].copy_from_slice(&RUMBLE_NEUTRAL);
    buf[6..10].copy_from_slice(&RUMBLE_NEUTRAL);
    buf[10] = id;
    buf[11..11 + args.len()].copy_from_slice(args);
    OUTPUT_LEN
}

// USB identity ---------------------------------------------------------------

/// The USB status request; its `0x81 0x01` reply holds the Bluetooth address.
pub const USB_STATUS_REQUEST: [u8; 2] = [OUT_USB, USB_STATUS];

/// Extracts the MAC address (12 lowercase hex digits) from the USB status
/// reply, where it is stored least significant byte first.
pub fn parse_usb_status_mac(report: &[u8]) -> Option<String> {
    if report.len() < 10 || report[0] != REPORT_USB_REPLY || report[1] != USB_STATUS {
        return None;
    }
    let mac = &report[4..10];
    if mac.iter().all(|&b| b == 0) || mac.iter().all(|&b| b == 0xFF) {
        return None;
    }
    Some(mac.iter().rev().map(|b| format!("{b:02x}")).collect())
}

// Setup ----------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Step {
    /// USB `0x80` command; `reply` if the controller answers it.
    Usb {
        cmd: u8,
        reply: bool,
    },
    Sub {
        id: u8,
        args: [u8; 5],
        len: usize,
    },
}

const fn spi_read(addr: u32, size: u8) -> Step {
    let a = addr.to_le_bytes();
    Step::Sub {
        id: SUB_SPI_READ,
        args: [a[0], a[1], a[2], a[3], size],
        len: 5,
    }
}

const USB_STEPS: [Step; 5] = [
    Step::Usb {
        cmd: USB_STATUS,
        reply: true,
    },
    Step::Usb {
        cmd: USB_HANDSHAKE,
        reply: true,
    },
    Step::Usb {
        cmd: USB_HIGH_SPEED,
        reply: true,
    },
    Step::Usb {
        cmd: USB_HANDSHAKE,
        reply: true,
    },
    Step::Usb {
        cmd: USB_ONLY,
        reply: false,
    },
];

const COMMON_STEPS: [Step; 5] = [
    Step::Sub {
        id: SUB_SET_INPUT_MODE,
        args: [REPORT_FULL, 0, 0, 0, 0],
        len: 1,
    },
    Step::Sub {
        id: SUB_ENABLE_IMU,
        args: [1, 0, 0, 0, 0],
        len: 1,
    },
    Step::Sub {
        id: SUB_ENABLE_VIBRATION,
        args: [1, 0, 0, 0, 0],
        len: 1,
    },
    spi_read(SPI_FACTORY_STICKS, 18),
    spi_read(SPI_USER_STICKS, 22),
];

/// The setup sequence, as a small state machine the device layer drives:
/// write [`SwitchInit::request`], feed every report read to
/// [`SwitchInit::on_report`] until it returns `true` (the step is done), or
/// call [`SwitchInit::skip`] when the controller does not answer in time.
#[derive(Debug, Clone)]
pub struct SwitchInit {
    usb: bool,
    step: usize,
    counter: u8,
    answered: bool,
    /// Stick calibration read so far (defaults until read).
    pub sticks: StickCalibration,
    /// Bluetooth address from the USB status reply.
    pub mac: Option<String>,
}

impl SwitchInit {
    pub fn new(transport: Transport) -> SwitchInit {
        SwitchInit {
            usb: transport == Transport::Usb,
            step: 0,
            counter: 0,
            answered: false,
            sticks: StickCalibration::default(),
            mac: None,
        }
    }

    fn current(&self) -> Option<Step> {
        let usb_steps = if self.usb { USB_STEPS.len() } else { 0 };
        if self.step < usb_steps {
            Some(USB_STEPS[self.step])
        } else {
            COMMON_STEPS.get(self.step - usb_steps).copied()
        }
    }

    pub fn is_done(&self) -> bool {
        self.current().is_none()
    }

    /// The controller sent at least one report since setup started.
    pub fn answered(&self) -> bool {
        self.answered
    }

    /// Writes the current step's report into `buf` (at least
    /// [`OUTPUT_LEN`] bytes). `None` when setup is done.
    pub fn request(&mut self, buf: &mut [u8]) -> Option<usize> {
        match self.current()? {
            Step::Usb { cmd, .. } => {
                buf[..OUTPUT_LEN].fill(0);
                buf[0] = OUT_USB;
                buf[1] = cmd;
                Some(2)
            }
            Step::Sub { id, args, len } => {
                let c = self.counter;
                self.counter = (self.counter + 1) & 0x0F;
                Some(subcommand(c, id, &args[..len], buf))
            }
        }
    }

    /// Whether the current step gets a reply (otherwise just move on).
    pub fn expects_reply(&self) -> bool {
        !matches!(self.current(), Some(Step::Usb { reply: false, .. }))
    }

    /// Moves to the next step without a reply.
    pub fn skip(&mut self) {
        if !self.is_done() {
            self.step += 1;
        }
    }

    /// Handles a report from the controller; returns `true` if it completed
    /// the current step (which then advances).
    pub fn on_report(&mut self, r: &[u8]) -> bool {
        if r.is_empty() {
            return false;
        }
        self.answered = true;
        let done = match self.current() {
            Some(Step::Usb { cmd, .. }) => {
                let ok = r.len() >= 2 && r[0] == REPORT_USB_REPLY && r[1] == cmd;
                if ok && cmd == USB_STATUS {
                    self.mac = parse_usb_status_mac(r);
                }
                ok
            }
            Some(Step::Sub { id, args, .. }) => {
                let ok = r.len() >= 15 && r[0] == REPORT_REPLY && r[14] == id && r[13] & 0x80 != 0;
                if ok && id == SUB_SPI_READ {
                    self.on_spi(&args, r);
                }
                ok
            }
            None => false,
        };
        if done {
            self.step += 1;
        }
        done
    }

    fn on_spi(&mut self, args: &[u8; 5], r: &[u8]) {
        // Reply data: address (4 bytes), size, then the bytes read.
        let size = args[4] as usize;
        if r.len() < 20 + size || r[15..20] != args[..] {
            return;
        }
        let data = &r[20..20 + size];
        let addr = u32::from_le_bytes([args[0], args[1], args[2], args[3]]);
        match addr {
            SPI_FACTORY_STICKS => {
                if let Some(l) = StickRange::from_flash(&data[0..9], true) {
                    self.sticks.left = l;
                }
                if let Some(r) = StickRange::from_flash(&data[9..18], false) {
                    self.sticks.right = r;
                }
            }
            SPI_USER_STICKS => {
                // User calibration wins where present.
                if data[0..2] == USER_CAL_MAGIC {
                    if let Some(l) = StickRange::from_flash(&data[2..11], true) {
                        self.sticks.left = l;
                    }
                }
                if data[11..13] == USER_CAL_MAGIC {
                    if let Some(r) = StickRange::from_flash(&data[13..22], false) {
                        self.sticks.right = r;
                    }
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output::PlayerLeds;

    /// Packs six 12-bit values the way the flash stores them.
    fn pack(v: [u16; 6]) -> [u8; 9] {
        let mut b = [0u8; 9];
        for i in 0..3 {
            let (a, c) = (v[2 * i], v[2 * i + 1]);
            b[3 * i] = (a & 0xFF) as u8;
            b[3 * i + 1] = ((a >> 8) as u8 & 0x0F) | (((c & 0x0F) as u8) << 4);
            b[3 * i + 2] = (c >> 4) as u8;
        }
        b
    }

    fn put_stick(r: &mut [u8], at: usize, x: u16, y: u16) {
        r[at] = (x & 0xFF) as u8;
        r[at + 1] = ((x >> 8) as u8 & 0x0F) | (((y & 0x0F) as u8) << 4);
        r[at + 2] = (y >> 4) as u8;
    }

    /// A `0x21` reply to subcommand `id` with `data`.
    fn reply(id: u8, data: &[u8]) -> Vec<u8> {
        let mut r = vec![0u8; 64];
        r[0] = REPORT_REPLY;
        r[13] = 0x90;
        r[14] = id;
        r[15..15 + data.len()].copy_from_slice(data);
        r
    }

    #[test]
    fn stick_calibration_decoding() {
        // Left: above (1500, 1400), center (2000, 2100), below (1300, 1200).
        let left = pack([1500, 1400, 2000, 2100, 1300, 1200]);
        let r = StickRange::from_flash(&left, true).unwrap();
        assert_eq!((r.center_x, r.center_y), (2000, 2100));
        assert_eq!((r.above_x, r.above_y), (1500, 1400));
        assert_eq!((r.below_x, r.below_y), (1300, 1200));
        // Right: center, below, above.
        let right = pack([1900, 2000, 1350, 1250, 1450, 1550]);
        let r = StickRange::from_flash(&right, false).unwrap();
        assert_eq!((r.center_x, r.center_y), (1900, 2000));
        assert_eq!((r.below_x, r.below_y), (1350, 1250));
        assert_eq!((r.above_x, r.above_y), (1450, 1550));
        // Blank flash or nonsense is ignored.
        assert_eq!(StickRange::from_flash(&[0xFF; 9], true), None);
        assert_eq!(StickRange::from_flash(&[0; 9], true), None);
    }

    #[test]
    fn stick_mapping() {
        let r = StickRange::default();
        assert_eq!(r.map(2048, 2048), (128, 128));
        // Full right and full up (y grows downward after mapping).
        assert_eq!(r.map(2048 + 1400, 2048 + 1400), (255, 0));
        assert_eq!(r.map(2048 - 1400, 2048 - 1400), (0, 255));
        // Beyond the calibrated reach is clamped.
        assert_eq!(r.map(4095, 0), (255, 255));
    }

    #[test]
    fn parses_full_report() {
        let mut r = [0u8; 64];
        r[0] = REPORT_FULL;
        r[1] = 42;
        r[2] = 0x81; // level 8, not charging, powered by cable
        r[3] = 0x04 | 0x80; // B (bottom), ZR
        r[4] = 0x10 | 0x02; // Home, Plus
        r[5] = 0x02 | 0x40; // Up, L
        put_stick(&mut r, 6, 2048 + 1400, 2048);
        put_stick(&mut r, 9, 2048, 2048 - 1400);
        r[37..39].copy_from_slice(&4096i16.to_le_bytes()); // accel x = 1 g
        r[43..45].copy_from_slice(&(-100i16).to_le_bytes()); // gyro x

        let mut s = ControllerState::default();
        assert!(parse(&r, &StickCalibration::default(), &mut s));
        assert!(s.buttons.contains(Buttons::CROSS));
        assert!(!s.buttons.contains(Buttons::CIRCLE));
        assert!(s.buttons.contains(Buttons::R2));
        assert!(s.buttons.contains(Buttons::PS));
        assert!(s.buttons.contains(Buttons::OPTIONS));
        assert!(s.buttons.contains(Buttons::DPAD_UP));
        assert!(s.buttons.contains(Buttons::L1));
        assert_eq!((s.l2, s.r2), (0, 255));
        assert_eq!((s.left_stick.x, s.left_stick.y), (255, 128));
        assert_eq!((s.right_stick.x, s.right_stick.y), (128, 255));
        assert_eq!(s.battery.percent, 100);
        assert_eq!(s.battery.charging, Charging::Full);
        assert!(s.battery.cable);
        assert_eq!(s.motion.accel[0], 4096);
        assert_eq!(s.motion.gyro[0], -100);
        assert_eq!(s.counter, 42);
        assert!(s.full);

        // Released, battery low and charging.
        r[2] = 0x50;
        r[3] = 0;
        assert!(parse(&r, &StickCalibration::default(), &mut s));
        assert!(!s.buttons.contains(Buttons::CROSS));
        assert_eq!(s.battery.percent, 40);
        assert_eq!(s.battery.charging, Charging::Charging);

        // Other reports are not input.
        assert!(!parse(&[0x3F; 12], &StickCalibration::default(), &mut s));
        assert!(!parse(&r[..20], &StickCalibration::default(), &mut s));
    }

    #[test]
    fn rumble_encoding() {
        assert_eq!(encode_rumble(0, 0), RUMBLE_NEUTRAL);
        // Silent amplitudes keep the neutral frequencies.
        let full = encode_rumble(255, 255);
        assert_eq!(full[0], 0x00);
        assert_eq!(full[1], 0xC8 + 0x01); // max high amplitude + frequency bits
        assert_eq!(full[2], 0x40); // step 100 is even: no extra bit
        assert_eq!(full[3], 0x40 + 50);
        // An odd low step sets byte 2 bit 7.
        let low = encode_rumble(200, 0);
        assert_eq!(low[1], 0x01);
        let step = amplitude_step(200.0 / 255.0);
        assert_eq!(step % 2, 1);
        assert_eq!(low[2], 0x40 | 0x80);
        assert_eq!(low[3], (step >> 1) as u8 + 0x40);
    }

    #[test]
    fn player_light_patterns() {
        assert_eq!(player_lights(PlayerLeds::for_player(1).0), 0b0001);
        assert_eq!(player_lights(PlayerLeds::for_player(2).0), 0b0011);
        assert_eq!(player_lights(PlayerLeds::for_player(4).0), 0b1111);
        assert_eq!(player_lights(PlayerLeds::ALL.0), 0b1111);
        assert_eq!(player_lights(0), 0);
    }

    #[test]
    fn output_sends_lights_only_when_they_change() {
        let mut out = SwitchOutput::default();
        let mut buf = [0u8; 64];
        let mut state = OutputState {
            player_leds: PlayerLeds::for_player(2),
            ..OutputState::default()
        };
        assert_eq!(out.build(&state, &mut buf), OUTPUT_LEN);
        assert_eq!(buf[0], OUT_SUBCOMMAND);
        assert_eq!(buf[1], 0);
        assert_eq!(&buf[2..6], &RUMBLE_NEUTRAL);
        assert_eq!(&buf[10..12], &[SUB_PLAYER_LIGHTS, 0b0011]);

        state.rumble_strong = 255;
        out.build(&state, &mut buf);
        assert_eq!(buf[0], OUT_RUMBLE);
        assert_eq!(buf[1], 1);
        assert_eq!(&buf[2..6], &encode_rumble(255, 0));
        assert_eq!(&buf[6..10], &encode_rumble(255, 0));
        assert_eq!(buf[10], 0);
    }

    #[test]
    fn power_off_command() {
        let mut out = SwitchOutput::default();
        let mut buf = [0u8; 64];
        let state = OutputState {
            power_off: true,
            ..OutputState::default()
        };
        out.build(&state, &mut buf);
        assert_eq!(buf[0], OUT_SUBCOMMAND);
        assert_eq!(&buf[2..6], &RUMBLE_NEUTRAL);
        assert_eq!(&buf[10..12], &[SUB_HCI_STATE, 0x00]);
    }

    #[test]
    fn usb_mac() {
        let mut r = [0u8; 64];
        r[..4].copy_from_slice(&[0x81, 0x01, 0x00, 0x03]);
        r[4..10].copy_from_slice(&[0x66, 0x55, 0x44, 0x33, 0x22, 0x11]);
        assert_eq!(parse_usb_status_mac(&r).as_deref(), Some("112233445566"));
        r[1] = 0x02;
        assert_eq!(parse_usb_status_mac(&r), None);
    }

    #[test]
    fn bluetooth_setup_sequence() {
        let mut init = SwitchInit::new(Transport::Bluetooth);
        let mut buf = [0u8; 64];
        assert!(!init.answered());

        // Input mode, IMU, vibration.
        for (id, arg) in [(0x03, 0x30), (0x40, 1), (0x48, 1)] {
            let n = init.request(&mut buf).unwrap();
            assert_eq!(n, OUTPUT_LEN);
            assert_eq!(buf[0], OUT_SUBCOMMAND);
            assert_eq!(&buf[10..12], &[id, arg]);
            // Unrelated reports don't complete the step.
            assert!(!init.on_report(&[REPORT_FULL; 49]));
            assert!(!init.on_report(&reply(0x99, &[])));
            assert!(init.on_report(&reply(id, &[])));
        }
        assert!(init.answered());

        // Factory stick calibration.
        init.request(&mut buf).unwrap();
        assert_eq!(&buf[10..16], &[0x10, 0x3D, 0x60, 0x00, 0x00, 18]);
        let mut data = vec![0x3D, 0x60, 0, 0, 18];
        data.extend(pack([1500, 1500, 2000, 2000, 1500, 1500]));
        data.extend(pack([1900, 1900, 1400, 1400, 1400, 1400]));
        assert!(init.on_report(&reply(0x10, &data)));
        assert_eq!(init.sticks.left.center_x, 2000);
        assert_eq!(init.sticks.right.center_x, 1900);

        // User calibration: only the left stick has one.
        init.request(&mut buf).unwrap();
        assert_eq!(&buf[10..16], &[0x10, 0x10, 0x80, 0x00, 0x00, 22]);
        let mut data = vec![0x10, 0x80, 0, 0, 22, 0xB2, 0xA1];
        data.extend(pack([1500, 1500, 2100, 2100, 1500, 1500]));
        data.extend([0xFF; 11]);
        assert!(init.on_report(&reply(0x10, &data)));
        assert_eq!(init.sticks.left.center_x, 2100);
        assert_eq!(init.sticks.right.center_x, 1900);

        assert!(init.is_done());
        assert_eq!(init.request(&mut buf), None);
    }

    #[test]
    fn usb_setup_sequence() {
        let mut init = SwitchInit::new(Transport::Usb);
        let mut buf = [0u8; 64];
        for cmd in [0x01, 0x02, 0x03, 0x02] {
            assert_eq!(init.request(&mut buf), Some(2));
            assert_eq!(&buf[..2], &[0x80, cmd]);
            assert!(init.expects_reply());
            let mut r = [0u8; 64];
            r[0] = 0x81;
            r[1] = cmd;
            if cmd == 0x01 {
                r[4..10].copy_from_slice(&[6, 5, 4, 3, 2, 1]);
            }
            assert!(init.on_report(&r));
        }
        assert_eq!(init.mac.as_deref(), Some("010203040506"));
        // "USB only" gets no reply.
        assert_eq!(init.request(&mut buf), Some(2));
        assert_eq!(&buf[..2], &[0x80, 0x04]);
        assert!(!init.expects_reply());
        init.skip();
        // Then the same subcommands as Bluetooth.
        init.request(&mut buf).unwrap();
        assert_eq!(&buf[10..12], &[0x03, 0x30]);
        // A controller that never answers can still be skipped through.
        while !init.is_done() {
            init.skip();
        }
        assert_eq!(init.sticks, StickCalibration::default());
    }
}
