//! The Settings service: typed settings with limits, kept per profile across restarts
//! (Software Architecture r1.16 §4 and the Settings interface in §11.1).
//!
//! This first slice holds the appearance set of Settings › Display and appearance (Interface
//! Design r1.20 §13; Visual Design Specification §3) and the values behind the quick-settings
//! tiles and sliders. Every value is checked against its limits before it is stored, so the Shell
//! UI and other services can rely on what they read. Stage 3 adds further keys, and system-wide
//! settings, through the same interface.

use std::{collections::BTreeMap, collections::HashMap, path::PathBuf};

use zbus::{
    interface,
    object_server::SignalEmitter,
    zvariant::{OwnedValue, Value as Variant},
};

use crate::store;

/// A setting's value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    Bool(bool),
    Int(i32),
    /// One of a fixed list of options.
    Choice(String),
}

/// A setting's type, default and limits.
#[derive(Debug)]
pub enum Kind {
    Bool { default: bool },
    Int { min: i32, max: i32, default: i32 },
    Choice { options: &'static [&'static str], default: &'static str },
}

#[derive(Debug)]
pub struct Spec {
    pub key: &'static str,
    pub kind: Kind,
    /// Shown and stored, but nothing behind it yet in emulation (gap G6, UNIT-OS-DEC-0131): the
    /// Shell UI marks it as an emulation placeholder.
    pub simulated: bool,
}

const fn spec(key: &'static str, kind: Kind) -> Spec {
    Spec { key, kind, simulated: false }
}

const fn simulated(key: &'static str, default: bool) -> Spec {
    Spec { key, kind: Kind::Bool { default }, simulated: true }
}

/// The accent choices (Visual Design Specification §3.1). `theme` is the chosen theme's own
/// default accent, which the theme picker shows first (§4.5).
pub const ACCENTS: &[&str] = &[
    "theme", "#11756B", "#3D6FD8", "#7A5CF0", "#C2416F", "#D2512F", "#B7791F", "#2E7D4F",
];

/// Every setting this version knows.
pub const SCHEMA: &[Spec] = &[
    // Theme and colour (Interface Design §13). Daylight is the default (UNIT-OS-DEC-0065).
    spec("appearance.theme", Kind::Choice { options: &["daylight", "night-shift", "pebble"], default: "daylight" }),
    spec("appearance.mode", Kind::Choice { options: &["light", "dark", "automatic"], default: "light" }),
    spec("appearance.accent", Kind::Choice { options: ACCENTS, default: "theme" }),
    // Glass opacity in percent, per mode: never clearer than the user limit, which keeps text at
    // 5:1 or better over any backdrop (Visual Design Specification §3.2; UNIT-OS-DEC-0063). The
    // renderer's own 4.5:1 floor sits below these limits.
    spec("appearance.glass-opacity-light", Kind::Int { min: 60, max: 100, default: 66 }),
    spec("appearance.glass-opacity-dark", Kind::Int { min: 68, max: 100, default: 72 }),
    spec("appearance.reduce-transparency", Kind::Bool { default: false }),
    // Shape: one corner-rounding slider for the whole system, in pixels; icon shape.
    spec("appearance.corner-radius", Kind::Int { min: 0, max: 32, default: 20 }),
    spec("appearance.icon-shape", Kind::Choice {
        options: &["rounded", "square", "squircle", "circle", "teardrop"],
        default: "rounded",
    }),
    // Fonts covering all nine interface languages; Inter falls back to Noto Sans CJK for
    // Chinese, Japanese and Korean. Only Inter is installed in this version.
    spec("appearance.font", Kind::Choice { options: &["Inter"], default: "Inter" }),
    // Quick settings (SC02). Brightness never goes fully dark, so the screen stays usable.
    spec("quick.brightness", Kind::Int { min: 5, max: 100, default: 80 }),
    spec("quick.volume", Kind::Int { min: 0, max: 100, default: 50 }),
    spec("quick.power-mode", Kind::Choice { options: &["saver", "balanced", "performance"], default: "balanced" }),
    simulated("quick.wifi", true),
    simulated("quick.bluetooth", false),
    simulated("quick.battery-saver", false),
    simulated("quick.rotation-lock", false),
    simulated("quick.vr-mode", false),
];

