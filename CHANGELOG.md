# Changelog

All notable changes to NovaMixer appear in this file. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and NovaMixer follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [2.0.0] - 2026-09-25

NovaMixer 2.0.0 is a complete rewrite in Tauri 2, Rust, and Svelte 5. It replaces the earlier C#
and WPF implementation, which is not part of this repository.

### Added

- A public README with a banner, showcase screenshots of the real app, and install instructions,
  plus an `llms.txt` index for search and AI crawlers.
- Release assets: an NSIS installer, an MSI, and a portable ZIP that keeps its settings in a
  `data` folder beside the executable, each with a SHA-256 checksum.
- `scripts/capture-showcase.mjs`, which captures README screenshots from the running app over the
  Chrome DevTools Protocol, so they show real applications and real meter levels.
- Settings uses a section index, working panels, and a live status rail with the current output,
  application/session counts, controllability, scenes/groups, and verified efficiency-mode state.
  About combines product identity, runtime facts, capabilities, and the same live diagnostics.
- **The mixer is now built around applications rather than Windows audio sessions.** Windows gives
  one application several sessions whenever it likes, so the first build of this rewrite showed
  Discord as two identical rows and lost a row the moment its session expired. An application is now
  a persistent object that owns its sessions as children, stays listed while it is closed, and keeps
  its settings reachable. Setting its volume is a policy: every live session takes that value, and a
  session created later inherits it.
- Manage applications directly: add one from what is playing now or by browsing for a closed one,
  rename it, pin it, hide it, assign it to a group, remember its level, and remove it. Removing
  forgets NovaMixer's settings and never closes or uninstalls the application, which the confirmation
  states plainly.
- A per-application disclosure that lists its individual audio streams, for the moment a user needs
  to know which of an application's two streams is the loud one.
- Scenes: save every level exactly as it sounds right now and bring the whole set back with one
  click, with an optional fade.
- A command palette on Ctrl+K over applications, scenes, and actions.
- Windows efficiency mode, opt-in. Task Manager shows the green leaf only when a process has both a
  low base priority and EcoQoS, so NovaMixer applies both — and exempts the audio worker thread, or
  the mixer would feel sluggish under load.
- Per-application volume and mute, with one row per live audio session. The previous version offered
  only a single slider per group.
- A live level meter per application and for the output device, with peak-hold.
- Idle, muted, locked, and system-sounds sessions stay listed and are distinguishable at a glance,
  matching the Windows mixer.
- Light and dark themes across every surface, plus a density switch.
- Global hotkeys captured from a real key press rather than chosen from a list, so a binding is
  always reachable on the user's keyboard.
- Adaptive hotkey stepping: a smaller step at low volume, a larger one at high volume.
- A volume-aware tray icon with show and hide, mute, and quit.
- Start with Windows, start minimized, and close to tray.
- English and Spanish, switchable at runtime.
- A command-line `doctor` for diagnosis without the interface, including a `--watch` mode that
  prints session callbacks as they arrive.

### Changed

- One top bar replaces the navigation sidebar and the title bar. It carries the logo, the four
  views as tabs, the command palette, theme, language, shortcuts, and the window controls.
- Every application is one channel row: identity, the fader with its meter directly beneath it,
  the readout, and mute, on one shared grid. The output row leads the list with 25/50/75/100
  presets. At the default window size every source is visible without scrolling.
- Every view opens with the same one-row header. Groups, Settings, and About share one two-column
  layout. About states the author, version, licence, and project links.
- Scrollbars are the native control, tinted to the theme.
- Status labels state what Windows reports: *Active*, *Idle* (a session with no sound), *Locked*,
  or *No audio* (saved, with no session). The sections read *Pinned*, *Open* (the application has a session), and *Saved*.
  Earlier labels said "Playing now" for silent sessions and "Closed" for applications that were
  open.
- Application icons are extracted at 128 px instead of 32 px. Packaged applications use their app
  ID. The `icon` wire field is now up to 128 px (see `contracts/ipc.md`).
- Rewrote the application on Tauri 2, Rust, Svelte 5, and TypeScript.
- Replaced NAudio with the Windows Core Audio session API through the `windows` crate, on a
  dedicated multithreaded-apartment worker that owns every COM interface.
- Defined a versioned IPC contract in `contracts/ipc.md` covering sessions, groups, settings,
  commands, and events, with generated command bindings and an architecture check that fails the
  build on a raw transport call.
- Settings moved to `%APPDATA%\NovaMixer\v2\settings.json` with atomic writes and a backup copy. A
  one-time migration converts the v1 configuration; the v1 file is left untouched.
