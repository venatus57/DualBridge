//! Virtual controller backends.
//!
//! Games on Windows mostly speak XInput, so DualBridge exposes each physical
//! controller as a virtual Xbox 360 controller through the ViGEmBus driver
//! ([`vigem`], Windows only). [`hidhide`] hides the physical controller from
//! games so they don't see it twice. [`conflicts`] spots other controller
//! programs that would add their own virtual controllers. [`mock`] records
//! updates for tests.
//!
//! On macOS there is no virtual gamepad backend (it would need an Apple
//! entitlement); [`default_backend`] returns an [`UnsupportedBackend`] there.

use dualbridge_core::mapping::XInputState;
use serde::Serialize;

pub mod conflicts;
pub mod hidhide;
pub mod mock;
#[cfg(windows)]
pub mod vigem;

#[derive(Debug, thiserror::Error)]
pub enum VirtualError {
    #[error("the ViGEmBus driver is not installed")]
    DriverMissing,
    #[error("virtual controllers are not supported on this platform")]
    Unsupported,
    #[error("virtual controller error: {0}")]
    Other(String),
}

/// Can this backend create virtual controllers right now?
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BackendStatus {
    Ready,
    /// The driver needs to be installed (the app can offer to do it).
    DriverMissing,
    /// Not available on this OS.
    Unsupported,
}

/// Called with (large motor, small motor) when a game sets rumble.
pub type RumbleCallback = Box<dyn FnMut(u8, u8) + Send>;

/// One plugged-in virtual controller. Dropping it unplugs it.
pub trait VirtualPad: Send {
    /// Sends a new state to games. Called on the input thread for every
    /// report: implementations must not allocate, log or block on locks.
    fn update(&mut self, state: &XInputState) -> Result<(), VirtualError>;
    /// The XInput player index (0 to 3) Windows assigned, if known.
    fn user_index(&mut self) -> Option<u32> {
        None
    }
}

pub trait VirtualBackend: Send {
    fn status(&mut self) -> BackendStatus;
    /// Plugs in a new virtual Xbox 360 controller.
    fn create_xbox360(
        &mut self,
        rumble: RumbleCallback,
    ) -> Result<Box<dyn VirtualPad>, VirtualError>;
}

/// Backend for platforms without virtual controller support.
#[derive(Debug, Default)]
pub struct UnsupportedBackend;

impl VirtualBackend for UnsupportedBackend {
    fn status(&mut self) -> BackendStatus {
        BackendStatus::Unsupported
    }

    fn create_xbox360(&mut self, _: RumbleCallback) -> Result<Box<dyn VirtualPad>, VirtualError> {
        Err(VirtualError::Unsupported)
    }
}

/// The best backend for this platform.
pub fn default_backend() -> Box<dyn VirtualBackend> {
    #[cfg(windows)]
    {
        Box::new(vigem::VigemBackend::new())
    }
    #[cfg(not(windows))]
    {
        Box::new(UnsupportedBackend)
    }
}
