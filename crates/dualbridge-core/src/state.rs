//! The normalized controller state shared by every model.

use serde::{Deserialize, Serialize};

/// Pressed buttons, as a bit set. The D-pad is included as four buttons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Buttons(pub u32);

impl Buttons {
    pub const CROSS: Buttons = Buttons(1 << 0);
    pub const CIRCLE: Buttons = Buttons(1 << 1);
    pub const SQUARE: Buttons = Buttons(1 << 2);
    pub const TRIANGLE: Buttons = Buttons(1 << 3);
    pub const L1: Buttons = Buttons(1 << 4);
    pub const R1: Buttons = Buttons(1 << 5);
    /// Digital L2 (set by the controller when the trigger is pressed a little).
    pub const L2: Buttons = Buttons(1 << 6);
    pub const R2: Buttons = Buttons(1 << 7);
    /// "Share" on DualShock 4, "Create" on DualSense.
    pub const SHARE: Buttons = Buttons(1 << 8);
    pub const OPTIONS: Buttons = Buttons(1 << 9);
    pub const L3: Buttons = Buttons(1 << 10);
    pub const R3: Buttons = Buttons(1 << 11);
    pub const PS: Buttons = Buttons(1 << 12);
    pub const TOUCHPAD: Buttons = Buttons(1 << 13);
    /// DualSense microphone mute button.
    pub const MUTE: Buttons = Buttons(1 << 14);
    pub const DPAD_UP: Buttons = Buttons(1 << 15);
    pub const DPAD_DOWN: Buttons = Buttons(1 << 16);
    pub const DPAD_LEFT: Buttons = Buttons(1 << 17);
    pub const DPAD_RIGHT: Buttons = Buttons(1 << 18);
    /// DualSense Edge left function button.
    pub const LEFT_FN: Buttons = Buttons(1 << 19);
    /// DualSense Edge right function button.
    pub const RIGHT_FN: Buttons = Buttons(1 << 20);
    /// DualSense Edge left back paddle.
    pub const LEFT_PADDLE: Buttons = Buttons(1 << 21);
    /// DualSense Edge right back paddle.
    pub const RIGHT_PADDLE: Buttons = Buttons(1 << 22);

    pub const NONE: Buttons = Buttons(0);

    /// Every named button with a stable identifier, in display order.
    pub const ALL: [(Buttons, &'static str); 23] = [
        (Buttons::CROSS, "cross"),
        (Buttons::CIRCLE, "circle"),
        (Buttons::SQUARE, "square"),
        (Buttons::TRIANGLE, "triangle"),
        (Buttons::L1, "l1"),
        (Buttons::R1, "r1"),
        (Buttons::L2, "l2"),
        (Buttons::R2, "r2"),
        (Buttons::SHARE, "share"),
        (Buttons::OPTIONS, "options"),
        (Buttons::L3, "l3"),
        (Buttons::R3, "r3"),
        (Buttons::PS, "ps"),
        (Buttons::TOUCHPAD, "touchpad"),
        (Buttons::MUTE, "mute"),
        (Buttons::DPAD_UP, "dpad_up"),
        (Buttons::DPAD_DOWN, "dpad_down"),
        (Buttons::DPAD_LEFT, "dpad_left"),
        (Buttons::DPAD_RIGHT, "dpad_right"),
        (Buttons::LEFT_FN, "left_fn"),
        (Buttons::RIGHT_FN, "right_fn"),
        (Buttons::LEFT_PADDLE, "left_paddle"),
        (Buttons::RIGHT_PADDLE, "right_paddle"),
    ];

    #[inline]
    pub const fn contains(self, other: Buttons) -> bool {
        self.0 & other.0 == other.0 && other.0 != 0
    }

    #[inline]
    pub fn set(&mut self, other: Buttons, on: bool) {
        if on {
            self.0 |= other.0;
        } else {
            self.0 &= !other.0;
        }
    }

    #[inline]
    pub const fn union(self, other: Buttons) -> Buttons {
        Buttons(self.0 | other.0)
    }

    pub fn from_name(name: &str) -> Option<Buttons> {
        Buttons::ALL
            .iter()
            .find(|(_, n)| *n == name)
            .map(|(b, _)| *b)
    }

    /// Sets the four D-pad bits from a hat switch value (0 = up, clockwise,
    /// 8 or more = centered).
    #[inline]
    pub fn set_hat(&mut self, hat: u8) {
        let (up, right, down, left) = match hat {
            0 => (true, false, false, false),
            1 => (true, true, false, false),
            2 => (false, true, false, false),
            3 => (false, true, true, false),
            4 => (false, false, true, false),
            5 => (false, false, true, true),
            6 => (false, false, false, true),
            7 => (true, false, false, true),
            _ => (false, false, false, false),
        };
        self.set(Buttons::DPAD_UP, up);
        self.set(Buttons::DPAD_RIGHT, right);
        self.set(Buttons::DPAD_DOWN, down);
        self.set(Buttons::DPAD_LEFT, left);
    }
}

impl std::ops::BitOr for Buttons {
    type Output = Buttons;
    fn bitor(self, rhs: Buttons) -> Buttons {
        self.union(rhs)
    }
}

/// An analog stick. `0x80` is (roughly) centered; `x` grows to the right and
/// `y` grows downward, as reported by the controller.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Stick {
    pub x: u8,
    pub y: u8,
}

impl Default for Stick {
    fn default() -> Self {
        Stick { x: 0x80, y: 0x80 }
    }
}

/// One touchpad contact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Touch {
    pub active: bool,
    /// Tracking ID, increments with each new contact (7 bits).
    pub id: u8,
    pub x: u16,
    pub y: u16,
}

/// Battery charging status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Charging {
    #[default]
    Discharging,
    Charging,
    Full,
    /// The controller reported a charging error (temperature, voltage...).
    Error,
}

/// Battery level and charging state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Battery {
    /// Approximate charge, 0 to 100.
    pub percent: u8,
    pub charging: Charging,
    /// A USB cable is plugged in (it may also be used for data).
    pub cable: bool,
}

/// Raw motion sensor readings. Use [`crate::calibration`] to convert them to
/// physical units.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Motion {
    /// Angular velocity: pitch (x), yaw (y), roll (z).
    pub gyro: [i16; 3],
    /// Acceleration: x, y, z.
    pub accel: [i16; 3],
    /// Sensor timestamp from the controller.
    pub timestamp: u32,
}

/// Everything the controller reports, normalized across models.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct ControllerState {
    pub buttons: Buttons,
    pub left_stick: Stick,
    pub right_stick: Stick,
    /// Analog L2, 0 (released) to 255 (fully pressed).
    pub l2: u8,
    /// Analog R2, 0 (released) to 255 (fully pressed).
    pub r2: u8,
    pub touch: [Touch; 2],
    pub motion: Motion,
    pub battery: Battery,
    /// Report counter from the controller, useful to detect dropped reports.
    pub counter: u8,
    /// DualSense adaptive trigger status bytes (left, right). Zero on DS4.
    pub trigger_feedback: [u8; 2],
    /// Headphones plugged in the controller's audio jack.
    pub headphones: bool,
    /// `false` when the report only had the basic fields (Bluetooth "simple"
    /// mode before the controller was switched to full reports).
    pub full: bool,
}
