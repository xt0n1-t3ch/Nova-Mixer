# NovaMixer IPC contract

This file is the agreed frontend/backend boundary. The Rust DTOs in `crates/novamixer-contracts`
and the TypeScript types in `frontend/src/lib/api.ts` must both match it exactly. Field names are
`snake_case` on the wire in both directions.

Version: 2

## What changed in version 2, and why

Version 1 modelled the product on the Windows audio *session*. That was wrong. Windows gives one
application several sessions whenever it wants, so Discord rendered as two identical rows, and a row
vanished the moment its session expired. There was nothing durable for a user to name, keep, or
configure.

Version 2 makes the **application** the primary object. An `Application` is persistent, carries the
user's remembered volume and preferences, and owns zero or more live `AudioSession` children. The
sessions still exist — they are what the OS actually controls — but they are a detail the user opens
when they want it, not the main list.

Consequences the implementation must honour:

- An application stays listed while it is closed, so its settings remain reachable.
- Setting an application's volume sets every one of its live sessions to that value, and a session
  created later inherits it. See "Aggregate semantics".
- `live_id` is never persisted. `app_key` is the only durable identity.

## Identity

`app_key` resolves in this order, lowercased:

1. AUMID, for a packaged application.
2. Canonical executable path.
3. Executable file name.

`identity_kind` records which one matched, so the interface can warn that a name-matched application
may need relinking after a move or reinstall.

A path identity replaces every directory component that is a Squirrel.Windows version folder with
the fixed token `app-*`. A version folder is `app-` followed by two or more dot-separated all-digit
parts, compared case-insensitively (`app-1.0.9256`). Squirrel installs each update into a new
folder, so without this rule Discord, Slack, and similar applications would change identity on
every update. The file name is never rewritten, and folders such as `app`, `apps`, `app-data`,
`app-1`, or `myapp-1.0` keep their name. For example
`C:\Users\x\AppData\Local\Discord\app-1.0.9258\Discord.exe` resolves to
`c:\users\x\appdata\local\discord\app-*\discord.exe`. `executable_path` stays the concrete path of
the executable last seen running.

Loading settings re-derives stored `path` identities with this rule. Entries that now share an
`app_key` collapse into one, and group `app_keys` and scene entries are rewritten to the new key
without duplicates. The kept entry is the one whose `executable_path` exists, else the one with the
highest version folder, else the first. `pinned` and `hidden` survive when any merged entry had
them. `custom_name`, `group_id`, and `icon` fall back to another merged entry when the kept one has
none. The remembered `volume` and `muted` come from the first ranked entry that is `remembered`.

## Types

### `Application`

The persistent, user-facing object. One row in the Applications view.

| Field | Type | Notes |
|:---|:---|:---|
| `app_key` | `string` | Durable identity, stable across restarts. |
| `identity_kind` | `IdentityKind` | How `app_key` was derived. |
| `display_name` | `string` | Name shown: `custom_name` when set, else the discovered name. |
| `custom_name` | `string \| null` | User's rename. `null` restores the discovered name. |
| `executable_name` | `string \| null` | e.g. `"Discord.exe"`. |
| `executable_path` | `string \| null` | Canonical full path when known. |
| `icon` | `string \| null` | PNG `data:` URL, longest edge at most 128 px, or `null`. Roughly 10–30 KB of base64 per application. |
| `volume` | `number` | The application's level, `0.0`–`1.0`. See "Aggregate semantics". |
| `muted` | `boolean` | |
| `mixed` | `boolean` | `true` when live sessions disagree and no policy has been applied yet. The interface shows an indeterminate control rather than inventing an average. |
| `remembered` | `boolean` | `true` when NovaMixer reapplies `volume` and `muted` to new sessions. |
| `pinned` | `boolean` | Sorts above everything else. |
| `hidden` | `boolean` | Excluded from the list unless the user reveals hidden entries. |
| `sort_order` | `number` | Manual order within a section. |
| `running` | `boolean` | `true` when it owns at least one live session. |
| `controllable` | `boolean` | `false` when every live session refuses control. |
| `is_system_sounds` | `boolean` | |
| `group_id` | `string \| null` | Owning group, if any. |
| `sessions` | `AudioSession[]` | Live children. Empty when not running. |
| `peak` | `number` | Highest child peak, `0.0`–`1.0`. Never summed: summing unrelated peaks would read as clipping. |

