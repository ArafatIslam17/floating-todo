//! Typed application settings stored as JSON values in `app_settings`.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SoundName {
    Chime,
    Bell,
    Soft,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowGeometry {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    /// Height to restore when leaving compact mode.
    pub expanded_height: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub sound_enabled: bool,
    pub sound_name: SoundName,
    pub sound_volume: f64,
    pub repeat_count: i64,
    pub repeat_interval_min: i64,
    pub default_snooze_min: i64,
    pub default_lead_min: Option<i64>,
    pub always_on_top: bool,
    pub launch_at_login: bool,
    pub opacity: f64,
    pub acrylic: bool,
    pub compact: bool,
    pub show_completed: bool,
    pub bring_forward_on_alert: bool,
    pub global_shortcut: String,
    #[serde(skip)]
    pub window_geometry: Option<WindowGeometry>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            sound_enabled: true,
            sound_name: SoundName::Chime,
            sound_volume: 0.7,
            repeat_count: 2,
            repeat_interval_min: 5,
            default_snooze_min: 10,
            default_lead_min: None,
            always_on_top: true,
            launch_at_login: false,
            opacity: 0.94,
            acrylic: false,
            compact: false,
            show_completed: false,
            bring_forward_on_alert: true,
            global_shortcut: "Ctrl+Alt+Space".into(),
            window_geometry: None,
        }
    }
}

/// Partial update from the UI. Every field optional; `defaultLeadMin: null` clears it,
/// so that field uses a double Option.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsPatch {
    pub sound_enabled: Option<bool>,
    pub sound_name: Option<SoundName>,
    pub sound_volume: Option<f64>,
    pub repeat_count: Option<i64>,
    pub repeat_interval_min: Option<i64>,
    pub default_snooze_min: Option<i64>,
    #[serde(default, deserialize_with = "double_option")]
    pub default_lead_min: Option<Option<i64>>,
    pub always_on_top: Option<bool>,
    pub launch_at_login: Option<bool>,
    pub opacity: Option<f64>,
    pub acrylic: Option<bool>,
    pub compact: Option<bool>,
    pub show_completed: Option<bool>,
    pub bring_forward_on_alert: Option<bool>,
    pub global_shortcut: Option<String>,
}

fn double_option<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<Option<i64>>, D::Error> {
    Option::<i64>::deserialize(d).map(Some)
}

fn in_range<T: PartialOrd + std::fmt::Display>(v: T, lo: T, hi: T, what: &str) -> AppResult<T> {
    if v >= lo && v <= hi {
        Ok(v)
    } else {
        Err(AppError::validation(format!("{what} must be between {lo} and {hi}.")))
    }
}

