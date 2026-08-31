mod commands;
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
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
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
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let paths =
                paths::AppPaths::resolve(app.handle()).map_err(|error| error.to_string())?;
            paths.ensure_dirs()?;
            let handle = app.handle().clone();
            let application =
                novamixer_application::NovaMixerApplication::start(move |event| match event {
                    audio_sessions::AudioEvent::SessionAdded(value) => {
                        let _ = handle.emit("session-added", value);
                    }
                    audio_sessions::AudioEvent::SessionUpdated(value) => {
                        let _ = handle.emit("session-updated", value);
                    }
                    audio_sessions::AudioEvent::SessionRemoved { live_id } => {
                        let _ = handle
                            .emit("session-removed", serde_json::json!({ "live_id": live_id }));
                    }
                    audio_sessions::AudioEvent::MasterUpdated(value) => {
                        let _ = handle.emit("master-updated", value);
                    }
                    audio_sessions::AudioEvent::EndpointChanged(value) => {
                        let _ = handle.emit("endpoint-changed", value);
                    }
                    audio_sessions::AudioEvent::Peaks(value) => {
                        let _ = handle.emit("peaks", value);
                    }
                })
                .map_err(|error| error.to_string())?;
            app.manage(state::AppState::new(application));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_sessions,
            set_session_volume,
            set_session_mute,
            set_master_volume,
            set_master_mute,
            set_group_volume,
            set_active_group,
            upsert_group,
            delete_group,
            list_running_apps,
            get_settings,
            save_settings,
            restore_backup,
            set_hotkeys,
            open_data_folder,
            set_metering_active,
            open_devtools,
        ])
        .run(tauri::generate_context!())
        .expect("error while running NovaMixer");
}
