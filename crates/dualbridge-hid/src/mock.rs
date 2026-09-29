//! A scriptable HID backend for tests and UI development without hardware.
//!
//! [`MockBackend::add`] plugs in a controller and returns a [`MockController`]
//! handle to push input reports, inspect written output reports, and
//! disconnect or reconnect it. See `tests/manager.rs` for full examples.
//
// The usage example lives in the unit tests below rather than in a doctest:
// rustdoc fails to link doctests that pull in hidapi on Windows (MSVC).

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use dualbridge_core::calibration::{feature_report_id, feature_report_len};
use dualbridge_core::device::{PID_DS4_V2, PID_DUALSENSE, PID_DUALSENSE_EDGE, SONY_VID};
use dualbridge_core::{Model, Transport};

use crate::{DeviceInfo, HidBackend, HidDevice, HidError, HidResult};

#[derive(Default)]
struct MockState {
    connected: bool,
    reports: VecDeque<Vec<u8>>,
    written: Vec<Vec<u8>>,
    features: HashMap<u8, Vec<u8>>,
    open_count: usize,
    max_opens: Option<usize>,
    /// Switched off but still listed, like a Bluetooth controller on Windows:
    /// reads time out and feature requests fail.
    silent: bool,
}

struct MockShared {
    info: DeviceInfo,
    state: Mutex<MockState>,
    cond: Condvar,
}

/// Test-side handle to one mock controller.
#[derive(Clone)]
pub struct MockController {
    shared: Arc<MockShared>,
}

impl MockController {
    pub fn info(&self) -> &DeviceInfo {
        &self.shared.info
    }

    /// Queues an input report for the device to "send".
    pub fn push_report(&self, report: &[u8]) {
        let mut s = self.shared.state.lock().unwrap();
        s.reports.push_back(report.to_vec());
        self.shared.cond.notify_all();
    }

    /// Sets the answer to a feature report request (keyed by `report[0]`).
    pub fn set_feature_report(&self, report: &[u8]) {
        let mut s = self.shared.state.lock().unwrap();
        s.features.insert(report[0], report.to_vec());
    }

    /// Output reports written to the device so far.
    pub fn written(&self) -> Vec<Vec<u8>> {
        self.shared.state.lock().unwrap().written.clone()
    }

    /// Number of times the device was opened.
    pub fn open_count(&self) -> usize {
        self.shared.state.lock().unwrap().open_count
    }

    /// Limits how many handles can be opened (to test the single-handle path).
    pub fn set_max_opens(&self, max: usize) {
        self.shared.state.lock().unwrap().max_opens = Some(max);
    }

    /// Unplugs the device: open handles start failing and enumeration no
    /// longer lists it.
    pub fn disconnect(&self) {
        let mut s = self.shared.state.lock().unwrap();
        s.connected = false;
        self.shared.cond.notify_all();
    }

    /// Plugs the device back in (or switches it back on).
    pub fn reconnect(&self) {
        let mut s = self.shared.state.lock().unwrap();
        s.connected = true;
        s.silent = false;
        s.reports.clear();
    }

    /// Simulates a controller switched off while its device stays listed and
    /// open (what Windows often does with Bluetooth controllers).
    pub fn go_silent(&self) {
        let mut s = self.shared.state.lock().unwrap();
        s.silent = true;
        s.reports.clear();
    }

    /// Waits until at least `n` output reports were written.
    pub fn wait_written(&self, n: usize, timeout: Duration) -> Vec<Vec<u8>> {
        let s = self.shared.state.lock().unwrap();
        let (s, _) = self
            .shared
            .cond
            .wait_timeout_while(s, timeout, |s| s.written.len() < n)
            .unwrap();
        s.written.clone()
    }
}

/// In-memory backend. Clones share the same set of devices.
#[derive(Clone, Default)]
pub struct MockBackend {
    devices: Arc<Mutex<Vec<Arc<MockShared>>>>,
}

