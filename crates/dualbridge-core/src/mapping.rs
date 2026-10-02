//! Mapping a PlayStation controller state to a virtual Xbox 360 controller.
//!
//! [`MappingConfig::compile`] turns the user-facing settings into a
//! [`Mapper`], which the input thread calls for every report. `Mapper::map`
//! only does arithmetic: no allocation, no locking.

use serde::{Deserialize, Serialize};

use crate::state::{Buttons, ControllerState, Stick};

/// XInput button bits.
pub mod xbutton {
    pub const DPAD_UP: u16 = 0x0001;
    pub const DPAD_DOWN: u16 = 0x0002;
    pub const DPAD_LEFT: u16 = 0x0004;
    pub const DPAD_RIGHT: u16 = 0x0008;
    pub const START: u16 = 0x0010;
    pub const BACK: u16 = 0x0020;
    pub const LEFT_THUMB: u16 = 0x0040;
    pub const RIGHT_THUMB: u16 = 0x0080;
    pub const LEFT_SHOULDER: u16 = 0x0100;
    pub const RIGHT_SHOULDER: u16 = 0x0200;
    pub const GUIDE: u16 = 0x0400;
    pub const A: u16 = 0x1000;
    pub const B: u16 = 0x2000;
    pub const X: u16 = 0x4000;
    pub const Y: u16 = 0x8000;
}

/// State of a virtual Xbox 360 controller, in XInput units.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct XInputState {
    pub buttons: u16,
    pub left_trigger: u8,
    pub right_trigger: u8,
    /// -32768 (left) to 32767 (right).
    pub thumb_lx: i16,
    /// -32768 (down) to 32767 (up).
    pub thumb_ly: i16,
    pub thumb_rx: i16,
    pub thumb_ry: i16,
}

/// What a controller button does on the virtual controller.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ButtonTarget {
    None,
    A,
    B,
    X,
    Y,
    LeftShoulder,
    RightShoulder,
    /// Full press of the left trigger.
    LeftTrigger,
    RightTrigger,
    Back,
    Start,
    Guide,
    LeftThumb,
    RightThumb,
    DpadUp,
    DpadDown,
    DpadLeft,
    DpadRight,
}

impl ButtonTarget {
    fn bits(self) -> u16 {
        use xbutton::*;
        match self {
            ButtonTarget::A => A,
            ButtonTarget::B => B,
            ButtonTarget::X => X,
            ButtonTarget::Y => Y,
            ButtonTarget::LeftShoulder => LEFT_SHOULDER,
            ButtonTarget::RightShoulder => RIGHT_SHOULDER,
            ButtonTarget::Back => BACK,
            ButtonTarget::Start => START,
            ButtonTarget::Guide => GUIDE,
            ButtonTarget::LeftThumb => LEFT_THUMB,
            ButtonTarget::RightThumb => RIGHT_THUMB,
            ButtonTarget::DpadUp => DPAD_UP,
            ButtonTarget::DpadDown => DPAD_DOWN,
            ButtonTarget::DpadLeft => DPAD_LEFT,
            ButtonTarget::DpadRight => DPAD_RIGHT,
            ButtonTarget::None | ButtonTarget::LeftTrigger | ButtonTarget::RightTrigger => 0,
        }
    }
}

/// Default target for each button: the physical position on an Xbox pad.
pub fn default_target(button: Buttons) -> ButtonTarget {
    const TABLE: [(Buttons, ButtonTarget); 19] = [
        (Buttons::CROSS, ButtonTarget::A),
        (Buttons::CIRCLE, ButtonTarget::B),
        (Buttons::SQUARE, ButtonTarget::X),
        (Buttons::TRIANGLE, ButtonTarget::Y),
        (Buttons::L1, ButtonTarget::LeftShoulder),
        (Buttons::R1, ButtonTarget::RightShoulder),
        (Buttons::SHARE, ButtonTarget::Back),
        (Buttons::OPTIONS, ButtonTarget::Start),
        (Buttons::L3, ButtonTarget::LeftThumb),
        (Buttons::R3, ButtonTarget::RightThumb),
        (Buttons::PS, ButtonTarget::Guide),
        (Buttons::TOUCHPAD, ButtonTarget::Back),
        (Buttons::DPAD_UP, ButtonTarget::DpadUp),
        (Buttons::DPAD_DOWN, ButtonTarget::DpadDown),
        (Buttons::DPAD_LEFT, ButtonTarget::DpadLeft),
        (Buttons::DPAD_RIGHT, ButtonTarget::DpadRight),
        (Buttons::LEFT_PADDLE, ButtonTarget::None),
        (Buttons::RIGHT_PADDLE, ButtonTarget::None),
        (Buttons::MUTE, ButtonTarget::None),
    ];
    TABLE
        .iter()
        .find(|(b, _)| *b == button)
        .map_or(ButtonTarget::None, |(_, t)| *t)
}