pub fn spec_for(key: &str) -> Option<&'static Spec> {
    SCHEMA.iter().find(|s| s.key == key)
}

impl Spec {
    pub fn default_value(&self) -> Value {
        match self.kind {
            Kind::Bool { default } => Value::Bool(default),
            Kind::Int { default, .. } => Value::Int(default),
            Kind::Choice { default, .. } => Value::Choice(default.to_string()),
        }
    }

    /// Checks a value against this setting's type and limits.
    pub fn check(&self, value: &Value) -> Result<(), SetError> {
        match (&self.kind, value) {
            (Kind::Bool { .. }, Value::Bool(_)) => Ok(()),
            (Kind::Int { min, max, .. }, Value::Int(v)) if v < min || v > max => {
                Err(SetError::OutOfRange(format!("{} must be from {} to {}", self.key, min, max)))
            }
            (Kind::Int { .. }, Value::Int(_)) => Ok(()),
            (Kind::Choice { options, .. }, Value::Choice(v)) if !options.contains(&v.as_str()) => {
                Err(SetError::NotAnOption(format!("{} must be one of: {}", self.key, options.join(", "))))
            }
            (Kind::Choice { .. }, Value::Choice(_)) => Ok(()),
            _ => Err(SetError::WrongType(format!("{} takes {}", self.key, self.type_name()))),
        }
    }

    pub fn type_name(&self) -> &'static str {
        match self.kind {
            Kind::Bool { .. } => "bool",
            Kind::Int { .. } => "int",
            Kind::Choice { .. } => "choice",
        }
    }

    /// Reads a stored text value; None if it is not valid for this setting.
    fn parse(&self, text: &str) -> Option<Value> {
        let value = match self.kind {
            Kind::Bool { .. } => Value::Bool(text.parse().ok()?),
            Kind::Int { .. } => Value::Int(text.parse().ok()?),
            Kind::Choice { .. } => Value::Choice(text.to_string()),
        };
        self.check(&value).ok().map(|_| value)
    }
}

impl Value {
    fn to_text(&self) -> String {
        match self {
            Value::Bool(b) => b.to_string(),
            Value::Int(i) => i.to_string(),
            Value::Choice(s) => s.clone(),
        }
    }

    pub fn to_variant(&self) -> OwnedValue {
        let v = match self {
            Value::Bool(b) => Variant::from(*b),
            Value::Int(i) => Variant::from(*i),
            Value::Choice(s) => Variant::from(s.as_str()),
        };
        v.try_into_owned().expect("a plain value holds no file descriptor")
    }

    /// Reads a D-Bus value: a boolean, a whole number that fits in 32 bits, or a string.
    pub fn from_variant(v: &Variant<'_>) -> Option<Value> {
        Some(match v {
            Variant::Value(inner) => return Value::from_variant(inner),
            Variant::Bool(b) => Value::Bool(*b),
            Variant::I32(i) => Value::Int(*i),
            Variant::I64(i) => Value::Int(i32::try_from(*i).ok()?),
            Variant::U32(i) => Value::Int(i32::try_from(*i).ok()?),
            Variant::I16(i) => Value::Int(i32::from(*i)),
            Variant::U16(i) => Value::Int(i32::from(*i)),
            Variant::U8(i) => Value::Int(i32::from(*i)),
            Variant::Str(s) => Value::Choice(s.to_string()),
            _ => return None,
        })
    }
}

/// Why a value was refused.
#[derive(Debug, PartialEq, Eq)]
pub enum SetError {
    UnknownKey(String),
    WrongType(String),
    OutOfRange(String),
    NotAnOption(String),
}

/// The current settings, and where they are kept.
pub struct Settings {
    values: BTreeMap<&'static str, Value>,
    path: Option<PathBuf>,
}

const FILE_HEADER: &str = "Unit settings (written by unit-settings; edit through Settings)";

impl Settings {
    /// Every setting at its default, not kept anywhere (tests).
    pub fn defaults() -> Self {
        Settings {
            values: SCHEMA.iter().map(|s| (s.key, s.default_value())).collect(),
            path: None,
        }
    }

