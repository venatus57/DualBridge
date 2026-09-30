//! Output report building: lightbar, rumble, player LEDs, mic LED and
//! DualSense adaptive triggers.
//!
//! | Report                      | Length | Payload starts at |
//! |-----------------------------|--------|-------------------|
//! | DS4 USB `0x05`              | 32     | 1                 |
//! | DS4 Bluetooth `0x11`        | 78     | 3 (CRC32 at 74)   |
//! | DualSense USB `0x02`        | 48     | 1                 |
//! | DualSense Bluetooth `0x31`  | 78     | 3 (CRC32 at 74)   |
//!
//! The Switch Pro Controller's reports are built by [`crate::switch`].
//!
//! Building a report never allocates.

use serde::{Deserialize, Serialize};

use crate::color::Rgb;
use crate::crc;
use crate::device::{Model, Transport};
use crate::switch::SwitchOutput;

/// Largest output report of any supported controller.
pub const MAX_OUTPUT_LEN: usize = 78;

pub const REPORT_DS4_USB: u8 = 0x05;
pub const REPORT_DS4_BT: u8 = 0x11;
pub const REPORT_DUALSENSE_USB: u8 = 0x02;
pub const REPORT_DUALSENSE_BT: u8 = 0x31;

/// Lightbar blinking (DualShock 4 only). Durations are in units of 10 ms.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Flash {
    pub on: u8,
    pub off: u8,
}

/// DualSense microphone LED.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MicLed {
    #[default]
    Off,
    On,
    Pulse,
}

/// Brightness of the DualSense player LEDs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlayerLedBrightness {
    #[default]
    High,
    Medium,
    Low,
}

/// The five DualSense player LEDs under the touchpad, bit 0 being the leftmost.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PlayerLeds(pub u8);

impl PlayerLeds {
    pub const OFF: PlayerLeds = PlayerLeds(0);
    pub const ALL: PlayerLeds = PlayerLeds(0b11111);

    /// The pattern a PlayStation 5 uses for players 1 to 4 (and all five LEDs
    /// for player 5). Higher slots cycle back.
    pub fn for_player(player: u8) -> PlayerLeds {
        const PATTERNS: [u8; 5] = [0b00100, 0b01010, 0b10101, 0b11011, 0b11111];
        match player {
            0 => PlayerLeds::OFF,
            n => PlayerLeds(PATTERNS[(n as usize - 1) % PATTERNS.len()]),
        }
    }

    /// Lights `count` LEDs from the left (0 to 5), e.g. as a battery gauge.
    pub fn bar(count: u8) -> PlayerLeds {
        PlayerLeds(((1u16 << count.min(5)) - 1) as u8)
    }
}

/// A DualSense adaptive trigger effect.
///
/// Zones split the trigger travel into 10 steps (0 = released, 9 = fully
/// pressed); strengths and amplitudes go from 1 (weakest) to 8 (strongest).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum TriggerEffect {
    /// No resistance.
    #[default]
    Off,
    /// Constant resistance from `position` to the end of the travel.
    Feedback { position: u8, strength: u8 },
    /// A resistance "wall" between `start` (2..=7) and `end` (start+1..=8) that
    /// gives way with a click, like a gun trigger.
    Weapon { start: u8, end: u8, strength: u8 },
    /// Vibration from `position` to the end, `frequency` in Hz (1..=255).
    Vibration {
        position: u8,
        amplitude: u8,
        frequency: u8,
    },
    /// Raw 11-byte effect block, for experimenting with other modes.
    Raw { bytes: [u8; 11] },
}

impl TriggerEffect {
    /// Encodes the effect as the 11-byte block of the DualSense output report.
    /// Out-of-range parameters are clamped; impossible ones turn the effect off.
    pub fn to_bytes(self) -> [u8; 11] {
        let mut out = [0u8; 11];
        match self {
            TriggerEffect::Off => out[0] = 0x05,
            TriggerEffect::Feedback { position, strength } => {
                let (zones, forces) = zones(position, strength);
                out[0] = 0x21;
                out[1..3].copy_from_slice(&zones.to_le_bytes());
                out[3..7].copy_from_slice(&forces.to_le_bytes());
            }
            TriggerEffect::Weapon {
                start,
                end,
                strength,
            } => {
                let start = start.clamp(2, 7);
                let end = end.clamp(start + 1, 8);
                let strength = strength.clamp(1, 8);
                let mask: u16 = (1 << start) | (1 << end);
                out[0] = 0x25;
                out[1..3].copy_from_slice(&mask.to_le_bytes());
                out[3] = strength - 1;
            }
            TriggerEffect::Vibration {
                position,
                amplitude,
                frequency,
            } => {
                if frequency == 0 {
                    out[0] = 0x05;
                    return out;
                }
                let (zones, amps) = zones(position, amplitude);
                out[0] = 0x26;
                out[1..3].copy_from_slice(&zones.to_le_bytes());
                out[3..7].copy_from_slice(&amps.to_le_bytes());
                out[9] = frequency;
            }
            TriggerEffect::Raw { bytes } => out = bytes,
        }
        out
    }
}