### `IdentityKind`

`"aumid" | "path" | "filename"`

### `AudioSession`

One live Windows audio session. A child of an application, never a top-level row.

| Field | Type | Notes |
|:---|:---|:---|
| `live_id` | `string` | `"{endpoint_id}::{session_instance_id}"`. Stable while the session lives. Never persisted. |
| `app_key` | `string` | Owning application. |
| `display_name` | `string` | The session's own name, which can differ from the application's. |
| `process_id` | `number \| null` | `null` for system sounds or a cross-process session. |
| `volume` | `number` | Linear amplitude scalar, `0.0`–`1.0`. |
| `muted` | `boolean` | |
| `state` | `SessionState` | |
| `controllable` | `boolean` | `false` when the OS rejects volume calls for this session. |
| `peak` | `number` | Current level. |

### `SessionState`

`"active" | "inactive" | "expired"`

`inactive` sessions stay listed and controllable, matching the Windows mixer. The backend never
emits an `expired` session.

### `MasterState`

| Field | Type | Notes |
|:---|:---|:---|
| `endpoint_id` | `string` | Opaque. Compare only, never parse. |
| `endpoint_name` | `string` | Friendly device name. |
| `volume` | `number` | `0.0`–`1.0` on the endpoint taper. |
| `muted` | `boolean` | |
| `peak` | `number` | |

### `MixerSnapshot`

Returned by `list_applications`. One call gives the whole world.

| Field | Type |
|:---|:---|
| `master` | `MasterState` |
| `applications` | `Application[]` |

### `AudioDevice`

| Field | Type | Notes |
|:---|:---|:---|
| `id` | `string` | Opaque endpoint id. |
| `name` | `string` | Friendly name. |
| `is_default` | `boolean` | |

### `AppCandidate`

An application the user can add, shown in the picker.

| Field | Type | Notes |
|:---|:---|:---|
| `app_key` | `string` | |
| `display_name` | `string` | |
| `executable_name` | `string \| null` | |
| `executable_path` | `string \| null` | |
| `icon` | `string \| null` | |
| `running` | `boolean` | `true` when it currently owns a session. Running candidates sort first. |
| `already_managed` | `boolean` | Shown as already added, and not selectable. |

### `PeakBatch`

Payload of the `peaks` event. One event per tick for everything, never one per session.

| Field | Type | Notes |
|:---|:---|:---|
| `timestamp_ms` | `number` | Monotonic milliseconds since app start. |
| `master_peak` | `number` | |
| `applications` | `AppPeak[]` | Only applications with a working meter. |

### `AppPeak`

| Field | Type | Notes |
|:---|:---|:---|
| `app_key` | `string` | |
| `peak` | `number` | Highest child peak. |
| `sessions` | `SessionPeak[]` | Per-session detail, for an expanded row. |

### `SessionPeak`

| Field | Type |
|:---|:---|
| `live_id` | `string` |
| `peak` | `number` |

### `Scene`

A named set of levels applied together.

| Field | Type | Notes |
|:---|:---|:---|
| `id` | `string` | UUID v4. |
| `name` | `string` | |
| `icon` | `string \| null` | Lucide icon key. |
| `master_volume` | `number \| null` | `null` leaves the endpoint alone. |
| `entries` | `SceneEntry[]` | |
| `fade_ms` | `number` | Ramp duration when applying. `0` is instant. |

### `SceneEntry`

| Field | Type | Notes |
|:---|:---|:---|
| `app_key` | `string` | |
| `volume` | `number \| null` | `null` leaves the level alone. |
| `muted` | `boolean \| null` | `null` leaves mute alone. |

### `Group`

Unchanged from version 1 except that members are now plain `app_key` strings, because the
application registry already owns every other detail.

| Field | Type | Notes |
|:---|:---|:---|
| `id` | `string` | UUID v4. |
| `name` | `string` | |
| `is_default` | `boolean` | Exactly one group is default. |
| `volume` | `number` | `0.0`–`1.0`, applied to every member. |
| `app_keys` | `string[]` | |
| `startup_volume` | `number \| null` | Applied when a member's session is created. |
| `auto_mute_on_launch` | `boolean` | Wins over `startup_volume`. |
| `hotkeys_enabled` | `boolean` | |

