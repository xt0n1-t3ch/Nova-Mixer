# NovaMixer test suite

Frontend logic, components, and contracts run on Vitest with happy-dom. The backend runs on
`cargo test`. Run both before shipping.

## Run

| Layer | Command | What it covers |
|:---|:---|:---|
| All (pre-ship) | `task test` | Frontend vitest plus the full Rust workspace |
| Frontend | `pnpm --filter novamixer-frontend test` | Unit, integration, component, and contract suites |
| Frontend (watch) | `pnpm --filter novamixer-frontend test:watch` | Same, watch mode |
| Backend | `cargo test --workspace` | Every Rust crate plus the Tauri command surface |
| Types | `pnpm --filter novamixer-frontend check` | svelte-check, 0 errors / 0 warnings |
| Lint | `pnpm --filter novamixer-frontend lint` | ESLint over TypeScript and Svelte |
| Generated IPC | `cargo xtask check-bindings` | Committed bindings match the command registry |
| Architecture | `cargo xtask check-architecture` | No raw Tauri transport calls outside the generated layer |
| Product identity | `cargo xtask check-product` | One version across all five manifests |
| Live diagnosis | `cargo run -p novamixer-cli -- doctor` | Real audio endpoint, live sessions, and data root |
| Efficiency mode (in-process) | `cargo test -p novamixer-app efficiency` | Windows only: EcoQoS and idle priority applied, queried back, and restored |
| Efficiency mode (live) | `pwsh -NoProfile -File scripts/check-efficiency.ps1` | Read-only: EcoQoS, priority class, and leaf state of each running NovaMixer process and its WebView2 children |
| Meters (live) | `cargo run -p novamixer-cli -- doctor --watch --peaks` | Read-only: prints the `IAudioMeterInformation::GetPeakValue` batch the interface meters draw, once a second, so a level can be checked against real audio |

Vitest config: [frontend/vitest.config.ts](../frontend/vitest.config.ts). Tauri APIs are mocked in
[setup.ts](setup.ts) so store and component modules import cleanly outside a WebView. Shared test
data lives in [helpers/fixtures.ts](helpers/fixtures.ts) and mirrors `AppSettings::default()` in
`novamixer-contracts`.

## Frontend — unit (`tests/unit/`)

| File | Module under test | Coverage |
|:---|:---|:---|
| [volume.test.ts](unit/volume.test.ts) | `lib/volume` | Scalar/percent conversion and round-trip, clamping of negative, over-range, NaN, and Infinity input, mute-aware value text, adaptive hotkey step across all three bands, step accumulation without float drift, meter band thresholds, peak attack/decay half-life, peak-hold window |
| [ux.test.ts](unit/ux.test.ts) | `lib/ux` | Search matching over nullable fields, initial-letter fallback, view-id guard, accelerator formatting, layout-independent hotkey capture from the physical key (a Spanish Shift+7 gives `Digit7`, not `/`), media and function keys allowed alone, bare letters refused, debounce trailing edge plus `flush` and `cancel` |

## Frontend — integration (`tests/integration/`)

| File | Surface | Coverage |
|:---|:---|:---|
| [applicationLifecycle.test.ts](integration/applicationLifecycle.test.ts) | `lib/stores` event transitions | **The regression suite for the v1 bug**: a session created after startup appears and is controllable, survives close-and-relaunch under a new `live_id`, and is never duplicated. Also covers update-replaces-one-row, optimistic-value clearing, idle sessions staying controllable, removal clearing peaks and pending state, endpoint change replacing rather than merging, peak batching, visibility preferences, and sort ordering |
| [liveVolume.test.ts](integration/liveVolume.test.ts) | `lib/stores` volume sending | A level reaches Windows while the fader moves, not only on release; a 50-step drag keeps one IPC call in flight, drops the intermediate values, and always ends on the newest level |

## Frontend — components (`tests/components/`)

| File | Component | Coverage |
|:---|:---|:---|
| [Range.test.ts](components/Range.test.ts) | `components/Range.svelte` | Native slider role, label and `aria-valuetext`, one commit per stepping key, no commit on an ignored key, input-during-drag versus commit-on-release, double-click reset, and total silence while disabled |
| [AppRow.test.ts](components/AppRow.test.ts) | `components/AppRow.svelte` | Name and executable rendering, per-app slider labelling, percentage display, pending value winning during a drag, mute button state and labelling, disabled controls for an uncontrollable session, idle session staying interactive, the honest status labels (`No audio` for an application with no session, `Idle` when every session is silent), system-sounds naming, group attribution, compact density, and Spanish labels |
| [TopBar.test.ts](components/TopBar.test.ts) | `components/TopBar.svelte` | Every view named and the current one marked with `aria-current`, the count limited to applications with an active session (an idle session or a saved application is not counted), switching views, opening the command palette, and the window controls an undecorated window needs |
| [Settings.test.ts](components/Settings.test.ts) | `views/Settings.svelte` | The Hotkeys section opens and shows its three captures instead of crashing on the repeated `g` key; Appearance and Storage open |

