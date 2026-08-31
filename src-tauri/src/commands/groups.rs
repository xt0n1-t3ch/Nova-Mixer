use crate::{
    error::{AppError, AppResult},
    state::AppState,
};
use novamixer_contracts::{AppBinding, Group};
use tauri::State;
fn app_error(error: novamixer_application::ApplicationError) -> AppError {
    AppError::Other(error.to_string())
}
#[tauri::command]
pub async fn set_group_volume(
    state: State<'_, AppState>,
    group_id: String,
    volume: f32,
) -> AppResult<()> {
    state
        .application
        .set_group_volume(&group_id, volume)
        .await
        .map_err(app_error)
}
#[tauri::command]
pub async fn set_active_group(
    state: State<'_, AppState>,
    group_id: Option<String>,
) -> AppResult<()> {
    state
        .application
        .set_active_group(group_id)
        .await
        .map_err(app_error)
}
#[tauri::command]
pub async fn upsert_group(state: State<'_, AppState>, group: Group) -> AppResult<Group> {
    state
        .application
        .upsert_group(group)
        .await
        .map_err(app_error)
}
#[tauri::command]
pub async fn delete_group(state: State<'_, AppState>, group_id: String) -> AppResult<()> {
    state
        .application
        .delete_group(&group_id)
        .await
        .map_err(app_error)
}
#[tauri::command]
pub async fn list_running_apps(state: State<'_, AppState>) -> AppResult<Vec<AppBinding>> {
    state.application.running_apps().await.map_err(app_error)
}