### `HotkeyBinding`

| Field | Type | Notes |
|:---|:---|:---|
| `action` | `HotkeyAction` | |
| `accelerator` | `string \| null` | Tauri accelerator, e.g. `"AudioVolumeUp"`. `null` unbinds. |

### `HotkeyAction`

`"volume_up" | "volume_down" | "mute_toggle"`

### `UiPrefs`

| Field | Type | Notes |
|:---|:---|:---|
| `theme` | `"dark" \| "light"` | |
| `language` | `string` | `"en"` or `"es"`. |
| `sidebar_collapsed` | `boolean` | Unused since the navigation moved to the top bar. Kept so existing settings files load unchanged. |
| `density` | `"compact" \| "comfy"` | |
| `show_offline` | `boolean` | Show applications that are not currently running. |
| `show_hidden` | `boolean` | Reveal entries marked `hidden`. |
| `show_system_sounds` | `boolean` | |

### `AppSettings`

| Field | Type | Notes |
|:---|:---|:---|
| `schema_version` | `number` | Currently `2`. |
| `ui_prefs` | `UiPrefs` | |
| `applications` | `Application[]` | The persistent registry. Live-only fields are ignored on write. |
| `groups` | `Group[]` | |
| `scenes` | `Scene[]` | |
| `active_group_id` | `string \| null` | |
| `hotkeys` | `HotkeyBinding[]` | |
| `volume_step` | `number` | Hotkey step as a scalar, e.g. `0.05`. |
| `smart_volume` | `boolean` | Scale the step by the current level. |
| `launch_on_startup` | `boolean` | |
| `start_minimized` | `boolean` | |
| `minimize_to_tray` | `boolean` | |
| `auto_save` | `boolean` | |
| `efficiency_mode` | `boolean` | See "Efficiency mode". |

### `ApiError`

Every command rejects with this shape.

| Field | Type | Notes |
|:---|:---|:---|
| `kind` | `string` | One of `audio_unavailable`, `session_gone`, `app_unknown`, `not_controllable`, `unsupported`, `validation`, `io`, `other`. |
| `message` | `string` | Human-readable, localization-neutral English. |

## Aggregate semantics

An application row is a **policy**, not a summary statistic.

- Setting an application's volume sets every live, controllable child session to that exact value,
  and a session created afterwards receives the same value. This is what makes "Discord is always
  25%" true, which is the behaviour users report as missing from comparable tools.
- Mute follows the same rule: mute sets all children muted, and later sessions arrive muted.
- When NovaMixer adopts an application whose sessions already disagree, `mixed` is `true` and the
  interface shows an indeterminate control. The first user move resolves it by synchronizing every
  child. The backend never invents an average.
- `peak` is the maximum child peak, never the sum.

## Commands

### Applications

| Command | Args | Returns |
|:---|:---|:---|
| `list_applications` | — | `MixerSnapshot` |
| `set_app_volume` | `{ app_key: string, volume: number }` | `void` |
| `set_app_mute` | `{ app_key: string, muted: boolean }` | `void` |
| `set_session_volume` | `{ live_id: string, volume: number }` | `void` |
| `set_session_mute` | `{ live_id: string, muted: boolean }` | `void` |
| `add_application` | `{ path: string }` | `Application` |
| `remove_application` | `{ app_key: string }` | `void` |
| `update_application` | `{ app_key: string, patch: ApplicationPatch }` | `Application` |
| `reorder_applications` | `{ app_keys: string[] }` | `void` |
| `list_app_candidates` | — | `AppCandidate[]` |

`remove_application` forgets NovaMixer's settings for that application. It never terminates or
uninstalls anything, and the wording in the interface must make that unmistakable.

### `ApplicationPatch`

Every field is optional; an omitted field is unchanged.

| Field | Type |
|:---|:---|
| `custom_name` | `string \| null` |
| `remembered` | `boolean` |
| `pinned` | `boolean` |
| `hidden` | `boolean` |
| `group_id` | `string \| null` |

### Master and devices

