use crate::{
    error::{AppError, AppResult},
    state::AppState,
};
use novamixer_contracts::EfficiencyStatus;
#[cfg(debug_assertions)]
use tauri::Manager;
use tauri::State;
#[tauri::command]
pub async fn set_efficiency_mode(
    state: State<'_, AppState>,
    enabled: bool,
) -> AppResult<EfficiencyStatus> {
    let status = crate::efficiency::set(enabled);
    if status.enabled == enabled {
        let mut settings = state.application.settings();
        settings.efficiency_mode = enabled;
        state
            .application
            .save_settings(settings)
            .await
            .map_err(|e| AppError::Other(e.to_string()))?;
    }
    Ok(status)
}
#[tauri::command]
pub fn get_efficiency_status() -> EfficiencyStatus {
    crate::efficiency::get()
}
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
    #[cfg(debug_assertions)]
    {
        let window = app
            .get_webview_window("main")
            .ok_or_else(|| AppError::Other("main window not found".into()))?;
        window.open_devtools();
        Ok(())
    }
    #[cfg(not(debug_assertions))]
    {
        let _ = app;
        Err(AppError::Other(
            "developer tools are available only in debug builds".into(),
        ))
    }
}
