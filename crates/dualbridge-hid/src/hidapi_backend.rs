//! Real devices, through the `hidapi` crate.

use std::collections::HashMap;
use std::ffi::CString;

use dualbridge_core::device::{normalize_mac, parse_usb_mac, usb_mac_feature_report, SONY_VID};
use dualbridge_core::{Model, Transport};
use hidapi::{BusType, HidApi};

use crate::{DeviceInfo, HidBackend, HidDevice, HidError, HidResult};

pub struct HidapiBackend {
    api: HidApi,
    /// MAC address read from each USB controller (by path), so the same
    /// controller on USB and Bluetooth gets the same identity.
    usb_macs: HashMap<String, Option<String>>,
}

impl HidapiBackend {
    pub fn new() -> HidResult<HidapiBackend> {
        let api = HidApi::new().map_err(other)?;
        Ok(HidapiBackend {
            api,
            usb_macs: HashMap::new(),
        })
    }
}

fn other(e: hidapi::HidError) -> HidError {
    HidError::Other(e.to_string())
}

impl HidBackend for HidapiBackend {
    fn enumerate(&mut self) -> HidResult<Vec<DeviceInfo>> {
        self.api.reset_devices().map_err(other)?;
        self.api.add_devices(SONY_VID, 0).map_err(other)?;
        let mut out: Vec<DeviceInfo> = Vec::new();
        let mut usb_to_identify = Vec::new();
        for d in self.api.device_list() {
            let Some(model) = Model::from_ids(d.vendor_id(), d.product_id()) else {
                continue;
            };
            let transport = match d.bus_type() {
                BusType::Bluetooth => Transport::Bluetooth,
                _ => Transport::Usb,
            };
            let path = d.path().to_string_lossy().into_owned();
            if out.iter().any(|o| o.path == path) {
                continue;
            }
            let hidapi_serial = d.serial_number().map(str::to_owned);
            let serial = match transport {
                // Windows and macOS report the Bluetooth address as serial.
                Transport::Bluetooth => hidapi_serial
                    .as_deref()
                    .and_then(normalize_mac)
                    .or(hidapi_serial),
                // Over USB the serial is something else; read the MAC below.
                Transport::Usb => {
                    usb_to_identify.push(out.len());
                    hidapi_serial
                }
            };
            out.push(DeviceInfo {
                path,
                vendor_id: d.vendor_id(),
                product_id: d.product_id(),
                serial,
                model,
                transport,
            });
        }
        for i in usb_to_identify {
            if let Some(mac) = self.usb_mac(&out[i]) {
                out[i].serial = Some(mac);
            }
        }
        self.usb_macs
            .retain(|path, _| out.iter().any(|d| &d.path == path));
        Ok(out)
    }

    fn open(&mut self, info: &DeviceInfo) -> HidResult<Box<dyn HidDevice>> {
        self.open_device(info)
            .map(|dev| Box::new(HidapiDevice { dev }) as Box<dyn HidDevice>)
    }
}

impl HidapiBackend {
    fn open_device(&self, info: &DeviceInfo) -> HidResult<hidapi::HidDevice> {
        let path = CString::new(info.path.as_str()).map_err(|e| HidError::Other(e.to_string()))?;
        self.api.open_path(&path).map_err(other)
    }

    /// The controller's Bluetooth MAC address, read once per USB device.
    fn usb_mac(&mut self, info: &DeviceInfo) -> Option<String> {
        if let Some(mac) = self.usb_macs.get(&info.path) {
            return mac.clone();
        }
        let mac = self.open_device(info).ok().and_then(|dev| {
            let (id, len) = usb_mac_feature_report(info.model);
            let mut buf = [0u8; 64];
            buf[0] = id;
            let n = dev.get_feature_report(&mut buf[..len]).ok()?;
            parse_usb_mac(info.model, &buf[..n])
        });
        self.usb_macs.insert(info.path.clone(), mac.clone());
        mac
    }
}

struct HidapiDevice {
    dev: hidapi::HidDevice,
}

impl HidDevice for HidapiDevice {
    fn read_timeout(&mut self, buf: &mut [u8], timeout_ms: i32) -> HidResult<usize> {
        self.dev
            .read_timeout(buf, timeout_ms)
            .map_err(|_| HidError::Disconnected)
    }

    fn write(&mut self, report: &[u8]) -> HidResult<usize> {
        self.dev.write(report).map_err(other)
    }

    fn get_feature_report(&mut self, buf: &mut [u8]) -> HidResult<usize> {
        self.dev.get_feature_report(buf).map_err(other)
    }
}
