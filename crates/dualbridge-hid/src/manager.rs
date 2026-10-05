//! Keeps track of connected controllers and their slots.
//!
//! Call [`ControllerManager::poll`] periodically (about once a second) from a
//! background thread: it notices new and removed controllers and starts or
//! stops their I/O threads. Slots stay packed: when a controller leaves, the
//! ones after it move down (controller 3 becomes 2), and a new controller
//! takes the next number.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use dualbridge_core::output::OutputState;
use serde::Serialize;

use crate::runtime::{ControllerHandle, ControllerShared, InputSink};
use crate::{DeviceInfo, HidBackend, HidResult};

/// Maximum number of controllers handled at once.
pub const MAX_SLOTS: usize = 8;

/// Something that changed during a [`ControllerManager::poll`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ManagerEvent {
    /// `slot` is 1-based.
    Connected {
        slot: u8,
        info: DeviceInfo,
    },
    Disconnected {
        slot: u8,
        info: DeviceInfo,
    },
    /// Another connection (USB or Bluetooth) of a controller that is already
    /// in use. It is not used, but should be hidden from games too.
    Duplicate {
        info: DeviceInfo,
    },
    /// No free slot for this controller.
    NoFreeSlot {
        info: DeviceInfo,
    },
    /// The controller was found but could not be opened.
    OpenFailed {
        info: DeviceInfo,
        error: String,
    },
}

/// Builds what a new controller needs: the sink for its input reports and its
/// first output state (lighting). Called with the 1-based slot number.
pub trait ControllerFactory: Send {
    fn create(&mut self, info: &DeviceInfo, slot: u8) -> (Box<dyn InputSink>, OutputState);
}

impl<F> ControllerFactory for F
where
    F: FnMut(&DeviceInfo, u8) -> (Box<dyn InputSink>, OutputState) + Send,
{
    fn create(&mut self, info: &DeviceInfo, slot: u8) -> (Box<dyn InputSink>, OutputState) {
        self(info, slot)
    }
}

struct Slot {
    handle: ControllerHandle,
}

pub struct ControllerManager<B: HidBackend> {
    backend: B,
    factory: Box<dyn ControllerFactory>,
    slots: [Option<Slot>; MAX_SLOTS],
    /// Paths that failed to open, so we don't retry (and report) every poll.
    failed: HashMap<String, u32>,
    /// Other connections of controllers already in use (paths).
    duplicates: HashSet<String>,
}

impl<B: HidBackend> ControllerManager<B> {
    pub fn new(backend: B, factory: impl ControllerFactory + 'static) -> Self {
        ControllerManager {
            backend,
            factory: Box::new(factory),
            slots: Default::default(),
            failed: HashMap::new(),
            duplicates: HashSet::new(),
        }
    }

    /// Detects connected and disconnected controllers.
    pub fn poll(&mut self) -> HidResult<Vec<ManagerEvent>> {
        let mut events = Vec::new();
        let devices = self.backend.enumerate()?;

        // Drop controllers whose threads ended or that are no longer listed.
        let mut removed = false;
        for i in 0..MAX_SLOTS {
            let gone = match &self.slots[i] {
                Some(s) => {
                    let shared = s.handle.shared();
                    !shared.is_connected() || !devices.iter().any(|d| d.path == shared.info.path)
                }
                None => false,
            };
            if gone {
                let slot = self.slots[i].take().unwrap();
                let info = slot.handle.shared().info.clone();
                slot.handle.stop();
                removed = true;
                events.push(ManagerEvent::Disconnected {
                    slot: i as u8 + 1,
                    info,
                });
            }
        }
        if removed {
            self.compact();
        }
        self.failed
            .retain(|path, _| devices.iter().any(|d| &d.path == path));
        self.duplicates
            .retain(|path| devices.iter().any(|d| &d.path == path));

        for info in devices {
            let mut same_path = false;
            let mut same_controller = false;
            for s in self.slots.iter().flatten() {
                let i = &s.handle.shared().info;
                same_path |= i.path == info.path;
                same_controller |= i.identity() == info.identity();
            }
            if same_path {
                continue;
            }
            if same_controller {
                // The same controller on USB and Bluetooth at once: keep the
                // connection already in use.
                if self.duplicates.insert(info.path.clone()) {
                    events.push(ManagerEvent::Duplicate { info });
                }
                continue;
            }
            // Retry a failing device only every few polls.
            if let Some(n) = self.failed.get_mut(&info.path) {
                *n += 1;
                if *n % 5 != 0 {
                    continue;
                }
            }
            let Some(index) = self.slots.iter().position(Option::is_none) else {
                events.push(ManagerEvent::NoFreeSlot { info });
                continue;
            };
            let slot_number = index as u8 + 1;
            let (sink, output) = self.factory.create(&info, slot_number);
            match ControllerHandle::start(&mut self.backend, info.clone(), sink, output) {
                Ok(handle) => {
                    self.failed.remove(&info.path);
                    self.slots[index] = Some(Slot { handle });
                    events.push(ManagerEvent::Connected {
                        slot: slot_number,
                        info,
                    });
                }
                Err(e) => {
                    let first_failure = !self.failed.contains_key(&info.path);
                    self.failed.entry(info.path.clone()).or_insert(0);
                    if first_failure {
                        events.push(ManagerEvent::OpenFailed {
                            info,
                            error: e.to_string(),
                        });
                    }
                }
            }
        }
        Ok(events)
    }

    /// Moves controllers down to fill the gaps left by disconnected ones.
    fn compact(&mut self) {
        let remaining: Vec<Slot> = self.slots.iter_mut().filter_map(Option::take).collect();
        for (i, slot) in remaining.into_iter().enumerate() {
            self.slots[i] = Some(slot);
        }
    }

    /// Other connections of controllers already in use (see
    /// [`ManagerEvent::Duplicate`]).
    pub fn duplicate_paths(&self) -> impl Iterator<Item = &String> {
        self.duplicates.iter()
    }

    /// Connected controllers with their 1-based slot numbers.
    pub fn controllers(&self) -> impl Iterator<Item = (u8, &Arc<ControllerShared>)> {
        self.slots
            .iter()
            .enumerate()
            .filter_map(|(i, s)| s.as_ref().map(|s| (i as u8 + 1, s.handle.shared())))
    }

    /// The controller in a 1-based slot.
    pub fn get(&self, slot: u8) -> Option<&Arc<ControllerShared>> {
        let i = (slot as usize).checked_sub(1)?;
        self.slots.get(i)?.as_ref().map(|s| s.handle.shared())
    }

    /// Swaps two 1-based slots. Returns `false` if a slot number is out of
    /// range.
    pub fn swap_slots(&mut self, a: u8, b: u8) -> bool {
        let (Some(a), Some(b)) = ((a as usize).checked_sub(1), (b as usize).checked_sub(1)) else {
            return false;
        };
        if a >= MAX_SLOTS || b >= MAX_SLOTS {
            return false;
        }
        self.slots.swap(a, b);
        // Swapping with an empty slot must not leave a gap.
        self.compact();
        true
    }

    /// Stops every controller.
    pub fn shutdown(&mut self) {
        for s in self.slots.iter_mut() {
            if let Some(s) = s.take() {
                s.handle.stop();
            }
        }
    }
}

impl<B: HidBackend> Drop for ControllerManager<B> {
    fn drop(&mut self) {
        self.shutdown();
    }
}
