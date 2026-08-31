use crate::state::AppState;
use novamixer_contracts::{HotkeyAction, HotkeyBinding};
use tauri::{
    image::Image,
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, Runtime,
};
use tauri_plugin_autostart::ManagerExt as AutostartExt;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

const TRAY_ID: &str = "novamixer-tray";
const SHOW_HIDE_ID: &str = "tray-show-hide";
const MUTE_ID: &str = "tray-mute";
const QUIT_ID: &str = "tray-quit";

pub fn setup<R: Runtime>(app: &AppHandle<R>) -> Result<(), Box<dyn std::error::Error>> {
    build_tray(app)?;
    let settings = app.state::<AppState>().application.settings();
    sync_autostart(app, settings.launch_on_startup)?;
    register_hotkeys(app, &settings.hotkeys)?;
    if settings.start_minimized {
        if let Some(window) = app.get_webview_window("main") {
            window.hide()?;
        }
    }
    Ok(())
}

pub fn sync_settings<R: Runtime>(
    app: &AppHandle<R>,
    settings: &novamixer_contracts::AppSettings,
) -> Result<(), Box<dyn std::error::Error>> {
    sync_autostart(app, settings.launch_on_startup)?;
    register_hotkeys(app, &settings.hotkeys)?;
    Ok(())
}

fn build_tray<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let show_hide = MenuItem::with_id(
        app,
        SHOW_HIDE_ID,
        "Show or hide NovaMixer",
        true,
        None::<&str>,
    )?;
    let mute = MenuItem::with_id(app, MUTE_ID, "Toggle mute", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, QUIT_ID, "Quit NovaMixer", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show_hide, &mute, &quit])?;
    let icon = volume_icon(1.0, false);
    TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon)
        .tooltip("NovaMixer volume 100%")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            SHOW_HIDE_ID => toggle_window(app),
            MUTE_ID => spawn_tray_mute(app.clone()),
            QUIT_ID => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_window(tray.app_handle());
            }
        })
        .build(app)?;
    Ok(())
}

pub fn update_tray<R: Runtime>(app: &AppHandle<R>, volume: f32, muted: bool) {
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let _ = tray.set_icon(Some(volume_icon(volume, muted)));
        let percent = (audio_policy::clamp_scalar(volume) * 100.0).round() as u32;
        let text = if muted {
            format!("NovaMixer muted at {percent}%")
        } else {
            format!("NovaMixer volume {percent}%")
        };
        let _ = tray.set_tooltip(Some(text));
    }
}

pub fn register_hotkeys<R: Runtime>(
    app: &AppHandle<R>,
    hotkeys: &[HotkeyBinding],
) -> Result<(), Box<dyn std::error::Error>> {
    app.global_shortcut().unregister_all()?;
    for binding in hotkeys {
        let Some(accelerator) = binding.accelerator.clone() else {
            continue;
        };
        let action = binding.action.clone();
        // A binding the platform rejects — an unparseable accelerator, or one
        // another application already owns — is logged and skipped. Propagating
        // it would abort the setup hook and take the whole application down over
        // one unusable shortcut, which is what a settings file written by an
        // older build can easily contain.
        if let Err(error) =
            app.global_shortcut()
                .on_shortcut(accelerator.as_str(), move |app, _, event| {
                    if event.state() == ShortcutState::Pressed {
                        spawn_hotkey(app.clone(), action.clone());
                    }
                })
        {
            tracing::warn!(
                accelerator = %accelerator,
                action = ?binding.action,
                error = %error,
                "global hotkey could not be registered; leaving it unbound"
            );
        }
    }
    Ok(())
}

fn sync_autostart<R: Runtime>(
    app: &AppHandle<R>,
    enabled: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let autostart = app.autolaunch();
    if enabled && !autostart.is_enabled()? {
        autostart.enable()?;
    } else if !enabled && autostart.is_enabled()? {
        autostart.disable()?;
    }
    Ok(())
}

fn spawn_hotkey<R: Runtime>(app: AppHandle<R>, action: HotkeyAction) {
    tauri::async_runtime::spawn(async move {
        if let Err(err) = apply_hotkey(&app, action.clone()).await {
            tracing::warn!(error = %err, "global hotkey action failed");
            return;
        }
        let _ = app.emit("hotkey-fired", serde_json::json!({ "action": action }));
    });
}

