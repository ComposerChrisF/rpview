# Persistent frame cache: non-atomic writes + “exists ⇒ valid” — a crash mid-write poisons a frame for the cache key’s lifetime

**Severity:** low-medium — needs a crash/kill at the wrong moment, but the damage is permanent per key
**Type:** CODE bug
**Where:** `src/utils/image_loader.rs:171-186` (`if dest.exists() { reuse }` else `anim_data.frames[i].image.save(&dest)` — direct write to the final path); same pattern at `src/components/image_viewer.rs:669-695` and `:1704-1712` (`cache_frame`)
**Verified:** all three write sites read; no tmp+rename anywhere in the frame-cache path (contrast: settings and image saves are atomic via `NamedTempFile::persist`)

## Description

Cached animation frames are written with `image.save(&dest)` directly to the final cache path.  If the process is killed mid-write (or the disk fills), a truncated PNG remains at the destination.  Every later run finds `dest.exists()` true and treats existence as validity — the cache key (`{path_fnv}_{mtime}`) doesn’t change, so the poisoned frame is trusted **forever**: the render path displays a broken/failed frame each time playback reaches it (GPUI’s `img()` fails to decode), with no self-healing.

## Reproduction (Rust test)

```rust
#[test]
fn truncated_cached_frame_must_not_be_trusted() {
    // Arrange: valid cache key for a real animated GIF; write garbage at frame 0's path.
    let key = image_key(&gif_path).unwrap();
    let p = raw_frame_path(&key, 0).unwrap();
    std::fs::write(&p, b"\x89PNG\r\n\x1a\n_truncated").unwrap();

    // Act: async load (loader sees dest.exists() and reuses it today).
    let loaded = /* drive load_image_async to completion */;

    // Assert: the frame the viewer will render must be decodable.
    assert!(image::open(&loaded.initial_frame_paths[0]).is_ok());  // FAILS today
}
```

## Suggested Fix

1. **Atomic writes:** write each frame via `tempfile::NamedTempFile::new_in(cache_root)` + `persist(dest)` (the pattern already used for settings and saves).  This alone prevents new poisoning.
2. **Optional self-heal for existing poison:** on reuse, cheap validity check — file nonempty and starts with the PNG magic (8 bytes) — else re-save over it.  Full decode is unnecessary; the magic check catches truncation-at-0 and garbage.

## Why This Fix

Atomic rename guarantees the cache path only ever holds a complete file (crash leaves the temp, not a truncated dest), so “exists” becomes a sound proxy for “valid” again; the magic check retroactively cleans any already-poisoned caches without a version bump or purge.
