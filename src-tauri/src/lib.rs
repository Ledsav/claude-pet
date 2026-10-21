mod chat;
mod stats;
mod tray;

use chat::{History, RateLimiter};
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
    let (result, snapshot) = {
        let mut pet = state.pet.lock().unwrap();
        pet.tick(now_ms());
        let result = f(&mut pet);
        (result, pet.clone())
    };
    if let Err(e) = state.store.save(&snapshot) {
        eprintln!("failed to save pet: {e}");
    }
    let _ = app.emit("stats", &snapshot);
    tray::show_stats(app, &snapshot);
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

    let status = with_pet(&app, |pet| pet.describe());
    let prompt = chat::build_prompt(&status, &state.history.lock().unwrap(), &message);
    let reply = chat::ask_claude(&state.brain, &prompt).await?;

    state.history.lock().unwrap().push(message, reply.clone());
    with_pet(&app, Pet::chatted);
    Ok(reply)
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
        .invoke_handler(tauri::generate_handler![get_pet, feed, play, sleep, chat])
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            let store = Store::new(&data_dir);
            let pet = store.load(now_ms());
            app.manage(AppState {
                pet: Mutex::new(pet),
                store,
                brain: chat::brain_dir(&data_dir)?,
                limiter: Mutex::new(RateLimiter::new(
                    Duration::from_secs(4),
                    30,
                    Duration::from_secs(60 * 60),
                )),
                history: Mutex::new(History::default()),
                thinking: AtomicBool::new(false),
            });

            let window = app
                .get_webview_window("main")
                .expect("main window is declared in tauri.conf.json");
            place_bottom_right(&window)?;
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
