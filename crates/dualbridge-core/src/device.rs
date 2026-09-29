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
}
