# An unreadable settings file is treated as absent/corrupt → defaults → the next routine save overwrites the user’s real settings

**Severity:** high (data loss class) — positive-evidence-of-absence violation
**Type:** CODE bug
**Where:** `src/utils/settings_io.rs:152` (`if !path.exists()` — bare bool), `:157-164` (read-error catch-all → defaults, **no backup**), `:173-184` (backup only in the parse-error arm); destructive step downstream: any routine save (`src/app_handlers.rs:99` B key, `:127` close settings, `:256,268,282,295,387,399,411,421` window-bounds observers)
**Verified:** code read in full; the overwrite is the app’s normal save behavior

## Description

Three probes gate the fall-back-to-defaults, and two answer two ways in a three-way world:

1. `path.exists()` returns `false` for EACCES/EIO/ELOOP on any path component — not just NotFound.
2. The `read_to_string` error arm treats **every** read error (permissions, I/O error, transient volume issue) as “use defaults”, and unlike the parse-error arm it makes **no backup**.

The app then saves settings constantly (toggling the background, dragging or closing a floating window, closing the settings window).  After a load that returned defaults for a merely-_unreadable_ file, the first such save atomically replaces the user’s real `settings.json` with defaults.  The data still existed; only the tool’s view of it was empty — the exact safesync/r2-sync fault pattern from `positive-evidence-of-absence.md`.

Related but distinct: the missing-file/create-on-first-run regression and the corrupt-file overwrite are covered in `settings-file-not-created-on-first-run`; this report is specifically about **Unknown being folded into Absent**.

## Reproduction (Rust test, Unix)

```rust
#[test]
fn unreadable_settings_must_not_be_overwritten_by_later_saves() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("settings.json");
    let mut custom = AppSettings::default();
    custom.viewer_behavior.state_cache_size = 42;
    save_settings_to_path(&custom, &path).unwrap();

    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o000)).unwrap();
    let loaded = load_settings_from_path(&path);        // read error → defaults today
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();

    // Simulate the app's next routine save of what it thinks are current settings:
    save_settings_to_path(&loaded, &path).unwrap();

    let reread = load_settings_from_path(&path);
    assert_eq!(reread.viewer_behavior.state_cache_size, 42,
        "user's settings were overwritten with defaults after a transient read failure");  // FAILS today
}
```

Assert on the destructive side effect (file content), not on log output.

## Suggested Fix

Make the load result three-state.  Concretely:

1. Replace `path.exists()` with matching the `read_to_string` error: `ErrorKind::NotFound` → the missing-file path (defaults; plus create-on-first-run per the companion bug).  Any **other** read error → return defaults for this session but record a “settings not loaded — persistence disabled” flag (e.g. a `SETTINGS_WRITABLE: AtomicBool` in `settings_io`, or return a richer type `LoadOutcome { settings, persistable: bool }`).
2. `save_settings` / `save_settings_debounced` early-return (with a loud stderr warning) while the flag says the on-disk file was never successfully read.  A subsequent successful load (or explicit save from the settings window, which is an unambiguous user intent) re-enables persistence — the settings-window save can clear the flag since the user is knowingly writing.
3. In the parse-error arm, only proceed to defaults if the backup copy succeeded (never destroy the sole copy).

## Why This Fix

Destruction (overwriting the user’s file) becomes licensed only by positive evidence: either the file was provably absent (NotFound) or the user explicitly saved from the UI.  A transient permission/I/O failure can no longer cascade into silent replacement, and the failure is loud instead of invisible.
