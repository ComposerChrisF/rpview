# Animation-load I/O errors are silently swallowed — an animated image quietly displays as static

**Severity:** low — silent degradation with no message
**Type:** CODE bug
**Where:** `src/utils/image_loader.rs:131-133` (`load_animation(&path).ok().flatten()` — discards the error `load_animation` deliberately propagates); `src/utils/animation.rs:117-123` (`load_animation` distinguishes `AppError::Io` from decode failures precisely so callers can tell)
**Verified:** both sites read; the distinction constructed in `animation.rs` is destroyed one call up

## Description

`load_animation` carefully propagates I/O errors (file unreadable mid-load) while mapping decode oddities to `Ok(None)` (treat as static).  The async loader then flattens both to `None`, so a transient read failure on an animated GIF silently yields a static first frame — no error, no log, and the animation controls (`O`, `[`, `]`) do nothing for a file the user knows is animated.

## Reproduction

Hard to hit deterministically with real I/O; test via the seam: factor the loader’s animation step into a function taking `Result<Option<AnimationData>, AppError>` and assert an `Err(Io)` produces a user-visible signal (warning line in the `LoadedImageData` or an error message), not a silent `None`.  Manual approximation: `chmod 000` a GIF after `get_image_dimensions` would need a race — hence the seam-based test.

## Suggested Fix

Match instead of `.ok().flatten()`:

```rust
let animation_data = match crate::utils::animation::load_animation(&path) {
    Ok(opt) => opt,
    Err(e) => {
        eprintln!("[LOAD] animation decode failed for {}: {} — showing static frame", path.display(), e);
        None
    }
};
```

Optionally carry a `degraded: Option<String>` note on `LoadedImageData` for a toast.  Minimum bar: the stderr line.

## Why This Fix

The distinction `animation.rs` already computes stops being thrown away; degradation becomes observable instead of indistinguishable from a genuinely static image.
