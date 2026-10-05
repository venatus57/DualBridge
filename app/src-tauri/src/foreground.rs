//! Name of the executable owning the foreground window, for automatic
//! per-game profile switching.

/// Returns e.g. `C:\Games\Foo\foo.exe`, or `None` if unknown or unsupported.
pub fn foreground_executable() -> Option<String> {
    #[cfg(windows)]
    {
        windows_impl::foreground_executable()
    }
    #[cfg(not(windows))]
    {
        None
    }
}

#[cfg(windows)]
mod windows_impl {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
        PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetWindowThreadProcessId,
    };

    pub fn foreground_executable() -> Option<String> {
        // SAFETY: plain Win32 calls with valid arguments; the process handle
        // is closed before returning.
        unsafe {
            let hwnd = GetForegroundWindow();
            if hwnd.is_null() {
                return None;
            }
            let mut pid = 0u32;
            GetWindowThreadProcessId(hwnd, &mut pid);
            if pid == 0 {
                return None;
            }
            let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
            if process.is_null() {
                return None;
            }
            let mut buf = [0u16; 1024];
            let mut len = buf.len() as u32;
            let ok =
                QueryFullProcessImageNameW(process, PROCESS_NAME_WIN32, buf.as_mut_ptr(), &mut len);
            CloseHandle(process);
            if ok == 0 {
                return None;
            }
            Some(String::from_utf16_lossy(&buf[..len as usize]))
        }
    }
}
