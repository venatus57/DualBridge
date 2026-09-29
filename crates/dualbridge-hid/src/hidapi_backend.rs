//! Real devices, through the `hidapi` crate.

use std::ffi::CString;

use dualbridge_core::device::SONY_VID;
use dualbridge_core::{Model, Transport};
use hidapi::{BusType, HidApi};

use crate::{DeviceInfo, HidBackend, HidDevice, HidError, HidResult};

pub struct HidapiBackend {
    api: HidApi,
}

impl HidapiBackend {
    pub fn new() -> HidResult<HidapiBackend> {
        let api = HidApi::new().map_err(other)?;
        Ok(HidapiBackend { api })
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
            out.push(DeviceInfo {
                path,
                vendor_id: d.vendor_id(),
                product_id: d.product_id(),
                serial: d.serial_number().map(str::to_owned),
                model,
                transport,
            });
        }
        Ok(out)
    }

    fn open(&mut self, info: &DeviceInfo) -> HidResult<Box<dyn HidDevice>> {
        let path = CString::new(info.path.as_str()).map_err(|e| HidError::Other(e.to_string()))?;
        let dev = self.api.open_path(&path).map_err(other)?;
        Ok(Box::new(HidapiDevice { dev }))
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
