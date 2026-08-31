# Understand the NovaMixer architecture

NovaMixer separates Windows audio work, application policy, the Tauri command boundary, and the Svelte interface. This page explains those boundaries and their contract owners.

## Workspace boundaries

The Cargo workspace contains the desktop shell, shared contracts, task automation, and audio crates. The pnpm workspace contains the Svelte frontend.

## Contract ownership

`contracts/ipc.md` owns every wire name and type. `crates/novamixer-contracts` implements those data transfer objects. `src-tauri/src/lib.rs` owns registered commands, and `crates/xtask` projects that registry into TypeScript.

## Offline security model

NovaMixer performs no network requests during normal use. The Content Security Policy (CSP) permits application assets, Tauri IPC, embedded data images, and local fonts. Tauri capabilities grant only the plugins and window actions that the application uses.
