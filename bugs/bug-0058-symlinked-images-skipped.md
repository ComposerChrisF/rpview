# Symlinked images are silently skipped by directory scans, and a dropped/CLI symlink lands on the wrong image

**Severity:** medium-high — silent omission of files the user can see in Finder; wrong image displayed
**Type:** CODE bug, plus a small SPEC gap (no symlink policy documented anywhere)
**Where:** `src/utils/file_scanner.rs:44` (`scan_directory`); consumers: `src/cli.rs:61-86`, `src/utils/file_scanner.rs:100` (`process_dropped_path`), `src/app_handlers.rs:1094-1098` (`import_image_paths`)
**Verified:** by std semantics — `DirEntry::file_type()` explicitly does not traverse symlinks (returns the symlink type itself), so `is_file()` is `false` for every symlink

## Description

`scan_directory` filters entries with:

```rust
if entry.file_type()?.is_file() && is_supported_image(&path) {
```

`DirEntry::file_type()` does **not** follow symlinks, so a symlink to an image is never `is_file()` and is silently dropped from every scan.  This was introduced as a syscall-saving optimization in the v0.9.4 code-review round (“use `entry.file_type()?.is_file()` instead of `path.is_file()` to avoid extra syscall” — see TODO.md Phase 14, round 4); the prior `path.is_file()` followed symlinks, so this is a behavior regression, not just a policy choice.

The inconsistency with the _entry-point_ probes makes it worse — those DO follow symlinks (`Path::is_file()`):

1. **Drop a symlinked image** onto the window: `process_dropped_path` accepts it (`path.is_file()` → true), scans the parent, the symlink is missing from the scan, `position()` fails, and the code falls back to `unwrap_or(0)` (`file_scanner.rs:100`) — the **first image in the directory is displayed instead of the dropped one**, silently.
2. **`rpview symlink.png`** (single-file CLI case, `cli.rs:61`): same sequence — the file the user explicitly named is not in the list; index falls back to 0.  If the directory contains no other (non-symlink) images, `all_images` is empty and rpview reports “No images found” for a viewable image the user explicitly named.
3. A directory of deduplicated images (files replaced by symlinks — a real pattern in this portfolio; see r2-sync bug-0011 precedent) displays as partially or completely empty.

No spec document (CLI.md, DESIGN.md) states a symlink policy, so the omission is also silent at the spec level.  Per the portfolio rule (`positive-evidence-of-absence.md` §Symlink Policy Is Per-Tool): decide per tool from what the tool is for, and write it down.  A _viewer_ should almost certainly display what Finder shows — i.e. follow symlinks.

## Reproduction (Rust test)

```rust
#[test]
fn scan_directory_includes_symlinked_images() {
    let dir = tempfile::TempDir::new().unwrap();
    let target = dir.path().join("real.png");
    image::DynamicImage::new_rgba8(2, 2).save(&target).unwrap();
    let link = dir.path().join("link.png");
    std::os::unix::fs::symlink(&target, &link).unwrap();

    let result = scan_directory(dir.path()).unwrap();
    assert!(result.contains(&link), "symlinked image must be listed");  // FAILS today
}

#[test]
fn process_dropped_symlink_selects_dropped_file() {
    let dir = tempfile::TempDir::new().unwrap();
    let a = dir.path().join("a.png");
    image::DynamicImage::new_rgba8(2, 2).save(&a).unwrap();
    let target = dir.path().join("z_target.png");
    image::DynamicImage::new_rgba8(2, 2).save(&target).unwrap();
    let link = dir.path().join("m_link.png");
    std::os::unix::fs::symlink(&target, &link).unwrap();

    let (images, idx) = process_dropped_path(&link).unwrap();
    assert_eq!(images[idx], link);  // FAILS today: idx falls back to 0 (a.png)
}
```

## Suggested Fix

In `scan_directory`, treat a symlink as its target for the file-ness test, while keeping the cheap non-symlink fast path:

```rust
let ft = entry.file_type()?;
let is_file = if ft.is_symlink() {
    std::fs::metadata(&path).map(|m| m.is_file()).unwrap_or(false)
} else {
    ft.is_file()
};
if is_file && is_supported_image(&path) {
    images.push(path);
}
```

(The `unwrap_or(false)` is acceptable here because the gate protects _inclusion in a view list_, not a destructive action — a broken symlink should be skipped.)  Then document the policy — “symlinks to images are followed and displayed” — in CLI.md §Image File Discovery and in DESIGN.md.

## Why This Fix

It restores the pre-v0.9.4 behavior (symlinks followed) while preserving the optimization for the 99% case (regular files still use the cached `DirEntry` file type, no extra syscall).  The extra `fs::metadata` syscall is paid only for actual symlinks.  Both repro tests pass because the symlink then appears in the scan, which also fixes the index fallback in `process_dropped_path` and the CLI single-file case without touching them.
