mod chat;
mod settings;
mod stats;
mod tray;

use chat::{History, RateLimiter};
use settings::{Command, Settings};
use stats::{now_ms, Outcome, Pet, Store};
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    },
    thread,
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, State, WebviewWindow};

const TICK: Duration = Duration::from_secs(5);

pub struct AppState {
    pet: Mutex<Pet>,
    store: Store,
    settings: Mutex<Settings>,
    data_dir: PathBuf,
    /// Folder whose CLAUDE.md is the pet's personality.
    pub brain: PathBuf,
    limiter: Mutex<RateLimiter>,
    history: Mutex<History>,
    thinking: AtomicBool,
}

/// Clears the "thinking" flag however the chat call ends.
struct Thinking<'a>(&'a AtomicBool);

impl Drop for Thinking<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

/// Catches the pet up on decay, applies `f`, then saves and broadcasts the result.
/// Every stat change goes through here so the UI, tray and save file never disagree.
pub fn with_pet<R>(app: &AppHandle, f: impl FnOnce(&mut Pet) -> R) -> R {
    let state = app.state::<AppState>();
    let (speed, name) = {
        let s = state.settings.lock().unwrap();
        (s.speed, s.name.clone())
    };
    let (result, snapshot) = {
        let mut pet = state.pet.lock().unwrap();
        pet.tick(now_ms(), speed);
        let result = f(&mut pet);
        (result, pet.clone())
    };
    if let Err(e) = state.store.save(&snapshot) {
        eprintln!("failed to save pet: {e}");
    }
    let _ = app.emit("stats", &snapshot);
    tray::show_stats(app, &name, &snapshot);
    result
}

/// Runs an action and tells the UI how the pet reacted, whoever triggered it.
pub fn perform(app: &AppHandle, action: fn(&mut Pet) -> Outcome) -> Outcome {
    let outcome = with_pet(app, action);
    let _ = app.emit("reaction", &outcome);
    outcome
}

#[tauri::command]
fn get_pet(app: AppHandle) -> Pet {
    with_pet(&app, |pet| pet.clone())
}

#[tauri::command]
fn feed(app: AppHandle) -> Outcome {
    perform(&app, Pet::feed)
}

#[tauri::command]
fn play(app: AppHandle) -> Outcome {
    perform(&app, Pet::play)
}

#[tauri::command]
fn sleep(app: AppHandle) -> Outcome {
    perform(&app, Pet::toggle_sleep)
}

/// Sends a message to the pet. One call at a time, and rate-limited so a chatty
/// afternoon doesn't eat the Claude subscription.
#[tauri::command]
async fn chat(app: AppHandle, state: State<'_, AppState>, message: String) -> Result<String, String> {
    let message = chat::clean_message(&message)?;
    if state.thinking.swap(true, Ordering::SeqCst) {
        return Err("hold on, i'm still thinking…".into());
    }
    let _thinking = Thinking(&state.thinking);
    if let Err(wait) = state.limiter.lock().unwrap().try_acquire(Instant::now()) {
        return Err(format!("my brain needs a break… try again in {}s", wait.as_secs().max(1)));
    }

    let (name, model) = {
        let s = state.settings.lock().unwrap();
        (s.name.clone(), s.model.clone())
    };
    let status = with_pet(&app, |pet| pet.describe());
    let prompt = chat::build_prompt(&name, &status, &state.history.lock().unwrap(), &message);
    let reply = chat::ask_claude(&state.brain, &model, &prompt).await?;

    state.history.lock().unwrap().push(message, reply.clone());
    with_pet(&app, Pet::chatted);
    Ok(reply)
}

#[tauri::command]
fn get_settings(state: State<'_, AppState>) -> Settings {
    state.settings.lock().unwrap().clone()
}

/// Handles chat-box input starting with `/`. These never reach Claude.
#[tauri::command]
fn command(app: AppHandle, state: State<'_, AppState>, input: String) -> Result<String, String> {
    match settings::parse(&input)? {
        Command::Help => Ok(settings::HELP.into()),
        Command::ShowSettings => Ok(state.settings.lock().unwrap().describe()),
        Command::Set(key, value) => {
            let updated = {
                let mut s = state.settings.lock().unwrap();
                let msg = s.set(&key, &value)?;
                if let Err(e) = s.save(&state.data_dir) {
                    eprintln!("failed to save settings: {e}");
                }
                (msg, s.clone())
            };
            apply_settings(&app, &updated.1);
            Ok(updated.0)
        }
        Command::Personality => {
            tray::open_personality(&app);
            Ok("opening my personality file… save it and i'll change on the next chat".into())
        }
        Command::Stats => Ok(with_pet(&app, |pet| pet.describe())),
        Command::Clear => {
            state.history.lock().unwrap().clear();
            Ok("conversation forgotten. who are you again? :)".into())
        }
        Command::Reset { confirmed: false } => {
            Err("this hatches a brand-new pet with fresh stats. type /reset confirm to do it".into())
        }
        Command::Reset { confirmed: true } => {
            with_pet(&app, |pet| *pet = Pet::new(now_ms()));
            state.history.lock().unwrap().clear();
            Ok("*crack* …a fresh new pet! hello!".into())
        }
    }
}

/// Pushes settings that affect the window, the limiter and the UI.
fn apply_settings(app: &AppHandle, s: &Settings) {
    let state = app.state::<AppState>();
    state.limiter.lock().unwrap().set_max_calls(s.hourly_limit);
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.set_always_on_top(s.on_top);
    }
    let _ = app.emit("settings", s);
    with_pet(app, |_| ()); // refresh the tray tooltip with the new name
}

/// Parks the pet in the bottom-right corner of the monitor it opens on.
fn place_bottom_right(window: &WebviewWindow) -> tauri::Result<()> {
    if let Some(monitor) = window.current_monitor()? {
        let area = monitor.work_area();
        let size = window.outer_size()?;
        let margin = 24;
        window.set_position(PhysicalPosition::new(
            area.position.x + area.size.width as i32 - size.width as i32 - margin,
            area.position.y + area.size.height as i32 - size.height as i32,
        ))?;
    }
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![get_pet, feed, play, sleep, chat, get_settings, command])
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            let settings = Settings::load(&data_dir);
            let store = Store::new(&data_dir);
            let pet = store.load(now_ms(), settings.speed);
            let hourly_limit = settings.hourly_limit;
            app.manage(AppState {
                pet: Mutex::new(pet),
                store,
                settings: Mutex::new(settings.clone()),
                data_dir: data_dir.clone(),
                brain: chat::brain_dir(&data_dir)?,
                limiter: Mutex::new(RateLimiter::new(
                    Duration::from_secs(4),
                    hourly_limit,
                    Duration::from_secs(60 * 60),
                )),
                history: Mutex::new(History::default()),
                thinking: AtomicBool::new(false),
            });

            let window = app
                .get_webview_window("main")
                .expect("main window is declared in tauri.conf.json");
            place_bottom_right(&window)?;
            window.set_always_on_top(settings.on_top)?;
            window.show()?;
            tray::build(app.handle())?;

            let handle = app.handle().clone();
            thread::spawn(move || loop {
                with_pet(&handle, |_| ());
                thread::sleep(TICK);
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Claude Pet");
}
