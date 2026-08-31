# Run NovaMixer tests

This registry lists the maintained test suites and commands.

| Suite | Command | Coverage |
|---|---|---|
| Rust workspace | `cargo test --workspace` | Contracts, backend helpers, and task automation |
| Frontend unit and component | `pnpm --filter novamixer-frontend test` | Unit, integration, component, and contract tests |
| Frontend typecheck | `pnpm --filter novamixer-frontend check` | Svelte and TypeScript contracts |
| End to end | `pnpm test:e2e` | Packaged desktop workflows |
| Full validation | `task validate` | Formatting, linting, tests, generated files, and architecture |
