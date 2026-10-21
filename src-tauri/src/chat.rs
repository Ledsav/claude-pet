use std::{
    collections::VecDeque,
    fs,
    path::{Path, PathBuf},
    process::Stdio,
    time::{Duration, Instant},
};

/// Seeded into the brain folder on first run; the user's edited copy wins after that.
const DEFAULT_PERSONALITY: &str = include_str!("../personality/CLAUDE.md");
const MAX_MESSAGE_CHARS: usize = 500;
const HISTORY_TURNS: usize = 6;
const CLI_TIMEOUT: Duration = Duration::from_secs(90);

/// Allows at most `max_calls` per sliding `window`, with at least `min_gap` between calls.
pub struct RateLimiter {
    min_gap: Duration,
    window: Duration,
    max_calls: usize,
    calls: VecDeque<Instant>,
}

impl RateLimiter {
    pub fn new(min_gap: Duration, max_calls: usize, window: Duration) -> Self {
        Self { min_gap, window, max_calls, calls: VecDeque::new() }
    }

    pub fn set_max_calls(&mut self, max_calls: usize) {
        self.max_calls = max_calls;
    }

    /// Records a call and returns Ok, or returns how long until the next call is allowed.
    pub fn try_acquire(&mut self, now: Instant) -> Result<(), Duration> {
        while self.calls.front().is_some_and(|t| now.duration_since(*t) >= self.window) {
            self.calls.pop_front();
        }
        let gap_wait = self
            .calls
            .back()
            .map(|last| self.min_gap.saturating_sub(now.duration_since(*last)))
            .unwrap_or_default();
        let window_wait = if self.calls.len() >= self.max_calls {
            self.window - now.duration_since(self.calls[0])
        } else {
            Duration::ZERO
        };
        let wait = gap_wait.max(window_wait);
        if !wait.is_zero() {
            return Err(wait);
        }
        self.calls.push_back(now);
        Ok(())
    }
}

/// Rolling memory of the last few exchanges so the pet can follow a conversation.
#[derive(Default)]
pub struct History {
    turns: VecDeque<(String, String)>,
}

impl History {
    pub fn push(&mut self, human: String, pet: String) {
        self.turns.push_back((human, pet));
        while self.turns.len() > HISTORY_TURNS {
            self.turns.pop_front();
        }
    }

    pub fn clear(&mut self) {
        self.turns.clear();
    }
}

/// Skins that come with a personality of their own. Every other skin shares the main one.
const SKIN_PERSONALITIES: [(&str, &str); 1] = [("spine", include_str!("../personality/spine.md"))];

fn skin_personality(skin: &str) -> Option<&'static str> {
    SKIN_PERSONALITIES.iter().find(|(s, _)| *s == skin).map(|(_, text)| *text)
}

/// Whether picking this skin swaps in its own personality.
pub fn has_own_personality(skin: &str) -> bool {
    skin_personality(skin).is_some()
}

/// The built-in personality for a skin.
pub fn default_personality(skin: &str) -> &'static str {
    skin_personality(skin).unwrap_or(DEFAULT_PERSONALITY)
}

/// The folder `claude` runs in for this skin. Claude Code picks up its CLAUDE.md as the
/// pet's personality. Skins with their own personality get their own folder, so switching
/// skins never overwrites the user's edits to another one.
pub fn brain_dir(data_dir: &Path, skin: &str) -> std::io::Result<PathBuf> {
    let dir = if has_own_personality(skin) { data_dir.join(format!("brain-{skin}")) } else { data_dir.join("brain") };
    fs::create_dir_all(&dir)?;
    let file = dir.join("CLAUDE.md");
    if !file.exists() {
        fs::write(&file, default_personality(skin))?;
    }
    Ok(dir)
}

/// Overwrites the personality with the skin's built-in default.
pub fn reset_personality(brain: &Path, skin: &str) -> std::io::Result<()> {
    fs::write(brain.join("CLAUDE.md"), default_personality(skin))
}

/// Prompt for an unprompted pep talk.
pub fn build_nudge_prompt(name: &str, status: &str, history: &History) -> String {
    build_prompt(
        name,
        status,
        history,
        "(nobody said anything: the human is busy working at their computer. Give them a short, \
         specific nudge of motivation to keep going, or if it's been a while, a gentle reminder to \
         stretch or drink water. Don't start with a greeting.)",
    )
}

