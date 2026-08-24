# gpu_presets unit tests write into the REAL user settings directory and leave litter on assertion failure

**Severity:** low — test-hygiene defect; user’s config dir polluted by `cargo test`
**Type:** CODE bug (tests)
**Where:** `src/utils/gpu_presets.rs:182-363` (tests call `save_preset`, which resolves through `presets_dir()` → `settings_io::get_settings_path()` → `~/Library/Application Support/rpview/gpu-presets/` on the developer’s machine); cleanup is a trailing `cleanup(&name)` that never runs if an assert fires first
**Verified:** `presets_dir()` path resolution read; tests contain no temp-dir redirection

## Description

Every gpu_presets test creates real files like `__rpview_test_gpu_preset_roundtrip.json` in the user’s actual config directory.  On a failing assertion the file stays behind forever, and `list_preset_names` in the _running app_ will show the test preset in the dropdown.  Tests also assume the config dir is writable (fails in sandboxed CI).

## Reproduction

`cargo test -p rpview gpu_presets` then `ls ~/Library/Application\ Support/rpview/gpu-presets/` during the run — test files appear in the live directory.  Break an assertion → the file persists.

## Suggested Fix

Make the directory injectable: `fn presets_dir() -> PathBuf` gains a test override — simplest is a `#[cfg(test)]` thread-local/`OnceLock` override set by a test fixture that owns a `tempfile::TempDir` (mirroring how `settings_io` tests use `*_to_path` variants).  Alternatively refactor to `save_preset_in(dir, name, preset)` internals with thin public wrappers, and point the tests at a `TempDir`.

## Why This Fix

Tests become hermetic (no shared global state, no litter, CI-safe) without changing production behavior; the `*_to_path`/`*_in` pattern is already the crate’s established solution in `settings_io`.
