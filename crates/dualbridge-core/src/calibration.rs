//! Motion sensor calibration.
//!
//! Both controllers store factory calibration in a feature report:
//!
//! | Controller  | USB report | Bluetooth report |
//! |-------------|------------|------------------|
//! | DualShock 4 | `0x02`     | `0x05` (+ CRC32) |
//! | DualSense   | `0x05`     | `0x05` (+ CRC32) |
//!
//! After the report ID it holds 17 little-endian `i16` values: three gyro
//! biases, six gyro readings taken at a known positive and negative speed,
//! that speed (plus and minus), and six accelerometer readings taken at +1 g
//! and -1 g on each axis. The order of the six gyro readings differs between
//! DS4 USB (all "plus" first) and the others (plus/minus pairs per axis).

use serde::{Deserialize, Serialize};

use crate::crc;
use crate::device::{Model, Transport};
use crate::state::Motion;
use crate::switch::StickCalibration;

pub const FEATURE_DS4_USB: u8 = 0x02;
pub const FEATURE_DS4_BT: u8 = 0x05;
pub const FEATURE_DUALSENSE: u8 = 0x05;

/// Feature report ID holding the calibration for a model and transport.
pub fn feature_report_id(model: Model, transport: Transport) -> u8 {
    match (model, transport) {
        (Model::DualShock4, Transport::Usb) => FEATURE_DS4_USB,
        (Model::DualShock4, Transport::Bluetooth) => FEATURE_DS4_BT,
        _ => FEATURE_DUALSENSE,
    }
}

/// Buffer size to request when reading the calibration feature report.
pub fn feature_report_len(model: Model, transport: Transport) -> usize {
    match (model, transport) {
        (Model::DualShock4, Transport::Usb) => 37,
        _ => 41,
    }
}

/// Converts one raw axis to physical units: `(raw - bias) * scale`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AxisCalibration {
    pub bias: f32,
    pub scale: f32,
}

impl AxisCalibration {
    #[inline]
    pub fn apply(self, raw: i16) -> f32 {
        (raw as f32 - self.bias) * self.scale
    }
}

/// Calibration for the six motion axes.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Calibration {
    /// Raw to degrees per second: pitch, yaw, roll.
    pub gyro: [AxisCalibration; 3],
    /// Raw to g: x, y, z.
    pub accel: [AxisCalibration; 3],
    /// Stick centers and ranges (Switch Pro Controller only).
    #[serde(default)]
    pub sticks: StickCalibration,
}

/// Nominal sensitivities, used when a controller reports no usable calibration.
const DEFAULT_GYRO_PER_DEG_S: f32 = 1024.0 / 64.0;
const DEFAULT_ACCEL_PER_G: f32 = 8192.0;

impl Default for Calibration {
    fn default() -> Self {
        let gyro = AxisCalibration {
            bias: 0.0,
            scale: 1.0 / DEFAULT_GYRO_PER_DEG_S,
        };
        let accel = AxisCalibration {
            bias: 0.0,
            scale: 1.0 / DEFAULT_ACCEL_PER_G,
        };
        Calibration {
            gyro: [gyro; 3],
            accel: [accel; 3],
            sticks: StickCalibration::default(),
        }
    }
}

impl Calibration {
    /// Nominal motion calibration of the Switch Pro Controller (±2000 deg/s
    /// gyro, ±8 g accelerometer) with the given stick calibration.
    pub fn switch(sticks: StickCalibration) -> Calibration {
        let axis = |scale| AxisCalibration { bias: 0.0, scale };
        Calibration {
            gyro: [axis(0.061); 3],
            accel: [axis(1.0 / 4096.0); 3],
            sticks,
        }
    }
}

/// Motion in physical units.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct CalibratedMotion {
    /// Degrees per second: pitch, yaw, roll.
    pub gyro: [f32; 3],
    /// In g: x, y, z.
    pub accel: [f32; 3],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum CalibrationError {
    #[error("calibration report too short: {0} bytes")]
    TooShort(usize),
    #[error("unexpected calibration report ID 0x{0:02X}")]
    WrongReport(u8),
    #[error("calibration report CRC mismatch")]
    BadCrc,
}

