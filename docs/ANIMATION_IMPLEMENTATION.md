# Animation Implementation in GPUI

This document describes the lessons learned while implementing automatic animation playback for GIF and WEBP files in rpview-gpui using the GPUI framework.

## Overview

Implementing continuous animation in GPUI requires understanding how GPUI's reactive rendering model works. Unlike traditional game loops that render continuously, GPUI only re-renders when notified of changes. This makes animation implementation non-trivial.

## What DIDN'T Work

### Approach 1: Calling `cx.notify()` in the Render Method
**What we tried:**
```rust
impl Render for App {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Update animation frame
        if let Some(ref mut anim_state) = self.viewer.image_state.animation {
            if anim_state.is_playing {
                // Update frame...
                cx.notify();  // ← Doesn't work!
            }
        }
        // ...
    }
}
```

**Why it failed:**
- `cx.notify()` called during rendering is a no-op
- GPUI doesn't schedule a new render just because you notified during the current render
- The animation would render the first frame and then stall
- Result: Animation stuck at frame 1 or 2

### Approach 2: Background Async Tasks with `cx.spawn()`
**What we tried:**
```rust
fn start_animation_timer(&mut self, cx: &mut Context<Self>) {
    let task = cx.spawn(|this: WeakEntity<App>, cx: &mut AsyncApp| async move {
        loop {
            cx.background_executor().timer(Duration::from_millis(16)).await;
            let _ = this.update(cx, |app, cx| {
                app.update_animation_frame(cx);
                cx.notify();
            });
        }
    });
}
```

**Why it failed:**
- Rust lifetime issues: Can't hold `&mut AsyncApp` across `await` points
- Error: `lifetime may not live long enough`
- The `async move` block captures `cx` but we need to use it after the `.await`
- After the timer awaits, we can't use the captured `cx` reference anymore
- Even with type annotations, the borrow checker rejects this pattern

**Attempted fixes that also failed:**
1. Storing executor separately before async block - still can't use `cx` after await
2. Using `cx.update()` - `cx` isn't accessible in async context
3. Using `cx.dispatch_action()` - same lifetime issues
4. Using background executor spawn directly - no access to GPUI context

### Approach 3: Using `cx.defer()`
**What we tried:**
```rust
if anim_state.is_playing {
    cx.defer(|cx| {
        cx.notify();
    });
}
```

**Why it failed:**
- The deferred closure receives `AppContext`, not `Context<Self>`
- `AppContext` requires an `EntityId` parameter for `notify()`
- We don't have the entity ID in the closure scope
- Type mismatch errors

## What DID Work

### The Solution: `window.request_animation_frame()` from Render Method

**The key insight:** GPUI provides `window.request_animation_frame()` specifically for continuous animation. This API is explicitly documented for "video players and animated GIFs" and can be called directly from the render method.

