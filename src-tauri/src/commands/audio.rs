use crate::{
    error::{AppError, AppResult},
    state::AppState,
};
use novamixer_contracts::MixerSnapshot;
use tauri::State;

fn audio_error(error: audio_sessions::AudioError) -> AppError {
    match error {
        audio_sessions::AudioError::SessionGone => AppError::SessionGone,
        audio_sessions::AudioError::NotControllable => AppError::NotControllable,
        audio_sessions::AudioError::InvalidVolume => AppError::Validation(error.to_string()),
        audio_sessions::AudioError::Unavailable(_) => AppError::AudioUnavailable,
    }
}

#[tauri::command]
pub async fn list_sessions(state: State<'_, AppState>) -> AppResult<MixerSnapshot> {
    state
        .application
        .audio
        .list_sessions()
        .await
        .map_err(audio_error)
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
pub async fn set_metering_active(state: State<'_, AppState>, active: bool) -> AppResult<()> {
    state
        .application
        .audio
        .set_metering_active(active)
        .await
        .map_err(audio_error)
}
