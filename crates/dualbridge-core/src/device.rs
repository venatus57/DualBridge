//! Controller models, transports, and USB identifiers.

use serde::{Deserialize, Serialize};

/// Sony's USB vendor ID.
pub const SONY_VID: u16 = 0x054C;
/// DualShock 4, first revision (CUH-ZCT1).
pub const PID_DS4_V1: u16 = 0x05C4;
/// DualShock 4, second revision (CUH-ZCT2).
pub const PID_DS4_V2: u16 = 0x09CC;
/// Sony wireless adapter for the DualShock 4.
pub const PID_DS4_DONGLE: u16 = 0x0BA0;
/// DualSense.
pub const PID_DUALSENSE: u16 = 0x0CE6;
/// DualSense Edge.
pub const PID_DUALSENSE_EDGE: u16 = 0x0DF2;

/// A supported controller family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Model {
    DualShock4,
    DualSense,
    DualSenseEdge,
}

impl Model {
    /// Identifies a controller from its USB vendor and product IDs.
    pub fn from_ids(vendor_id: u16, product_id: u16) -> Option<Model> {
        if vendor_id != SONY_VID {
            return None;
        }
        match product_id {
            PID_DS4_V1 | PID_DS4_V2 | PID_DS4_DONGLE => Some(Model::DualShock4),
            PID_DUALSENSE => Some(Model::DualSense),
            PID_DUALSENSE_EDGE => Some(Model::DualSenseEdge),
            _ => None,
        }
    }

    /// `true` for the DualSense and DualSense Edge, which share one protocol.
    pub fn is_dualsense(self) -> bool {
        matches!(self, Model::DualSense | Model::DualSenseEdge)
    }

    /// Human-readable product name.
    pub fn display_name(self) -> &'static str {
        match self {
            Model::DualShock4 => "DualShock 4",
            Model::DualSense => "DualSense",
            Model::DualSenseEdge => "DualSense Edge",
        }
    }

    /// Touchpad resolution (width, height) in touch units.
    pub fn touchpad_size(self) -> (u16, u16) {
        match self {
            Model::DualShock4 => (1920, 942),
            Model::DualSense | Model::DualSenseEdge => (1920, 1080),
        }
    }

    /// Size of the full input report for this model on this transport.
    pub fn input_report_len(self, transport: Transport) -> usize {
        match (self, transport) {
            (_, Transport::Usb) => 64,
            (_, Transport::Bluetooth) => 78,
        }
    }
}

/// Feature report (ID, length) holding the controller's Bluetooth MAC
/// address when it is connected over USB.
pub fn usb_mac_feature_report(model: Model) -> (u8, usize) {
    match model {
        Model::DualShock4 => (0x12, 16),
        Model::DualSense | Model::DualSenseEdge => (0x09, 20),
    }
}

/// Extracts the MAC address from the USB MAC feature report (bytes 1 to 6,
/// least significant first), as 12 lowercase hex digits.
pub fn parse_usb_mac(model: Model, report: &[u8]) -> Option<String> {
    let (id, _) = usb_mac_feature_report(model);
    if report.first() != Some(&id) || report.len() < 7 {
        return None;
    }
    let mac = &report[1..7];
    if mac.iter().all(|&b| b == 0) || mac.iter().all(|&b| b == 0xFF) {
        return None;
    }
    Some(mac.iter().rev().map(|b| format!("{b:02x}")).collect())
}

/// Normalizes a MAC address written in any common form (`A4:AE:12:34:56:78`,
/// `a4ae12345678`, `a4-ae-...`) to 12 lowercase hex digits.
pub fn normalize_mac(s: &str) -> Option<String> {
    let hex: String = s
        .chars()
        .filter(|c| c.is_ascii_hexdigit())
        .map(|c| c.to_ascii_lowercase())
        .collect();
    let has_only_mac_chars = s
        .chars()
        .all(|c| c.is_ascii_hexdigit() || matches!(c, ':' | '-' | ' '));
    (hex.len() == 12 && has_only_mac_chars).then_some(hex)
}

/// How the controller is connected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Transport {
    Usb,
    Bluetooth,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifies_models() {
        assert_eq!(
            Model::from_ids(SONY_VID, PID_DS4_V1),
            Some(Model::DualShock4)
        );
        assert_eq!(
            Model::from_ids(SONY_VID, PID_DS4_V2),
            Some(Model::DualShock4)
        );
        assert_eq!(
            Model::from_ids(SONY_VID, PID_DUALSENSE),
            Some(Model::DualSense)
        );
        assert_eq!(
            Model::from_ids(SONY_VID, PID_DUALSENSE_EDGE),
            Some(Model::DualSenseEdge)
        );
        assert_eq!(Model::from_ids(0x045E, PID_DUALSENSE), None);
        assert_eq!(Model::from_ids(SONY_VID, 0x1234), None);
    }

    #[test]
    fn usb_and_bluetooth_macs_match() {
        // USB report: MAC stored least significant byte first.
        let mut r = [0u8; 20];
        r[0] = 0x09;
        r[1..7].copy_from_slice(&[0x78, 0x56, 0x34, 0x12, 0xAE, 0xA4]);
        let usb = parse_usb_mac(Model::DualSense, &r).unwrap();
        assert_eq!(usb, "a4ae12345678");
        // Bluetooth serial as Windows or macOS report it.
        assert_eq!(
            normalize_mac("A4:AE:12:34:56:78").as_deref(),
            Some(usb.as_str())
        );
        assert_eq!(normalize_mac("a4ae12345678").as_deref(), Some(usb.as_str()));
        assert_eq!(
            normalize_mac("a4-ae-12-34-56-78").as_deref(),
            Some(usb.as_str())
        );
        assert_eq!(normalize_mac("not a mac"), None);
        assert_eq!(normalize_mac("1234"), None);

        let mut ds4 = [0u8; 16];
        ds4[0] = 0x12;
        ds4[1..7].copy_from_slice(&[1, 2, 3, 4, 5, 6]);
        assert_eq!(
            parse_usb_mac(Model::DualShock4, &ds4).as_deref(),
            Some("060504030201")
        );
        ds4[0] = 0x09;
        assert_eq!(parse_usb_mac(Model::DualShock4, &ds4), None);
        assert_eq!(
            parse_usb_mac(Model::DualSense, &[0x09, 0, 0, 0, 0, 0, 0]),
            None
        );
    }
}
