//! HidHide integration ("exclusive mode").
//!
//! With a virtual Xbox controller plugged in, games that also understand
//! PlayStation controllers would see the player twice. HidHide is a filter
//! driver that hides chosen devices from every application except an allow
//! list; DualBridge adds itself to the list and hides the physical controllers.
//!
//! We drive it through its command-line tool, `HidHideCLI.exe`, which ships
//! with the driver. HidHide only accepts configuration changes from an
//! administrator process, so the app runs these commands through a short
//! elevated helper (one UAC prompt), and only when something must change:
//! HidHide remembers its configuration across restarts.

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

/// Finds `HidHideCLI.exe` in its default install location.
#[cfg(windows)]
pub fn cli_path() -> Option<std::path::PathBuf> {
    let base = std::env::var_os("ProgramFiles")?;
    let path = std::path::PathBuf::from(base)
        .join("Nefarius Software Solutions")
        .join("HidHide")
        .join("x64")
        .join("HidHideCLI.exe");
    path.exists().then_some(path)
}

/// Is HidHide installed?
pub fn is_installed() -> bool {
    #[cfg(windows)]
    {
        cli_path().is_some()
    }
    #[cfg(not(windows))]
    {
        false
    }
}

/// One `HidHideCLI.exe` invocation (its arguments).
pub type Command = Vec<String>;

/// Commands that let `app_exe` through HidHide, hide the given devices (by
/// hidapi path) from every other application, and turn hiding on.
///
/// HidHide keeps this configuration (in the registry) until it is changed
/// again, so it only needs to be done once per controller.
pub fn hide_commands(app_exe: &str, hid_paths: &[String]) -> Vec<Command> {
    let mut cmds = vec![vec!["--app-reg".to_string(), app_exe.to_string()]];
    for p in hid_paths {
        if let Some(instance) = instance_path_from_hid_path(p) {
            cmds.push(vec!["--dev-hide".to_string(), instance]);
        }
    }
    cmds.push(vec!["--cloak-on".to_string()]);
    cmds
}

/// Commands that make the given devices visible to every application again.
pub fn unhide_commands(hid_paths: &[String]) -> Vec<Command> {
    hid_paths
        .iter()
        .filter_map(|p| instance_path_from_hid_path(p))
        .map(|instance| vec!["--dev-unhide".to_string(), instance])
        .collect()
}

/// Runs HidHide commands. HidHide only accepts changes from an elevated
/// (administrator) process, so this must run elevated; the app does that by
/// relaunching itself as a short-lived elevated helper.
pub fn run_commands(cmds: &[Command]) -> Result<(), String> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let exe = cli_path().ok_or("HidHide is not installed")?;
        for args in cmds {
            let out = std::process::Command::new(&exe)
                .args(args)
                .creation_flags(CREATE_NO_WINDOW)
                .output()
                .map_err(|e| e.to_string())?;
            if !out.status.success() {
                let mut msg = String::from_utf8_lossy(&out.stderr).trim().to_string();
                if msg.is_empty() {
                    msg = String::from_utf8_lossy(&out.stdout).trim().to_string();
                }
                return Err(format!("HidHideCLI {}: {msg}", args.join(" ")));
            }
        }
        Ok(())
    }
    #[cfg(not(windows))]
    {
        let _ = cmds;
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

    #[test]
    fn builds_commands() {
        let paths = vec![
            r"\\?\HID#VID_054C&PID_0CE6&MI_03#7&1&0&0000#{4d1e55b2-f16f-11cf-88cb-001111000030}"
                .to_string(),
            "/dev/hidraw0".to_string(),
        ];
        let hide = hide_commands(r"C:\Program Files\DualBridge\DualBridge.exe", &paths);
        assert_eq!(
            hide,
            vec![
                vec![
                    "--app-reg".to_string(),
                    r"C:\Program Files\DualBridge\DualBridge.exe".to_string()
                ],
                vec![
                    "--dev-hide".to_string(),
                    r"HID\VID_054C&PID_0CE6&MI_03\7&1&0&0000".to_string()
                ],
                vec!["--cloak-on".to_string()],
            ]
        );
        assert_eq!(
            unhide_commands(&paths),
            vec![vec![
                "--dev-unhide".to_string(),
                r"HID\VID_054C&PID_0CE6&MI_03\7&1&0&0000".to_string()
            ]]
        );
    }
}