- Volume is stored as a linear amplitude scalar everywhere, matching what the Windows mixer reports
  for the same session.


### Removed

- The search field in the title bar and its `/` shortcut. The command palette (`Ctrl K`) finds any
  application.

### Fixed

- Settings › Hotkeys opens. The shortcut list keyed each key by its label, and "g then g" has two
  identical keys, so the section crashed and Startup stayed on screen.
- Recording a hotkey works on every keyboard layout. It read the typed character, so on a Spanish
  layout Shift+7 became `/` and the accelerator was rejected; it now reads the physical key.
- Global hotkeys respond while held down. A group hotkey saved the whole settings file on every
  repeat, so presses queued behind disk writes and failed with "Access is denied". The level now
  lands at once, the group value is saved once after the burst, and the interface updates.
- Volume follows the fader in real time. The level was sent to Windows only when the fader was
  released, and every change then waited for the whole settings file and its backup to be
  written. Levels now reach Windows while the fader moves, with one call in flight per control
  and the newest value always winning, and settings are saved once, 400 ms after the last change,
  off the volume path. NovaMixer also ignores the change notifications its own writes cause, so
  a drag no longer queues a session refresh behind every step.
- An application that updates itself into a new `app-<version>` folder (Discord, Slack, and other
  Squirrel.Windows apps) is one application. It no longer appears twice or loses its level, group,
  and pin after an update, and settings with one entry per version load as a single entry.
- Launch on startup now registers the executable that is running. A `Run` entry left by another
  build (a debug binary or an older install) kept starting that stale executable.
- Saving settings no longer leaves `.settings*.json.*.tmp` files in the data folder. Saves that
  ran in the same instant (a hotkey burst saves once per press) failed with "Access is denied" and
  abandoned their temp files. Saves now run one at a time, a failed save removes its temp file, and
  startup deletes temp files left by earlier runs.
- Meter levels are clamped to 0..1. Windows reports peaks above 1.0 for a floating-point stream
  louder than full scale (a Spotify stream read 2.5), which overflowed the meter.
- The design preview no longer animates its meters with a generated waveform. It has no audio
  device, so its meters stay at zero; only the shipped app draws levels, and those are Windows
  readings.
- Application icons no longer have dark edges. The shell returns premultiplied alpha, and the icon
  was downscaled after alpha was made straight, which blended black into every soft edge.
- An application launched after NovaMixer can now be controlled. The previous version enumerated
  Windows audio sessions once at startup and cached the results by executable name, so a session
  created later was absent from that cache; moving its slider changed the stored value and logged a
  warning while the audio stayed put. The rewrite registers `IAudioSessionNotification`, performs the
  mandatory initial `GetSessionEnumerator` and `GetCount` call, and owns a live session registry, so
  a new session arrives by callback rather than by polling. Verified on a real machine: an
  application started after NovaMixer appeared on its own, and its volume changed and held.
- Closing and relaunching an application no longer breaks its volume control. The saved level is
  reapplied to the new session before it is shown, so a row never appears at the wrong level and
  then corrects itself.
- Changing the default output device now rebuilds every session rather than leaving NovaMixer bound
  to the previous endpoint.
- A session whose process denies metadata access stays listed and controllable instead of
  disappearing.
- An application's row now follows a volume change made anywhere else — the Windows mixer, the
  application's own controls, a script. Live sessions are read as the truth whenever they agree,
  rather than the row continuing to report the last level NovaMixer set.
- An application configured before this release no longer appears twice. A saved entry identified
  only by file name is merged into the canonical executable path the moment that application is
  seen running, keeping its saved level, name, and group.
- Real application icons and product names are resolved for remembered/offline applications too,
  not only for live sessions. Icons are cached in settings so Thorium, Microsoft Edge, and Spotify
  keep their actual logos across restarts instead of reverting to letter badges.
- The 860x600 minimum layout keeps complete names and secondary state such as `2 streams` and
  `In Main`; it sheds pin and meter affordances before it sheds facts.
- A global shortcut the platform rejects no longer prevents the application from starting. The
  binding is logged and left unbound, and a settings file carrying the old, unparseable
  `MediaVolume*` key names is repaired on load.
- Losing the output device mid-operation no longer marks the affected application permanently
  uncontrollable; it rebuilds the endpoint instead.
- The settings file is now replaced in a single operation, so an interrupted save can no longer
  leave the configuration missing.

[Unreleased]: https://github.com/xt0n1-t3ch/Nova-Mixer/compare/v2.0.0...HEAD
[2.0.0]: https://github.com/xt0n1-t3ch/Nova-Mixer/releases/tag/v2.0.0