impl Calibration {
    /// Parses the calibration feature report (starting with its report ID).
    ///
    /// Axes with degenerate values (a zero range, which some third-party
    /// controllers report) fall back to nominal sensitivities instead of
    /// failing the whole report.
    pub fn from_feature_report(
        model: Model,
        transport: Transport,
        report: &[u8],
    ) -> Result<Calibration, CalibrationError> {
        let len = feature_report_len(model, transport);
        let report = report
            .get(..len)
            .ok_or(CalibrationError::TooShort(report.len()))?;
        let id = feature_report_id(model, transport);
        if report[0] != id {
            return Err(CalibrationError::WrongReport(report[0]));
        }
        if transport == Transport::Bluetooth && !crc::verify(crc::SEED_FEATURE, report) {
            return Err(CalibrationError::BadCrc);
        }

        let v = |i: usize| i16::from_le_bytes([report[1 + 2 * i], report[2 + 2 * i]]) as f32;
        let bias = [v(0), v(1), v(2)];
        let (plus, minus) = if model == Model::DualShock4 && transport == Transport::Usb {
            ([v(3), v(4), v(5)], [v(6), v(7), v(8)])
        } else {
            ([v(3), v(5), v(7)], [v(4), v(6), v(8)])
        };
        let speed_2x = v(9) + v(10);

        let defaults = Calibration::default();
        let mut cal = defaults;
        for axis in 0..3 {
            let denom = (plus[axis] - bias[axis]).abs() + (minus[axis] - bias[axis]).abs();
            if denom > 0.0 && speed_2x > 0.0 {
                cal.gyro[axis] = AxisCalibration {
                    bias: bias[axis],
                    scale: speed_2x / denom,
                };
            }
            let acc_plus = v(11 + 2 * axis);
            let acc_minus = v(12 + 2 * axis);
            let range = acc_plus - acc_minus;
            if range > 0.0 {
                cal.accel[axis] = AxisCalibration {
                    bias: acc_plus - range / 2.0,
                    scale: 2.0 / range,
                };
            }
        }
        Ok(cal)
    }

    /// Converts raw motion readings to physical units.
    #[inline]
    pub fn apply(&self, m: &Motion) -> CalibratedMotion {
        CalibratedMotion {
            gyro: [
                self.gyro[0].apply(m.gyro[0]),
                self.gyro[1].apply(m.gyro[1]),
                self.gyro[2].apply(m.gyro[2]),
            ],
            accel: [
                self.accel[0].apply(m.accel[0]),
                self.accel[1].apply(m.accel[1]),
                self.accel[2].apply(m.accel[2]),
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report(id: u8, len: usize, values: [i16; 17]) -> Vec<u8> {
        let mut r = vec![0u8; len];
        r[0] = id;
        for (i, v) in values.iter().enumerate() {
            r[1 + 2 * i..3 + 2 * i].copy_from_slice(&v.to_le_bytes());
        }
        r
    }

    #[test]
    fn ds4_usb_layout() {
        // Biases 10/20/30; +/-1000 counts from bias at +/-500 deg/s.
        let r = report(
            FEATURE_DS4_USB,
            37,
            [
                10, 20, 30, // bias
                1010, 1020, 1030, // plus (pitch, yaw, roll)
                -990, -980, -970, // minus
                500, 500, // speed
                8200, -8184, 8192, -8192, 8000, -8000, // accel
            ],
        );
        let cal = Calibration::from_feature_report(Model::DualShock4, Transport::Usb, &r).unwrap();
        for axis in 0..3 {
            assert!((cal.gyro[axis].scale - 0.5).abs() < 1e-6);
        }
        assert_eq!(cal.gyro[1].bias, 20.0);
        let m = Motion {
            gyro: [210, 20, -170],
            accel: [8200, 0, -8000],
            timestamp: 0,
        };
        let c = cal.apply(&m);
        assert!((c.gyro[0] - 100.0).abs() < 1e-3);
        assert!(c.gyro[1].abs() < 1e-3);
        assert!((c.gyro[2] + 100.0).abs() < 1e-3);
        assert!((c.accel[0] - 1.0).abs() < 1e-3);
        assert!(c.accel[1].abs() < 1e-3);
        assert!((c.accel[2] + 1.0).abs() < 1e-3);
    }

    #[test]
    fn paired_layout_with_crc() {
        let mut r = report(
            FEATURE_DUALSENSE,
            41,
            [
                0, 0, 0, // bias
                2000, -2000, // pitch +/-
                1000, -1000, // yaw +/-
                500, -500, // roll +/-
                1000, 1000, // speed
                8192, -8192, 8192, -8192, 8192, -8192,
            ],
        );
        crc::sign(crc::SEED_FEATURE, &mut r);
        let cal =
            Calibration::from_feature_report(Model::DualSense, Transport::Bluetooth, &r).unwrap();
        assert!((cal.gyro[0].scale - 0.5).abs() < 1e-6);
        assert!((cal.gyro[1].scale - 1.0).abs() < 1e-6);
        assert!((cal.gyro[2].scale - 2.0).abs() < 1e-6);

        r[5] ^= 0xFF;
        assert_eq!(
            Calibration::from_feature_report(Model::DualSense, Transport::Bluetooth, &r),
            Err(CalibrationError::BadCrc)
        );
    }

    #[test]
    fn degenerate_values_fall_back() {
        let r = report(FEATURE_DUALSENSE, 41, [0; 17]);
        let cal = Calibration::from_feature_report(Model::DualSense, Transport::Usb, &r).unwrap();
        assert_eq!(cal, Calibration::default());
    }

    #[test]
    fn wrong_report() {
        let r = report(0x09, 41, [0; 17]);
        assert_eq!(
            Calibration::from_feature_report(Model::DualSense, Transport::Usb, &r),
            Err(CalibrationError::WrongReport(0x09))
        );
        assert_eq!(
            Calibration::from_feature_report(Model::DualSense, Transport::Usb, &r[..5]),
            Err(CalibrationError::TooShort(5))
        );
    }
}