impl MockBackend {
    /// Adds a connected controller. `serial` doubles as its identity.
    pub fn add(&self, model: Model, transport: Transport, serial: &str) -> MockController {
        let product_id = match model {
            Model::DualShock4 => PID_DS4_V2,
            Model::DualSense => PID_DUALSENSE,
            Model::DualSenseEdge => PID_DUALSENSE_EDGE,
        };
        let mut devices = self.devices.lock().unwrap();
        let info = DeviceInfo {
            path: format!(
                "mock://{}/{serial}/{}",
                devices.len(),
                transport_name(transport)
            ),
            vendor_id: SONY_VID,
            product_id,
            serial: Some(serial.to_string()),
            model,
            transport,
        };
        // Answer the calibration request like a real controller would.
        let mut calibration = vec![0u8; feature_report_len(model, transport)];
        calibration[0] = feature_report_id(model, transport);
        if transport == Transport::Bluetooth {
            dualbridge_core::crc::sign(dualbridge_core::crc::SEED_FEATURE, &mut calibration);
        }
        let shared = Arc::new(MockShared {
            info,
            state: Mutex::new(MockState {
                connected: true,
                features: HashMap::from([(calibration[0], calibration)]),
                ..MockState::default()
            }),
            cond: Condvar::new(),
        });
        devices.push(shared.clone());
        MockController { shared }
    }
}

fn transport_name(t: Transport) -> &'static str {
    match t {
        Transport::Usb => "usb",
        Transport::Bluetooth => "bt",
    }
}

impl HidBackend for MockBackend {
    fn enumerate(&mut self) -> HidResult<Vec<DeviceInfo>> {
        let devices = self.devices.lock().unwrap();
        Ok(devices
            .iter()
            .filter(|d| d.state.lock().unwrap().connected)
            .map(|d| d.info.clone())
            .collect())
    }

    fn open(&mut self, info: &DeviceInfo) -> HidResult<Box<dyn HidDevice>> {
        let devices = self.devices.lock().unwrap();
        let shared = devices
            .iter()
            .find(|d| d.info.path == info.path)
            .ok_or(HidError::Disconnected)?
            .clone();
        {
            let mut s = shared.state.lock().unwrap();
            if !s.connected {
                return Err(HidError::Disconnected);
            }
            if s.max_opens.is_some_and(|max| s.open_count >= max) {
                return Err(HidError::Other("device busy".into()));
            }
            s.open_count += 1;
        }
        Ok(Box::new(MockDevice { shared }))
    }
}

struct MockDevice {
    shared: Arc<MockShared>,
}

impl HidDevice for MockDevice {
    fn read_timeout(&mut self, buf: &mut [u8], timeout_ms: i32) -> HidResult<usize> {
        let s = self.shared.state.lock().unwrap();
        let timeout = Duration::from_millis(timeout_ms.max(0) as u64);
        let (mut s, _) = self
            .shared
            .cond
            .wait_timeout_while(s, timeout, |s| {
                s.connected && (s.silent || s.reports.is_empty())
            })
            .unwrap();
        if !s.connected {
            return Err(HidError::Disconnected);
        }
        if s.silent {
            return Ok(0);
        }
        match s.reports.pop_front() {
            Some(r) => {
                let n = r.len().min(buf.len());
                buf[..n].copy_from_slice(&r[..n]);
                Ok(n)
            }
            None => Ok(0),
        }
    }

    fn write(&mut self, report: &[u8]) -> HidResult<usize> {
        let mut s = self.shared.state.lock().unwrap();
        if !s.connected {
            return Err(HidError::Disconnected);
        }
        s.written.push(report.to_vec());
        self.shared.cond.notify_all();
        Ok(report.len())
    }

    fn get_feature_report(&mut self, buf: &mut [u8]) -> HidResult<usize> {
        let s = self.shared.state.lock().unwrap();
        if !s.connected {
            return Err(HidError::Disconnected);
        }
        if s.silent {
            return Err(HidError::Other("device does not answer".into()));
        }
        let r = s
            .features
            .get(&buf[0])
            .ok_or_else(|| HidError::Other(format!("no feature report 0x{:02X}", buf[0])))?;
        let n = r.len().min(buf.len());
        buf[..n].copy_from_slice(&r[..n]);
        Ok(n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plug_and_unplug() {
        let mut backend = MockBackend::default();
        let pad = backend.add(Model::DualSense, Transport::Usb, "AA:BB");
        assert_eq!(backend.enumerate().unwrap().len(), 1);
        pad.disconnect();
        assert!(backend.enumerate().unwrap().is_empty());
        pad.reconnect();
        assert_eq!(
            backend.enumerate().unwrap()[0].serial.as_deref(),
            Some("AA:BB")
        );
    }
}
