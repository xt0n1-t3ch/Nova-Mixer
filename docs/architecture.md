# Understand the NovaMixer architecture

NovaMixer separates persistent application policy, live Windows audio controls, desktop integration, and the Svelte interface. This page identifies each backend owner and the data flow between them.

## The contract defines the boundary

`contracts/ipc.md` owns wire types, commands, events, and invariants. `crates/novamixer-contracts` implements those data transfer objects. `crates/xtask` generates the frontend command registry from `src-tauri/src/lib.rs`.

## The audio worker owns COM interfaces

`crates/audio-sessions` runs one multithreaded apartment worker. The worker owns endpoint and session COM interfaces, notification sinks, application aggregation, metering, and output device operations.

Callers send plain Rust values through a channel. No COM interface crosses the worker boundary. Fake `SessionSource` implementations test aggregation and policy without Windows COM.

## The application service owns durable operations

`crates/novamixer-application` combines the audio registry with `crates/settings-store`. It manages applications, groups, scenes, and settings writes. `crates/audio-policy` resolves identities and computes group startup policy.

`src-tauri/src/commands` exposes the contract to the frontend. `src-tauri/src/desktop.rs` owns the tray, autostart, close-to-tray behavior, and global shortcuts.

## Settings use schema version 2

`crates/settings-store` writes the persistent application registry. It migrates version 1 groups from embedded application objects to `app_keys`, then seeds applications from those objects. The older PascalCase C# format enters the same version 2 model.

## Normal operation stays offline

NovaMixer makes no network requests during normal use. The Content Security Policy permits application assets, Tauri inter-process communication, embedded icon data, and local fonts.
