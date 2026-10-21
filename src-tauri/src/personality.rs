//! The personality editor: a normal window for editing the current skin's CLAUDE.md.

use crate::{chat, AppState};
use std::fs;
use tauri::{AppHandle, Manager, State, WebviewUrl, WebviewWindowBuilder};

const LABEL: &str = "personality";
const MAX_CHARS: usize = 20_000;

/// Opens the editor, or brings it to the front if it's already open.
pub fn open_editor(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(LABEL) {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
        return;
    }
    let built = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App("personality.html".into()))
        .title("Personality")
        .inner_size(560.0, 620.0)
        .min_inner_size(420.0, 420.0)
        .center()
        .build();
    if let Err(e) = built {
        eprintln!("couldn't open personality editor: {e}");
    }
}

fn file(state: &AppState) -> std::path::PathBuf {
    state.brain().join("CLAUDE.md")
}

#[tauri::command]
pub fn open_personality(app: AppHandle) {
    open_editor(&app);
}

/// Which personality the editor is showing: the shared one, or the skin's own.
#[derive(serde::Serialize)]
pub struct PersonalityInfo {
    skin: String,
    own: bool,
}

#[tauri::command]
pub fn personality_info(state: State<'_, AppState>) -> PersonalityInfo {
    let skin = state.skin();
    PersonalityInfo { own: chat::has_own_personality(&skin), skin }
}

#[tauri::command]
pub fn get_personality(state: State<'_, AppState>) -> Result<String, String> {
    fs::read_to_string(file(&state)).map_err(|e| format!("couldn't read personality: {e}"))
}

#[tauri::command]
pub fn save_personality(state: State<'_, AppState>, text: String) -> Result<(), String> {
    if text.trim().is_empty() {
        return Err("the personality can't be empty. use reset to restore the default".into());
    }
    if text.chars().count() > MAX_CHARS {
        return Err(format!("keep it under {MAX_CHARS} characters; it's sent with every message"));
    }
    fs::write(file(&state), text).map_err(|e| format!("couldn't save personality: {e}"))
}

/// Restores the built-in personality and returns it.
#[tauri::command]
pub fn reset_personality(state: State<'_, AppState>) -> Result<String, String> {
    chat::reset_personality(&state.brain(), &state.skin()).map_err(|e| format!("couldn't reset: {e}"))?;
    get_personality(state)
}

/// For people who'd rather use their own editor.
#[tauri::command]
pub fn open_personality_file(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    app.opener()
        .open_path(file(&state).to_string_lossy(), None::<&str>)
        .map_err(|e| format!("couldn't open the file: {e}"))
}
