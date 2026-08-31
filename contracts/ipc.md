# NovaMixer IPC contract

This file is the agreed frontend/backend boundary. The Rust DTOs in `crates/novamixer-contracts`
and the TypeScript types in `frontend/src/lib/api.ts` must both match it exactly. Field names are
`snake_case` on the wire in both directions.

Version: 1

## Types

### `AudioSession`

One live Windows audio session on the tracked render endpoint.

| Field | Type | Notes |
|:---|:---|:---|
| `live_id` | `string` | `"{endpoint_id}::{session_instance_id}"`. Stable while the session lives. Never persisted. |
| `app_key` | `string` | Persistence identity: AUMID, else canonical exe path, else exe file name, lowercased. |
| `display_name` | `string` | Best available human name. Falls back to the exe stem. |
| `executable_name` | `string \| null` | e.g. `"Spotify.exe"`. `null` when metadata access was denied. |
| `executable_path` | `string \| null` | Canonical full path when readable. |
| `process_id` | `number \| null` | `null` for system sounds or a cross-process session. |
| `icon` | `string \| null` | PNG `data:` URL, 32x32. `null` when no icon could be extracted. |
| `volume` | `number` | Linear amplitude scalar, `0.0`–`1.0`. |
| `muted` | `boolean` | |
| `state` | `SessionState` | |
| `is_system_sounds` | `boolean` | |
| `controllable` | `boolean` | `false` when volume calls are rejected (protected process). UI disables the slider. |
| `group_id` | `string \| null` | Group this session's `app_key` belongs to, resolved by the backend. |

### `SessionState`

`"active" | "inactive" | "expired"`

`inactive` sessions stay listed and controllable, matching the Windows mixer. `expired` sessions
are removed by the backend and never reach the frontend as a live row.

### `MasterState`

| Field | Type | Notes |
|:---|:---|:---|
| `endpoint_id` | `string` | Opaque. Compare only, never parse. |
| `endpoint_name` | `string` | Friendly device name. |
| `volume` | `number` | `0.0`–`1.0` scalar on the endpoint taper. |
| `muted` | `boolean` | |

### `MixerSnapshot`

Returned by `list_sessions`. One call gives the whole world.

| Field | Type |
|:---|:---|
| `master` | `MasterState` |
| `sessions` | `AudioSession[]` |

### `PeakBatch`

Payload of the `peaks` event. One event per tick for every session, never one per session.

| Field | Type | Notes |
|:---|:---|:---|
| `timestamp_ms` | `number` | Monotonic milliseconds since app start. |
| `master_peak` | `number` | `0.0`–`1.0`. |
| `sessions` | `SessionPeak[]` | Only sessions with a working meter. |

### `SessionPeak`

| Field | Type |
|:---|:---|
| `live_id` | `string` |
| `peak` | `number` |

### `AppBinding`

An application bound to a group. Identity is the `app_key`, not the live session.

| Field | Type | Notes |
|:---|:---|:---|
| `app_key` | `string` | Matches `AudioSession.app_key`. |
| `display_name` | `string` | User-visible label, editable. |
| `executable_name` | `string \| null` | Shown as the secondary line. |
| `executable_path` | `string \| null` | Used to re-extract the icon when the app is not running. |

### `Group`

| Field | Type | Notes |
|:---|:---|:---|
| `id` | `string` | UUID v4. |
| `name` | `string` | |
| `is_default` | `boolean` | Exactly one group is default. |
| `volume` | `number` | `0.0`–`1.0`. Applied to every bound app. |
| `apps` | `AppBinding[]` | |
| `startup_volume` | `number \| null` | Applied when a bound app's session is created. `null` disables. |
| `auto_mute_on_launch` | `boolean` | Mutes a bound app's session on creation. Wins over `startup_volume`. |
| `hotkeys_enabled` | `boolean` | Whether global hotkeys act on this group when it is active. |

### `HotkeyBinding`

| Field | Type | Notes |
|:---|:---|:---|
| `action` | `HotkeyAction` | |
| `accelerator` | `string \| null` | Tauri accelerator string, e.g. `"CmdOrCtrl+Alt+Up"` or `"MediaVolumeUp"`. `null` unbinds. |

