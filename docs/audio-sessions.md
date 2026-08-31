# Windows audio sessions

NovaMixer follows the default render endpoint for the `eConsole` role. The worker obtains an `IMMDeviceEnumerator`, then the current `IMMDevice`. It activates `IAudioSessionManager2`, `IAudioEndpointVolume`, and `IAudioMeterInformation` on that device. Each session is read through `IAudioSessionControl2`, controlled through `ISimpleAudioVolume`, and metered through `IAudioMeterInformation` when available.

## Discovery and the live registry

The session enumerator is a snapshot used for startup and repair. It is not NovaMixer's live-session list. The live registry is keyed by `"{endpoint_id}::{session_instance_id}"` and owns the controls used for volume changes.

Startup registers session notifications, obtains the enumerator, and calls `GetCount()` before adopting existing sessions. That call is mandatory. Windows does not deliver new-session notifications until a client first retrieves the existing list. Microsoft documents the requirement in [RegisterSessionNotification](https://learn.microsoft.com/windows/win32/api/audiopolicy/nf-audiopolicy-iaudiosessionmanager2-registersessionnotification). Omitting it reproduces the old failure where applications launched after NovaMixer could not be controlled.

A ten-second reconciliation enumerates and diffs sessions against the registry. It repairs a missed callback and logs only when it finds drift. Callback discovery remains the primary path.

## Threading and COM ownership

One named OS thread, `windows-audio`, initializes COM as a multithreaded apartment. Tauri and CLI callers send plain commands through a crossbeam channel and await Tokio oneshot replies. No COM interface is stored in application state or sent through a Rust channel.

Core Audio can invoke `OnSessionCreated` on another COM thread. The callback registers the incoming interface in the COM Global Interface Table and sends only its integer cookie. The worker resolves and revokes that cookie before adopting the session. If registration fails, it queues immediate reconciliation.

Shutdown stops ticks, unregisters per-session event sinks, unregisters manager and endpoint notification sinks, drops endpoint interfaces, and calls `CoUninitialize` on the worker thread.

## Session state and policy

A new session is read, assigned an identity, matched to a group, and given its saved policy before NovaMixer emits `session-added`. Auto-mute has priority over startup volume. Startup volume has priority over the group's current volume. Without a matching group, NovaMixer leaves the session unchanged.

`AudioSessionStateInactive` remains in the registry and stays controllable, matching the Windows mixer. `Expired` and disconnect notifications remove it. Volume, mute, state, display-name, and icon changes update the existing row. A control call rejected for access reasons marks the row not controllable instead of deleting it. Process metadata failures do not affect volume control.

A default render-device change tears down old registrations, drops endpoint-bound interfaces, activates the new endpoint chain, primes and adopts its sessions, then emits one complete `endpoint-changed` snapshot. Device invalidation follows the same rebuild path.

## Metering

The worker uses one batched timer. It samples the endpoint and every session meter and emits one `peaks` payload per tick. The active interval is 50 ms. The idle interval is 250 ms. `set_metering_active` switches between them. NovaMixer never starts one timer or event stream per session.

## Identity

`live_id` combines the opaque endpoint ID and session-instance ID. It lives only as long as that session.

`app_key` is the persistence key. An Application User Model ID wins when available. Otherwise NovaMixer uses the canonical executable path, then the bare executable name. Every form is lowercase. Group matching also compares bare executable names so version 1 settings still match sessions whose runtime identity has a full path.

COM string getters return allocated `PWSTR` values. One conversion helper copies the UTF-16 text and frees it with `CoTaskMemFree`. `IsSystemSoundsSession` is compared directly with `S_OK` because both `S_OK` and `S_FALSE` are successful HRESULT values with different meanings. Cross-process sessions can return `AUDCLNT_S_NO_SINGLE_PROCESS`; they do not receive a single process ID.

## Known limitations

Elevated and protected applications may expose metadata while rejecting volume changes. NovaMixer keeps these sessions visible and disables their controls after the rejection. Exclusive-mode applications can bypass shared Core Audio sessions and may have no controllable row or meter. NovaMixer follows only the default console render endpoint; it does not merge sessions from several output devices.
