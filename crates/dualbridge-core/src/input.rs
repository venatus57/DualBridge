//! Input report parsing for the DualShock 4 and the DualSense.
//!
//! Report layouts (offsets are relative to the first data byte, after the
//! report ID and any Bluetooth header):
//!
//! | Report                         | Data starts at | Length |
//! |--------------------------------|----------------|--------|
//! | DS4 USB `0x01`                 | 1              | 64     |
//! | DS4 Bluetooth `0x11`           | 3              | 78     |
//! | DualSense USB `0x01`           | 1              | 64     |
//! | DualSense Bluetooth `0x31`     | 2              | 78     |
//! | Bluetooth "simple" `0x01` (both) | 1            | 10     |
//!
//! Bluetooth controllers start in "simple" mode, which only carries sticks,
//! buttons and triggers. Reading a calibration feature report switches them to
//! full reports; that is done by the device layer.
//!
//! Nothing here allocates, so it is safe to call from the input thread.

use crate::crc;
use crate::device::{Model, Transport};
use crate::state::{Battery, Buttons, Charging, ControllerState, Touch};

/// DS4 USB input report, and the "simple" Bluetooth report of both models.
pub const REPORT_BASIC: u8 = 0x01;
/// DS4 full Bluetooth input report.
pub const REPORT_DS4_BT: u8 = 0x11;
/// DualSense full Bluetooth input report.
pub const REPORT_DUALSENSE_BT: u8 = 0x31;

const BT_REPORT_LEN: usize = 78;
const USB_REPORT_LEN: usize = 64;
const SIMPLE_REPORT_LEN: usize = 10;

/// Why a report could not be parsed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ParseError {
    #[error("report is empty")]
    Empty,
    #[error("report 0x{id:02X} is too short: {got} bytes, expected at least {expected}")]
    TooShort { id: u8, expected: usize, got: usize },
    #[error("unexpected report ID 0x{0:02X}")]
    UnknownReport(u8),
    #[error("Bluetooth report CRC mismatch")]
    BadCrc,
}

/// Parses input reports for one controller.
#[derive(Debug, Clone, Copy)]
pub struct InputParser {
    pub model: Model,
    pub transport: Transport,
    /// Check the CRC32 of full Bluetooth reports and reject corrupted ones.
    pub verify_crc: bool,
}

impl InputParser {
    pub fn new(model: Model, transport: Transport) -> InputParser {
        InputParser {
            model,
            transport,
            verify_crc: true,
        }
    }

    /// Parses `report` (starting with the report ID) into `state`.
    ///
    /// On error `state` is left unchanged. Fields a report does not carry
    /// (for example motion data in simple Bluetooth reports) keep their
    /// previous value.
    pub fn parse(&self, report: &[u8], state: &mut ControllerState) -> Result<(), ParseError> {
        let id = *report.first().ok_or(ParseError::Empty)?;
        match (self.model.is_dualsense(), self.transport, id) {
            (false, Transport::Usb, REPORT_BASIC) => {
                let r = require(report, USB_REPORT_LEN)?;
                parse_ds4_full(&r[1..], state);
            }
            (false, Transport::Bluetooth, REPORT_DS4_BT) => {
                let r = require(report, BT_REPORT_LEN)?;
                self.check_crc(r)?;
                parse_ds4_full(&r[3..], state);
            }
            (true, Transport::Usb, REPORT_BASIC) => {
                let r = require(report, USB_REPORT_LEN)?;
                parse_dualsense_full(&r[1..], self.model, state);
            }
            (true, Transport::Bluetooth, REPORT_DUALSENSE_BT) => {
                let r = require(report, BT_REPORT_LEN)?;
                self.check_crc(r)?;
                parse_dualsense_full(&r[2..], self.model, state);
            }
            (_, Transport::Bluetooth, REPORT_BASIC) => {
                let r = require(report, SIMPLE_REPORT_LEN)?;
                parse_simple(&r[1..], state);
            }
            _ => return Err(ParseError::UnknownReport(id)),
        }
        Ok(())
    }

