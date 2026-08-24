# `delete_preset` gates deletion on a bare `path.exists()` and the caller discards the `Result` — deletion can silently not happen while the UI says it did

**Severity:** low-medium (hard finding under the positive-evidence-of-absence rule; consequence is a lying UI, not data loss)
**Type:** CODE bug
**Where:** `src/utils/gpu_presets.rs:174-180` (`if path.exists() { remove_file()? } Ok(())`); `src/components/gpu_pipeline_controls.rs:449-455` (`let _ = gpu_presets::delete_preset(&name);` — even a genuine `Err` is swallowed and the dropdown rebuilt as if deletion succeeded)
**Verified:** both sites read directly

## Description

`path.exists()` folds “could not stat” (EACCES, EIO, unsearchable directory) into “not there”.  When the probe fails for a file that exists, `delete_preset` returns `Ok(())` having deleted nothing: the UI removes the entry from the dropdown, the file survives, and the preset reappears on next launch.  The caller compounds it by discarding the `Result` entirely, so even a real `remove_file` error is invisible.

This is the exact pattern the portfolio rule names (“a bare `.exists()` gating a delete”), inverted-consequence variant: here the failure mode is claiming success falsely rather than deleting wrongly — still a two-state probe in a three-state world.

## Reproduction (Rust test, Unix)

```rust
#[test]
fn delete_preset_must_not_claim_success_when_it_could_not_look() {
    // Arrange: preset saved to a dir made unsearchable.
    save_preset(name, &preset).unwrap();
    chmod_000(presets_dir());
    let result = delete_preset(name);
    chmod_755(presets_dir());
    // Today: result is Ok(()) and the file still exists.
    assert!(result.is_err() || load_preset(name).is_none());  // FAILS today
}
```

## Suggested Fix

```rust
pub fn delete_preset(name: &str) -> Result<(), String> {
    match std::fs::remove_file(preset_path(name)) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),  // idempotent, keeps the documented contract
        Err(e) => Err(format!("delete: {e}")),
    }
}
```

And in `delete_current_preset`, surface an `Err` (status line or toast) instead of `let _ =`, and only remove the dropdown entry on `Ok`.  The existing `delete_nonexistent_succeeds` test remains valid.

## Why This Fix

`remove_file`’s own error is the three-way probe: success, provably-absent (`NotFound`), or Unknown — only the first two return `Ok`.  The UI then reflects what actually happened on disk.
