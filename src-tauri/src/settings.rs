use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};

/// User-tweakable settings, changed with `/settings <key> <value>` in the chat box.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Settings {
    pub name: String,
    /// Model passed to `claude --model`.
    pub model: String,
    /// Multiplier on how fast stats decay (and sleep restores energy).
    pub speed: f64,
    pub on_top: bool,
    /// Max chat messages per hour.
    pub hourly_limit: usize,
    /// Sprite sheet to draw, one of `SKINS`.
    pub skin: String,
    /// Minutes between unprompted pep talks; 0 turns them off.
    pub motivate: u32,
}

/// Must match the sheets in public/sprites (see scripts/sprites/skins.mjs).
pub const SKINS: [&str; 5] = ["blob", "sprout", "beancat", "ember", "spark"];

impl Default for Settings {
    fn default() -> Self {
        Self {
            name: "Mochi".into(),
            model: std::env::var("CLAUDE_PET_MODEL").unwrap_or_else(|_| "haiku".into()),
            speed: 1.0,
            on_top: true,
            hourly_limit: 30,
            skin: "blob".into(),
            motivate: 45,
        }
    }
}

pub const KEYS: &str = "name, skin, model, speed, ontop, limit, motivate";

impl Settings {
    pub fn load(dir: &Path) -> Self {
        fs::read_to_string(path(dir))
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, dir: &Path) -> std::io::Result<()> {
        fs::create_dir_all(dir)?;
        fs::write(path(dir), serde_json::to_vec_pretty(self)?)
    }

    pub fn describe(&self) -> String {
        format!(
            "name: {} · skin: {} · model: {} · speed: {}x · ontop: {} · limit: {}/hour · motivate: {}",
            self.name,
            self.skin,
            self.model,
            self.speed,
            if self.on_top { "on" } else { "off" },
            self.hourly_limit,
            if self.motivate == 0 { "off".to_string() } else { format!("every {} min", self.motivate) }
        )
    }

    /// Validates and applies one setting, returning a confirmation line.
    pub fn set(&mut self, key: &str, value: &str) -> Result<String, String> {
        let value = value.trim();
        match key {
            "name" => {
                let len = value.chars().count();
                if !(1..=24).contains(&len) {
                    return Err("names are 1–24 characters".into());
                }
                self.name = value.into();
                Ok(format!("call me {value} from now on!"))
            }
            "model" => {
                let ok = ["haiku", "sonnet", "opus"].contains(&value) || value.starts_with("claude-");
                if !ok || value.contains(char::is_whitespace) {
                    return Err("model must be haiku, sonnet, opus or a full claude-… id".into());
                }
                self.model = value.into();
                Ok(format!("brain swapped to {value}"))
            }
            "speed" => {
                let speed: f64 = value.parse().map_err(|_| "speed is a number like 0.5 or 2")?;
                if !(0.0..=10.0).contains(&speed) {
                    return Err("speed must be between 0 (paused) and 10".into());
                }
                self.speed = speed;
                Ok(if speed == 0.0 { "time is frozen. stats won't change".into() } else { format!("stats now change at {speed}x speed") })
            }
            "ontop" => {
                self.on_top = match value {
                    "on" | "true" | "yes" => true,
                    "off" | "false" | "no" => false,
                    _ => return Err("ontop is on or off".into()),
                };
                Ok(format!("always on top: {value}"))
            }
            "limit" => {
                let limit: usize = value.parse().map_err(|_| "limit is a whole number")?;
                if !(1..=200).contains(&limit) {
                    return Err("limit must be between 1 and 200 messages per hour".into());
                }
                self.hourly_limit = limit;
                Ok(format!("up to {limit} chats per hour"))
            }
            "skin" => {
                if !SKINS.contains(&value) {
                    return Err(format!("skins: {}", SKINS.join(", ")));
                }
                self.skin = value.into();
                Ok(format!("*poof* new look: {value}!"))
            }
            "motivate" => {
                let minutes: u32 = match value {
                    "off" => 0,
                    v => v.parse().map_err(|_| "motivate is minutes between pep talks, or off")?,
                };
                if minutes > 240 {
                    return Err("motivate must be 0–240 minutes".into());
                }
                self.motivate = minutes;
                Ok(if minutes == 0 { "ok, no unprompted pep talks".into() } else { format!("i'll cheer you on every {minutes} minutes") })
            }
            _ => Err(format!("unknown setting '{key}'. try: {KEYS}")),
        }
    }
}

