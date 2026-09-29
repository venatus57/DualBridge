//! HidHide integration ("exclusive mode").
//!
//! With a virtual Xbox controller plugged in, games that also understand
//! PlayStation controllers would see the player twice. HidHide is a filter
//! driver that hides chosen devices from every application except an allow
//! list; DualBridge adds itself to the list and hides the physical controllers.
//!
//! We drive it through its command-line tool, `HidHideCLI.exe`, which ships
//! with the driver.

/// Converts a Windows HID device path, as returned by hidapi
/// (`\\?\HID#VID_054C&PID_0CE6&MI_03#7&1a2b3c&0&0000#{4d1e55b2-...}`), to the
/// device instance path HidHide expects (`HID\VID_054C&PID_0CE6&MI_03\7&1A2B3C&0&0000`).
pub fn instance_path_from_hid_path(path: &str) -> Option<String> {
    let p = path
        .strip_prefix(r"\\?\")
        .or_else(|| path.strip_prefix(r"\\.\"))
        .unwrap_or(path);
    // Drop the trailing interface class GUID segment.
    let p = match p.rfind("#{") {
        Some(i) => &p[..i],
        None => p,
    };
    if !p.to_ascii_uppercase().starts_with("HID#") {
        return None;
    }
    Some(p.replace('#', "\\").to_ascii_uppercase())
}

#[cfg(windows)]
mod cli {
    use std::path::PathBuf;
    use std::process::Command;

    /// Finds `HidHideCLI.exe` in its default install location.
    pub fn find() -> Option<PathBuf> {
        let base = std::env::var_os("ProgramFiles")?;
        let path = PathBuf::from(base)
            .join("Nefarius Software Solutions")
            .join("HidHide")
            .join("x64")
            .join("HidHideCLI.exe");
        path.exists().then_some(path)
    }

    pub fn run(args: &[&str]) -> Result<(), String> {
        let exe = find().ok_or("HidHide is not installed")?;
        let mut cmd = Command::new(exe);
        cmd.args(args);
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }
        let out = cmd.output().map_err(|e| e.to_string())?;
        if out.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
        }
    }
}

/// Is HidHide installed?
pub fn is_installed() -> bool {
    #[cfg(windows)]
    {
        cli::find().is_some()
    }
    #[cfg(not(windows))]
    {
        false
    }
}

/// Allows the running executable through HidHide, hides the given devices,
/// and turns hiding on.
pub fn hide_devices(hid_paths: &[String]) -> Result<(), String> {
    #[cfg(windows)]
    {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        cli::run(&["--app-reg", &exe.to_string_lossy()])?;
        for p in hid_paths {
            if let Some(instance) = instance_path_from_hid_path(p) {
                cli::run(&["--dev-hide", &instance])?;
            }
        }
        cli::run(&["--cloak-on"])
    }
    #[cfg(not(windows))]
    {
        let _ = hid_paths;
        Err("HidHide is only available on Windows".into())
    }
}

/// Makes the given devices visible again.
pub fn unhide_devices(hid_paths: &[String]) -> Result<(), String> {
    #[cfg(windows)]
    {
        for p in hid_paths {
            if let Some(instance) = instance_path_from_hid_path(p) {
                cli::run(&["--dev-unhide", &instance])?;
            }
        }
        Ok(())
    }
    #[cfg(not(windows))]
    {
        let _ = hid_paths;
        Err("HidHide is only available on Windows".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_hid_paths() {
        assert_eq!(
            instance_path_from_hid_path(
                r"\\?\hid#vid_054c&pid_0ce6&mi_03#7&1a2b3c&0&0000#{4d1e55b2-f16f-11cf-88cb-001111000030}"
            )
            .as_deref(),
            Some(r"HID\VID_054C&PID_0CE6&MI_03\7&1A2B3C&0&0000")
        );
        assert_eq!(
            instance_path_from_hid_path(
                r"\\?\HID#{00001124-0000-1000-8000-00805f9b34fb}_VID&0002054c_PID&09cc#8&2f5c3d&0&0000#{4d1e55b2-f16f-11cf-88cb-001111000030}"
            )
            .as_deref(),
            Some(r"HID\{00001124-0000-1000-8000-00805F9B34FB}_VID&0002054C_PID&09CC\8&2F5C3D&0&0000")
        );
        assert_eq!(instance_path_from_hid_path("/dev/hidraw3"), None);
    }
}
