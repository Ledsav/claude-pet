mod stats;
mod tray;

use stats::{now_ms, Outcome, Pet, Store};
use std::{sync::Mutex, thread, time::Duration};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, WebviewWindow};

const TICK: Duration = Duration::from_secs(5);

pub struct AppState {
    pet: Mutex<Pet>,
    store: Store,
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
        .invoke_handler(tauri::generate_handler![get_pet, feed, play, sleep])
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            let store = Store::new(&data_dir);
            let pet = store.load(now_ms());
            app.manage(AppState { pet: Mutex::new(pet), store });

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
