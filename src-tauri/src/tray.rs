use crate::{perform, personality, stats::Pet};
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager,
};

pub const TRAY_ID: &str = "pet-tray";

pub fn toggle_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        if window.is_visible().unwrap_or(false) {
            let _ = window.hide();
        } else {
            let _ = window.show();
        }
    }
}

/// Keeps the tray tooltip showing current stats.
pub fn show_stats(app: &AppHandle, name: &str, pet: &Pet) {
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let tip = format!(
            "{name}{} · food {:.0} · mood {:.0} · energy {:.0}",
            if pet.asleep { " (asleep)" } else { "" },
            pet.hunger,
            pet.mood,
            pet.energy
        );
        let _ = tray.set_tooltip(Some(tip));
    }
}

pub fn build(app: &AppHandle) -> tauri::Result<()> {
    let item = |id: &str, label: &str| MenuItem::with_id(app, id, label, true, None::<&str>);
    let menu = Menu::with_items(
        app,
        &[
            &item("toggle", "Show / hide pet")?,
            &PredefinedMenuItem::separator(app)?,
            &item("feed", "Feed")?,
            &item("play", "Play")?,
            &item("sleep", "Sleep / wake")?,
            &PredefinedMenuItem::separator(app)?,
            &item("personality", "Edit personality…")?,
            &item("quit", "Quit")?,
        ],
    )?;

    TrayIconBuilder::with_id(TRAY_ID)
        .icon(app.default_window_icon().cloned().expect("bundle icon"))
        .tooltip("Mochi")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "toggle" => toggle_window(app),
            "feed" => drop(perform(app, Pet::feed)),
            "play" => drop(perform(app, Pet::play)),
            "sleep" => drop(perform(app, Pet::toggle_sleep)),
            "personality" => personality::open_editor(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                toggle_window(tray.app_handle());
            }
        })
        .build(app)?;
    Ok(())
}
