use crate::{
    error::{AppError, AppResult},
    state::AppState,
};
use novamixer_contracts::{AppSettings, HotkeyBinding};
use tauri::{Emitter, State};
fn app_error(error: novamixer_application::ApplicationError) -> AppError {
    AppError::Other(error.to_string())
}
#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> AppSettings {
    state.application.settings()
}
#[tauri::command]
pub async fn save_settings(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    settings: AppSettings,
) -> AppResult<()> {
    state
        .application
        .save_settings(settings.clone())
        .await
        .map_err(app_error)?;
    *state.settings.write() = settings.clone();
    let _ = app.emit("settings-updated", settings);
    Ok(())
}
#[tauri::command]
pub fn restore_backup(app: tauri::AppHandle, state: State<'_, AppState>) -> AppResult<AppSettings> {
    let settings = state.application.restore_backup().map_err(app_error)?;
    *state.settings.write() = settings.clone();
    let _ = app.emit("settings-updated", settings.clone());
    Ok(settings)
}
#[tauri::command]
pub async fn set_hotkeys(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    hotkeys: Vec<HotkeyBinding>,
) -> AppResult<()> {
    let mut settings = state.application.settings();
    settings.hotkeys = hotkeys;
    save_settings(app, state, settings).await
}
