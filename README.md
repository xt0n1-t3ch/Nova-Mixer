# NovaMixer

NovaMixer is an offline Windows volume mixer that controls each application without opening the system mixer. The rewrite uses Tauri 2, Rust, Svelte 5, and TypeScript.

## Control per-app volume

NovaMixer tracks Windows audio sessions, keeps inactive sessions visible, and applies saved group policies when applications start. Groups let you adjust related applications together while retaining per-app controls.

## Build and run

Install Node.js 22, pnpm 9, the stable Rust toolchain, and the Windows Tauri prerequisites. Then run:

```bash
pnpm install
pnpm dev
```

Build installers with `pnpm build`. Run repository checks with `task validate`.

## Newly launched application fix

The legacy application could miss sessions created after startup. The rewrite treats Windows session-created notifications as authoritative. It applies saved volume and mute policy before emitting `session-added`, so a new row never appears at the wrong level and then jumps.

## Repository layout

- `crates/novamixer-contracts`: shared Inter-Process Communication (IPC) data types
- `crates/xtask`: generated-file and architecture validators
- `src-tauri`: desktop shell and Rust command boundary
- `frontend`: Svelte application
- `contracts/ipc.md`: authoritative frontend and backend contract