/// Used when Claude is unavailable or rate-limited, so pep talks never go silent.
pub fn fallback_pep(seed: u64) -> &'static str {
    const LINES: [&str; 8] = [
        "you've got this! one small step at a time",
        "tiny progress is still progress. keep going!",
        "deep breath, sip of water, back at it. i believe in you",
        "focus mode: on. i'll guard the desktop *boing*",
        "stretch break? then let's crush the next task",
        "proud of you for showing up today",
        "pick the smallest next step and do just that one",
        "future you is gonna be so happy you did this",
    ];
    LINES[(seed % LINES.len() as u64) as usize]
}

pub fn clean_message(raw: &str) -> Result<String, String> {
    let msg = raw.trim();
    if msg.is_empty() {
        return Err("say something first!".into());
    }
    Ok(msg.chars().take(MAX_MESSAGE_CHARS).collect())
}

pub fn build_prompt(name: &str, status: &str, history: &History, message: &str) -> String {
    let mut prompt = format!("[your name] {name}\n[your current state] {status}\n");
    if !history.turns.is_empty() {
        prompt.push_str("[recent conversation]\n");
        for (human, pet) in &history.turns {
            prompt.push_str(&format!("human: {human}\nyou: {pet}\n"));
        }
    }
    prompt.push_str(&format!("[human says]\n{message}\n\nReply in character as the pet."));
    prompt
}

/// `claude.exe` from PATH, or the native installer's location when the app was
/// launched without the user's shell PATH. CLAUDE_PET_CLI overrides both.
fn cli_path() -> PathBuf {
    if let Some(p) = std::env::var_os("CLAUDE_PET_CLI") {
        return p.into();
    }
    let local = std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
        .map(|home| PathBuf::from(home).join(".local").join("bin").join(if cfg!(windows) { "claude.exe" } else { "claude" }));
    match local {
        Some(p) if p.exists() => p,
        _ => "claude".into(),
    }
}

pub const INSTALL_HELP: &str = "i need Claude Code to think! install it from claude.com/claude-code, \
     run `claude` once in a terminal to log in, then type /doctor";
const LOGIN_HELP: &str = "my brain isn't logged in. open a terminal, run `claude` and log in, then try again";

/// A `claude` process that never flashes a console window.
fn cli_command() -> tokio::process::Command {
    let mut cmd = tokio::process::Command::new(cli_path());
    cmd.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).kill_on_drop(true);
    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

/// Checks the CLI is installed, for `/doctor` and the first-run check.
pub async fn check_cli() -> Result<String, String> {
    let mut cmd = cli_command();
    cmd.arg("--version");
    match tokio::time::timeout(Duration::from_secs(20), cmd.output()).await {
        Ok(Ok(out)) if out.status.success() => {
            let version = String::from_utf8_lossy(&out.stdout).trim().to_string();
            Ok(format!("brain found: claude {version}. if chat fails, run `claude` in a terminal to log in"))
        }
        _ => Err(INSTALL_HELP.into()),
    }
}

