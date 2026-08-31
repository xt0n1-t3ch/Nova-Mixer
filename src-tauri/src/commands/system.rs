use crate::{
    error::{AppError, AppResult},
    state::AppState,
};
use tauri::{Manager, State};
#[tauri::command]
pub fn open_data_folder(state: State<'_, AppState>) -> AppResult<()> {
    std::process::Command::new("explorer.exe")
        .arg(state.application.data_root())
        .spawn()
        .map_err(AppError::Io)?;
    Ok(())
}
#[tauri::command]
pub fn open_devtools(app: tauri::AppHandle) -> AppResult<()> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| AppError::Other("main window not found".into()))?;
    window.open_devtools();
    Ok(())
}
