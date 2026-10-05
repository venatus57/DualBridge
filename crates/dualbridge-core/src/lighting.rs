//! Lighting effects engine.
//!
//! [`LightingConfig::render`] is a pure function of the configuration and a
//! [`LightingContext`] (time, slot, battery...). That keeps effects
//! deterministic, easy to test, and lets the UI preview exactly what the
//! controller will show by calling the same code.

use serde::{Deserialize, Serialize};

use crate::color::Rgb;
use crate::output::{MicLed, PlayerLedBrightness, PlayerLeds};
use crate::state::{Battery, Charging};

/// Default lightbar color for each slot (1-based slot = index + 1).
pub const SLOT_COLORS: [Rgb; 8] = [
    Rgb::new(0, 64, 255),   // blue
    Rgb::new(255, 16, 16),  // red
    Rgb::new(0, 220, 60),   // green
    Rgb::new(255, 40, 160), // pink
    Rgb::new(255, 120, 0),  // orange
    Rgb::new(0, 220, 220),  // cyan
    Rgb::new(140, 40, 255), // purple
    Rgb::new(255, 210, 0),  // yellow
];

/// Default lightbar color for a 1-based slot number.
pub fn slot_color(slot: u8) -> Rgb {
    SLOT_COLORS[(slot.max(1) as usize - 1) % SLOT_COLORS.len()]
}

/// What the lightbar does.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Effect {
    /// Lightbar off.
    Off,
    /// The slot's default color.
    #[default]
    SlotColor,
    /// One fixed color.
    Static { color: Rgb },
    /// Fades in and out.
    Breathing {
        color: Rgb,
        period_ms: u32,
        /// Lowest brightness of the cycle, 0 to 1.
        min_level: f32,
    },
    /// Cycles through every hue.
    Rainbow {
        period_ms: u32,
        /// 0 (white) to 1 (vivid).
        saturation: f32,
    },
    /// Goes through a list of colors, fading or jumping between them.
    ColorCycle {
        colors: Vec<Rgb>,
        /// Time spent on each color.
        step_ms: u32,
        smooth: bool,
    },
    /// Flashes on and off.
    Strobe { color: Rgb, on_ms: u32, off_ms: u32 },
    /// Color follows the battery level, from `empty` to `full`.
    BatteryLevel { empty: Rgb, full: Rgb },
}

/// Pulse a warning color when the battery gets low.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LowBatteryWarning {
    pub enabled: bool,
    /// Warn at or below this percentage.
    pub threshold: u8,
    pub color: Rgb,
    pub period_ms: u32,
}

impl Default for LowBatteryWarning {
    fn default() -> Self {
        LowBatteryWarning {
            enabled: true,
            threshold: 15,
            color: Rgb::RED,
            period_ms: 1500,
        }
    }
}

/// What the DualSense player LEDs show.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PlayerLedMode {
    Off,
    /// The PlayStation-style pattern for the slot number.
    #[default]
    SlotNumber,
    /// Battery gauge: one LED per 20 %.
    Battery,
    /// A fixed pattern (bits 0 to 4, left to right).
    Custom {
        pattern: u8,
    },
}

/// What the DualSense microphone LED shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MicLedMode {
    Off,
    On,
    Pulse,
    /// On while the microphone is muted with the mute button.
    #[default]
    ShowMute,
}

/// Complete lighting settings for a controller.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LightingConfig {
    pub effect: Effect,
    /// Overall lightbar brightness, 0 to 1.
    pub brightness: f32,
    pub low_battery: LowBatteryWarning,
    /// Show a steady "charging" breathing effect while plugged in.
    pub charging_indicator: bool,
    /// Offset animated effects by slot so several controllers form a wave.
    pub offset_by_slot: bool,
    pub player_leds: PlayerLedMode,
    pub player_led_brightness: PlayerLedBrightness,
    pub mic_led: MicLedMode,
}

impl Default for LightingConfig {
    fn default() -> Self {
        LightingConfig {
            effect: Effect::SlotColor,
            brightness: 1.0,
            low_battery: LowBatteryWarning::default(),
            charging_indicator: false,
            offset_by_slot: false,
            player_leds: PlayerLedMode::SlotNumber,
            player_led_brightness: PlayerLedBrightness::Medium,
            mic_led: MicLedMode::ShowMute,
        }
    }
}

/// Inputs to an effect besides its configuration.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct LightingContext {
    /// Milliseconds since an arbitrary, fixed origin (e.g. app start).
    pub time_ms: u64,
    /// 1-based slot number.
    pub slot: u8,
    pub battery: Battery,
    pub mic_muted: bool,
}

/// The rendered frame: what to send to the controller.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct LightFrame {
    pub lightbar: Rgb,
    pub player_leds: PlayerLeds,
    pub player_led_brightness: PlayerLedBrightness,
    pub mic_led: MicLed,
}

/// Position in a repeating cycle of `period_ms`, from 0 to 1.
fn phase(time_ms: u64, period_ms: u32) -> f32 {
    let period = period_ms.max(1) as u64;
    (time_ms % period) as f32 / period as f32
}

