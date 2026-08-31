# Troubleshoot NovaMixer development

Use these checks when the scaffold does not build or the desktop window cannot start.

## Fix a missing Windows build dependency

Install the Microsoft C++ build tools, Windows SDK, WebView2, stable Rust, Node.js 22, and pnpm 9. Restart the shell after installation.

## Fix stale generated bindings

Run `cargo xtask generate-bindings`, inspect the change, then run `cargo xtask check-bindings`.

## Fix a blocked development port

Stop the process using port 1420. Run `node scripts/kill-novamixer.mjs` to stop a NovaMixer binary built from this repository.

## Fix an audio unavailable error

The scaffold returns `audio_unavailable` until the audio service injects its handle. After integration, confirm that Windows Audio runs and that a render endpoint exists.