## Frontend — contracts (`tests/contracts/`)

| File | Contract | Coverage |
|:---|:---|:---|
| [i18n.test.ts](contracts/i18n.test.ts) | Localization catalogs | Key parity between `en` and `es`, no empty message, identical placeholders per key, paired `_one`/`_other` forms, interpolation and plural selection, missing-key fallback, every literal `$t(...)` key existing, and every key composed from a prefix existing |
| [designSystem.test.ts](contracts/designSystem.test.ts) | `styles/global.css` | Each shared token declared exactly once, the 4px spacing scale, mixer-specific meter and slider tokens, every themed token overridden in light mode, density switching, the centralized control primitives, one global focus ring, reduced-motion and forced-colors handling, and component discipline: no raw hex, no hard-coded durations, no locally redeclared tokens |

## Backend — Rust

| Crate | Coverage |
|:---|:---|
| `novamixer-contracts` | Serde round-trips, literal snake_case wire names, defaults, and `validate()` repair |
| `audio-policy` | App-key resolution, group matching including legacy executable-name bindings, new-session policy precedence, and the adaptive step paired with `frontend/src/lib/volume.ts`. Squirrel.Windows `app-<version>` folders resolve to one `app-*` identity while look-alike folders (`app`, `apps`, `app-data`, `app-1`, `myapp-1.0`) keep theirs, and `migrate_path_identities` collapses per-version entries into one, keeping the existing executable or newest version, the user's pin, name, and level, and rewriting group keys |
| `audio-sessions` | Session registry state machine against a fake source, including control of sessions created after startup, remembered policy for preexisting sessions, filename-to-path identity upgrades, and runtime settings updates. `squirrel_update_attaches_to_application_saved_under_old_version` pins that a session from `app-1.0.9259` joins the Discord saved under `app-1.0.9256`, inherits its remembered level, and takes the live `executable_path`. Also pins the v2 regression: a live application absent from persisted settings survives `update_settings` and stays controllable, so writing settings can never erase a running source from the registry. `meter_level` keeps an in-range reading exact, clips a stream louder than full scale to 1.0, and turns NaN or a negative reading into silence |
| `novamixer-application` | `tests/live_identity.rs` replays the real startup order: settings hold Discord under the merged `app-*` key but the removed `app-1.0.9256` path, and the registry starts with a session already playing from `app-1.0.9258`. It asserts the snapshot and the settings `refresh_live_identities` would persist both carry the 9258 path and its icon, keep the rename and the remembered 0.25 level, and that a second pass changes nothing |
| `app-icons` | Windows only: a real executable's icon decodes to a 96–128 px PNG with visible pixels, both from a fresh thread and from a multithreaded COM apartment like the audio worker. It also pins that a premultiplied icon is downscaled before alpha is made straight, so anti-aliased edges keep their colour instead of darkening (`cargo test -p app-icons`) |
| `settings-store` | Save/load round-trip, backup recovery from a corrupt file, a corrupt current file never overwriting a good backup, legacy v1 config migration against a real captured fixture, and `squirrel_version_entries_load_as_one_application`: a file with two Discord version entries and a group on the old key loads as one application with the group on the merged key, and is rewritten so the next load is a no-op. `failed_replace_removes_its_temp_file_and_returns_the_error` forces the final replace of both `settings.json` and the backup to fail and asserts the error is returned with no `.tmp` file left; `concurrent_saves_leave_only_settings_and_backup` runs eight threads saving in lockstep and asserts every save succeeds and the folder holds exactly `settings.json` and `settings.backup.json`; `load_removes_only_stale_settings_temp_files` asserts startup deletes this store's temp files older than five minutes and leaves fresh or unrelated ones |
| `novamixer-app` | `AppError` wire shape and the classification each `ApplicationError` and `AudioError` maps to, so the interface can distinguish an unknown application from a generic failure. Windows only: `efficiency_mode_sets_and_restores_both_task_manager_properties` applies efficiency mode to the test process and confirms it independently. The EcoQoS state mask is set and the priority class is `IDLE_PRIORITY_CLASS`. It then drops the priority behind the status's back and confirms the status reports off, and confirms that disabling restores `NORMAL_PRIORITY_CLASS` with an explicit high-QoS opt-out. It fails when either Task Manager property is missing. `desktop::tests` pins that an enabled launch-on-startup preference always re-registers the running executable, even when a `Run` value already exists, and that disabling removes only an existing value |
