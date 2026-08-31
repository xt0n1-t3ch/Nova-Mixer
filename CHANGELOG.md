# Changelog

All notable changes to NovaMixer appear in this file. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and NovaMixer follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

NovaMixer 2.0.0 is a complete rewrite. The C# and WPF implementation is archived under `old/`.

### Fixed

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
- A global shortcut the platform rejects no longer prevents the application from starting. The
  binding is logged and left unbound, and a settings file carrying the old, unparseable
  `MediaVolume*` key names is repaired on load.
- Losing the output device mid-operation no longer marks the affected application permanently
  uncontrollable; it rebuilds the endpoint instead.
- The settings file is now replaced in a single operation, so an interrupted save can no longer
  leave the configuration missing.

### Added

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

[Unreleased]: https://github.com/xt0n1-t3ch/NovaMixer/compare/v2.0.0...HEAD
