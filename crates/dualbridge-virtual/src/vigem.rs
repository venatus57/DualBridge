//! Virtual Xbox 360 controllers through the ViGEmBus driver (Windows).
//!
//! ViGEmBus was retired by its author in 2023 but still installs and works on
//! Windows 10 and 11, and is what DS4Windows and most similar tools rely on.
//! See `docs/virtual-controller.md` for the status and alternatives.

use std::thread::JoinHandle;

use dualbridge_core::mapping::XInputState;
use vigem_client::{Client, TargetId, XButtons, XGamepad, Xbox360Wired};

use crate::{BackendStatus, RumbleCallback, VirtualBackend, VirtualError, VirtualPad};

#[derive(Default)]
pub struct VigemBackend {
    client: Option<Client>,
}

impl VigemBackend {
    pub fn new() -> VigemBackend {
        VigemBackend { client: None }
    }

    fn client(&mut self) -> Result<Client, VirtualError> {
        if self.client.is_none() {
            self.client = Some(Client::connect().map_err(|_| VirtualError::DriverMissing)?);
        }
        let client = self.client.as_ref().expect("connected above");
        client
            .try_clone()
            .map_err(|e| VirtualError::Other(e.to_string()))
    }
}

impl VirtualBackend for VigemBackend {
    fn status(&mut self) -> BackendStatus {
        match self.client() {
            Ok(_) => BackendStatus::Ready,
            Err(_) => BackendStatus::DriverMissing,
        }
    }

    fn create_xbox360(
        &mut self,
        mut rumble: RumbleCallback,
    ) -> Result<Box<dyn VirtualPad>, VirtualError> {
        let other = |e: vigem_client::Error| VirtualError::Other(e.to_string());
        let mut target = Xbox360Wired::new(self.client()?, TargetId::XBOX360_WIRED);
        target.plugin().map_err(other)?;
        target.wait_ready().map_err(other)?;
        let notifications = match target.request_notification() {
            Ok(req) => Some(req.spawn_thread(move |_, n| rumble(n.large_motor, n.small_motor))),
            Err(_) => None,
        };
        Ok(Box::new(VigemPad {
            target: Some(target),
            notifications,
        }))
    }
}

struct VigemPad {
    target: Option<Xbox360Wired<Client>>,
    notifications: Option<JoinHandle<()>>,
}

impl VirtualPad for VigemPad {
    fn update(&mut self, s: &XInputState) -> Result<(), VirtualError> {
        let pad = XGamepad {
            buttons: XButtons { raw: s.buttons },
            left_trigger: s.left_trigger,
            right_trigger: s.right_trigger,
            thumb_lx: s.thumb_lx,
            thumb_ly: s.thumb_ly,
            thumb_rx: s.thumb_rx,
            thumb_ry: s.thumb_ry,
        };
        match self.target.as_mut() {
            Some(t) => t
                .update(&pad)
                .map_err(|e| VirtualError::Other(e.to_string())),
            None => Ok(()),
        }
    }

    fn user_index(&mut self) -> Option<u32> {
        self.target.as_mut()?.get_user_index().ok()
    }
}

impl Drop for VigemPad {
    fn drop(&mut self) {
        // Unplugging aborts the pending notification request, which ends the
        // notification thread.
        if let Some(mut t) = self.target.take() {
            let _ = t.unplug();
        }
        if let Some(n) = self.notifications.take() {
            let _ = n.join();
        }
    }
}
