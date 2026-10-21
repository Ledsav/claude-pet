use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

const MAX: f64 = 100.0;
const HOUR_MS: f64 = 3_600_000.0;

/// Points per hour. Positive values drain the stat, negative ones refill it.
struct Rates {
    hunger: f64,
    energy: f64,
    mood: f64,
}

const AWAKE: Rates = Rates { hunger: 10.0, energy: 7.0, mood: 5.0 };
const ASLEEP: Rates = Rates { hunger: 4.0, energy: -30.0, mood: 0.0 };
/// Extra mood drain per hour while starving or exhausted.
const NEGLECT_MOOD: f64 = 6.0;

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// All stats run 0..=100 where higher is better: `hunger` 100 means full.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Pet {
    pub hunger: f64,
    pub mood: f64,
    pub energy: f64,
    pub asleep: bool,
    /// Unix ms of the last time decay was applied.
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Outcome {
    pub action: &'static str,
    pub ok: bool,
    pub message: String,
}

impl Outcome {
    fn ok(action: &'static str, message: &str) -> Self {
        Self { action, ok: true, message: message.into() }
    }
    fn refused(action: &'static str, message: &str) -> Self {
        Self { action, ok: false, message: message.into() }
    }
}

fn clamp(v: f64) -> f64 {
    v.clamp(0.0, MAX)
}

impl Pet {
    pub fn new(now: u64) -> Self {
        Self { hunger: 80.0, mood: 80.0, energy: 80.0, asleep: false, updated_at: now }
    }

    /// Applies decay for the time elapsed since the last update, including
    /// time the app was closed, scaled by `speed`. Wakes the pet once it is fully rested.
    pub fn tick(&mut self, now: u64, speed: f64) {
        let hours = now.saturating_sub(self.updated_at) as f64 / HOUR_MS * speed;
        self.updated_at = now;
        if hours <= 0.0 {
            return;
        }
        let rates = if self.asleep { &ASLEEP } else { &AWAKE };
        let neglected = self.hunger < 25.0 || (!self.asleep && self.energy < 20.0);

        self.hunger = clamp(self.hunger - rates.hunger * hours);
        self.energy = clamp(self.energy - rates.energy * hours);
        let neglect = if neglected { NEGLECT_MOOD } else { 0.0 };
        self.mood = clamp(self.mood - (rates.mood + neglect) * hours);

        if self.asleep && self.energy >= MAX {
            self.asleep = false;
        }
    }

    pub fn feed(&mut self) -> Outcome {
        if self.asleep {
            return Outcome::refused("feed", "zzz… (i'm sleeping)");
        }
        if self.hunger > 92.0 {
            self.mood = clamp(self.mood - 3.0);
            return Outcome::refused("feed", "i'm stuffed! no more please");
        }
        self.hunger = clamp(self.hunger + 30.0);
        self.mood = clamp(self.mood + 4.0);
        Outcome::ok("feed", "nom nom nom!")
    }

    pub fn play(&mut self) -> Outcome {
        if self.asleep {
            return Outcome::refused("play", "zzz… (five more minutes)");
        }
        if self.energy < 15.0 {
            return Outcome::refused("play", "too sleepy to play…");
        }
        if self.hunger < 15.0 {
            return Outcome::refused("play", "can't play, tummy's empty");
        }
        self.mood = clamp(self.mood + 20.0);
        self.energy = clamp(self.energy - 12.0);
        self.hunger = clamp(self.hunger - 6.0);
        Outcome::ok("play", "wheee! that was fun!")
    }

    /// Toggles sleep. Waking up early is allowed but makes the pet a bit grumpy.
    pub fn toggle_sleep(&mut self) -> Outcome {
        if self.asleep {
            self.asleep = false;
            if self.energy < 60.0 {
                self.mood = clamp(self.mood - 5.0);
                return Outcome::ok("wake", "*yawn* …already?");
            }
            return Outcome::ok("wake", "good morning!");
        }
        if self.energy > 90.0 {
            return Outcome::refused("sleep", "but i'm not tired!");
        }
        self.asleep = true;
        Outcome::ok("sleep", "night night…")
    }

    /// Chatting is good company.
    pub fn chatted(&mut self) {
        self.mood = clamp(self.mood + 2.0);
    }

    /// A one-line summary handed to Claude so replies match how the pet feels.
    pub fn describe(&self) -> String {
        format!(
            "fullness {:.0}/100 (0 = starving, 100 = stuffed), mood {:.0}/100, energy {:.0}/100, {}",
            self.hunger,
            self.mood,
            self.energy,
            if self.asleep { "currently asleep (was woken by the message)" } else { "awake" }
        )
    }
}

/// Reads and writes the pet as JSON in the app data directory.
pub struct Store {
    path: PathBuf,
}

impl Store {
    pub fn new(dir: &Path) -> Self {
        Self { path: dir.join("pet.json") }
    }