impl Settings {
    /// Validate a patch and return the changed `(key, json)` pairs plus the merged settings.
    pub fn apply_patch(&self, p: &SettingsPatch) -> AppResult<(Settings, Vec<(&'static str, Value)>)> {
        let mut s = self.clone();
        let mut changed: Vec<(&'static str, Value)> = Vec::new();
        macro_rules! set {
            ($field:ident, $key:literal, $val:expr) => {{
                let v = $val;
                s.$field = v.clone();
                changed.push(($key, serde_json::to_value(v).expect("serialisable")));
            }};
        }
        if let Some(v) = p.sound_enabled { set!(sound_enabled, "sound_enabled", v) }
        if let Some(v) = p.sound_name { set!(sound_name, "sound_name", v) }
        if let Some(v) = p.sound_volume {
            if !v.is_finite() { return Err(AppError::validation("Volume must be a number.")); }
            set!(sound_volume, "sound_volume", in_range(v, 0.0, 1.0, "Volume")?)
        }
        if let Some(v) = p.repeat_count { set!(repeat_count, "repeat_count", in_range(v, 0, 3, "Repeat count")?) }
        if let Some(v) = p.repeat_interval_min { set!(repeat_interval_min, "repeat_interval_min", in_range(v, 2, 30, "Repeat interval")?) }
        if let Some(v) = p.default_snooze_min { set!(default_snooze_min, "default_snooze_min", in_range(v, 1, 240, "Snooze")?) }
        if let Some(v) = p.default_lead_min {
            let v = match v { Some(m) => Some(in_range(m, 1, 1440, "Default reminder")?), None => None };
            set!(default_lead_min, "default_lead_min", v)
        }
        if let Some(v) = p.always_on_top { set!(always_on_top, "always_on_top", v) }
        if let Some(v) = p.launch_at_login { set!(launch_at_login, "launch_at_login", v) }
        if let Some(v) = p.opacity {
            if !v.is_finite() { return Err(AppError::validation("Opacity must be a number.")); }
            set!(opacity, "opacity", in_range(v, 0.85, 1.0, "Opacity")?)
        }
        if let Some(v) = p.acrylic { set!(acrylic, "acrylic", v) }
        if let Some(v) = p.compact { set!(compact, "compact", v) }
        if let Some(v) = p.show_completed { set!(show_completed, "show_completed", v) }
        if let Some(v) = p.bring_forward_on_alert { set!(bring_forward_on_alert, "bring_forward_on_alert", v) }
        if let Some(v) = &p.global_shortcut {
            let v = v.trim().to_string();
            if v.is_empty() || v.len() > 64 {
                return Err(AppError::validation("Enter a shortcut such as Ctrl+Alt+Space."));
            }
            set!(global_shortcut, "global_shortcut", v)
        }
        Ok((s, changed))
    }

    /// Build settings from stored key/value JSON. Invalid values fall back to defaults
    /// (returned as warnings) so a damaged row cannot prevent startup.
    pub fn from_stored(stored: &HashMap<String, String>) -> (Settings, Vec<String>) {
        let mut s = Settings::default();
        let mut warnings = Vec::new();
        for (key, raw) in stored {
            let parsed: Result<Value, _> = serde_json::from_str(raw);
            let Ok(val) = parsed else {
                warnings.push(format!("setting {key}: invalid JSON, using default"));
                continue;
            };
            let single = serde_json::json!({ camel(key): val });
            if key == "window_geometry" {
                match serde_json::from_value::<WindowGeometry>(val) {
                    Ok(g) => s.window_geometry = Some(g),
                    Err(_) => warnings.push("setting window_geometry: invalid, ignored".into()),
                }
                continue;
            }
            match serde_json::from_value::<SettingsPatch>(single) {
                Ok(one) => {
                    // Validate this key alone so one bad value doesn't discard the rest.
                    match s.apply_patch(&one) {
                        Ok((merged, _)) => s = merged,
                        Err(e) => warnings.push(format!("setting {key}: {e}; using default")),
                    }
                }
                Err(_) => warnings.push(format!("setting {key}: wrong type, using default")),
            }
        }
        (s, warnings)
    }
}

fn camel(snake: &str) -> String {
    let mut out = String::with_capacity(snake.len());
    let mut up = false;
    for c in snake.chars() {
        if c == '_' { up = true; } else if up { out.push(c.to_ascii_uppercase()); up = false; } else { out.push(c); }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn patch_validates_ranges() {
        let s = Settings::default();
        assert!(s.apply_patch(&SettingsPatch { opacity: Some(0.5), ..Default::default() }).is_err());
        assert!(s.apply_patch(&SettingsPatch { sound_volume: Some(1.5), ..Default::default() }).is_err());
        assert!(s.apply_patch(&SettingsPatch { repeat_count: Some(4), ..Default::default() }).is_err());
        let (m, changed) = s.apply_patch(&SettingsPatch { opacity: Some(0.9), repeat_count: Some(0), ..Default::default() }).unwrap();
        assert_eq!(m.opacity, 0.9);
        assert_eq!(m.repeat_count, 0);
        assert_eq!(changed.len(), 2);
    }

    #[test]
    fn default_lead_can_be_cleared_with_null() {
        let p: SettingsPatch = serde_json::from_str(r#"{"defaultLeadMin": null}"#).unwrap();
        assert_eq!(p.default_lead_min, Some(None));
        let p: SettingsPatch = serde_json::from_str(r#"{}"#).unwrap();
        assert_eq!(p.default_lead_min, None);
        let p: SettingsPatch = serde_json::from_str(r#"{"defaultLeadMin": 15}"#).unwrap();
        assert_eq!(p.default_lead_min, Some(Some(15)));
    }

    #[test]
    fn stored_values_round_trip_and_bad_values_fall_back() {
        let mut stored = HashMap::new();
        stored.insert("sound_name".to_string(), "\"bell\"".to_string());
        stored.insert("opacity".to_string(), "0.2".to_string()); // out of range
        stored.insert("repeat_count".to_string(), "\"lots\"".to_string()); // wrong type
        stored.insert("always_on_top".to_string(), "{not json".to_string());
        stored.insert("window_geometry".to_string(), r#"{"x":10,"y":20,"width":340,"height":420,"expandedHeight":420}"#.to_string());
        let (s, warnings) = Settings::from_stored(&stored);
        assert_eq!(s.sound_name, SoundName::Bell);
        assert_eq!(s.opacity, Settings::default().opacity);
        assert_eq!(s.repeat_count, 2);
        assert!(s.always_on_top);
        assert_eq!(s.window_geometry.unwrap().width, 340);
        assert_eq!(warnings.len(), 3);
    }
}
