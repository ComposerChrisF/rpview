# rpview-gpui

The current state is kept elsewhere; this file only points to it.

- **Version:** `Cargo.toml`, or `rpview --version`.
- **What it does, and its keys and settings:** `README.md`.
- **Where work stands:** the top of `TODO.md`.
- **What is open:** `bugs/` and `plans/`.

## Gotchas

- `ccf-gpui-widgets` comes from GitHub, pinned in `Cargo.lock`.  Take a newer commit with `cargo update -p ccf-gpui-widgets` once it is pushed; a bare `cargo update` moves it too.  To build against a local checkout, use a `[patch."https://github.com/ComposerChrisF/ccf-gpui-widgets"]` path override that is never committed, either in `cargo build --config …` or in a gitignored `.cargo/config.toml`.  While the override is active, Cargo rewrites `Cargo.lock` without the pin.  Before committing, drop the override and run `cargo metadata --locked --format-version 1 >/dev/null`; if it fails, the pin is gone, and `git checkout Cargo.lock` restores it.  That restores the whole committed lock, so redo any other lock change made meanwhile (`cargo update -p ccf-gpui-widgets --precise` cannot restore the pin: while the pin is missing it answers “did not match any packages”).  CI and the release build run with `--locked`, so a lock without the pin fails there.
