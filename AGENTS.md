# Agent instructions

Work within your assigned paths and preserve the owners listed below.

## Contract ownership

- `contracts/ipc.md` owns IPC commands, events, fields, and wire types
- `product.toml` owns product identity and public links
- `src-tauri/src/lib.rs` owns the command registry
- `frontend/src/generated/` contains generated projections only
- `frontend/src/styles/global.css` owns every design token; components never redeclare one
- `docs/design-review.md` owns the visual rules the interface is judged against

Do not duplicate an authoritative contract in another handwritten file. Keep one owner per contract.

## Change requirements

- Use Conventional Commits
- Run all six validator categories in `CONTRIBUTING.md`
- Update `docs/index.md` when documentation changes
- Update `tests/index.md` when tests or test commands change
- Update `contracts/ipc.md` when a wire field changes meaning, size, or shape
- Do not edit generated files by hand

## Interface rules

- The chassis is monochrome; colour means signal (meters, mute, locked, destructive actions)
- Every view opens with `PageHeader.svelte`: one row, title plus actions, no subtitle
- One top bar (`TopBar.svelte`) carries navigation, commands, preferences and window controls; no sidebar
- Every application is one channel row on `--grid-channel`; the output row leads and never scrolls
- Meters show only Windows `GetPeakValue` data, clamped to 0..1. Never animate a meter with invented data
- Everything stays visible: nothing clips or scrolls sideways; only the channel list scrolls, natively
- Status labels state what Windows reports: Active, Idle, Locked, or No audio. Never infer "closed"
- Banks are Pinned, Open (has a session) and Saved; a bank name never claims sound that a row does not show
- Application icons are the 128 px shell artwork from `crates/app-icons`, drawn whole

## Inspect the interface

- Browser preview, no Tauri: `corepack pnpm --filter novamixer-frontend dev`, then
  `http://localhost:1420/preview.html?view=applications&theme=dark&lang=en`
- The preview data in `frontend/src/preview/mockBackend.ts` uses real applications and real icons
  only. Do not invent an application or give one another application's icon
- The running app: start it with `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9333`
  and use `scripts/cdp.mjs`. Do not use desktop automation to inspect it
- Efficiency mode on a live process: `pwsh scripts/check-efficiency.ps1`
- Real meter readings in the console: `cargo run -p novamixer-cli -- doctor --watch --peaks`

## Build and install

- `node scripts/kill-novamixer.mjs` closes debug and release builds from this repository
- `corepack pnpm build` writes the NSIS and MSI installers under `target/release/bundle/`
- The NSIS installer is per user and installs to `%LOCALAPPDATA%\NovaMixer`
