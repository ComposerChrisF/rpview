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

### The Solution: `cx.on_next_frame()` with Self-Scheduling

**The key insight:** GPUI provides `cx.on_next_frame()` specifically for scheduling work on future frames. This is the correct API for continuous rendering.

**Implementation:**
```rust
fn update_animation_frame(&mut self, window: &mut Window, cx: &mut Context<Self>) {
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
            
            // Schedule next frame update - creates continuous loop
            cx.on_next_frame(window, |this: &mut App, window, cx| {
                this.update_animation_frame(window, cx);
            });
            
            cx.notify();
        }
    }
}

fn start_animation_timer(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    // Kick off the self-scheduling loop
    self.update_animation_frame(window, cx);
}
```

**Why this works:**
1. `cx.on_next_frame()` schedules a callback to run on the next render frame
2. The callback receives proper `&mut App` and `Context` references
3. The callback calls itself again, creating a self-sustaining loop
4. The loop continues as long as `is_playing` is true
5. When `is_playing` becomes false, the method stops scheduling itself
6. No async/await, no lifetime issues, no background threads needed

**Key requirements:**
- Must call `start_animation_timer()` when loading an animated image
- Must pass `Window` reference (required by `on_next_frame` API)
- Must call `cx.notify()` to trigger the actual render
- Must track `last_frame_update` time to respect frame durations

## Critical Implementation Details

### 1. Starting Animation on Load
The animation won't start automatically just because `is_playing` is true. You must explicitly kick off the loop:

```rust
fn update_viewer(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    if let Some(path) = self.app_state.current_image().cloned() {
        self.viewer.load_image(path.clone());
        
        // Start animation if this is an animated image and it's set to play
        if let Some(ref anim_state) = self.viewer.image_state.animation {
            if anim_state.is_playing {
                self.last_frame_update = Instant::now();
                self.start_animation_timer(window, cx);  // ← Essential!
            }
        }
    }
}
```

**Without this:** Animation will appear frozen until user manually toggles it.

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
fn handle_toggle_animation(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    if let Some(ref mut anim_state) = self.viewer.image_state.animation {
        anim_state.is_playing = !anim_state.is_playing;
        if anim_state.is_playing {
            self.last_frame_update = Instant::now();
            self.start_animation_timer(window, cx);
        }
        // When pausing, the loop stops scheduling itself automatically
        cx.notify();
    }
}
```

No need to explicitly cancel tasks or stop timers.

## Architecture Pattern

The final architecture follows this pattern:

```
User Action (load image / press 'O')
    ↓
start_animation_timer()
    ↓
update_animation_frame()
    ↓
    ├─→ Check elapsed time
    ├─→ Advance frame if needed
    ├─→ cx.on_next_frame(self.update_animation_frame)  ← Schedules next iteration
    └─→ cx.notify()  ← Triggers render
    ↓
Next Frame
    ↓
update_animation_frame()  ← Loop continues
    ↓
    ... (continues until is_playing = false)
```

## Performance Considerations

### Frame Rate
The `on_next_frame()` callback runs at GPUI's render rate (typically 60 FPS). This is appropriate for animation because:
- Most GIF/WEBP frame rates are ≤ 60 FPS
- We check elapsed time to respect actual frame durations
- Higher frame rates would waste CPU on identical frames

### Memory
Each animation frame is cached to a temporary PNG file:
- First 5 frames pre-cached on load (instant playback start)
- Remaining frames cached on-demand during playback
- Temporary files cleaned up when switching images or closing app

## Common Pitfalls

### 1. Forgetting to Call start_animation_timer()
**Symptom:** Animation appears frozen even though `is_playing` is true.
**Fix:** Call `start_animation_timer()` in `update_viewer()` when loading animated images.

### 2. Not Passing Window to on_next_frame()
**Error:** `this method takes 2 arguments but 1 argument was supplied`
**Fix:** `cx.on_next_frame(window, |this, window, cx| { ... })`

### 3. Type Annotation Issues
**Error:** `type annotations needed`
**Fix:** Add explicit type to closure: `|this: &mut App, window, cx|`

### 4. Calling notify() Inside Render
**Symptom:** Animation doesn't advance.
**Fix:** Use `on_next_frame()` pattern instead.

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
- Use `cx.on_next_frame()` to schedule callbacks
- Create a self-scheduling loop that continues while needed
- Don't try to use async/await with GPUI contexts
- Don't call `cx.notify()` during render expecting continuous updates
- Always kick off the loop explicitly when starting animation

This pattern is idiomatic for GPUI and avoids all the lifetime and async complexity that other approaches encounter.
