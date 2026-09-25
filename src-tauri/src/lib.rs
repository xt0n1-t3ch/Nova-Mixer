mod commands;
mod desktop;
mod efficiency;
mod error;
mod ipc_bindings;
mod logging;
mod paths;
mod state;

use commands::{audio::*, groups::*, settings::*, system::*};
use tauri::{Emitter, Manager};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(debug_assertions)]
    {
        const CDP_REMOTE_DEBUGGING_PORT: u16 = 9333;
        std::env::set_var(
            "WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS",
            format!("--remote-debugging-port={CDP_REMOTE_DEBUGGING_PORT}"),
        );
    }
    let _log_guard = logging::init();
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            // Reuse the tray restore path: `show()` alone does not move a
            // minimized Windows window back from its off-screen sentinel
            // coordinates (`-32000,-32000`).
            desktop::show_window(app);
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_process::init())
        // The updater plugin is deliberately absent: it refuses to initialize
        // without a `plugins.updater` block, and there is no release endpoint to
        // point one at yet. Registering it anyway panicked at startup.
        .setup(|app| {
            let paths =
                paths::AppPaths::resolve(app.handle()).map_err(|error| error.to_string())?;
            paths.ensure_dirs()?;
            let handle = app.handle().clone();
            let application =
                match novamixer_application::NovaMixerApplication::start(move |event| match event {
                    audio_sessions::AudioEvent::ApplicationAdded(value) => {
                        let _ = handle.emit("application-added", value);
                    }
                    audio_sessions::AudioEvent::ApplicationUpdated(value) => {
                        let _ = handle.emit("application-updated", value);
                    }
                    audio_sessions::AudioEvent::ApplicationRemoved { app_key } => {
                        let _ = handle.emit(
                            "application-removed",
                            serde_json::json!({ "app_key": app_key }),
                        );
                    }
                    audio_sessions::AudioEvent::MasterUpdated(value) => {
                        desktop::update_tray(&handle, value.volume, value.muted);
                        let _ = handle.emit("master-updated", value);
                    }
                    audio_sessions::AudioEvent::EndpointChanged(value) => {
                        let _ = handle.emit("endpoint-changed", value);
                    }
                    audio_sessions::AudioEvent::Peaks(value) => {
                        let _ = handle.emit("peaks", value);
                    }
                }) {
                    Ok(application) => application,
                    Err(error) => {
                        let message = format!("Windows audio could not be initialized.\n\n{error}");
                        tracing::error!(%error, "application startup failed");
                        desktop::show_startup_error(&message);
                        return Err(error.into());
                    }
                };
            let efficiency_enabled = application.settings().efficiency_mode;
            app.manage(state::AppState::new(application));
            if efficiency_enabled {
                let status = efficiency::set(true);
                if !status.enabled {
                    tracing::warn!(detail = ?status.detail, "cannot reapply efficiency mode");
                }
            }
            desktop::setup(app.handle()).map_err(|error| error.to_string())?;
            // The worker has adopted the sessions that were already playing; write
            // what they report about their executables (a Squirrel app's current
            // `app-<version>` folder) back to the saved applications.
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let state = handle.state::<state::AppState>();
                match state.application.sync_live_identities().await {
                    Ok(Some(settings)) => {
                        *state.settings.write() = settings.clone();
                        let _ = handle.emit("settings-updated", settings);
                    }
                    Ok(None) => {}
                    Err(error) => {
                        tracing::warn!(%error, "cannot save live application paths at startup")
                    }
                }
            });
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let minimize = window
                    .state::<state::AppState>()
                    .application
                    .settings()
                    .minimize_to_tray;
                if minimize {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            list_applications,
            set_app_volume,
            set_app_mute,
            set_session_volume,
            set_session_mute,
            add_application,
            remove_application,
            update_application,
            reorder_applications,
            list_app_candidates,
            set_master_volume,
            set_master_mute,
            list_output_devices,
            set_default_output,
            set_group_volume,
            set_active_group,
            upsert_group,
            delete_group,
            upsert_scene,
            delete_scene,
            apply_scene,
            capture_scene,
            get_settings,
            save_settings,
            restore_backup,
            set_hotkeys,
            set_efficiency_mode,
            get_efficiency_status,
            open_data_folder,
            set_metering_active,
            open_devtools,
        ])
        .run(tauri::generate_context!())
        .expect("error while running NovaMixer");
}