/// Smooth 0 → 1 → 0 wave over one cycle.
fn triangle_smooth(p: f32) -> f32 {
    0.5 - 0.5 * (p * std::f32::consts::TAU).cos()
}

impl LightingConfig {
    /// Computes the lighting for one instant.
    pub fn render(&self, ctx: &LightingContext) -> LightFrame {
        let mut time = ctx.time_ms;
        if self.offset_by_slot {
            // Shift each slot by 1/8 of the effect's cycle.
            let cycle = self.effect_cycle_ms() as u64;
            time += cycle * (ctx.slot.saturating_sub(1) as u64 % 8) / 8;
        }

        let low = self.low_battery;
        let battery_low = low.enabled
            && ctx.battery.charging == Charging::Discharging
            && ctx.battery.percent <= low.threshold;

        let lightbar = if battery_low {
            let level = 0.15 + 0.85 * triangle_smooth(phase(time, low.period_ms));
            low.color.scale(level)
        } else {
            let base = self.effect_color(time, ctx);
            if self.charging_indicator && ctx.battery.charging == Charging::Charging {
                base.scale(0.3 + 0.7 * triangle_smooth(phase(time, 3000)))
            } else {
                base
            }
        };

        let player_leds = match self.player_leds {
            PlayerLedMode::Off => PlayerLeds::OFF,
            PlayerLedMode::SlotNumber => PlayerLeds::for_player(ctx.slot),
            PlayerLedMode::Battery => PlayerLeds::bar(ctx.battery.percent.div_ceil(20)),
            PlayerLedMode::Custom { pattern } => PlayerLeds(pattern & 0x1F),
        };

        let mic_led = match self.mic_led {
            MicLedMode::Off => MicLed::Off,
            MicLedMode::On => MicLed::On,
            MicLedMode::Pulse => MicLed::Pulse,
            MicLedMode::ShowMute if ctx.mic_muted => MicLed::On,
            MicLedMode::ShowMute => MicLed::Off,
        };

        LightFrame {
            lightbar: lightbar.scale(self.brightness),
            player_leds,
            player_led_brightness: self.player_led_brightness,
            mic_led,
        }
    }

    /// Length of the effect's animation cycle, used for slot offsets.
    fn effect_cycle_ms(&self) -> u32 {
        match &self.effect {
            Effect::Breathing { period_ms, .. } | Effect::Rainbow { period_ms, .. } => *period_ms,
            Effect::ColorCycle {
                colors, step_ms, ..
            } => step_ms.saturating_mul(colors.len() as u32),
            Effect::Strobe { on_ms, off_ms, .. } => on_ms.saturating_add(*off_ms),
            _ => 0,
        }
    }