### `HotkeyAction`

`"volume_up" | "volume_down" | "mute_toggle"`

### `UiPrefs`

| Field | Type | Notes |
|:---|:---|:---|
| `theme` | `"dark" \| "light"` | |
| `language` | `string` | `"en"` or `"es"`. |
| `sidebar_collapsed` | `boolean` | |
| `density` | `"compact" \| "comfy"` | |
| `show_inactive` | `boolean` | Show sessions in the `inactive` state. |
| `show_system_sounds` | `boolean` | |

### `AppSettings`

| Field | Type | Notes |
|:---|:---|:---|
| `schema_version` | `number` | Currently `1`. |
| `ui_prefs` | `UiPrefs` | |
| `groups` | `Group[]` | |
| `active_group_id` | `string \| null` | |
| `hotkeys` | `HotkeyBinding[]` | |
| `volume_step` | `number` | Hotkey step as a scalar, e.g. `0.05`. |
| `smart_volume` | `boolean` | Scale the step by the current level. |
| `launch_on_startup` | `boolean` | |
| `start_minimized` | `boolean` | |
| `minimize_to_tray` | `boolean` | |
| `auto_save` | `boolean` | |

### `ApiError`

Every command rejects with this shape.

| Field | Type | Notes |
|:---|:---|:---|
| `kind` | `string` | Machine-readable variant, e.g. `"audio_unavailable"`, `"session_gone"`, `"not_controllable"`, `"validation"`, `"io"`, `"other"`. |
| `message` | `string` | Human-readable, already localized-neutral English. |

## Commands

| Command | Args | Returns |
|:---|:---|:---|
| `list_sessions` | — | `MixerSnapshot` |
| `set_session_volume` | `{ live_id: string, volume: number }` | `void` |
| `set_session_mute` | `{ live_id: string, muted: boolean }` | `void` |
| `set_master_volume` | `{ volume: number }` | `void` |
| `set_master_mute` | `{ muted: boolean }` | `void` |
| `set_group_volume` | `{ group_id: string, volume: number }` | `void` |
| `set_active_group` | `{ group_id: string \| null }` | `void` |
| `upsert_group` | `{ group: Group }` | `Group` |
| `delete_group` | `{ group_id: string }` | `void` |
| `list_running_apps` | — | `AppBinding[]` |
| `get_settings` | — | `AppSettings` |
| `save_settings` | `{ settings: AppSettings }` | `void` |
| `restore_backup` | — | `AppSettings` |
| `set_hotkeys` | `{ hotkeys: HotkeyBinding[] }` | `void` |
| `open_data_folder` | — | `void` |
| `set_metering_active` | `{ active: boolean }` | `void` |
| `open_devtools` | — | `void` |

`set_metering_active` lets the frontend drop peak polling to the idle rate when the window is
hidden or the mixer view is not visible.

## Events

| Event | Payload | When |
|:---|:---|:---|
| `session-added` | `AudioSession` | A session was created and its saved policy was already applied. |
| `session-updated` | `AudioSession` | Volume, mute, state, or metadata changed. |
| `session-removed` | `{ live_id: string }` | Session expired or disconnected. |
| `master-updated` | `MasterState` | Endpoint volume or mute changed. |
| `endpoint-changed` | `MixerSnapshot` | Default render endpoint changed. Replaces the whole list. |
| `peaks` | `PeakBatch` | Metering tick. |
| `settings-updated` | `AppSettings` | Backend mutated settings (hotkey action, migration, tray). |
| `hotkey-fired` | `{ action: HotkeyAction }` | For transient UI feedback only. The backend already applied it. |

## Invariants

1. `session-added` is emitted only after the saved group policy has been applied, so the frontend
   never renders a row at the wrong volume and then corrects it.
2. `live_id` is never persisted. `app_key` is the only durable identity.
3. Volume values are linear amplitude scalars everywhere on the wire. Percentage formatting and
   any perceptual curve belong to the frontend.
4. The backend never emits a session whose `state` is `expired`.
5. `endpoint-changed` fully replaces the session list; the frontend must not merge it.