/// Stick response.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct StickConfig {
    /// Radial dead zone, 0 to 1 of the stick travel. Movement inside it is ignored.
    pub deadzone: f32,
    /// Radius at which the output reaches 100 %, 0 to 1.
    pub outer: f32,
    /// Minimum output once outside the dead zone, 0 to 1 (counters a game's
    /// own dead zone).
    pub anti_deadzone: f32,
    /// Response curve exponent: 1 = linear, above 1 = finer control near the
    /// center, below 1 = more sensitive near the center.
    pub curve: f32,
    pub invert_x: bool,
    pub invert_y: bool,
    /// "Keyboard precision": the stick only sends full presses in 8 (or 4)
    /// directions, like arrow keys. No half-pushed or slightly-off
    /// directions, so a sideways push never turns into a down input.
    pub digital: bool,
    /// Digital mode: how far the stick must be pushed (0 to 1) before a
    /// direction counts.
    pub digital_threshold: f32,
    /// Digital mode: angular width of each diagonal zone, in degrees (0 to
    /// 60). 0 gives 4 directions only; the straight directions get the rest
    /// of each 90° quarter.
    pub diagonal_width: f32,
}

impl Default for StickConfig {
    fn default() -> Self {
        StickConfig {
            deadzone: 0.0,
            outer: 1.0,
            anti_deadzone: 0.0,
            curve: 1.0,
            invert_x: false,
            invert_y: false,
            digital: false,
            digital_threshold: 0.5,
            diagonal_width: 30.0,
        }
    }
}

/// Trigger response.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TriggerConfig {
    /// Presses below this value (0 to 255) are ignored.
    pub deadzone: u8,
    /// Presses at or above this value (0 to 255) count as fully pressed.
    pub max: u8,
    pub curve: f32,
}

impl Default for TriggerConfig {
    fn default() -> Self {
        TriggerConfig {
            deadzone: 0,
            max: 255,
            curve: 1.0,
        }
    }
}

/// User-facing mapping settings.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct MappingConfig {
    pub left_stick: StickConfig,
    pub right_stick: StickConfig,
    pub l2: TriggerConfig,
    pub r2: TriggerConfig,
    /// Overrides of the default button targets, by button name
    /// (see [`Buttons::ALL`]).
    pub buttons: Vec<(String, ButtonTarget)>,
    /// Swap the left and right sticks.
    pub swap_sticks: bool,
}

const N_BUTTONS: usize = Buttons::ALL.len();

/// Compiled mapping, ready for the input thread.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mapper {
    /// For each entry of [`Buttons::ALL`]: XInput bits, and trigger flags.
    targets: [(u32, u16, u8); N_BUTTONS],
    left_stick: StickConfig,
    right_stick: StickConfig,
    l2: TriggerConfig,
    r2: TriggerConfig,
    swap_sticks: bool,
}

const FORCE_LT: u8 = 1;
const FORCE_RT: u8 = 2;

impl MappingConfig {
    pub fn compile(&self) -> Mapper {
        let mut targets = [(0u32, 0u16, 0u8); N_BUTTONS];
        for (i, (button, name)) in Buttons::ALL.iter().enumerate() {
            let target = self
                .buttons
                .iter()
                .rev()
                .find(|(n, _)| n == name)
                .map_or_else(|| default_target(*button), |(_, t)| *t);
            let force = match target {
                ButtonTarget::LeftTrigger => FORCE_LT,
                ButtonTarget::RightTrigger => FORCE_RT,
                _ => 0,
            };
            targets[i] = (button.0, target.bits(), force);
        }
        Mapper {
            targets,
            left_stick: self.left_stick,
            right_stick: self.right_stick,
            l2: self.l2,
            r2: self.r2,
            swap_sticks: self.swap_sticks,
        }
    }
}

impl Default for Mapper {
    fn default() -> Self {
        MappingConfig::default().compile()
    }
}

