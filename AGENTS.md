# Agent instructions

Work within your assigned paths and preserve the owners listed below.

## Contract ownership

- `contracts/ipc.md` owns IPC commands, events, fields, and wire types
- `product.toml` owns product identity and public links
- `src-tauri/src/lib.rs` owns the command registry
- `frontend/src/generated/` contains generated projections only

Do not duplicate an authoritative contract in another handwritten file. Keep one owner per contract.

## Change requirements

- Use Conventional Commits
- Run all six validator categories in `CONTRIBUTING.md`
- Update `docs/index.md` when documentation changes
- Update `tests/index.md` when tests or test commands change
- Do not edit generated files by hand