fn path(dir: &Path) -> PathBuf {
    dir.join("settings.json")
}

/// Everything you can type in the chat box that starts with `/`.
#[derive(Debug, PartialEq)]
pub enum Command {
    Help,
    ShowSettings,
    Set(String, String),
    Personality { reset: bool },
    Motivate,
    Stats,
    Clear,
    Reset { confirmed: bool },
}

pub const HELP: &str = "/settings · /settings <key> <value> · /motivate · /stats · /personality [reset] · /clear · /reset";

pub fn parse(input: &str) -> Result<Command, String> {
    let input = input.trim().trim_start_matches('/');
    let (cmd, rest) = input.split_once(char::is_whitespace).unwrap_or((input, ""));
    let rest = rest.trim();
    match cmd.to_lowercase().as_str() {
        "help" | "?" => Ok(Command::Help),
        "settings" | "set" if rest.is_empty() => Ok(Command::ShowSettings),
        "settings" | "set" => {
            let (key, value) = rest.split_once(char::is_whitespace).ok_or("usage: /settings <key> <value>")?;
            Ok(Command::Set(key.to_lowercase(), value.trim().into()))
        }
        "personality" => Ok(Command::Personality { reset: rest == "reset" }),
        "motivate" | "pep" => Ok(Command::Motivate),
        "stats" => Ok(Command::Stats),
        "clear" => Ok(Command::Clear),
        "reset" => Ok(Command::Reset { confirmed: rest == "confirm" }),
        other => Err(format!("unknown command /{other}. try /help")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_commands() {
        assert_eq!(parse("/help"), Ok(Command::Help));
        assert_eq!(parse("/settings"), Ok(Command::ShowSettings));
        assert_eq!(parse("/settings  Name   Sir Blob "), Ok(Command::Set("name".into(), "Sir Blob".into())));
        assert_eq!(parse("/reset"), Ok(Command::Reset { confirmed: false }));
        assert_eq!(parse("/personality reset"), Ok(Command::Personality { reset: true }));
        assert_eq!(parse("/motivate"), Ok(Command::Motivate));
        assert_eq!(parse("/reset confirm"), Ok(Command::Reset { confirmed: true }));
        assert!(parse("/settings name").is_err());
        assert!(parse("/dance").is_err());
    }

    #[test]
    fn validates_settings() {
        let mut s = Settings::default();
        assert!(s.set("name", "Pip").is_ok());
        assert_eq!(s.name, "Pip");
        assert!(s.set("name", &"x".repeat(30)).is_err());
        assert!(s.set("model", "sonnet").is_ok());
        assert!(s.set("model", "gpt-4").is_err());
        assert!(s.set("speed", "2.5").is_ok());
        assert_eq!(s.speed, 2.5);
        assert!(s.set("speed", "-1").is_err());
        assert!(s.set("speed", "fast").is_err());
        assert!(s.set("ontop", "off").is_ok());
        assert!(!s.on_top);
        assert!(s.set("limit", "0").is_err());
        assert!(s.set("skin", "spark").is_ok());
        assert!(s.set("skin", "dragon").is_err());
        assert!(s.set("motivate", "off").is_ok());
        assert_eq!(s.motivate, 0);
        assert!(s.set("motivate", "999").is_err());
        assert!(s.set("colour", "red").is_err());
    }

    #[test]
    fn round_trips_and_fills_missing_fields() {
        let dir = std::env::temp_dir().join(format!("claude-pet-settings-{}", crate::stats::now_ms()));
        let mut s = Settings::default();
        s.set("name", "Pip").unwrap();
        s.save(&dir).unwrap();
        assert_eq!(Settings::load(&dir), s);
        fs::write(dir.join("settings.json"), r#"{"name":"Old"}"#).unwrap();
        let old = Settings::load(&dir);
        assert_eq!((old.name.as_str(), old.speed), ("Old", 1.0));
        fs::remove_dir_all(dir).unwrap();
    }
}