/// Runs `claude -p <prompt>` in the brain folder with every tool disabled,
/// so the pet can only talk.
pub async fn ask_claude(brain: &Path, model: &str, prompt: &str) -> Result<String, String> {
    let mut cmd = cli_command();
    cmd.arg("-p")
        .arg(prompt)
        .args(["--model", model])
        .args(["--tools", ""])
        .arg("--strict-mcp-config")
        .arg("--no-session-persistence")
        // Skip the user's global settings (hooks, plugins) so they can't leak into replies.
        .args(["--setting-sources", "project,local"])
        .current_dir(brain);

    let output = tokio::time::timeout(CLI_TIMEOUT, cmd.output())
        .await
        .map_err(|_| "i zoned out… (claude took too long)".to_string())?
        .map_err(|_| INSTALL_HELP.to_string())?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if !output.status.success() || stdout.is_empty() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let detail = stderr.lines().chain(stdout.lines()).find(|l| !l.trim().is_empty()).unwrap_or("no output");
        let lower = detail.to_lowercase();
        if ["login", "log in", "api key", "auth"].iter().any(|k| lower.contains(k)) {
            return Err(LOGIN_HELP.into());
        }
        return Err(format!("brain fog… ({detail})"));
    }
    Ok(stdout)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn limiter() -> RateLimiter {
        RateLimiter::new(Duration::from_secs(5), 3, Duration::from_secs(60))
    }

    #[test]
    fn enforces_minimum_gap() {
        let mut rl = limiter();
        let t0 = Instant::now();
        assert!(rl.try_acquire(t0).is_ok());
        assert_eq!(rl.try_acquire(t0 + Duration::from_secs(2)), Err(Duration::from_secs(3)));
        assert!(rl.try_acquire(t0 + Duration::from_secs(5)).is_ok());
    }

    #[test]
    fn enforces_window_quota() {
        let mut rl = limiter();
        let t0 = Instant::now();
        for i in 0..3 {
            assert!(rl.try_acquire(t0 + Duration::from_secs(10 * i)).is_ok());
        }
        assert_eq!(rl.try_acquire(t0 + Duration::from_secs(30)), Err(Duration::from_secs(30)));
        assert!(rl.try_acquire(t0 + Duration::from_secs(60)).is_ok());
    }

    #[test]
    fn rejected_calls_do_not_count() {
        let mut rl = limiter();
        let t0 = Instant::now();
        rl.try_acquire(t0).unwrap();
        let _ = rl.try_acquire(t0 + Duration::from_secs(1));
        assert!(rl.try_acquire(t0 + Duration::from_secs(5)).is_ok());
    }

    #[test]
    fn history_keeps_only_recent_turns() {
        let mut h = History::default();
        for i in 0..10 {
            h.push(format!("q{i}"), format!("a{i}"));
        }
        assert_eq!(h.turns.len(), HISTORY_TURNS);
        assert_eq!(h.turns[0].0, "q4");
    }

    #[test]
    fn prompt_includes_state_history_and_message() {
        let mut h = History::default();
        h.push("hi".into(), "hello!".into());
        let p = build_prompt("Pip", "food 50/100", &h, "how are you?");
        assert!(p.starts_with("[your name] Pip\n[your current state] food 50/100"));
        assert!(p.contains("human: hi\nyou: hello!"));
        assert!(p.contains("[human says]\nhow are you?"));
    }

    #[test]
    fn messages_are_trimmed_and_capped() {
        assert!(clean_message("   ").is_err());
        assert_eq!(clean_message("  hey ").unwrap(), "hey");
        assert_eq!(clean_message(&"x".repeat(2000)).unwrap().len(), MAX_MESSAGE_CHARS);
    }

    #[test]
    fn brain_is_seeded_once_and_user_edits_survive() {
        let dir = std::env::temp_dir().join(format!("claude-pet-brain-{}", crate::stats::now_ms()));
        let brain = brain_dir(&dir, "blob").unwrap();
        let file = brain.join("CLAUDE.md");
        assert_eq!(fs::read_to_string(&file).unwrap(), DEFAULT_PERSONALITY);
        fs::write(&file, "custom").unwrap();
        brain_dir(&dir, "blob").unwrap();
        assert_eq!(fs::read_to_string(&file).unwrap(), "custom");
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn spine_has_its_own_personality_and_others_share_one() {
        let dir = std::env::temp_dir().join(format!("claude-pet-skins-{}", crate::stats::now_ms()));
        let main = brain_dir(&dir, "blob").unwrap();
        assert_eq!(brain_dir(&dir, "sprout").unwrap(), main, "ordinary skins share a brain");
        let spine = brain_dir(&dir, "spine").unwrap();
        assert_ne!(spine, main);
        let text = fs::read_to_string(spine.join("CLAUDE.md")).unwrap();
        assert!(text.contains("spine") && text.contains("sarcastic"));

        // Editing one never touches the other, and reset restores the right default.
        fs::write(main.join("CLAUDE.md"), "my edits").unwrap();
        reset_personality(&spine, "spine").unwrap();
        assert_eq!(fs::read_to_string(main.join("CLAUDE.md")).unwrap(), "my edits");
        assert_eq!(fs::read_to_string(spine.join("CLAUDE.md")).unwrap(), default_personality("spine"));
        fs::remove_dir_all(dir).unwrap();
    }

    /// Talks to the real CLI. Run with `cargo test -- --ignored`.
    #[test]
    #[ignore]
    fn real_claude_replies_in_character() {
        let dir = std::env::temp_dir().join(format!("claude-pet-live-{}", crate::stats::now_ms()));
        let brain = brain_dir(&dir, "blob").unwrap();
        let prompt = build_prompt("Mochi", "food 20/100 (low = starving), mood 60/100, energy 70/100, awake", &History::default(), "hi! what's your name?");
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        let reply = rt.block_on(ask_claude(&brain, "haiku", &prompt)).unwrap();
        println!("mochi: {reply}");
        assert!(reply.to_lowercase().contains("mochi"));
        fs::remove_dir_all(dir).unwrap();
    }
}
