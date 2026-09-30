//! Platform-independent core of DualBridge.
//!
//! Everything in this crate is pure Rust with no OS dependencies, so it can be
//! unit-tested anywhere (including CI machines with no controller attached).
//!
//! - [`device`]: controller models, transports and USB/Bluetooth identifiers
//! - [`state`]: the normalized [`ControllerState`] every parser fills in
//! - [`input`]: input report parsing for DualShock 4 and DualSense
//! - [`output`]: output report building (lightbar, rumble, LEDs, adaptive triggers)
//! - [`lighting`]: lightbar / player LED effects engine
//! - [`mapping`]: controller state to virtual Xbox controller
//! - [`profile`]: saved per-controller / per-game settings
//! - [`calibration`]: motion sensor calibration from feature reports
//! - [`crc`]: the CRC32 used by Bluetooth reports
//! - [`switch`]: the Nintendo Switch Pro Controller protocol
//!
//! The parsing and building functions never allocate: they work on caller
//! provided buffers so they can run on the latency-critical input thread.

pub mod calibration;
pub mod color;
pub mod crc;
pub mod device;
pub mod input;
pub mod lighting;
pub mod mapping;
pub mod output;
pub mod profile;
pub mod state;
pub mod switch;

pub use color::Rgb;
pub use device::{Model, Transport};
pub use state::ControllerState;