impl Mapper {
    /// Converts a controller state to the virtual controller state.
    #[inline]
    pub fn map(&self, s: &ControllerState) -> XInputState {
        let mut buttons = 0u16;
        let mut force = 0u8;
        for &(bit, target, f) in &self.targets {
            if s.buttons.0 & bit != 0 {
                buttons |= target;
                force |= f;
            }
        }
        let (ls, rs) = if self.swap_sticks {
            (s.right_stick, s.left_stick)
        } else {
            (s.left_stick, s.right_stick)
        };
        let (lx, ly) = map_stick(ls, &self.left_stick);
        let (rx, ry) = map_stick(rs, &self.right_stick);
        XInputState {
            buttons,
            left_trigger: if force & FORCE_LT != 0 {
                255
            } else {
                map_trigger(s.l2, &self.l2)
            },
            right_trigger: if force & FORCE_RT != 0 {
                255
            } else {
                map_trigger(s.r2, &self.r2)
            },
            thumb_lx: lx,
            thumb_ly: ly,
            thumb_rx: rx,
            thumb_ry: ry,
        }
    }
}

/// Raw axis byte to -1..=1 (up/right positive after the caller flips Y).
#[inline]
fn axis(v: u8) -> f32 {
    // 0x80 is center; 0 and 255 are the ends.
    if v >= 128 {
        (v as f32 - 128.0) / 127.0
    } else {
        (v as f32 - 128.0) / 128.0
    }
}

#[inline]
fn to_i16(v: f32) -> i16 {
    let v = v.clamp(-1.0, 1.0);
    if v >= 0.0 {
        (v * 32767.0).round() as i16
    } else {
        (v * 32768.0).round() as i16
    }
}

/// Applies dead zone, outer radius, anti-dead zone and curve to a stick.
pub fn map_stick(stick: Stick, c: &StickConfig) -> (i16, i16) {
    let mut x = axis(stick.x);
    // Controllers report Y growing downward; XInput wants up positive.
    let mut y = -axis(stick.y);
    if c.invert_x {
        x = -x;
    }
    if c.invert_y {
        y = -y;
    }
    if c.digital {
        return digital_stick(x, y, c);
    }
    let is_default = c.deadzone <= 0.0
        && c.outer >= 1.0
        && c.anti_deadzone <= 0.0
        && (c.curve - 1.0).abs() < f32::EPSILON;
    if is_default {
        return (to_i16(x), to_i16(y));
    }
    let mag = (x * x + y * y).sqrt();
    let dz = c.deadzone.clamp(0.0, 0.99);
    if mag <= dz || mag == 0.0 {
        return (0, 0);
    }
    let outer = c.outer.clamp(dz + 0.01, 1.0);
    let t = ((mag - dz) / (outer - dz)).min(1.0);
    let t = t.powf(c.curve.clamp(0.1, 10.0));
    let ad = c.anti_deadzone.clamp(0.0, 0.99);
    let out = ad + (1.0 - ad) * t;
    // Keep the direction; the circle's diagonal may exceed the square's
    // range after scaling, which to_i16 clamps.
    let k = out / mag;
    (to_i16(x * k), to_i16(y * k))
}

/// Digital mode: full deflection in one of 8 (or 4) directions, or nothing.
fn digital_stick(x: f32, y: f32, c: &StickConfig) -> (i16, i16) {
    let threshold = c.digital_threshold.clamp(0.05, 0.95);
    if x * x + y * y < threshold * threshold {
        return (0, 0);
    }
    let (ax, ay) = (x.abs(), y.abs());
    let full = |v: f32| if v < 0.0 { -32768 } else { 32767 };
    // Diagonal when the angle to the nearest axis is past the straight
    // zone: tan(angle) = minor / major component.
    let half_diag = c.diagonal_width.clamp(0.0, 60.0) / 2.0;
    let limit = (45.0 - half_diag).to_radians().tan();
    let (minor, major) = if ax < ay { (ax, ay) } else { (ay, ax) };
    if half_diag > 0.0 && minor >= major * limit {
        (full(x), full(y))
    } else if ax >= ay {
        (full(x), 0)
    } else {
        (0, full(y))
    }
}