    fn check_crc(&self, report: &[u8]) -> Result<(), ParseError> {
        if self.verify_crc && !crc::verify(crc::SEED_INPUT, report) {
            return Err(ParseError::BadCrc);
        }
        Ok(())
    }
}

/// Returns the first `len` bytes of `report`, or an error if it is shorter.
/// Some platforms pad reports, so longer buffers are accepted and trimmed.
#[inline]
fn require(report: &[u8], len: usize) -> Result<&[u8], ParseError> {
    report.get(..len).ok_or(ParseError::TooShort {
        id: report[0],
        expected: len,
        got: report.len(),
    })
}

#[inline]
fn i16_at(d: &[u8], i: usize) -> i16 {
    i16::from_le_bytes([d[i], d[i + 1]])
}

/// Face buttons and hat switch, shared by every layout.
#[inline]
fn face_and_hat(b: u8, buttons: &mut Buttons) {
    buttons.set_hat(b & 0x0F);
    buttons.set(Buttons::SQUARE, b & 0x10 != 0);
    buttons.set(Buttons::CROSS, b & 0x20 != 0);
    buttons.set(Buttons::CIRCLE, b & 0x40 != 0);
    buttons.set(Buttons::TRIANGLE, b & 0x80 != 0);
}

/// Shoulders, Share/Create, Options and stick clicks, shared by every layout.
#[inline]
fn shoulders(b: u8, buttons: &mut Buttons) {
    buttons.set(Buttons::L1, b & 0x01 != 0);
    buttons.set(Buttons::R1, b & 0x02 != 0);
    buttons.set(Buttons::L2, b & 0x04 != 0);
    buttons.set(Buttons::R2, b & 0x08 != 0);
    buttons.set(Buttons::SHARE, b & 0x10 != 0);
    buttons.set(Buttons::OPTIONS, b & 0x20 != 0);
    buttons.set(Buttons::L3, b & 0x40 != 0);
    buttons.set(Buttons::R3, b & 0x80 != 0);
}

/// Touch point: `[active/id, x low, x high | y low, y high]`, 12 bits per axis.
#[inline]
fn touch(d: &[u8]) -> Touch {
    Touch {
        active: d[0] & 0x80 == 0,
        id: d[0] & 0x7F,
        x: d[1] as u16 | ((d[2] as u16 & 0x0F) << 8),
        y: (d[2] as u16 >> 4) | ((d[3] as u16) << 4),
    }
}

/// Bluetooth simple report (both models), `d` starts after the report ID.
fn parse_simple(d: &[u8], s: &mut ControllerState) {
    s.left_stick.x = d[0];
    s.left_stick.y = d[1];
    s.right_stick.x = d[2];
    s.right_stick.y = d[3];
    face_and_hat(d[4], &mut s.buttons);
    shoulders(d[5], &mut s.buttons);
    s.buttons.set(Buttons::PS, d[6] & 0x01 != 0);
    s.buttons.set(Buttons::TOUCHPAD, d[6] & 0x02 != 0);
    s.counter = d[6] >> 2;
    s.l2 = d[7];
    s.r2 = d[8];
    s.full = false;
}

