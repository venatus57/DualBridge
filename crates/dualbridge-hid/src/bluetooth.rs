//! Switching a Bluetooth controller off from the computer.
//!
//! PlayStation controllers have no "power off" command, but they turn
//! themselves off when the computer drops their Bluetooth link. On Windows the
//! link is dropped with `IOCTL_BTH_DISCONNECT_DEVICE` on the Bluetooth radio,
//! which needs no administrator rights. Other platforms are not supported yet.

/// Why a controller could not be switched off.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PowerOffError {
    #[error("the controller's Bluetooth address is unknown")]
    NoAddress,
    #[error("no Bluetooth radio found")]
    NoRadio,
    #[error("Windows refused to disconnect the controller (error {0})")]
    Refused(u32),
    #[error("switching controllers off is only supported on Windows")]
    Unsupported,
}

/// Parses a MAC address (12 hex digits, most significant first, as stored in
/// [`crate::DeviceInfo::serial`]) into the 48-bit value Windows uses.
pub fn parse_address(mac: &str) -> Option<u64> {
    let hex = dualbridge_core::device::normalize_mac(mac)?;
    u64::from_str_radix(&hex, 16).ok()
}

/// Drops the Bluetooth link to the device with this MAC address, on every
/// radio (most computers have one).
pub fn disconnect(mac: &str) -> Result<(), PowerOffError> {
    let addr = parse_address(mac).ok_or(PowerOffError::NoAddress)?;
    #[cfg(windows)]
    {
        windows_impl::disconnect(addr)
    }
    #[cfg(not(windows))]
    {
        let _ = addr;
        Err(PowerOffError::Unsupported)
    }
}

#[cfg(windows)]
mod windows_impl {
    use super::PowerOffError;
    use windows_sys::Win32::Devices::Bluetooth::{
        BluetoothFindFirstRadio, BluetoothFindNextRadio, BluetoothFindRadioClose,
        BLUETOOTH_FIND_RADIO_PARAMS,
    };
    use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, HANDLE};
    use windows_sys::Win32::System::IO::DeviceIoControl;

    /// `CTL_CODE(FILE_DEVICE_BLUETOOTH, 0x03, METHOD_BUFFERED, FILE_ANY_ACCESS)`.
    const IOCTL_BTH_DISCONNECT_DEVICE: u32 = 0x0041_000C;

    pub fn disconnect(addr: u64) -> Result<(), PowerOffError> {
        let params = BLUETOOTH_FIND_RADIO_PARAMS {
            dwSize: std::mem::size_of::<BLUETOOTH_FIND_RADIO_PARAMS>() as u32,
        };
        let mut radio: HANDLE = std::ptr::null_mut();
        // SAFETY: `params` and `radio` are valid for the calls; every radio
        // handle returned is closed, and so is the search handle.
        unsafe {
            let find = BluetoothFindFirstRadio(&params, &mut radio);
            if find.is_null() {
                return Err(PowerOffError::NoRadio);
            }
            let mut result = Err(PowerOffError::NoRadio);
            loop {
                let mut returned = 0u32;
                let ok = DeviceIoControl(
                    radio,
                    IOCTL_BTH_DISCONNECT_DEVICE,
                    &addr as *const u64 as *const _,
                    std::mem::size_of::<u64>() as u32,
                    std::ptr::null_mut(),
                    0,
                    &mut returned,
                    std::ptr::null_mut(),
                );
                if ok != 0 {
                    result = Ok(());
                } else if result.is_err() {
                    result = Err(PowerOffError::Refused(GetLastError()));
                }
                CloseHandle(radio);
                if result.is_ok() || BluetoothFindNextRadio(find, &mut radio) == 0 {
                    break;
                }
            }
            BluetoothFindRadioClose(find);
            result
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_addresses() {
        assert_eq!(parse_address("a4ae12345678"), Some(0xA4AE_1234_5678));
        assert_eq!(parse_address("A4:AE:12:34:56:78"), Some(0xA4AE_1234_5678));
        assert_eq!(parse_address("mock://0/a/bt"), None);
        assert_eq!(disconnect("nope"), Err(PowerOffError::NoAddress));
    }
}
