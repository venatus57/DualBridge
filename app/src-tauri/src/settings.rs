//! App settings, saved as `settings.json` in the app's config directory.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use dualbridge_core::lighting::LightingConfig;
use dualbridge_core::profile::Profile;
use serde::{Deserialize, Serialize};

pub const SETTINGS_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub version: u32,
    /// `None` follows the system language.
    pub language: Option<String>,
    pub first_run_done: bool,
    /// Closing the window keeps DualBridge running in the tray.
    pub minimize_to_tray: bool,
    /// Start hidden in the tray (useful with "start with Windows").
    pub start_minimized: bool,
    /// "Start with the computer" was turned on by default once already, so
    /// the user's later choice is kept.
    pub autostart_configured: bool,
    /// Hide the physical controllers from games with HidHide (Windows), so
    /// games don't see each controller twice. Only has an effect when
    /// HidHide is installed.
    pub exclusive_mode: bool,
    /// Controllers (hidapi paths) DualBridge has hidden with HidHide. HidHide
    /// remembers them, so they are only hidden once (one UAC prompt).
    pub hidden_devices: BTreeSet<String>,
    /// Switch profiles automatically when a game listed in a profile is in
    /// the foreground.
    pub auto_profile_switch: bool,
    /// Always contains at least one profile; the first one is the default.
    pub profiles: Vec<Profile>,
    /// Profile name chosen for each controller (by identity).
    pub assignments: BTreeMap<String, String>,
    /// User-given names for controllers (by identity).
    pub controller_names: BTreeMap<String, String>,
    /// Lighting chosen for one controller (by identity). It takes priority
    /// over the lighting of the controller's profile.
    pub controller_lighting: BTreeMap<String, LightingConfig>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            version: SETTINGS_VERSION,
            language: None,
            first_run_done: false,
            minimize_to_tray: true,
            start_minimized: false,
            autostart_configured: false,
            exclusive_mode: true,
            hidden_devices: BTreeSet::new(),
            auto_profile_switch: true,
            profiles: vec![Profile::default()],
            assignments: BTreeMap::new(),
            controller_names: BTreeMap::new(),
            controller_lighting: BTreeMap::new(),
        }
    }
}

impl Settings {
    /// Loads settings, falling back to defaults if the file is missing or
    /// unreadable (a broken file is kept aside as `settings.json.bak`).
    pub fn load(path: &Path) -> Settings {
        let mut s = match std::fs::read_to_string(path) {
            Ok(text) => match serde_json::from_str::<Settings>(&text) {
                Ok(s) => s,
                Err(_) => {
                    let _ = std::fs::copy(path, path.with_extension("json.bak"));
                    Settings::default()
                }
            },
            Err(_) => Settings::default(),
        };
        s.normalize();
        s
    }

