//! Other controller programs that conflict with DualBridge.
//!
//! Tools such as DS4Windows also read PlayStation controllers and create their
//! own virtual Xbox controller for each one, and they put themselves on
//! HidHide's allow list. Running one next to DualBridge makes every
//! controller show up twice in games, even with exclusive mode on.

/// Known conflicting programs: (executable name, display name).
pub const KNOWN: [(&str, &str); 5] = [
    ("ds4windows.exe", "DS4Windows"),
    ("dsx.exe", "DSX"),
    ("dualsensex.exe", "DualSenseX"),
    ("inputmapper.exe", "InputMapper"),
    ("betterjoy.exe", "BetterJoy"),
];

/// Display names of the known programs among the given executable names
/// (case-insensitive), in [`KNOWN`] order, without duplicates.
pub fn find_known<'a>(exe_names: impl IntoIterator<Item = &'a str>) -> Vec<&'static str> {
    let running: Vec<String> = exe_names
        .into_iter()
        .map(|n| n.to_ascii_lowercase())
        .collect();
    KNOWN
        .iter()
        .filter(|(exe, _)| running.iter().any(|r| r == exe))
        .map(|(_, name)| *name)
        .collect()
}

/// Conflicting programs running right now. Always empty outside Windows.
pub fn running() -> Vec<&'static str> {
    #[cfg(windows)]
    {
        let names = windows_impl::process_names();
        find_known(names.iter().map(String::as_str))
    }
    #[cfg(not(windows))]
    {
        Vec::new()
    }
}

#[cfg(windows)]
mod windows_impl {
    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
        TH32CS_SNAPPROCESS,
    };

    /// Executable names of every running process.
    pub fn process_names() -> Vec<String> {
        let mut names = Vec::new();
        // SAFETY: the snapshot handle is checked and closed; `entry` is a
        // plain C struct with `dwSize` set as the API requires.
        unsafe {
            let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
            if snap == INVALID_HANDLE_VALUE {
                return names;
            }
            let mut entry: PROCESSENTRY32W = std::mem::zeroed();
            entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
            let mut ok = Process32FirstW(snap, &mut entry);
            while ok != 0 {
                let len = entry
                    .szExeFile
                    .iter()
                    .position(|&c| c == 0)
                    .unwrap_or(entry.szExeFile.len());
                names.push(String::from_utf16_lossy(&entry.szExeFile[..len]));
                ok = Process32NextW(snap, &mut entry);
            }
            CloseHandle(snap);
        }
        names
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_known_programs() {
        assert_eq!(
            find_known(["explorer.exe", "DS4Windows.exe", "steam.exe", "DSX.exe"]),
            vec!["DS4Windows", "DSX"]
        );
        assert!(find_known(["dualbridge.exe"]).is_empty());
    }
}