    /// Loads the profile's settings. Unknown keys and values outside the limits (an older or
    /// damaged file) fall back to the default, with a warning.
    pub fn load(path: PathBuf) -> Self {
        let mut settings = Settings::defaults();
        let stored = store::read_map(&path).unwrap_or_else(|e| {
            tracing::warn!("could not read {}: {}; using the defaults", path.display(), e);
            BTreeMap::new()
        });
        for (key, text) in &stored {
            match spec_for(key) {
                Some(spec) => match spec.parse(text) {
                    Some(value) => {
                        settings.values.insert(spec.key, value);
                    }
                    None => tracing::warn!("{}: stored value {:?} is not valid; using the default", key, text),
                },
                None => tracing::warn!("{}: not a known setting; ignored", key),
            }
        }
        settings.path = Some(path);
        settings
    }

    pub fn get(&self, key: &str) -> Option<&Value> {
        self.values.get(key)
    }

    pub fn all(&self) -> impl Iterator<Item = (&'static str, &Value)> {
        self.values.iter().map(|(k, v)| (*k, v))
    }

    /// Checks and stores a value. Returns whether it changed.
    pub fn set(&mut self, key: &str, value: Value) -> Result<bool, SetError> {
        let spec = spec_for(key).ok_or_else(|| SetError::UnknownKey(key.to_string()))?;
        spec.check(&value)?;
        Ok(self.values.insert(spec.key, value.clone()) != Some(value))
    }

    /// Returns to the defaults every setting whose key starts with `prefix` ("" for all).
    /// Returns the settings that changed, with their new values.
    pub fn reset(&mut self, prefix: &str) -> Vec<(&'static str, Value)> {
        let mut changed = Vec::new();
        for spec in SCHEMA.iter().filter(|s| s.key.starts_with(prefix)) {
            let default = spec.default_value();
            if self.values.insert(spec.key, default.clone()) != Some(default.clone()) {
                changed.push((spec.key, default));
            }
        }
        changed
    }

    /// Writes the settings to the profile's file.
    pub fn save(&self) -> std::io::Result<()> {
        let Some(path) = &self.path else { return Ok(()) };
        let map = self.values.iter().map(|(k, v)| (k.to_string(), v.to_text())).collect();
        store::write_atomic(path, &store::format(FILE_HEADER, &map))
    }
}

/// D-Bus errors of the Settings interface.
#[derive(Debug, zbus::DBusError)]
#[zbus(prefix = "os.unit.Settings1.Error")]
pub enum Error {
    #[zbus(error)]
    ZBus(zbus::Error),
    UnknownKey(String),
    WrongType(String),
    OutOfRange(String),
    NotAnOption(String),
    /// The value is valid but could not be written to the profile's storage; nothing changed.
    NotSaved(String),
}

impl From<SetError> for Error {
    fn from(e: SetError) -> Self {
        match e {
            SetError::UnknownKey(m) => Error::UnknownKey(m),
            SetError::WrongType(m) => Error::WrongType(m),
            SetError::OutOfRange(m) => Error::OutOfRange(m),
            SetError::NotAnOption(m) => Error::NotAnOption(m),
        }
    }
}

pub const BUS_NAME: &str = "os.unit.Settings1";
pub const OBJECT_PATH: &str = "/os/unit/Settings1";

/// The Settings interface, version 1.
pub struct SettingsService {
    settings: Settings,
}

impl SettingsService {
    pub fn new(settings: Settings) -> Self {
        SettingsService { settings }
    }

    /// Applies a change and saves it; if saving fails, the change is undone.
    fn apply(&mut self, change: impl FnOnce(&mut Settings) -> Result<(), Error>) -> Result<Vec<(&'static str, Value)>, Error> {
        let before = self.settings.values.clone();
        change(&mut self.settings)?;
        if let Err(e) = self.settings.save() {
            self.settings.values = before;
            tracing::error!("could not save the settings: {}", e);
            return Err(Error::NotSaved(e.to_string()));
        }
        Ok(self
            .settings
            .all()
            .filter(|(k, v)| before.get(k) != Some(*v))
            .map(|(k, v)| (k, v.clone()))
            .collect())
    }
}

#[interface(name = "os.unit.Settings1")]
impl SettingsService {
    /// The current value of a setting.
    async fn get(&self, key: &str) -> Result<OwnedValue, Error> {
        self.settings
            .get(key)
            .map(Value::to_variant)
            .ok_or_else(|| Error::UnknownKey(key.to_string()))
    }