/// DS4 full report, `d` starts at the left stick X byte.
fn parse_ds4_full(d: &[u8], s: &mut ControllerState) {
    parse_simple(d, s);
    // d[9..11]: 16-bit timestamp in units of ~5.33 us.
    s.motion.timestamp = u16::from_le_bytes([d[9], d[10]]) as u32;
    s.motion.gyro = [i16_at(d, 12), i16_at(d, 14), i16_at(d, 16)];
    s.motion.accel = [i16_at(d, 18), i16_at(d, 20), i16_at(d, 22)];

    let status = d[29];
    let level = status & 0x0F;
    let cable = status & 0x10 != 0;
    s.headphones = status & 0x20 != 0;
    s.battery = if cable {
        // While on a cable the level goes 0..=11, 11 meaning fully charged.
        if level >= 11 {
            Battery {
                percent: 100,
                charging: Charging::Full,
                cable,
            }
        } else {
            Battery {
                percent: (level * 10).min(100),
                charging: Charging::Charging,
                cable,
            }
        }
    } else {
        Battery {
            percent: (level * 10 + 5).min(100),
            charging: Charging::Discharging,
            cable,
        }
    };

    // d[32]: number of touch packets; the first one is at d[33..42].
    s.touch = [touch(&d[34..38]), touch(&d[38..42])];
    s.trigger_feedback = [0, 0];
    s.full = true;
}

/// DualSense full report, `d` starts at the left stick X byte.
fn parse_dualsense_full(d: &[u8], model: Model, s: &mut ControllerState) {
    s.left_stick.x = d[0];
    s.left_stick.y = d[1];
    s.right_stick.x = d[2];
    s.right_stick.y = d[3];
    s.l2 = d[4];
    s.r2 = d[5];
    s.counter = d[6];
    face_and_hat(d[7], &mut s.buttons);
    shoulders(d[8], &mut s.buttons);
    let b = d[9];
    s.buttons.set(Buttons::PS, b & 0x01 != 0);
    s.buttons.set(Buttons::TOUCHPAD, b & 0x02 != 0);
    s.buttons.set(Buttons::MUTE, b & 0x04 != 0);
    let edge = model == Model::DualSenseEdge;
    s.buttons.set(Buttons::LEFT_FN, edge && b & 0x10 != 0);
    s.buttons.set(Buttons::RIGHT_FN, edge && b & 0x20 != 0);
    s.buttons.set(Buttons::LEFT_PADDLE, edge && b & 0x40 != 0);
    s.buttons.set(Buttons::RIGHT_PADDLE, edge && b & 0x80 != 0);

    s.motion.gyro = [i16_at(d, 15), i16_at(d, 17), i16_at(d, 19)];
    s.motion.accel = [i16_at(d, 21), i16_at(d, 23), i16_at(d, 25)];
    s.motion.timestamp = u32::from_le_bytes([d[27], d[28], d[29], d[30]]);
    s.touch = [touch(&d[32..36]), touch(&d[36..40])];
    s.trigger_feedback = [d[42], d[41]];

    let status = d[52];
    let level = status & 0x0F;
    let charging = match status >> 4 {
        0x0 => Charging::Discharging,
        0x1 => Charging::Charging,
        0x2 => Charging::Full,
        _ => Charging::Error,
    };
    let percent = if charging == Charging::Full {
        100
    } else {
        (level * 10 + 5).min(100)
    };
    let plugs = d[53];
    s.headphones = plugs & 0x01 != 0;
    s.battery = Battery {
        percent,
        charging,
        cable: plugs & 0x18 != 0 || charging != Charging::Discharging,
    };
    s.full = true;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_bad_input() {
        let p = InputParser::new(Model::DualShock4, Transport::Usb);
        let mut s = ControllerState::default();
        assert_eq!(p.parse(&[], &mut s), Err(ParseError::Empty));
        assert_eq!(
            p.parse(&[0x01, 0, 0], &mut s),
            Err(ParseError::TooShort {
                id: 0x01,
                expected: 64,
                got: 3
            })
        );
        assert_eq!(
            p.parse(&[0x31; 78], &mut s),
            Err(ParseError::UnknownReport(0x31))
        );
    }

    #[test]
    fn touch_decoding() {
        // x = 0x789, y = 0x3AB, id 5, active.
        let t = touch(&[0x05, 0x89, 0xB7, 0x3A]);
        assert!(t.active);
        assert_eq!(t.id, 5);
        assert_eq!(t.x, 0x789);
        assert_eq!(t.y, 0x3AB);
        assert!(!touch(&[0x80, 0, 0, 0]).active);
    }
}