| Command | Args | Returns |
|:---|:---|:---|
| `set_master_volume` | `{ volume: number }` | `void` |
| `set_master_mute` | `{ muted: boolean }` | `void` |
| `list_output_devices` | — | `AudioDevice[]` |
| `set_default_output` | `{ device_id: string }` | `void` |

### Groups and scenes

| Command | Args | Returns |
|:---|:---|:---|
| `set_group_volume` | `{ group_id: string, volume: number }` | `void` |
| `set_active_group` | `{ group_id: string \| null }` | `void` |
| `upsert_group` | `{ group: Group }` | `Group` |
| `delete_group` | `{ group_id: string }` | `void` |
| `upsert_scene` | `{ scene: Scene }` | `Scene` |
| `delete_scene` | `{ scene_id: string }` | `void` |
| `apply_scene` | `{ scene_id: string }` | `void` |
| `capture_scene` | `{ name: string }` | `Scene` |

`capture_scene` snapshots the current level of every managed application into a new scene, so a user
can build one by ear rather than by typing numbers.

### Settings and system

| Command | Args | Returns |
|:---|:---|:---|
| `get_settings` | — | `AppSettings` |
| `save_settings` | `{ settings: AppSettings }` | `void` |
| `restore_backup` | — | `AppSettings` |
| `set_hotkeys` | `{ hotkeys: HotkeyBinding[] }` | `void` |
| `set_efficiency_mode` | `{ enabled: boolean }` | `EfficiencyStatus` |
| `get_efficiency_status` | — | `EfficiencyStatus` |
| `open_data_folder` | — | `void` |
| `set_metering_active` | `{ active: boolean }` | `void` |
| `open_devtools` | — | `void` |

### `EfficiencyStatus`

| Field | Type | Notes |
|:---|:---|:---|
| `supported` | `boolean` | `false` on a Windows build without EcoQoS. |
| `enabled` | `boolean` | Queried from the running process: EcoQoS **and** `IDLE_PRIORITY_CLASS`. Never the saved setting. |
| `detail` | `string \| null` | Why it is unsupported, or why applying failed. |

## Events

| Event | Payload | When |
|:---|:---|:---|
| `application-added` | `Application` | An application entered the registry, or a known one started running. Its saved policy is already applied. |
| `application-updated` | `Application` | Volume, mute, sessions, or metadata changed. |
| `application-removed` | `{ app_key: string }` | Forgotten by the user. |
| `master-updated` | `MasterState` | Endpoint volume or mute changed. |
| `endpoint-changed` | `MixerSnapshot` | Default output device changed. Replaces everything. |
| `peaks` | `PeakBatch` | Metering tick. |
| `settings-updated` | `AppSettings` | The backend mutated settings. |
| `hotkey-fired` | `{ action: HotkeyAction }` | Transient interface feedback only; the backend already applied it. |
| `scene-applied` | `{ scene_id: string }` | A scene finished applying. |

## Invariants

1. An application's saved policy is applied **before** it is announced, so the interface never
   renders a row at the wrong level and then corrects itself.
2. `live_id` is never persisted; `app_key` is the only durable identity.
3. Volume values are linear amplitude scalars everywhere on the wire. Percentage formatting belongs
   to the frontend and matches what the Windows mixer shows for the same session.
4. The backend never emits a session whose `state` is `expired`.
5. `endpoint-changed` fully replaces the application list; the frontend must not merge it.
6. An application with no live sessions is still emitted, with `running: false`, so its settings stay
   reachable while it is closed.
7. A failed volume call marks the affected session `controllable: false` and reports it; it never
   removes the application or fails a whole batch.

## Efficiency mode

Task Manager shows the green leaf only when a process has **both** a low base priority and EcoQoS.
Applying EcoQoS alone is not enough. Source: Microsoft, "Reduce process interference with Task
Manager Efficiency Mode".

Two constraints follow:

- The audio worker is timing-sensitive: it applies policy the moment a session appears and drives the
  metering tick. It must be exempted from execution-speed throttling, or efficiency mode makes the
  mixer feel sluggish under load.
- The setting is opt-in and reports failure through `EfficiencyStatus.detail`, rather than silently
  doing nothing on a Windows build that does not support it.
