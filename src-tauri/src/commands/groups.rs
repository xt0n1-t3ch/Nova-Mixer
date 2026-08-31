use crate::{
    error::{AppError, AppResult},
    state::AppState,
};
use novamixer_contracts::{Group, Scene};
use tauri::{Emitter, State};
fn app_error(e: novamixer_application::ApplicationError) -> AppError {
    AppError::Other(e.to_string())
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
pub async fn upsert_scene(state: State<'_, AppState>, scene: Scene) -> AppResult<Scene> {
    state
        .application
        .upsert_scene(scene)
        .await
        .map_err(app_error)
}
#[tauri::command]
pub async fn delete_scene(state: State<'_, AppState>, scene_id: String) -> AppResult<()> {
    state
        .application
        .delete_scene(&scene_id)
        .await
        .map_err(app_error)
}
#[tauri::command]
pub async fn apply_scene(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    scene_id: String,
) -> AppResult<()> {
    state
        .application
        .apply_scene(&scene_id)
        .await
        .map_err(app_error)?;
    let _ = app.emit("scene-applied", serde_json::json!({"scene_id":scene_id}));
    Ok(())
}
#[tauri::command]
pub async fn capture_scene(state: State<'_, AppState>, name: String) -> AppResult<Scene> {
    state
        .application
        .capture_scene(name)
        .await
        .map_err(app_error)
}
