//! Running HidHide commands with administrator rights.
//!
//! HidHide only accepts configuration changes from an elevated process, and
//! DualBridge runs as a normal user. To change it, the app relaunches its own
//! executable elevated (Windows shows one UAC prompt) with
//! `--hidhide-helper <commands>`; that helper runs `HidHideCLI.exe` and exits
//! before any window or Tauri code starts. HidHide keeps its configuration, so
//! this is only needed when a new controller has to be hidden.

use dualbridge_virtual::hidhide;

const HELPER_FLAG: &str = "--hidhide-helper";

/// Why an elevated run did not succeed.
#[derive(Debug, Clone, PartialEq, Eq)]
// Only Windows can decline elevation.
#[cfg_attr(not(windows), allow(dead_code))]
pub enum ElevateError {
    /// The user answered "No" to the UAC prompt.
    Declined,
    Failed(String),
}

impl std::fmt::Display for ElevateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ElevateError::Declined => write!(f, "administrator permission was declined"),
            ElevateError::Failed(msg) => write!(f, "{msg}"),
        }
    }
}

/// Encodes commands as one hex string, so no command-line quoting is needed
/// (device paths contain `&` and `\`).
#[cfg_attr(not(windows), allow(dead_code))]
fn encode(cmds: &[hidhide::Command]) -> String {
    let json = serde_json::to_vec(cmds).expect("commands serialize");
    json.iter().map(|b| format!("{b:02x}")).collect()
}

fn decode(hex: &str) -> Option<Vec<hidhide::Command>> {
    if !hex.len().is_multiple_of(2) {
        return None;
    }
    let bytes: Option<Vec<u8>> = (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(hex.get(i..i + 2)?, 16).ok())
        .collect();
    serde_json::from_slice(&bytes?).ok()
}

/// If this process was started as the elevated helper, runs the commands and
/// returns the exit code. Called first thing in `main`.
pub fn helper_main() -> Option<i32> {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) != Some(HELPER_FLAG) {
        return None;
    }
    let Some(cmds) = args.get(2).and_then(|h| decode(h)) else {
        return Some(2);
    };
    Some(match hidhide::run_commands(&cmds) {
        Ok(()) => 0,
        Err(_) => 1,
    })
}

/// Runs HidHide commands elevated, waiting for them to finish.
pub fn run_hidhide_elevated(cmds: &[hidhide::Command]) -> Result<(), ElevateError> {
    if cmds.is_empty() {
        return Ok(());
    }
    #[cfg(windows)]
    {
        windows_impl::run_self_elevated(&[HELPER_FLAG.to_string(), encode(cmds)])
    }
    #[cfg(not(windows))]
    {
        Err(ElevateError::Failed(
            "HidHide is only available on Windows".into(),
        ))
    }
}

#[cfg(windows)]
mod windows_impl {
    use super::ElevateError;
    use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, ERROR_CANCELLED};
    use windows_sys::Win32::System::Threading::{GetExitCodeProcess, WaitForSingleObject};
    use windows_sys::Win32::UI::Shell::{
        ShellExecuteExW, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_HIDE;

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    /// Starts this executable with `args` through the "runas" verb (UAC
    /// prompt) and waits up to two minutes for it to exit.
    pub fn run_self_elevated(args: &[String]) -> Result<(), ElevateError> {
        let exe = std::env::current_exe().map_err(|e| ElevateError::Failed(e.to_string()))?;
        // Arguments never contain spaces or quotes (flag + hex), so joining
        // with spaces is safe.
        let params = wide(&args.join(" "));
        let file = wide(&exe.to_string_lossy());
        let verb = wide("runas");

        // SAFETY: the info struct is zero-initialized (valid for this plain
        // C struct) and every pointer we set outlives the call; the process
        // handle is closed before returning.
        unsafe {
            let mut info: SHELLEXECUTEINFOW = std::mem::zeroed();
            info.cbSize = std::mem::size_of::<SHELLEXECUTEINFOW>() as u32;
            info.fMask = SEE_MASK_NOCLOSEPROCESS;
            info.lpVerb = verb.as_ptr();
            info.lpFile = file.as_ptr();
            info.lpParameters = params.as_ptr();
            info.nShow = SW_HIDE;
            if ShellExecuteExW(&mut info) == 0 {
                return Err(if GetLastError() == ERROR_CANCELLED {
                    ElevateError::Declined
                } else {
                    ElevateError::Failed(format!(
                        "could not start the elevated helper (error {})",
                        GetLastError()
                    ))
                });
            }
            if info.hProcess.is_null() {
                return Err(ElevateError::Failed("no helper process".into()));
            }
            WaitForSingleObject(info.hProcess, 120_000);
            let mut code = 1u32;
            GetExitCodeProcess(info.hProcess, &mut code);
            CloseHandle(info.hProcess);
            match code {
                0 => Ok(()),
                _ => Err(ElevateError::Failed(format!(
                    "HidHide did not accept the change (code {code})"
                ))),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_round_trip() {
        let cmds = vec![
            vec![
                "--app-reg".to_string(),
                r"C:\Program Files\DualBridge\DualBridge.exe".to_string(),
            ],
            vec![
                "--dev-hide".to_string(),
                r"HID\VID_054C&PID_0CE6&MI_03\7&1&0&0000".to_string(),
            ],
            vec!["--cloak-on".to_string()],
        ];
        let hex = encode(&cmds);
        assert!(hex.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(decode(&hex), Some(cmds));
        assert_eq!(decode("abc"), None);
        assert_eq!(decode("zz"), None);
    }
}
