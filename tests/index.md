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

Vitest config: [frontend/vitest.config.ts](../frontend/vitest.config.ts). Tauri APIs are mocked in
[setup.ts](setup.ts) so store and component modules import cleanly outside a WebView. Shared test
data lives in [helpers/fixtures.ts](helpers/fixtures.ts) and mirrors `AppSettings::default()` in
`novamixer-contracts`.

## Frontend — unit (`tests/unit/`)

| File | Module under test | Coverage |
|:---|:---|:---|
| [volume.test.ts](unit/volume.test.ts) | `lib/volume` | Scalar/percent conversion and round-trip, clamping of negative, over-range, NaN, and Infinity input, mute-aware value text, adaptive hotkey step across all three bands, step accumulation without float drift, meter band thresholds, peak attack/decay half-life, peak-hold window |
| [ux.test.ts](unit/ux.test.ts) | `lib/ux` | Search matching over nullable fields, deterministic icon tint, initial-letter fallback, view-id guard, accelerator formatting, debounce trailing edge plus `flush` and `cancel` |

## Frontend — integration (`tests/integration/`)

| File | Surface | Coverage |
|:---|:---|:---|
| [sessionLifecycle.test.ts](integration/sessionLifecycle.test.ts) | `lib/stores` event transitions | **The regression suite for the v1 bug**: a session created after startup appears and is controllable, survives close-and-relaunch under a new `live_id`, and is never duplicated. Also covers update-replaces-one-row, optimistic-value clearing, idle sessions staying controllable, removal clearing peaks and pending state, endpoint change replacing rather than merging, peak batching, visibility preferences, search filtering, and sort ordering |

## Frontend — components (`tests/components/`)

| File | Component | Coverage |
|:---|:---|:---|
| [Range.test.ts](components/Range.test.ts) | `components/Range.svelte` | Native slider role, label and `aria-valuetext`, one commit per stepping key, no commit on an ignored key, input-during-drag versus commit-on-release, double-click reset, and total silence while disabled |
| [AppRow.test.ts](components/AppRow.test.ts) | `components/AppRow.svelte` | Name and executable rendering, per-app slider labelling, percentage display, pending value winning during a drag, mute button state and labelling, disabled controls for an uncontrollable session, idle session staying interactive, system-sounds naming, group attribution, compact density, and Spanish labels |

## Frontend — contracts (`tests/contracts/`)

| File | Contract | Coverage |
|:---|:---|:---|
| [i18n.test.ts](contracts/i18n.test.ts) | Localization catalogs | Key parity between `en` and `es`, no empty message, identical placeholders per key, paired `_one`/`_other` forms, interpolation and plural selection, missing-key fallback, every literal `$t(...)` key existing, and every key composed from a prefix existing |
| [designSystem.test.ts](contracts/designSystem.test.ts) | `styles/global.css` | Each shared token declared exactly once, the 4px spacing scale, mixer-specific meter and slider tokens, every themed token overridden in light mode, density switching, the centralized control primitives, one global focus ring, reduced-motion and forced-colors handling, and component discipline: no raw hex, no hard-coded durations, no locally redeclared tokens |

## Backend — Rust

| Crate | Coverage |
|:---|:---|
| `novamixer-contracts` | Serde round-trips, literal snake_case wire names, defaults, and `validate()` repair |
| `audio-policy` | App-key resolution, group matching including legacy executable-name bindings, new-session policy precedence, and the adaptive step paired with `frontend/src/lib/volume.ts` |
| `audio-sessions` | Session registry state machine against a fake source, including `session_created_after_startup_is_controllable` — the backend half of the v1 regression |
| `settings-store` | Save/load round-trip, backup recovery from a corrupt file, and legacy v1 config migration against a real captured fixture |
