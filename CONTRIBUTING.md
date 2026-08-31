# Contribute to NovaMixer

Use this guide to keep changes reviewable and contracts synchronized.

## Submit a change

1. Create a focused branch
2. Use [Conventional Commits](https://www.conventionalcommits.org/)
3. Update `docs/index.md` or `tests/index.md` when you add, rename, or remove documentation or tests
4. Run the validators before opening a pull request

## Run the six validators

Run these validation categories:

1. Frontend typecheck and lint: `pnpm check` and `pnpm --filter novamixer-frontend lint`
2. Rust formatting: `cargo fmt --all -- --check`
3. Rust lint: `cargo clippy --workspace --all-targets -- -D warnings`
4. Tests: `pnpm test`
5. Generated contracts: `cargo xtask check-bindings` and `cargo xtask check-product`
6. Architecture: `cargo xtask check-architecture`

## Own contracts once

Each contract has one authoritative owner. `contracts/ipc.md` owns IPC names and wire shapes. `product.toml` owns product metadata. Generated files project those owners and must not become independent sources.
