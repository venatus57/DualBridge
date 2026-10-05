//! A backend that records what would be sent to games.

use std::sync::{Arc, Mutex};

use dualbridge_core::mapping::XInputState;

use crate::{BackendStatus, RumbleCallback, VirtualBackend, VirtualError, VirtualPad};

#[derive(Default)]
struct PadRecord {
    updates: Vec<XInputState>,
    rumble: Option<RumbleCallback>,
    unplugged: bool,
}

/// Test-side view of a mock virtual pad.
#[derive(Clone)]
pub struct MockPadHandle(Arc<Mutex<PadRecord>>);

impl MockPadHandle {
    pub fn updates(&self) -> Vec<XInputState> {
        self.0.lock().unwrap().updates.clone()
    }

    pub fn last(&self) -> Option<XInputState> {
        self.0.lock().unwrap().updates.last().copied()
    }

    /// Simulates a game setting rumble.
    pub fn game_rumble(&self, large: u8, small: u8) {
        if let Some(cb) = self.0.lock().unwrap().rumble.as_mut() {
            cb(large, small);
        }
    }

    pub fn is_unplugged(&self) -> bool {
        self.0.lock().unwrap().unplugged
    }
}

/// Records every created pad. Clones share the same records.
#[derive(Clone, Default)]
pub struct MockVirtualBackend {
    pads: Arc<Mutex<Vec<MockPadHandle>>>,
    status: Arc<Mutex<Option<BackendStatus>>>,
}

impl MockVirtualBackend {
    pub fn pads(&self) -> Vec<MockPadHandle> {
        self.pads.lock().unwrap().clone()
    }

    /// Pretends the driver is missing (or ready again).
    pub fn set_status(&self, status: BackendStatus) {
        *self.status.lock().unwrap() = Some(status);
    }
}

impl VirtualBackend for MockVirtualBackend {
    fn status(&mut self) -> BackendStatus {
        self.status.lock().unwrap().unwrap_or(BackendStatus::Ready)
    }

    fn create_xbox360(
        &mut self,
        rumble: RumbleCallback,
    ) -> Result<Box<dyn VirtualPad>, VirtualError> {
        match self.status() {
            BackendStatus::Ready => {}
            BackendStatus::DriverMissing => return Err(VirtualError::DriverMissing),
            BackendStatus::Unsupported => return Err(VirtualError::Unsupported),
        }
        let record = Arc::new(Mutex::new(PadRecord {
            rumble: Some(rumble),
            ..PadRecord::default()
        }));
        let handle = MockPadHandle(record.clone());
        self.pads.lock().unwrap().push(handle);
        Ok(Box::new(MockPad { record }))
    }
}

struct MockPad {
    record: Arc<Mutex<PadRecord>>,
}

impl VirtualPad for MockPad {
    fn update(&mut self, state: &XInputState) -> Result<(), VirtualError> {
        self.record.lock().unwrap().updates.push(*state);
        Ok(())
    }

    fn user_index(&mut self) -> Option<u32> {
        Some(0)
    }
}

impl Drop for MockPad {
    fn drop(&mut self) {
        self.record.lock().unwrap().unplugged = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_updates_and_rumble() {
        let backend = MockVirtualBackend::default();
        let rumble = Arc::new(Mutex::new((0u8, 0u8)));
        let r = rumble.clone();
        let mut pad = backend
            .clone()
            .create_xbox360(Box::new(move |l, s| *r.lock().unwrap() = (l, s)))
            .unwrap();
        pad.update(&XInputState {
            buttons: 0x1000,
            ..XInputState::default()
        })
        .unwrap();
        let h = &backend.pads()[0];
        assert_eq!(h.last().unwrap().buttons, 0x1000);
        h.game_rumble(10, 20);
        assert_eq!(*rumble.lock().unwrap(), (10, 20));
        drop(pad);
        assert!(h.is_unplugged());

        backend.set_status(BackendStatus::DriverMissing);
        assert!(matches!(
            backend.clone().create_xbox360(Box::new(|_, _| {})),
            Err(VirtualError::DriverMissing)
        ));
    }
}