/// Applies dead zone, max and curve to a trigger.
pub fn map_trigger(v: u8, c: &TriggerConfig) -> u8 {
    if c.deadzone == 0 && c.max == 255 && (c.curve - 1.0).abs() < f32::EPSILON {
        return v;
    }
    if v <= c.deadzone {
        return 0;
    }
    let max = c.max.max(c.deadzone.saturating_add(1));
    if v >= max {
        return 255;
    }
    let t = (v - c.deadzone) as f32 / (max - c.deadzone) as f32;
    (t.powf(c.curve.clamp(0.1, 10.0)) * 255.0).round() as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(buttons: Buttons) -> ControllerState {
        ControllerState {
            buttons,
            ..ControllerState::default()
        }
    }

    #[test]
    fn default_buttons() {
        let m = Mapper::default();
        let x = m.map(&state(
            Buttons::CROSS
                | Buttons::TRIANGLE
                | Buttons::OPTIONS
                | Buttons::DPAD_LEFT
                | Buttons::PS,
        ));
        use xbutton::*;
        assert_eq!(x.buttons, A | Y | START | DPAD_LEFT | GUIDE);
        assert_eq!(m.map(&state(Buttons::MUTE)).buttons, 0);
    }

    #[test]
    fn remapped_buttons() {
        let cfg = MappingConfig {
            buttons: vec![
                ("cross".into(), ButtonTarget::B),
                ("circle".into(), ButtonTarget::A),
                ("left_paddle".into(), ButtonTarget::RightTrigger),
                ("l1".into(), ButtonTarget::None),
            ],
            ..MappingConfig::default()
        };
        let m = cfg.compile();
        let x = m.map(&state(Buttons::CROSS | Buttons::L1 | Buttons::LEFT_PADDLE));
        assert_eq!(x.buttons, xbutton::B);
        assert_eq!(x.right_trigger, 255);
        assert_eq!(x.left_trigger, 0);
    }

    #[test]
    fn stick_ends_and_center() {
        let c = StickConfig::default();
        assert_eq!(map_stick(Stick { x: 128, y: 128 }, &c), (0, 0));
        assert_eq!(map_stick(Stick { x: 255, y: 0 }, &c), (32767, 32767));
        assert_eq!(map_stick(Stick { x: 0, y: 255 }, &c), (-32768, -32768));
        let inv = StickConfig {
            invert_y: true,
            ..c
        };
        assert_eq!(map_stick(Stick { x: 128, y: 0 }, &inv), (0, -32768));
    }

    #[test]
    fn stick_deadzone() {
        let c = StickConfig {
            deadzone: 0.2,
            ..StickConfig::default()
        };
        // 10 % push: ignored.
        assert_eq!(map_stick(Stick { x: 141, y: 128 }, &c), (0, 0));
        // Full push: still full.
        assert_eq!(map_stick(Stick { x: 255, y: 128 }, &c), (32767, 0));
        // 60 % push maps to 50 % after rescaling.
        let (x, _) = map_stick(
            Stick {
                x: 128 + 76,
                y: 128,
            },
            &c,
        );
        assert!((x - 16384).abs() < 200, "{x}");
        let ad = StickConfig {
            deadzone: 0.2,
            anti_deadzone: 0.25,
            ..StickConfig::default()
        };
        let (x, _) = map_stick(
            Stick {
                x: 128 + 26,
                y: 128,
            },
            &ad,
        );
        assert!((x - 8192).abs() < 400, "{x}");
    }

    #[test]
    fn triggers() {
        let c = TriggerConfig {
            deadzone: 55,
            max: 155,
            curve: 1.0,
        };
        assert_eq!(map_trigger(50, &c), 0);
        assert_eq!(map_trigger(105, &c), 128);
        assert_eq!(map_trigger(200, &c), 255);
        assert_eq!(map_trigger(77, &TriggerConfig::default()), 77);
    }

    #[test]
    fn digital_stick_snaps_to_directions() {
        let c = StickConfig {
            digital: true,
            ..StickConfig::default()
        };
        let at = |x: u8, y: u8| map_stick(Stick { x, y }, &c);
        // Below the threshold: nothing, even close to it.
        assert_eq!(at(128, 128), (0, 0));
        assert_eq!(at(180, 128), (0, 0));
        // A side push slightly down stays a pure side input.
        assert_eq!(at(255, 160), (32767, 0));
        assert_eq!(at(0, 100), (-32768, 0));
        // Down (y grows downward on the controller, XInput up is positive).
        assert_eq!(at(140, 255), (0, -32768));
        // A real diagonal is both axes at full.
        assert_eq!(at(240, 240), (32767, -32768));
        // 4 directions only: diagonals pick the dominant axis.
        let four = StickConfig {
            diagonal_width: 0.0,
            ..c
        };
        assert_eq!(map_stick(Stick { x: 250, y: 240 }, &four), (32767, 0));
        assert_eq!(map_stick(Stick { x: 240, y: 250 }, &four), (0, -32768));
        // Inversion still applies.
        let inv = StickConfig {
            invert_x: true,
            ..c
        };
        assert_eq!(map_stick(Stick { x: 255, y: 128 }, &inv), (-32768, 0));
    }

    #[test]
    fn swap_sticks() {
        let cfg = MappingConfig {
            swap_sticks: true,
            ..MappingConfig::default()
        };
        let mut s = state(Buttons::NONE);
        s.left_stick.x = 255;
        let x = cfg.compile().map(&s);
        assert_eq!((x.thumb_lx, x.thumb_rx), (0, 32767));
    }
}
