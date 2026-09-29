//! Controller discovery, hotplug and per-controller I/O threads.
//!
//! - [`HidBackend`] / [`HidDevice`]: the small slice of HID we need, with a
//!   real implementation on top of `hidapi` ([`hidapi_backend`]) and a
//!   scriptable one for tests ([`mock`]).
//! - [`manager::ControllerManager`]: up to [`manager::MAX_SLOTS`] controllers,
//!   stable slot assignment, hotplug.
//! - [`runtime`]: the latency-critical input thread and the output writer
//!   thread of one controller.

use dualbridge_core::{Model, Transport};
use serde::Serialize;

#[cfg(feature = "hidapi")]
pub mod hidapi_backend;
pub mod latency;
pub mod manager;
pub mod mock;
mod priority;
pub mod runtime;

/// A controller found during enumeration.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct DeviceInfo {
    /// Platform-specific path, unique per connected device.
    pub path: String,
    pub vendor_id: u16,
    pub product_id: u16,
    /// Serial number (the Bluetooth MAC address on most platforms), if known.
    pub serial: Option<String>,
    pub model: Model,
    pub transport: Transport,
}

impl DeviceInfo {
    /// Key used to recognize the same controller across reconnects.
    pub fn identity(&self) -> &str {
        self.serial
            .as_deref()
            .filter(|s| !s.is_empty())
            .unwrap_or(&self.path)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum HidError {
    #[error("device disconnected")]
    Disconnected,
    #[error("HID error: {0}")]
    Other(String),
}

pub type HidResult<T> = Result<T, HidError>;

/// An open HID device.
pub trait HidDevice: Send {
    /// Reads one input report. Returns `Ok(0)` on timeout.
    fn read_timeout(&mut self, buf: &mut [u8], timeout_ms: i32) -> HidResult<usize>;
    /// Writes one output report (starting with its report ID).
    fn write(&mut self, report: &[u8]) -> HidResult<usize>;
    /// Reads a feature report. `buf[0]` must hold the report ID.
    fn get_feature_report(&mut self, buf: &mut [u8]) -> HidResult<usize>;
}

/// A source of devices.
pub trait HidBackend: Send {
    /// Lists every supported controller currently connected.
    fn enumerate(&mut self) -> HidResult<Vec<DeviceInfo>>;
    /// Opens a device. May be called twice for the same device, to get
    /// separate handles for reading and writing.
    fn open(&mut self, info: &DeviceInfo) -> HidResult<Box<dyn HidDevice>>;
}