    /// Writes atomically (temp file + rename).
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = path.with_extension("json.tmp");
        let json = serde_json::to_string_pretty(self).expect("settings serialize");
        std::fs::write(&tmp, json)?;
        std::fs::rename(tmp, path)
    }

    /// Restores invariants after loading or editing.
    pub fn normalize(&mut self) {
        self.version = SETTINGS_VERSION;
        if self.profiles.is_empty() {
            self.profiles.push(Profile::default());
        }
        let names: Vec<String> = self.profiles.iter().map(|p| p.name.clone()).collect();
        self.assignments.retain(|_, p| names.contains(p));
    }

    pub fn default_profile(&self) -> &Profile {
        &self.profiles[0]
    }

    pub fn profile(&self, name: &str) -> Option<&Profile> {
        self.profiles.iter().find(|p| p.name == name)
    }

    /// The profile a controller should use: the foreground game's profile if
    /// any, else the controller's assigned profile, else the default one.
    pub fn profile_for(&self, identity: &str, game_profile: Option<&str>) -> &Profile {
        game_profile
            .and_then(|g| self.profile(g))
            .or_else(|| {
                self.assignments
                    .get(identity)
                    .and_then(|name| self.profile(name))
            })
            .unwrap_or_else(|| self.default_profile())
    }

    /// The lighting a controller should show: its own lighting if it has one,
    /// else its profile's.
    pub fn lighting_for(&self, identity: &str, game_profile: Option<&str>) -> &LightingConfig {
        self.controller_lighting
            .get(identity)
            .unwrap_or_else(|| &self.profile_for(identity, game_profile).lighting)
    }

    /// Adds or replaces a profile. `previous_name` renames an existing one
    /// (assignments follow the rename).
    pub fn upsert_profile(
        &mut self,
        profile: Profile,
        previous_name: Option<&str>,
    ) -> Result<(), String> {
        let name = profile.name.trim().to_string();
        if name.is_empty() {
            return Err("profile name is empty".into());
        }
        let old = previous_name.unwrap_or(&name).to_string();
        let clash = self.profiles.iter().any(|p| p.name == name && name != old);
        if clash {
            return Err(format!("a profile named \"{name}\" already exists"));
        }
        let profile = Profile { name, ..profile };
        match self.profiles.iter_mut().find(|p| p.name == old) {
            Some(p) => *p = profile.clone(),
            None => self.profiles.push(profile.clone()),
        }
        for v in self.assignments.values_mut() {
            if *v == old {
                *v = profile.name.clone();
            }
        }
        Ok(())
    }

    /// Removes a profile. The last remaining profile cannot be removed.
    pub fn delete_profile(&mut self, name: &str) -> Result<(), String> {
        if self.profiles.len() <= 1 {
            return Err("the last profile cannot be deleted".into());
        }
        self.profiles.retain(|p| p.name != name);
        self.normalize();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_resolution() {
        let mut s = Settings::default();
        s.upsert_profile(
            Profile {
                name: "Racing".into(),
                ..Profile::default()
            },
            None,
        )
        .unwrap();
        s.assignments.insert("pad".into(), "Racing".into());
        assert_eq!(s.profile_for("pad", None).name, "Racing");
        assert_eq!(s.profile_for("other", None).name, "Default");
        assert_eq!(s.profile_for("other", Some("Racing")).name, "Racing");
        assert_eq!(s.profile_for("pad", Some("Missing")).name, "Racing");
    }

    #[test]
    fn controller_lighting_overrides_profile() {
        use dualbridge_core::lighting::Effect;
        let mut s = Settings::default();
        let own = LightingConfig {
            effect: Effect::Off,
            ..LightingConfig::default()
        };
        s.controller_lighting.insert("pad".into(), own.clone());
        assert_eq!(s.lighting_for("pad", None), &own);
        assert_eq!(s.lighting_for("other", None), &LightingConfig::default());
    }

    #[test]
    fn rename_and_delete() {
        let mut s = Settings::default();
        let p = Profile {
            name: "A".into(),
            ..Profile::default()
        };
        s.upsert_profile(p.clone(), None).unwrap();
        s.assignments.insert("pad".into(), "A".into());
        s.upsert_profile(
            Profile {
                name: "B".into(),
                ..p.clone()
            },
            Some("A"),
        )
        .unwrap();
        assert_eq!(s.assignments["pad"], "B");
        assert!(s
            .upsert_profile(
                Profile {
                    name: "Default".into(),
                    ..p
                },
                Some("B")
            )
            .is_err());
        s.delete_profile("B").unwrap();
        assert!(s.assignments.is_empty());
        assert!(s.delete_profile("Default").is_err());
    }

    #[test]
    fn load_save_round_trip() {
        let dir = std::env::temp_dir().join(format!("dualbridge-test-{}", std::process::id()));
        let path = dir.join("settings.json");
        let s = Settings {
            first_run_done: true,
            ..Settings::default()
        };
        s.save(&path).unwrap();
        assert_eq!(Settings::load(&path), s);
        std::fs::write(&path, "{broken").unwrap();
        assert_eq!(Settings::load(&path), Settings::default());
        assert!(path.with_extension("json.bak").exists());
        let _ = std::fs::remove_dir_all(dir);
    }
}
