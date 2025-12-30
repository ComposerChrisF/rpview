# GPU Texture Preloading for Smooth Navigation

## Overview

This document describes the GPU texture preloading system that eliminates black flashing during image navigation. The implementation ensures that when users navigate between images, the GPU has already loaded the texture into memory, providing instant, seamless transitions.

## Problem Statement

### The Black Flash Issue

When navigating between images (pressing Next/Previous), users would experience a brief black flash before the new image appeared. This occurred because:

1. User presses Next/Previous
2. App immediately switches to new image path
3. GPUI's `img()` element receives new path
4. GPU needs to load texture from disk into GPU memory (100-200ms)
5. During texture loading, screen shows black
6. Once texture is loaded, image appears

This black flash was particularly noticeable with:
- Large images (slower to load into GPU memory)
- Sequential navigation (rapid next/next/next)
- High-resolution displays (more texture data to transfer)

## Solution: Continuous GPU Preloading

### Core Concept

Instead of loading textures on-demand during navigation, we **continuously preload** the next and previous images in the background during every render frame. When the user navigates, the texture is already in GPU memory.

### Implementation Strategy

The solution uses the same technique that successfully eliminated flashing in animation frame playback:

```rust
// In App::render() - runs every frame
let mut preload_paths = Vec::new();
if let Some(next_path) = self.app_state.next_image_path() {
    preload_paths.push(next_path.clone());
}
if let Some(prev_path) = self.app_state.previous_image_path() {
    preload_paths.push(prev_path.clone());
}
self.viewer.set_preload_paths(preload_paths);
```

```rust
// In ImageViewer::render() - render preload images invisibly
for preload_path in &self.preload_paths {
    if preload_path.exists() {
        container = container.child(
            img(preload_path.clone())
                .w(px(zoomed_width))      // Full size forces texture load
                .h(px(zoomed_height))
                .absolute()
                .left(px(-10000.0))       // Position way off-screen
                .top(px(0.0))
                .opacity(0.0)             // Make completely invisible
        );
    }
}
```

### Key Technical Details

1. **Rendering Location**: Preload images are positioned at `left: -10000px`, placing them far off-screen and outside the viewport clipping region.

2. **Visibility**: Using `opacity(0.0)` makes the images completely transparent, even if they were somehow visible.

3. **Size**: Images are rendered at full zoomed dimensions (`zoomed_width`, `zoomed_height`) to ensure GPUI loads the complete texture, not a scaled-down version.

4. **Timing**: Preloading happens in `App::render()`, which is called every frame. This ensures:
   - Images are preloaded BEFORE navigation occurs
   - Preload state updates if the image list changes
   - No special triggers needed - it's always running

5. **Element IDs**: Each preload image gets a unique ID based on its path:
   ```rust
   let preload_id = ElementId::Name(format!("preload-{}", preload_path.display()).into());
   ```
   This ensures GPUI can track and cache the textures properly.

## Performance Characteristics

### Memory Usage

- **Minimal overhead**: Only 2 additional textures in GPU memory at any time (next and previous)
- **No CPU overhead**: Images aren't decoded multiple times - GPUI handles texture caching
- **Automatic cleanup**: When preload paths change, old textures are naturally evicted by GPUI's cache

### Render Performance

- **Negligible impact**: Off-screen rendering with opacity 0 is extremely fast
- **GPU optimization**: Modern GPUs efficiently handle off-screen rendering
- **No frame drops**: The preload rendering happens during normal render passes

### User Experience

- **Instant navigation**: Textures are ready immediately when user navigates
- **Smooth scrolling**: No visual disruption during rapid navigation
- **Consistent behavior**: Works the same for all image formats and sizes

## Comparison to Animation Frame Preloading

This implementation mirrors the animation frame preloading that was already successfully working in the codebase:

| Aspect | Animation Frames | Navigation Images |
|--------|------------------|-------------------|
| **What's preloaded** | Next frame in animation | Next/previous images in list |
| **When preloading happens** | During animation playback | Every render frame |
| **Rendering technique** | Off-screen, opacity 0, full size | Off-screen, opacity 0, full size |
| **Number preloaded** | 1 frame ahead | 2 images (next + previous) |
| **Cache invalidation** | On frame advance | On navigation or list change |

The key insight was recognizing that the same technique that worked for animation frames would work for navigation - both scenarios involve loading textures from disk into GPU memory before they're needed for display.

## Code Locations

### Core Implementation Files

1. **src/state/app_state.rs** (lines 67-87)
   - `next_image_path()` - Returns path of next image
   - `previous_image_path()` - Returns path of previous image

2. **src/components/image_viewer.rs**
   - Line 85: `preload_paths` field added to `ImageViewer`
   - Lines 101-104: `set_preload_paths()` method
   - Lines 673-696: Preload rendering logic

3. **src/main.rs** (lines 752-762)
   - Preload path setup in `App::render()`

## Alternative Approaches Considered

### 1. Keep Old Image Until New Is Ready

**Concept**: Don't replace `current_image` until the new texture is loaded.

**Why not chosen**: 
- No way to detect when GPUI has loaded a texture
- Would require complex state management (old + new image simultaneously)
- Continuous preloading is simpler and more reliable

### 2. Async Image Loading

**Concept**: Load images on background threads, show loading spinner.

**Why not chosen**:
- Doesn't solve GPU texture loading (GPU work must happen on main thread)
- Adds complexity without solving the root problem
- Preloading is more elegant and seamless

### 3. Larger Preload Window

**Concept**: Preload 5-10 images ahead instead of just 2.

**Why not chosen**:
- Diminishing returns (users rarely jump 5+ images)
- Increased GPU memory usage
- Current 2-image preload is sufficient

## Testing and Verification

### Test Scenario

```bash
cargo run --release -- test_images/rust-logo.png test_images/rust-logo.tiff
```

1. Open the application with multiple images
2. Navigate forward/backward repeatedly with arrow keys
3. Observe that transitions are instant with no black flash

### Expected Behavior

- ✅ No black flash between images
- ✅ Instant image transitions
- ✅ Smooth experience even with large images
- ✅ No performance degradation

## Future Enhancements

### Potential Improvements

1. **Adaptive preload count**: Preload more images based on available GPU memory
2. **Directional prediction**: If user is going forward consistently, prioritize next over previous
3. **Smart cache size**: Adjust based on image sizes and available memory
4. **Preload on idle**: Use idle render frames to preload even further ahead

### Related Features

- Could extend to preload filtered versions of images
- Could integrate with thumbnail generation
- Could cache zoom levels for frequently viewed images

## Lessons Learned

1. **Pattern reuse**: The animation frame preloading pattern was directly applicable to navigation
2. **Render-time preloading**: Doing preload work during render() is more reliable than triggering on navigation events
3. **Full-size rendering matters**: Even off-screen, GPUI needs the full dimensions to load complete textures
4. **Simple solutions work**: The solution is remarkably simple (~50 lines of code total) yet completely eliminates the issue

## Conclusion

GPU texture preloading provides a seamless navigation experience with minimal code and zero user-visible overhead. By continuously rendering adjacent images off-screen, we ensure GPU textures are always ready when needed, eliminating the black flash that was previously visible during navigation.

The implementation is robust, efficient, and follows established patterns already present in the codebase for animation frame handling.
