# GPUI Async Rendering Patterns

This document covers patterns and pitfalls when implementing asynchronous operations in GPUI's render loop.

## Problem: Async Operations Not Updating UI

### Symptom

When an asynchronous operation completes (e.g., background image processing, network requests), the UI does not update to reflect the completion until an external event occurs (mouse movement, key press, etc.).

### Root Cause

GPUI's render loop is event-driven. The `render()` function is called when:
1. An explicit event occurs (mouse, keyboard, etc.)
2. `cx.notify()` is called
3. `window.request_animation_frame()` is called

**Critical Issue:** If you modify state that triggers async processing AFTER checking whether to request animation frames, the render loop will stop and won't poll for completion.

### Example Scenario

```rust
impl Render for App {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // 1. Check if we need to poll async operations
        if self.is_processing {
            window.request_animation_frame();  // ← Checked BEFORE processing starts
        }
        
        // ... many lines of code ...
        
        // 2. Poll UI controls that might trigger async work
        if self.slider_changed {
            self.start_async_processing();  // ← Sets is_processing = true
        }
        
        // 3. Render function completes
        // Problem: is_processing is now true, but we already checked it at step 1
        // No more animation frames are requested, so step 1 never runs again
        // The async result sits in a channel unread until an external event triggers render()
    }
}
```

### The Solution Pattern

**Immediately request animation frames when starting async operations:**

```rust
impl Render for App {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // 1. Check if async operation completed
        if self.check_async_completion() {
            cx.notify();  // Request re-render to show results
        }
        
        // 2. Keep polling while operation is in progress
        if self.is_processing {
            window.request_animation_frame();
        }
        
        // ... many lines of code ...
        
        // 3. Poll UI controls that might trigger async work
        if self.slider_changed {
            self.start_async_processing();  // Sets is_processing = true
            
            // ⭐ CRITICAL: Request animation frame immediately!
            if self.is_processing {
                window.request_animation_frame();
            }
        }
        
        // ... rest of render ...
    }
}
```

### Real-World Example: Filter Processing

**File:** `src/main.rs` (lines 720-730, 810-816)

**The Fix:**
```rust
// Early in render: Poll for completion
let just_finished_processing = self.viewer.check_filter_processing();
if just_finished_processing {
    cx.notify();
}

// Keep polling while processing
if self.viewer.is_processing_filters || just_finished_processing {
    window.request_animation_frame();
}

// ... later in render ...

// When slider changes, start filter processing
if slider_changed {
    self.viewer.image_state.filters = current_filters;
    self.viewer.update_filtered_cache();  // Sets is_processing_filters = true
    
    // ⭐ Request animation frames immediately to poll for completion
    if self.viewer.is_processing_filters {
        window.request_animation_frame();
    }
}
```

## Key Principles

### 1. Request Animation Frames at Point of Async Initiation

When you start an async operation that sets a "processing" flag, immediately check that flag and request animation frames:

```rust
self.start_async_work();
if self.is_async_working {
    window.request_animation_frame();
}
```

### 2. Poll Completion at Top of Render

Check for async completion early in the render function:

```rust
fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    // Check completion FIRST
    let just_finished = self.check_async_completion();
    if just_finished {
        cx.notify();  // Trigger one more render to show results
    }
    
    // Continue polling
    if self.is_processing || just_finished {
        window.request_animation_frame();
    }
    
    // ... rest of render ...
}
```

### 3. Use Non-Blocking Channel Reads

Use `try_recv()` instead of blocking `recv()` to avoid freezing the UI:

```rust
pub fn check_async_completion(&mut self) -> bool {
    if let Some(receiver) = &self.async_handle {
        if let Ok(result) = receiver.try_recv() {  // ← Non-blocking
            // Process result
            self.is_processing = false;
            return true;
        }
    }
    false
}
```

### 4. Request One More Frame After Completion

Even after an async operation completes, request one more animation frame to ensure the UI updates:

```rust
if self.is_processing || just_finished {
    window.request_animation_frame();
}
```

## Common Mistakes

### ❌ Mistake 1: Only Checking State at Top of Render

```rust
fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    if self.is_processing {
        window.request_animation_frame();
    }
    
    // ... later ...
    
    self.start_async_work();  // Sets is_processing = true
    // ❌ Forgot to request_animation_frame() here!
}
```

### ❌ Mistake 2: Assuming cx.notify() Is Enough

```rust
fn check_async_completion(&mut self) -> bool {
    if let Ok(result) = receiver.try_recv() {
        self.is_processing = false;
        cx.notify();  // ❌ This schedules a render, but...
        return true;
    }
    false
}

// In render():
if self.is_processing {  // ❌ False after completion
    window.request_animation_frame();  // ❌ Not called anymore!
}
```

**Fix:** Request one more frame when operation completes:

```rust
if self.is_processing || just_finished {
    window.request_animation_frame();
}
```

### ❌ Mistake 3: Blocking Channel Reads

```rust
// ❌ DON'T DO THIS - Freezes UI!
let result = receiver.recv().unwrap();

// ✅ DO THIS - Non-blocking
if let Ok(result) = receiver.try_recv() {
    // ...
}
```

## Testing Async UI Updates

To test if your async operations update the UI properly:

1. **Start the async operation** (e.g., move a slider, click a button)
2. **Immediately stop all input** (don't move mouse, don't press keys)
3. **Wait for operation to complete**
4. **Verify UI updates without any further input**

If the UI doesn't update until you move the mouse or press a key, you have this bug.

## Related Files

- `src/main.rs` - Main render loop with filter processing example
- `src/components/image_viewer.rs` - Async image loading and filter processing
- `docs/FILTER_ARCHITECTURE.md` - Filter system architecture

## Summary

When implementing async operations in GPUI:
1. ✅ Request animation frames **immediately** when starting async work
2. ✅ Poll for completion at the **top** of render
3. ✅ Use **non-blocking** channel reads (`try_recv()`)
4. ✅ Request **one more frame** after completion
5. ✅ Test without any mouse/keyboard input during async operation
