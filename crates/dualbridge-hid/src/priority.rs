//! Raises the priority of the input thread.

/// Keeps the priority boost alive; dropping it reverts the thread.
pub(crate) struct PriorityGuard {
    #[cfg(windows)]
    handle: windows_sys::Win32::Foundation::HANDLE,
}

/// Registers the calling thread with the Windows Multimedia Class Scheduler
/// Service as a "Games" task, so it is scheduled ahead of normal threads.
/// Does nothing on other platforms.
pub(crate) fn boost_current_thread() -> PriorityGuard {
    #[cfg(windows)]
    {
        use windows_sys::Win32::System::Threading::AvSetMmThreadCharacteristicsW;
        let task: Vec<u16> = "Games\0".encode_utf16().collect();
        let mut index = 0u32;
        // SAFETY: `task` is a valid NUL-terminated UTF-16 string that outlives
        // the call, and `index` is a valid out pointer.
        let handle = unsafe { AvSetMmThreadCharacteristicsW(task.as_ptr(), &mut index) };
        PriorityGuard { handle }
    }
    #[cfg(not(windows))]
    {
        PriorityGuard {}
    }
}

#[cfg(windows)]
impl Drop for PriorityGuard {
    fn drop(&mut self) {
        use windows_sys::Win32::System::Threading::AvRevertMmThreadCharacteristics;
        if !self.handle.is_null() {
            // SAFETY: the handle came from AvSetMmThreadCharacteristicsW on
            // this same thread.
            unsafe {
                AvRevertMmThreadCharacteristics(self.handle);
            }
        }
    }
}