/// Active-zone mask and packed 3-bit per-zone values, from `position` to zone 9.
fn zones(position: u8, value: u8) -> (u16, u32) {
    let position = position.min(9);
    let v = (value.clamp(1, 8) - 1) as u32;
    let mut mask = 0u16;
    let mut packed = 0u32;
    for zone in position..10 {
        mask |= 1 << zone;
        packed |= v << (3 * zone as u32);
    }
    (mask, packed)
}

/// Everything DualBridge can set on a controller.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct OutputState {
    /// Left, low-frequency ("heavy") motor.
    pub rumble_strong: u8,
    /// Right, high-frequency ("light") motor.
    pub rumble_weak: u8,
    pub lightbar: Rgb,
    /// DualShock 4 hardware blinking. Ignored on DualSense.
    pub flash: Option<Flash>,
    pub player_leds: PlayerLeds,
    pub player_led_brightness: PlayerLedBrightness,
    pub mic_led: MicLed,
    pub left_trigger: TriggerEffect,
    pub right_trigger: TriggerEffect,
    /// DualSense: fade out the blue startup light so our lightbar color
    /// shows. Send it once, in the first report after connecting.
    pub release_startup_light: bool,
}

/// Builds output reports for one controller, tracking the Bluetooth
/// sequence number.
#[derive(Debug, Clone)]
pub struct OutputBuilder {
    model: Model,
    transport: Transport,
    seq: u8,
    switch: SwitchOutput,
    buf: [u8; MAX_OUTPUT_LEN],
}

impl OutputBuilder {
    pub fn new(model: Model, transport: Transport) -> OutputBuilder {
        OutputBuilder {
            model,
            transport,
            seq: 0,
            switch: SwitchOutput::default(),
            buf: [0; MAX_OUTPUT_LEN],
        }
    }

    /// Builds the report for `state` and returns it, ready to be written.
    pub fn build(&mut self, state: &OutputState) -> &[u8] {
        self.buf = [0; MAX_OUTPUT_LEN];
        if self.model.is_switch() {
            let len = self.switch.build(state, &mut self.buf);
            return &self.buf[..len];
        }
        let len = match (self.model.is_dualsense(), self.transport) {
            (false, Transport::Usb) => {
                self.buf[0] = REPORT_DS4_USB;
                ds4_payload(state, &mut self.buf[1..]);
                32
            }
            (false, Transport::Bluetooth) => {
                self.buf[0] = REPORT_DS4_BT;
                // Enable HID output and CRC; low bits = poll interval (0 = fastest).
                self.buf[1] = 0xC0;
                self.buf[2] = 0x00;
                ds4_payload(state, &mut self.buf[3..]);
                crc::sign(crc::SEED_OUTPUT, &mut self.buf[..78]);
                78
            }
            (true, Transport::Usb) => {
                self.buf[0] = REPORT_DUALSENSE_USB;
                dualsense_payload(state, &mut self.buf[1..]);
                48
            }
            (true, Transport::Bluetooth) => {
                self.buf[0] = REPORT_DUALSENSE_BT;
                self.buf[1] = self.seq << 4;
                self.buf[2] = 0x10;
                self.seq = (self.seq + 1) & 0x0F;
                dualsense_payload(state, &mut self.buf[3..]);
                crc::sign(crc::SEED_OUTPUT, &mut self.buf[..78]);
                78
            }
        };
        &self.buf[..len]
    }
}

/// DS4 payload (after the report ID / Bluetooth header).
fn ds4_payload(s: &OutputState, p: &mut [u8]) {
    p[0] = 0x07; // rumble | lightbar | flash
    p[3] = s.rumble_weak;
    p[4] = s.rumble_strong;
    p[5] = s.lightbar.r;
    p[6] = s.lightbar.g;
    p[7] = s.lightbar.b;
    let flash = s.flash.unwrap_or_default();
    p[8] = flash.on;
    p[9] = flash.off;
}