**Current Implementation (GPUI's Recommended Pattern):**
```rust
impl Render for App {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Update animation frame if playing (GPUI's suggested pattern)
        if let Some(ref mut anim_state) = self.viewer.image_state.animation {
            if anim_state.is_playing && anim_state.frame_count > 0 {
                let now = Instant::now();
                let elapsed = now.duration_since(self.last_frame_update).as_millis() as u32;
                
                // Get current frame duration from metadata
                let frame_duration = anim_state.frame_durations
                    .get(anim_state.current_frame)
                    .copied()
                    .unwrap_or(100);
                
                // Advance frame if enough time has elapsed
                if elapsed >= frame_duration {
                    anim_state.current_frame = (anim_state.current_frame + 1) % anim_state.frame_count;
                    self.last_frame_update = now;
                }
                
                // Request next animation frame (GPUI's pattern for continuous animation)
                window.request_animation_frame();
            }
        }
        
        // ... rest of render method
    }
}
```

**Why this works:**
1. `window.request_animation_frame()` is GPUI's high-level API for continuous animation
2. It internally calls `on_next_frame()` with a notify callback
3. The render method is called every frame while animation is playing
4. When `is_playing` becomes false, the loop stops automatically
5. No manual callback scheduling needed
6. Follows GPUI's documented pattern for GIF/video playback

**Key requirements:**
- Call `window.request_animation_frame()` from render method when animation is playing
- Initialize `last_frame_update` when starting playback
- Track frame timing to respect per-frame durations from metadata

## Critical Implementation Details

### 1. Starting Animation on Load
Initialize the frame update timestamp when loading an animated image:

```rust
fn update_viewer(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    if let Some(path) = self.app_state.current_image().cloned() {
        self.viewer.load_image(path.clone());
        
        // Initialize animation timestamp if this is an animated image set to play
        if let Some(ref anim_state) = self.viewer.image_state.animation {
            if anim_state.is_playing {
                self.last_frame_update = Instant::now();
                cx.notify();  // Trigger first render
            }
        }
    }
}
```

**The render method will then automatically handle the continuous animation loop.**

### 2. Frame Timing
Respect the frame durations from GIF/WEBP metadata:

```rust
let frame_duration = anim_state.frame_durations
    .get(anim_state.current_frame)
    .copied()
    .unwrap_or(100);  // Default 100ms if missing

if elapsed >= frame_duration {
    // Advance frame
}
```

This ensures animations play at their intended speed.

### 3. State Management
Track when the last frame update occurred:

```rust
struct App {
    last_frame_update: Instant,
    // ...
}
```

Initialize it when starting playback:
```rust
self.last_frame_update = Instant::now();
self.start_animation_timer(window, cx);
```

### 4. Stopping Animation
The loop stops automatically when `is_playing` becomes false:

```rust
fn handle_toggle_animation(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
    if let Some(ref mut anim_state) = self.viewer.image_state.animation {
        anim_state.is_playing = !anim_state.is_playing;
        if anim_state.is_playing {
            self.last_frame_update = Instant::now();
        }
        cx.notify();
    }
}
```

When `is_playing` is false, the render method won't call `request_animation_frame()`, so the loop stops naturally.

## Architecture Pattern

The final architecture follows this pattern:

```
User Action (load image / press 'O')
    ↓
Set is_playing = true, cx.notify()
    ↓
render() method called
    ↓
Animation logic in render():
    ├─→ Check if is_playing
    ├─→ Check elapsed time
    ├─→ Advance frame if needed
    └─→ window.request_animation_frame()  ← Schedules next render
    ↓
Next Frame
    ↓
render() called again  ← Loop continues
    ↓
    ... (continues until is_playing = false)
```

## Performance Considerations

### Frame Rate
The `request_animation_frame()` API runs at GPUI's render rate (typically 60 FPS). This is appropriate for animation because:
- Most GIF/WEBP frame rates are ≤ 60 FPS
- We check elapsed time to respect actual frame durations
- Higher frame rates would waste CPU on identical frames
- The render method only updates the displayed frame when enough time has elapsed

### Memory
Each animation frame is cached to a temporary PNG file:
- First 5 frames pre-cached on load (instant playback start)
- Remaining frames cached on-demand during playback
- Temporary files cleaned up when switching images or closing app

## Common Pitfalls

### 1. Forgetting to Initialize last_frame_update
**Symptom:** Animation starts but has incorrect timing.
**Fix:** Set `self.last_frame_update = Instant::now()` when starting playback.

### 2. Calling cx.notify() Instead of window.request_animation_frame()
**Symptom:** Animation doesn't continue automatically.
**Fix:** Use `window.request_animation_frame()` from the render method to schedule continuous updates.

### 3. Placing Animation Logic Outside render()
**Symptom:** More complex code, harder to maintain.
**Fix:** GPUI's pattern is to handle continuous animation directly in the render method.

## Testing

To test animation implementation:
1. Open an animated GIF: `cargo run path/to/animated.gif`
2. Animation should play automatically
3. Press 'O' to pause - animation should stop
4. Press 'O' again to resume - animation should continue
5. Press '[' or ']' to step through frames manually
6. Navigate to another image and back - animation state should persist

## Summary

**The correct pattern for continuous rendering in GPUI:**
- Use `window.request_animation_frame()` from the render method for continuous animation
- Update animation state directly in render when `is_playing` is true
- Don't try to use async/await with GPUI contexts for animation
- Initialize timing state when starting animation, then let render handle the loop
- The animation loop stops naturally when `is_playing` becomes false

This pattern is idiomatic for GPUI, explicitly documented for GIF/video playback, and simpler than manual callback scheduling.


# Recent Additional Research (CLF)
GPUI (the framework powering the Zed editor) does not have a "traditional" timer-based animation system like CSS Transitions or JavaScript's `setTimeout` for UI. Instead, it relies on its **async executor** and a **reactive state pattern** to handle timing and frame updates.

While still evolving (GPUI is pre-1.0), here is how the "timer infrastructure" typically works for animations:

### 1. The Async Timer (Executor)

GPUI provides an integrated async runtime. You can spawn a task that "sleeps" and then updates the state to trigger a re-render.

* **Method:** Use `cx.spawn()` to create an async task.
* **Timer:** Inside that task, you can call `cx.background_executor().timer(duration).await`.
* **Update:** After the timer finishes, you use `this.update()` to modify state and call `cx.notify()` to tell GPUI to repaint the view.

### 2. Built-in Animation Elements

In recent versions (GPUI 2), there is a more structured `Animation` and `AnimationElement` system appearing in the API.

* **`gpui::Animation`**: A struct used to track the progress of a transition.
* **`with_animation`**: Some elements (like SVGs or specific Divs) have helper methods to apply rotations or transitions over a specified `Duration`.

### 3. External Transitions (`gpui_transitions`)

There is a companion crate called `gpui_transitions` often used in the Zed ecosystem. It provides:

* **`use_transition`**: An API similar to React hooks that interpolates between values (like colors or sizes) over time.
* **Evaluation**: When you call `.evaluate(window, cx)` on a transition that isn't finished, it automatically requests a new animation frame from the platform's event loop.

---

### Comparison of Approaches

| Approach | Best For... | Implementation |
| --- | --- | --- |
| **Async Spawn** | Simple delays or one-off state changes. | `cx.spawn( |
| **`gpui_transitions`** | Smoothly interpolating colors, positions, or opacity. | `window.use_transition(...)` |
| **`on_animation_frame`** | Custom, frame-by-frame logic (like a game loop). | Registering a callback for every screen refresh. |

---

## Validation from GPUI Documentation Research

The `on_next_frame()` pattern we discovered aligns with GPUI's intended approach for custom frame-by-frame animation logic. According to GPUI's animation infrastructure, there are three main approaches:

1. **`cx.spawn()` with async timers**: For simple delays or one-off state changes
   - We confirmed this has lifetime issues when trying to use it for continuous animation
   - The async executor approach is mentioned in GPUI documentation but is not suitable for our use case

2. **`gpui_transitions` crate**: For smoothly interpolating values (colors, positions, opacity)
   - Designed for CSS-like transitions between states
   - Not suitable for discrete frame playback where we need exact control over frame timing
   - Would require an additional dependency

3. **`on_next_frame()` / `on_animation_frame`**: For custom frame-by-frame logic - **our use case** ✓
   - This is the appropriate tool for GIF/WEBP animation
   - Provides precise control over discrete frame advancement
   - Allows us to respect per-frame timing metadata from the image format
   - No async complexity or lifetime issues

**Conclusion**: Our implementation correctly uses the frame-by-frame callback pattern, which is the idiomatic GPUI approach for animations that require custom timing logic. The self-scheduling loop pattern we developed is exactly what GPUI intended for use cases like ours, where we need to advance through discrete frames at variable intervals based on embedded metadata.

---

## Investigation of GPUI Animation APIs

After examining the GPUI 0.2.2 source code, here are the key findings:

### APIs Available for Animation

**1. `on_next_frame()`** - The low-level API we're using
```rust
/// Schedule the given closure to be run directly after the current frame is rendered.
pub fn on_next_frame(&self, callback: impl FnOnce(&mut Window, &mut App) + 'static)
```
- Schedules a callback for the next frame
- We use this in our `update_animation_frame()` method to create a self-scheduling loop

**2. `request_animation_frame()`** - Higher-level convenience method
```rust
/// Schedule a frame to be drawn on the next animation frame.
///
/// This is useful for elements that need to animate continuously, 
/// such as a video player or an animated GIF.
pub fn request_animation_frame(&self)
```
- GPUI's documentation explicitly mentions **"such as a video player or an animated GIF"** as the use case
- Internally, it just calls: `self.on_next_frame(move |_, cx| cx.notify(entity))`
- Can be called from within the `render()` method itself

**3. `on_animation_frame`** - Does NOT exist
- The research document mentioned this API, but it does not exist in GPUI 0.2.2
- The actual API is `request_animation_frame()`, not `on_animation_frame()`

### Alternative Pattern from GPUI's Examples

GPUI's official `opacity.rs` example demonstrates a different pattern than we used:

```rust
impl Render for HelloWorld {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.animating {
            self.opacity += 0.005;
            if self.opacity >= 1.0 {
                self.animating = false;
            } else {
                window.request_animation_frame();  // Called from render!
            }
        }
        // ... render UI
    }
}
```

**This contradicts our earlier finding** that calling animation-related methods from `render()` doesn't work. The difference:
- `cx.notify()` from render = doesn't work (we confirmed this)
- `window.request_animation_frame()` from render = works (GPUI's official pattern)

### Our Implementation vs. GPUI's Suggested Pattern

**Our current approach:**
- Separate `update_animation_frame()` method outside of render
- Manually schedule next frame with `cx.on_next_frame()`
- Explicitly call `cx.notify()`
- More code, but very explicit control

**GPUI's example approach:**
- Update animation state directly in `render()`
- Call `window.request_animation_frame()` from render
- Simpler, less code
- Follows GPUI's documented pattern

### Implementation Evolution

**We initially used `on_next_frame()` with a separate method**, but **switched to GPUI's recommended pattern** of calling `request_animation_frame()` from render because:

1. **Simpler code**: Less boilerplate, no manual callback scheduling
2. **Official pattern**: GPUI's docs explicitly show this approach for GIF/video
3. **More maintainable**: Animation logic is in one place (the render method)
4. **Less error-prone**: No forgotten method calls or callback lifetime issues

The render-based approach is the idiomatic GPUI pattern for continuous animation.

### Summary of GPUI Animation Approaches

| API | Use Case | Called From | Our Usage |
|-----|----------|-------------|-----------|
| `on_next_frame()` | Low-level frame scheduling | Outside render | ✓ Currently using |
| `request_animation_frame()` | Continuous animation (GIFs, video) | Inside or outside render | Could use as alternative |
| `cx.notify()` in render | N/A | Don't use | ✗ Confirmed doesn't work |
| `cx.spawn()` async | One-off delays | Outside render | ✗ Has lifetime issues |

Both `on_next_frame()` and `request_animation_frame()` are valid approaches for our GIF/WEBP animation. We chose the more explicit `on_next_frame()` pattern, which gives us finer control over the animation loop.
