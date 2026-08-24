# GIFs with zero-delay frames play at display refresh rate instead of the conventional ~10 fps

**Severity:** medium — common real-world GIFs (delay 0 is widespread) play absurdly fast and burn CPU/GPU
**Type:** CODE bug (convention gap; every browser clamps)
**Where:** `src/app_render.rs:198-231` (frame advance: `elapsed >= frame_duration` — with `frame_duration == 0` this is true every render tick); `src/utils/animation.rs:69-71` (`duration_ms = numer.checked_div(denom).unwrap_or(100)` — only a _missing/zero-denominator_ delay falls back to 100 ms; a present-but-zero delay stays 0)
**Verified:** both sites read; the fallback covers only `denom == 0`, not `numer == 0`

## Description

A large fraction of animated GIFs in the wild declare frame delays of 0 (or 1 centisecond), historically meaning “as fast as possible”.  Every browser and mainstream viewer clamps small delays: ≤ 10 ms is rendered as 100 ms (Chrome, Firefox, Safari convention).  rpview plays such files at the display’s refresh rate (60–120 fps) — and because each frame advance re-runs the GPU pipeline when active (`app_render.rs:224-229`), a zero-delay GIF with any stage enabled also pegs the GPU permanently.

## Reproduction

Build a GIF with explicit 0 delays:

```rust
#[test]
fn zero_delay_frames_are_clamped_to_convention() {
    // Encode a 3-frame GIF with Delay::from_numer_denom_ms(0, 1) per frame,
    // load via load_animation(), and assert every duration_ms >= 100.
    let data = load_animation(&zero_delay_gif_path).unwrap().unwrap();
    assert!(data.frames.iter().all(|f| f.duration_ms >= 100));  // FAILS today: all 0
}
```

Manual: any zero-delay GIF (e.g. produced by `magick -delay 0`) plays as a blur.

## Suggested Fix

Clamp at decode time in `collect_animation_frames` (`src/utils/animation.rs:69-71`): after computing `duration_ms`, apply the browser convention — `if duration_ms < 10 { duration_ms = 100 }` (with a short comment citing the convention).  Decode-time is the right layer: playback, the animation indicator’s timing, and the per-frame GPU reprocessing all inherit the fix.

## Why This Fix

Matches what users see in every other viewer of the same file, removes the render/GPU spin, and the fallback lives beside the existing `unwrap_or(100)` so all “no usable delay” cases resolve in one place.