    /// Loads the saved pet (catching up on decay while the app was closed),
    /// or hatches a new one if there is no readable save.
    pub fn load(&self, now: u64, speed: f64) -> Pet {
        let mut pet = fs::read_to_string(&self.path)
            .ok()
            .and_then(|s| serde_json::from_str::<Pet>(&s).ok())
            .unwrap_or_else(|| Pet::new(now));
        pet.tick(now, speed);
        pet
    }

    /// Writes to a temp file first so a crash mid-write can't corrupt the save.
    pub fn save(&self, pet: &Pet) -> std::io::Result<()> {
        if let Some(dir) = self.path.parent() {
            fs::create_dir_all(dir)?;
        }
        let tmp = self.path.with_extension("json.tmp");
        fs::write(&tmp, serde_json::to_vec_pretty(pet)?)?;
        fs::rename(tmp, &self.path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const H: u64 = 3_600_000;

    #[test]
    fn awake_pet_decays_over_time() {
        let mut pet = Pet::new(0);
        pet.tick(2 * H, 1.0);
        assert_eq!(pet.hunger, 60.0);
        assert_eq!(pet.energy, 66.0);
        assert_eq!(pet.mood, 70.0);
    }

    #[test]
    fn sleeping_restores_energy_and_wakes_when_full() {
        let mut pet = Pet { energy: 40.0, asleep: true, ..Pet::new(0) };
        pet.tick(H, 1.0);
        assert_eq!(pet.energy, 70.0);
        assert!(pet.asleep);
        pet.tick(3 * H, 1.0);
        assert_eq!(pet.energy, 100.0);
        assert!(!pet.asleep);
    }

    #[test]
    fn neglect_drains_mood_faster() {
        let mut fed = Pet::new(0);
        let mut starving = Pet { hunger: 10.0, ..Pet::new(0) };
        fed.tick(H, 1.0);
        starving.tick(H, 1.0);
        assert!(starving.mood < fed.mood);
    }

    #[test]
    fn stats_never_leave_range() {
        let mut pet = Pet::new(0);
        pet.tick(1000 * H, 1.0);
        assert_eq!((pet.hunger, pet.mood, pet.energy), (0.0, 0.0, 0.0));
        pet.hunger = 90.0;
        pet.energy = 90.0;
        pet.feed();
        assert!(pet.hunger <= 100.0);
    }

    #[test]
    fn speed_scales_decay_and_zero_pauses() {
        let mut fast = Pet::new(0);
        fast.tick(H, 2.0);
        assert_eq!(fast.hunger, 60.0);
        let mut paused = Pet::new(0);
        paused.tick(H, 0.0);
        assert_eq!(paused.hunger, 80.0);
        assert_eq!(paused.updated_at, H);
    }

    #[test]
    fn clock_going_backwards_is_harmless() {
        let mut pet = Pet::new(10 * H);
        let before = pet.clone();
        pet.tick(5 * H, 1.0);
        assert_eq!(pet.hunger, before.hunger);
        assert_eq!(pet.updated_at, 5 * H);
    }

    #[test]
    fn actions_respect_state() {
        let mut pet = Pet::new(0);
        assert!(pet.feed().ok);
        assert!(!pet.feed().ok, "should be too full after two meals");

        pet.energy = 10.0;
        assert!(!pet.play().ok);
        assert!(pet.toggle_sleep().ok);
        assert!(!pet.feed().ok, "can't eat while asleep");
        assert!(!pet.play().ok);
        assert_eq!(pet.toggle_sleep().action, "wake");

        pet.energy = 95.0;
        assert!(!pet.toggle_sleep().ok, "won't sleep when rested");
    }

    #[test]
    fn play_trades_energy_for_mood() {
        let mut pet = Pet { mood: 50.0, ..Pet::new(0) };
        assert!(pet.play().ok);
        assert_eq!(pet.mood, 70.0);
        assert_eq!(pet.energy, 68.0);
    }

    #[test]
    fn store_round_trips_and_catches_up() {
        let dir = std::env::temp_dir().join(format!("claude-pet-test-{}", now_ms()));
        let store = Store::new(&dir);
        let pet = Pet::new(0);
        store.save(&pet).unwrap();
        let loaded = store.load(H, 1.0);
        assert_eq!(loaded.hunger, 70.0);
        assert_eq!(loaded.updated_at, H);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn missing_or_corrupt_save_hatches_new_pet() {
        let dir = std::env::temp_dir().join(format!("claude-pet-test-bad-{}", now_ms()));
        fs::create_dir_all(&dir).unwrap();
        let store = Store::new(&dir);
        assert_eq!(store.load(7, 1.0), Pet::new(7));
        fs::write(dir.join("pet.json"), "not json").unwrap();
        assert_eq!(store.load(7, 1.0), Pet::new(7));
        fs::remove_dir_all(dir).unwrap();
    }
}