    fn effect_color(&self, time: u64, ctx: &LightingContext) -> Rgb {
        match &self.effect {
            Effect::Off => Rgb::BLACK,
            Effect::SlotColor => slot_color(ctx.slot),
            Effect::Static { color } => *color,
            Effect::Breathing {
                color,
                period_ms,
                min_level,
            } => {
                let min = min_level.clamp(0.0, 1.0);
                color.scale(min + (1.0 - min) * triangle_smooth(phase(time, *period_ms)))
            }
            Effect::Rainbow {
                period_ms,
                saturation,
            } => Rgb::from_hsv(phase(time, *period_ms) * 360.0, *saturation, 1.0),
            Effect::ColorCycle {
                colors,
                step_ms,
                smooth,
            } => {
                if colors.is_empty() {
                    return Rgb::BLACK;
                }
                let step = (*step_ms).max(1) as u64;
                let index = (time / step) as usize % colors.len();
                let current = colors[index];
                if *smooth {
                    let next = colors[(index + 1) % colors.len()];
                    current.lerp(next, (time % step) as f32 / step as f32)
                } else {
                    current
                }
            }
            Effect::Strobe {
                color,
                on_ms,
                off_ms,
            } => {
                let period = on_ms.saturating_add(*off_ms).max(1) as u64;
                if time % period < *on_ms as u64 {
                    *color
                } else {
                    Rgb::BLACK
                }
            }
            Effect::BatteryLevel { empty, full } => {
                empty.lerp(*full, ctx.battery.percent.min(100) as f32 / 100.0)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(time_ms: u64) -> LightingContext {
        LightingContext {
            time_ms,
            slot: 1,
            battery: Battery {
                percent: 80,
                charging: Charging::Discharging,
                cable: false,
            },
            mic_muted: false,
        }
    }

    fn with(effect: Effect) -> LightingConfig {
        LightingConfig {
            effect,
            ..LightingConfig::default()
        }
    }

    #[test]
    fn slot_colors_cycle() {
        assert_eq!(slot_color(1), SLOT_COLORS[0]);
        assert_eq!(slot_color(9), SLOT_COLORS[0]);
        assert_eq!(slot_color(0), SLOT_COLORS[0]);
        let c = LightingConfig::default();
        let mut x = ctx(0);
        x.slot = 3;
        assert_eq!(c.render(&x).lightbar, SLOT_COLORS[2]);
        assert_eq!(c.render(&x).player_leds, PlayerLeds::for_player(3));
    }

    #[test]
    fn static_and_brightness() {
        let mut c = with(Effect::Static {
            color: Rgb::new(200, 100, 50),
        });
        assert_eq!(c.render(&ctx(123)).lightbar, Rgb::new(200, 100, 50));
        c.brightness = 0.5;
        assert_eq!(c.render(&ctx(123)).lightbar, Rgb::new(100, 50, 25));
        c.brightness = 0.0;
        assert_eq!(c.render(&ctx(123)).lightbar, Rgb::BLACK);
    }

    #[test]
    fn breathing_cycle() {
        let c = with(Effect::Breathing {
            color: Rgb::WHITE,
            period_ms: 1000,
            min_level: 0.0,
        });
        assert_eq!(c.render(&ctx(0)).lightbar, Rgb::BLACK);
        assert_eq!(c.render(&ctx(500)).lightbar, Rgb::WHITE);
        assert_eq!(c.render(&ctx(1000)).lightbar, Rgb::BLACK);
    }

    #[test]
    fn rainbow_and_offset() {
        let mut c = with(Effect::Rainbow {
            period_ms: 3000,
            saturation: 1.0,
        });
        assert_eq!(c.render(&ctx(0)).lightbar, Rgb::RED);
        assert_eq!(c.render(&ctx(1000)).lightbar, Rgb::GREEN);
        assert_eq!(c.render(&ctx(2000)).lightbar, Rgb::BLUE);
        c.offset_by_slot = true;
        let mut x = ctx(0);
        x.slot = 5; // half a cycle later
        assert_eq!(c.render(&x).lightbar, Rgb::new(0, 255, 255));
    }

    #[test]
    fn color_cycle() {
        let colors = vec![Rgb::RED, Rgb::BLUE];
        let hard = with(Effect::ColorCycle {
            colors: colors.clone(),
            step_ms: 100,
            smooth: false,
        });
        assert_eq!(hard.render(&ctx(50)).lightbar, Rgb::RED);
        assert_eq!(hard.render(&ctx(150)).lightbar, Rgb::BLUE);
        assert_eq!(hard.render(&ctx(250)).lightbar, Rgb::RED);
        let smooth = with(Effect::ColorCycle {
            colors,
            step_ms: 100,
            smooth: true,
        });
        assert_eq!(smooth.render(&ctx(50)).lightbar, Rgb::new(128, 0, 128));
        let empty = with(Effect::ColorCycle {
            colors: vec![],
            step_ms: 100,
            smooth: true,
        });
        assert_eq!(empty.render(&ctx(50)).lightbar, Rgb::BLACK);
    }

    #[test]
    fn strobe() {
        let c = with(Effect::Strobe {
            color: Rgb::WHITE,
            on_ms: 50,
            off_ms: 150,
        });
        assert_eq!(c.render(&ctx(10)).lightbar, Rgb::WHITE);
        assert_eq!(c.render(&ctx(60)).lightbar, Rgb::BLACK);
        assert_eq!(c.render(&ctx(210)).lightbar, Rgb::WHITE);
    }

    #[test]
    fn battery_level_and_warning() {
        let c = with(Effect::BatteryLevel {
            empty: Rgb::RED,
            full: Rgb::GREEN,
        });
        let mut x = ctx(0);
        x.battery.percent = 100;
        assert_eq!(c.render(&x).lightbar, Rgb::GREEN);
        x.battery.percent = 50;
        assert_eq!(c.render(&x).lightbar, Rgb::new(128, 128, 0));

        // Low battery overrides the effect with a pulsing warning...
        x.battery.percent = 10;
        x.time_ms = 750; // peak of the 1500 ms pulse
        assert_eq!(c.render(&x).lightbar, Rgb::RED);
        // ...but not while charging.
        x.battery.charging = Charging::Charging;
        assert_eq!(
            c.render(&x).lightbar,
            Rgb::new(255, 0, 0).lerp(Rgb::GREEN, 0.1)
        );
    }

    #[test]
    fn player_and_mic_leds() {
        let mut c = LightingConfig {
            player_leds: PlayerLedMode::Battery,
            ..LightingConfig::default()
        };
        let mut x = ctx(0);
        x.battery.percent = 45;
        assert_eq!(c.render(&x).player_leds, PlayerLeds(0b00111));
        c.player_leds = PlayerLedMode::Custom { pattern: 0xFF };
        assert_eq!(c.render(&x).player_leds, PlayerLeds::ALL);
        assert_eq!(c.render(&x).mic_led, MicLed::Off);
        x.mic_muted = true;
        assert_eq!(c.render(&x).mic_led, MicLed::On);
    }

    #[test]
    fn config_serde_defaults() {
        let c: LightingConfig =
            serde_json::from_str(r##"{"effect":{"type":"static","color":{"r":1,"g":2,"b":3}}}"##)
                .unwrap();
        assert_eq!(
            c.effect,
            Effect::Static {
                color: Rgb::new(1, 2, 3)
            }
        );
        assert_eq!(c.brightness, 1.0);
        let json = serde_json::to_string(&LightingConfig::default()).unwrap();
        let back: LightingConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back, LightingConfig::default());
    }
}
