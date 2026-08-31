use crate::{
    error::{AppError, AppResult},
    state::AppState,
};
use novamixer_contracts::{
    AppCandidate, Application, ApplicationPatch, AudioDevice, MixerSnapshot,
};
use tauri::State;
pub fn audio_error(e: audio_sessions::AudioError) -> AppError {
    match e {
        audio_sessions::AudioError::SessionGone => AppError::SessionGone,
        audio_sessions::AudioError::AppUnknown => AppError::AppUnknown,
        audio_sessions::AudioError::NotControllable => AppError::NotControllable,
        audio_sessions::AudioError::InvalidVolume => AppError::Validation(e.to_string()),
        audio_sessions::AudioError::Unavailable(message) => AppError::AudioUnavailable(message),
        audio_sessions::AudioError::Unsupported(message) => AppError::Unsupported(message),
    }
}
fn app_error(e: novamixer_application::ApplicationError) -> AppError {
    AppError::Other(e.to_string())
}
#[tauri::command]
pub async fn list_applications(state: State<'_, AppState>) -> AppResult<MixerSnapshot> {
    state.application.snapshot().await.map_err(app_error)
}
#[tauri::command]
pub async fn set_app_volume(
    state: State<'_, AppState>,
    app_key: String,
    volume: f32,
) -> AppResult<()> {
    state
        .application
        .set_app_volume(&app_key, volume)
        .await
        .map_err(app_error)
}
#[tauri::command]
pub async fn set_app_mute(
    state: State<'_, AppState>,
    app_key: String,
    muted: bool,
) -> AppResult<()> {
    state
        .application
        .set_app_mute(&app_key, muted)
        .await
        .map_err(app_error)
}
#[tauri::command]
pub async fn set_session_volume(
    state: State<'_, AppState>,
    live_id: String,
    volume: f32,
) -> AppResult<()> {
    state
        .application
        .audio
        .set_session_volume(live_id, volume)
        .await
        .map_err(audio_error)
}
#[tauri::command]
pub async fn set_session_mute(
    state: State<'_, AppState>,
    live_id: String,
    muted: bool,
) -> AppResult<()> {
    state
        .application
        .audio
        .set_session_mute(live_id, muted)
        .await
        .map_err(audio_error)
}
#[tauri::command]
pub async fn add_application(state: State<'_, AppState>, path: String) -> AppResult<Application> {
    state
        .application
        .add_application(&path)
        .await
        .map_err(app_error)
}
#[tauri::command]
pub async fn remove_application(state: State<'_, AppState>, app_key: String) -> AppResult<()> {
    state
        .application
        .remove_application(&app_key)
        .await
        .map_err(app_error)
}
#[tauri::command]
pub async fn update_application(
    state: State<'_, AppState>,
    app_key: String,
    patch: ApplicationPatch,
) -> AppResult<Application> {
    state
        .application
        .update_application(&app_key, patch)
        .await
        .map_err(app_error)
}
#[tauri::command]
pub async fn reorder_applications(
    state: State<'_, AppState>,
    app_keys: Vec<String>,
) -> AppResult<()> {
    state
        .application
        .reorder_applications(app_keys)
        .await
        .map_err(app_error)
}
#[tauri::command]
pub async fn list_app_candidates(state: State<'_, AppState>) -> AppResult<Vec<AppCandidate>> {
    state.application.list_candidates().await.map_err(app_error)
}
#[tauri::command]
pub async fn set_master_volume(state: State<'_, AppState>, volume: f32) -> AppResult<()> {
    state
        .application
        .audio
        .set_master_volume(volume)
        .await
        .map_err(audio_error)
}
#[tauri::command]
pub async fn set_master_mute(state: State<'_, AppState>, muted: bool) -> AppResult<()> {
    state
        .application
        .audio
        .set_master_mute(muted)
        .await
        .map_err(audio_error)
}
#[tauri::command]
pub async fn list_output_devices(state: State<'_, AppState>) -> AppResult<Vec<AudioDevice>> {
    state
        .application
        .audio
        .list_output_devices()
        .await
        .map_err(audio_error)
}
#[tauri::command]
pub async fn set_default_output(state: State<'_, AppState>, device_id: String) -> AppResult<()> {
    state
        .application
        .audio
        .set_default_output(device_id)
        .await
        .map_err(audio_error)
}
#[tauri::command]
pub async fn set_metering_active(state: State<'_, AppState>, active: bool) -> AppResult<()> {
    state
        .application
        .audio
        .set_metering_active(active)
        .await
        .map_err(audio_error)
}
