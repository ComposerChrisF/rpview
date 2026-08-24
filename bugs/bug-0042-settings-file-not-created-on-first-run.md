# Settings file is not auto-created on first run, and a corrupt file is not replaced with defaults — contrary to docs, `--help`, and the original design

**Severity:** medium — every doc (and `--help`) promises behavior the code lost in a refactor
**Type:** CODE bug (regression) — the spec is consistent and predates the regression; restore the code
**Where:** `src/utils/settings_io.rs:150-190` (`load_settings_from_path`); stale-but-correct spec: doc comment `settings_io.rs:140-144`, `src/cli.rs:18` (“auto-created on first run”), `docs/SETTINGS.md:559-563`, TODO.md Phase 16.1/16.5
**Verified:** git archaeology below; current code path read in full

## Description

Two behaviors were lost in commit `8fc4a2f` (“Fix all clippy warnings and improve code quality”), which refactored `load_settings()` into the testable `load_settings_from_path()`:

1. **Missing file → create with defaults.**  Original code (from `dea6923`, Phase 16.1) called `save_settings(&defaults)` when the file did not exist.  Current code just returns `AppSettings::default()` without writing.  Result: on a fresh machine, `rpview` runs and exits without ever creating `settings.json`, until some save-triggering UI action happens.  `--help` (“Settings file (auto-created on first run)”), the function’s own doc comment (“If the settings file doesn’t exist, creates it with default settings”), and TODO.md Phase 16.5 (“Auto-creates on first run — ✅”) all still promise creation.
2. **Corrupt file → back up, then overwrite with defaults.**  Original code backed up and then wrote defaults over the corrupt file.  Current code backs up but leaves the corrupt file in place, so every subsequent launch re-parses it, re-fails, re-copies the same backup, and re-warns.  `docs/SETTINGS.md:559-563` documents a four-step contract whose step 4 is “Overwrite the corrupt file with valid defaults”.

The refactor was almost certainly aiming at purity for tests (a loader that writes as a side effect is awkward to test) and dropped the side effects without updating any of the four spec locations.

## Reproduction (Rust test)

```rust
#[test]
fn load_settings_creates_file_when_missing() {
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("settings.json");
    let _ = load_settings_from_path(&path);
    assert!(path.exists(), "first load must create the settings file");  // FAILS today
}

#[test]
fn load_settings_replaces_corrupt_file_after_backup() {
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("settings.json");
    std::fs::write(&path, "{ not json").unwrap();
    let _ = load_settings_from_path(&path);
    assert!(path.with_extension("json.backup").exists());
    let reparsed: Result<AppSettings, _> =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap());
    assert!(reparsed.is_ok(), "corrupt file must be replaced with valid defaults");  // FAILS today
}
```

## Suggested Fix

Restore both side effects in `load_settings_from_path` (they then hold for `load_settings()` too, and the tests exercise them via the path variant):

- Missing: `if !path.exists() { let d = AppSettings::default(); let _ = save_settings_to_path(&d, path); return d; }` — log the write error, still return defaults (the app must start regardless).
- Corrupt: after the existing backup succeeds, `let _ = save_settings_to_path(&defaults, path);`.  Only overwrite when the backup copy succeeded — never destroy the only copy of the user’s (corrupt but possibly hand-recoverable) file.  If the backup failed, leave the corrupt file in place and just return defaults in memory.

Note the backup-then-overwrite order already protects user data: `settings.json.backup` holds the corrupt original, exactly as `docs/SETTINGS.md` tells users to expect (it even documents the restore command).

## Why This Fix

The spec here is self-consistent across four independent locations and matches the original deliberate implementation; the divergence entered through an unrelated clippy/testability refactor with no CHANGELOG mention — the signature of an accidental regression rather than a decision.  Restoring the writes makes `--help`, the doc comment, docs/SETTINGS.md, and TODO.md all true again, and the backup-first guard keeps the one risky step (overwriting the corrupt file) conditional on the user’s data already being preserved.

## Git Evidence

- `dea6923` (Phase 16.1) — original: create-on-missing and overwrite-corrupt both present.
- `8fc4a2f` — refactor drops both; doc comment above the function left unchanged (still promises creation).
- `0d29131` (v0.20.6) — adds the `--help` text “auto-created on first run” _after_ the regression, copying the stale doc comment rather than the code.