    /// Every setting and its current value.
    async fn get_all(&self) -> HashMap<String, OwnedValue> {
        self.settings.all().map(|(k, v)| (k.to_string(), v.to_variant())).collect()
    }

    /// Checks a value against the setting's type and limits, stores it and saves it.
    async fn set(
        &mut self,
        key: &str,
        value: OwnedValue,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> Result<(), Error> {
        let value = Value::from_variant(&value).ok_or_else(|| {
            Error::WrongType(format!("{}: not a boolean, whole number or string", key))
        })?;
        let changed = self.apply(|s| s.set(key, value).map(|_| ()).map_err(Error::from))?;
        for (key, value) in changed {
            Self::changed(&emitter, key, value.to_variant()).await?;
        }
        Ok(())
    }

    /// Returns to the defaults every setting whose key starts with `prefix`: "appearance." for
    /// Reset to default in Display and appearance, "" for everything.
    async fn reset(&mut self, prefix: &str, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>) -> Result<(), Error> {
        let changed = self.apply(|s| {
            s.reset(prefix);
            Ok(())
        })?;
        for (key, value) in changed {
            Self::changed(&emitter, key, value.to_variant()).await?;
        }
        Ok(())
    }

    /// A setting's type ("bool", "int" or "choice"), default and limits: "min" and "max" for
    /// whole numbers, "options" for choices, and "simulated" when nothing is behind the setting
    /// yet in emulation.
    async fn describe(&self, key: &str) -> Result<(String, OwnedValue, HashMap<String, OwnedValue>), Error> {
        let spec = spec_for(key).ok_or_else(|| Error::UnknownKey(key.to_string()))?;
        let owned = |v: Variant<'_>| v.try_into_owned().expect("a plain value holds no file descriptor");
        let mut limits = HashMap::new();
        match spec.kind {
            Kind::Int { min, max, .. } => {
                limits.insert("min".to_string(), owned(Variant::from(min)));
                limits.insert("max".to_string(), owned(Variant::from(max)));
            }
            Kind::Choice { options, .. } => {
                limits.insert("options".to_string(), owned(Variant::from(options.to_vec())));
            }
            Kind::Bool { .. } => {}
        }
        limits.insert("simulated".to_string(), owned(Variant::from(spec.simulated)));
        Ok((spec.type_name().to_string(), spec.default_value().to_variant(), limits))
    }

    /// The interface version.
    #[zbus(property)]
    fn version(&self) -> u32 {
        1
    }

    /// A setting changed, by anyone.
    #[zbus(signal)]
    async fn changed(emitter: &SignalEmitter<'_>, key: &str, value: OwnedValue) -> zbus::Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testdir::TestDir;

    #[test]
    fn every_default_is_within_its_limits() {
        for spec in SCHEMA {
            assert_eq!(spec.check(&spec.default_value()), Ok(()), "{}", spec.key);
        }
    }

    #[test]
    fn keys_are_unique() {
        for (i, spec) in SCHEMA.iter().enumerate() {
            assert!(SCHEMA[i + 1..].iter().all(|s| s.key != spec.key), "{}", spec.key);
        }
    }

    #[test]
    fn glass_never_clearer_than_the_readability_limit() {
        let mut s = Settings::defaults();
        assert!(matches!(s.set("appearance.glass-opacity-light", Value::Int(59)), Err(SetError::OutOfRange(_))));
        assert!(matches!(s.set("appearance.glass-opacity-dark", Value::Int(67)), Err(SetError::OutOfRange(_))));
        assert_eq!(s.set("appearance.glass-opacity-light", Value::Int(60)), Ok(true));
        assert_eq!(s.set("appearance.glass-opacity-dark", Value::Int(68)), Ok(true));
        assert_eq!(s.get("appearance.glass-opacity-dark"), Some(&Value::Int(68)));
    }

    #[test]
    fn refuses_unknown_keys_wrong_types_and_other_options() {
        let mut s = Settings::defaults();
        assert!(matches!(s.set("appearance.nope", Value::Bool(true)), Err(SetError::UnknownKey(_))));
        assert!(matches!(s.set("appearance.theme", Value::Int(1)), Err(SetError::WrongType(_))));
        assert!(matches!(s.set("appearance.theme", Value::Choice("neon".into())), Err(SetError::NotAnOption(_))));
        assert!(matches!(s.set("appearance.accent", Value::Choice("#000000".into())), Err(SetError::NotAnOption(_))));
        assert!(matches!(s.set("appearance.corner-radius", Value::Int(33)), Err(SetError::OutOfRange(_))));
        assert_eq!(s.get("appearance.theme"), Some(&Value::Choice("daylight".into())));
    }

    #[test]
    fn setting_the_same_value_is_not_a_change() {
        let mut s = Settings::defaults();
        assert_eq!(s.set("quick.volume", Value::Int(50)), Ok(false));
        assert_eq!(s.set("quick.volume", Value::Int(51)), Ok(true));
    }

    #[test]
    fn kept_across_restart() {
        let dir = TestDir::new();
        let path = dir.path().join("settings");
        let mut s = Settings::load(path.clone());
        s.set("appearance.theme", Value::Choice("pebble".into())).unwrap();
        s.set("appearance.corner-radius", Value::Int(8)).unwrap();
        s.set("quick.wifi", Value::Bool(false)).unwrap();
        s.save().unwrap();
        let again = Settings::load(path);
        assert_eq!(again.get("appearance.theme"), Some(&Value::Choice("pebble".into())));
        assert_eq!(again.get("appearance.corner-radius"), Some(&Value::Int(8)));
        assert_eq!(again.get("quick.wifi"), Some(&Value::Bool(false)));
    }

    #[test]
    fn damaged_values_fall_back_to_defaults() {
        let dir = TestDir::new();
        let path = dir.path().join("settings");
        std::fs::write(
            &path,
            "appearance.glass-opacity-light=10\nappearance.theme=neon\nquick.volume=x\nold.key=1\nquick.brightness=40\n",
        )
        .unwrap();
        let s = Settings::load(path);
        assert_eq!(s.get("appearance.glass-opacity-light"), Some(&Value::Int(66)));
        assert_eq!(s.get("appearance.theme"), Some(&Value::Choice("daylight".into())));
        assert_eq!(s.get("quick.volume"), Some(&Value::Int(50)));
        assert_eq!(s.get("quick.brightness"), Some(&Value::Int(40)));
        assert_eq!(s.get("old.key"), None);
    }

    #[test]
    fn reset_returns_a_group_to_daylight() {
        let mut s = Settings::defaults();
        s.set("appearance.theme", Value::Choice("night-shift".into())).unwrap();
        s.set("appearance.mode", Value::Choice("dark".into())).unwrap();
        s.set("quick.volume", Value::Int(10)).unwrap();
        let changed = s.reset("appearance.");
        assert_eq!(changed.len(), 2);
        assert_eq!(s.get("appearance.theme"), Some(&Value::Choice("daylight".into())));
        assert_eq!(s.get("quick.volume"), Some(&Value::Int(10)));
    }

    #[test]
    fn reads_dbus_values() {
        assert_eq!(Value::from_variant(&Variant::from(true)), Some(Value::Bool(true)));
        assert_eq!(Value::from_variant(&Variant::from(7u32)), Some(Value::Int(7)));
        assert_eq!(Value::from_variant(&Variant::from(u32::MAX)), None);
        assert_eq!(Value::from_variant(&Variant::from("dark")), Some(Value::Choice("dark".into())));
        assert_eq!(Value::from_variant(&Variant::from(1.5f64)), None);
    }

    #[test]
    fn a_failed_save_changes_nothing() {
        let dir = TestDir::new();
        // A folder where the file should be: the write fails.
        let path = dir.path().join("settings");
        std::fs::create_dir_all(path.with_extension("new")).unwrap();
        let mut service = SettingsService::new(Settings::load(path));
        let result = service.apply(|s| s.set("quick.volume", Value::Int(10)).map(|_| ()).map_err(Error::from));
        assert!(matches!(result, Err(Error::NotSaved(_))));
        assert_eq!(service.settings.get("quick.volume"), Some(&Value::Int(50)));
    }

    #[test]
    fn matches_its_interface_file() {
        crate::check_interface_file(&SettingsService::new(Settings::defaults()), "os.unit.Settings1.xml");
    }
}