async fn apply_hotkey<R: Runtime>(
    app: &AppHandle<R>,
    action: HotkeyAction,
) -> Result<(), Box<dyn std::error::Error>> {
    let state = app.state::<AppState>();
    let settings = state.application.settings();
    let snapshot = state.application.audio.list_applications().await?;
    let active_group = settings
        .active_group_id
        .as_deref()
        .and_then(|id| settings.groups.iter().find(|group| group.id == id))
        .filter(|group| group.hotkeys_enabled);
    if let Some(group) = active_group {
        let target = match action {
            HotkeyAction::VolumeUp => audio_policy::stepped_volume(
                group.volume,
                audio_policy::effective_step(
                    group.volume,
                    settings.volume_step,
                    settings.smart_volume,
                ),
            ),
            HotkeyAction::VolumeDown => audio_policy::stepped_volume(
                group.volume,
                -audio_policy::effective_step(
                    group.volume,
                    settings.volume_step,
                    settings.smart_volume,
                ),
            ),
            HotkeyAction::MuteToggle => {
                for application in snapshot.applications.iter().filter(|application| {
                    group.app_keys.iter().any(|key| key == &application.app_key)
                }) {
                    state
                        .application
                        .set_app_mute(&application.app_key, !application.muted)
                        .await?;
                }
                return Ok(());
            }
        };
        state
            .application
            .set_group_volume(&group.id, target)
            .await?;
    } else {
        match action {
            HotkeyAction::VolumeUp => {
                let step = audio_policy::effective_step(
                    snapshot.master.volume,
                    settings.volume_step,
                    settings.smart_volume,
                );
                state
                    .application
                    .audio
                    .set_master_volume(audio_policy::stepped_volume(snapshot.master.volume, step))
                    .await?;
            }
            HotkeyAction::VolumeDown => {
                let step = audio_policy::effective_step(
                    snapshot.master.volume,
                    settings.volume_step,
                    settings.smart_volume,
                );
                state
                    .application
                    .audio
                    .set_master_volume(audio_policy::stepped_volume(snapshot.master.volume, -step))
                    .await?;
            }
            HotkeyAction::MuteToggle => {
                state
                    .application
                    .audio
                    .set_master_mute(!snapshot.master.muted)
                    .await?;
            }
        }
    }
    Ok(())
}

fn spawn_tray_mute<R: Runtime>(app: AppHandle<R>) {
    tauri::async_runtime::spawn(async move {
        if let Err(err) = apply_hotkey(&app, HotkeyAction::MuteToggle).await {
            tracing::warn!(error = %err, "tray mute action failed");
        }
    });
}

fn toggle_window<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window("main") {
        if window.is_visible().unwrap_or(false) {
            let _ = window.hide();
        } else {
            show_window(app);
        }
    }
}

fn show_window<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

fn volume_icon(volume: f32, muted: bool) -> Image<'static> {
    const SIZE: usize = 32;
    let mut rgba = vec![0u8; SIZE * SIZE * 4];
    let level = if muted {
        0
    } else {
        (audio_policy::clamp_scalar(volume) * 22.0).round() as usize
    };
    for y in 0..SIZE {
        for x in 0..SIZE {
            let index = (y * SIZE + x) * 4;
            let speaker = ((5..=11).contains(&x) && (12..=20).contains(&y))
                || ((11..=17).contains(&x)
                    && y >= 9 + x.saturating_sub(11) / 2
                    && y <= 23 - x.saturating_sub(11) / 2);
            let bar = (21..=26).contains(&x) && y >= SIZE - 5 - level;
            if speaker || bar {
                rgba[index] = 242;
                rgba[index + 1] = 246;
                rgba[index + 2] = 255;
                rgba[index + 3] = 255;
            }
            if muted
                && ((x as isize - y as isize).abs() <= 1 || (x + y).abs_diff(31) <= 1)
                && x >= 19
            {
                rgba[index] = 244;
                rgba[index + 1] = 63;
                rgba[index + 2] = 94;
                rgba[index + 3] = 255;
            }
        }
    }
    Image::new_owned(rgba, SIZE as u32, SIZE as u32)
}