/// DualSense payload (the 47-byte common block).
fn dualsense_payload(s: &OutputState, p: &mut [u8]) {
    // Flags 0: compatible rumble + haptics select, right/left trigger effects.
    p[0] = 0x01 | 0x02 | 0x04 | 0x08;
    // Flags 1: mic LED, lightbar, player LEDs.
    p[1] = 0x01 | 0x04 | 0x10;
    p[2] = s.rumble_weak;
    p[3] = s.rumble_strong;
    p[8] = match s.mic_led {
        MicLed::Off => 0,
        MicLed::On => 1,
        MicLed::Pulse => 2,
    };
    p[10..21].copy_from_slice(&s.right_trigger.to_bytes());
    p[21..32].copy_from_slice(&s.left_trigger.to_bytes());
    // Flags 2: player LED brightness.
    p[38] = 0x01;
    if s.release_startup_light {
        p[38] |= 0x02;
        p[41] = 0x02;
    }
    p[42] = match s.player_led_brightness {
        PlayerLedBrightness::High => 0,
        PlayerLedBrightness::Medium => 1,
        PlayerLedBrightness::Low => 2,
    };
    p[43] = s.player_leds.0 & 0x1F;
    p[44] = s.lightbar.r;
    p[45] = s.lightbar.g;
    p[46] = s.lightbar.b;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn player_patterns() {
        assert_eq!(PlayerLeds::for_player(1), PlayerLeds(0b00100));
        assert_eq!(PlayerLeds::for_player(4), PlayerLeds(0b11011));
        assert_eq!(PlayerLeds::for_player(6), PlayerLeds(0b00100));
        assert_eq!(PlayerLeds::for_player(0), PlayerLeds::OFF);
        assert_eq!(PlayerLeds::bar(0), PlayerLeds(0));
        assert_eq!(PlayerLeds::bar(3), PlayerLeds(0b00111));
        assert_eq!(PlayerLeds::bar(9), PlayerLeds::ALL);
    }

    #[test]
    fn trigger_off() {
        assert_eq!(
            TriggerEffect::Off.to_bytes(),
            [5, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
        );
    }

    #[test]
    fn trigger_feedback() {
        let b = TriggerEffect::Feedback {
            position: 8,
            strength: 8,
        }
        .to_bytes();
        // Zones 8 and 9, value 7 each: 7 << 24 | 7 << 27.
        let forces: u32 = (7 << 24) | (7 << 27);
        assert_eq!(b[0], 0x21);
        assert_eq!(u16::from_le_bytes([b[1], b[2]]), 0b11 << 8);
        assert_eq!(u32::from_le_bytes([b[3], b[4], b[5], b[6]]), forces);
    }

    #[test]
    fn trigger_weapon() {
        let b = TriggerEffect::Weapon {
            start: 4,
            end: 6,
            strength: 5,
        }
        .to_bytes();
        assert_eq!(&b[..4], &[0x25, 0b0101_0000, 0, 4]);
        // Clamping: end must be after start.
        let b = TriggerEffect::Weapon {
            start: 9,
            end: 1,
            strength: 0,
        }
        .to_bytes();
        assert_eq!(&b[..4], &[0x25, 0x80, 0x01, 0]);
    }

    #[test]
    fn trigger_vibration() {
        let b = TriggerEffect::Vibration {
            position: 0,
            amplitude: 1,
            frequency: 30,
        }
        .to_bytes();
        assert_eq!(b[0], 0x26);
        assert_eq!(u16::from_le_bytes([b[1], b[2]]), 0x3FF);
        assert_eq!(&b[3..7], &[0, 0, 0, 0]);
        assert_eq!(b[9], 30);
        let off = TriggerEffect::Vibration {
            position: 0,
            amplitude: 1,
            frequency: 0,
        };
        assert_eq!(off.to_bytes(), TriggerEffect::Off.to_bytes());
    }

    #[test]
    fn trigger_effect_serde() {
        let e = TriggerEffect::Weapon {
            start: 3,
            end: 5,
            strength: 8,
        };
        let json = serde_json::to_string(&e).unwrap();
        assert_eq!(json, r#"{"mode":"weapon","start":3,"end":5,"strength":8}"#);
        assert_eq!(serde_json::from_str::<TriggerEffect>(&json).unwrap(), e);
    }
}
