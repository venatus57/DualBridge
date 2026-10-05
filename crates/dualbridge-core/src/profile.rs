//! Profiles: named sets of settings, saved as JSON.
//!
//! The on-disk format carries a `version` field. Unknown fields are ignored
//! and missing ones take their defaults, so older and newer files still load.

use serde::{Deserialize, Serialize};

use crate::lighting::LightingConfig;
use crate::mapping::MappingConfig;
use crate::output::TriggerEffect;

/// Current profile format version.
pub const PROFILE_VERSION: u32 = 1;

/// What the virtual controller looks like to games.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VirtualKind {
    /// Xbox 360 controller: works with nearly every PC game.
    #[default]
    Xbox360,
    /// No virtual controller (lighting and battery only; games see the real
    /// controller).
    None,
}

/// A named set of settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Profile {
    pub version: u32,
    pub name: String,
    pub lighting: LightingConfig,
    pub mapping: MappingConfig,
    pub virtual_kind: VirtualKind,
    /// Forward game rumble to the controller.
    pub rumble: bool,
    /// Rumble strength, 0 to 1.
    pub rumble_strength: f32,
    /// DualSense adaptive trigger effects.
    pub left_trigger: TriggerEffect,
    pub right_trigger: TriggerEffect,
    /// Executable names (e.g. `eldenring.exe`) that activate this profile
    /// automatically. Case-insensitive.
    pub games: Vec<String>,
}

impl Default for Profile {
    fn default() -> Self {
        Profile {
            version: PROFILE_VERSION,
            name: "Default".into(),
            lighting: LightingConfig::default(),
            mapping: MappingConfig::default(),
            virtual_kind: VirtualKind::Xbox360,
            rumble: true,
            rumble_strength: 1.0,
            left_trigger: TriggerEffect::Off,
            right_trigger: TriggerEffect::Off,
            games: Vec::new(),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ProfileError {
    #[error("invalid profile: {0}")]
    Invalid(String),
    #[error("profile version {0} is newer than this version of DualBridge supports")]
    TooNew(u32),
}

impl Profile {
    pub fn from_json(json: &str) -> Result<Profile, ProfileError> {
        let p: Profile =
            serde_json::from_str(json).map_err(|e| ProfileError::Invalid(e.to_string()))?;
        if p.version > PROFILE_VERSION {
            return Err(ProfileError::TooNew(p.version));
        }
        Ok(Profile {
            version: PROFILE_VERSION,
            ..p
        })
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("profiles always serialize")
    }

    /// `true` if this profile should activate for the given executable name.
    pub fn matches_game(&self, exe: &str) -> bool {
        let exe = exe.rsplit(['/', '\\']).next().unwrap_or(exe);
        self.games.iter().any(|g| g.eq_ignore_ascii_case(exe))
    }

    /// Scales a rumble motor value by this profile's settings.
    pub fn scale_rumble(&self, v: u8) -> u8 {
        if !self.rumble {
            return 0;
        }
        (v as f32 * self.rumble_strength.clamp(0.0, 1.0)).round() as u8
    }
}

/// Picks the profile for the foreground executable, falling back to `default`.
pub fn profile_for_game<'a>(profiles: &'a [Profile], exe: &str) -> Option<&'a Profile> {
    profiles.iter().find(|p| p.matches_game(exe))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lighting::Effect;
    use crate::Rgb;

    #[test]
    fn round_trip() {
        let mut p = Profile {
            name: "Racing".into(),
            games: vec!["ForzaHorizon5.exe".into()],
            right_trigger: TriggerEffect::Feedback {
                position: 3,
                strength: 6,
            },
            ..Profile::default()
        };
        p.lighting.effect = Effect::Rainbow {
            period_ms: 4000,
            saturation: 1.0,
        };
        let back = Profile::from_json(&p.to_json()).unwrap();
        assert_eq!(back, p);
    }

    #[test]
    fn tolerant_loading() {
        let p = Profile::from_json(
            r##"{"name":"Old","future_field":42,"lighting":{"effect":{"type":"static","color":{"r":255,"g":0,"b":0}}}}"##,
        )
        .unwrap();
        assert_eq!(p.name, "Old");
        assert_eq!(p.version, PROFILE_VERSION);
        assert_eq!(p.lighting.effect, Effect::Static { color: Rgb::RED });
        assert!(p.rumble);
        assert!(matches!(
            Profile::from_json(r#"{"version":99}"#),
            Err(ProfileError::TooNew(99))
        ));
        assert!(Profile::from_json("not json").is_err());
    }

    #[test]
    fn game_matching() {
        let profiles = vec![
            Profile::default(),
            Profile {
                name: "Elden Ring".into(),
                games: vec!["eldenring.exe".into()],
                ..Profile::default()
            },
        ];
        let p = profile_for_game(&profiles, r"C:\Games\ELDEN RING\Game\EldenRing.exe").unwrap();
        assert_eq!(p.name, "Elden Ring");
        assert!(profile_for_game(&profiles, "notepad.exe").is_none());
    }

    #[test]
    fn rumble_scaling() {
        let mut p = Profile {
            rumble_strength: 0.5,
            ..Profile::default()
        };
        assert_eq!(p.scale_rumble(200), 100);
        p.rumble = false;
        assert_eq!(p.scale_rumble(200), 0);
    }
}
