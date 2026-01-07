# GPUI Scrollbars - Implementation Guide

This document describes how scrollbars work in GPUI and gpui-component, based on extensive testing with gpui-component 0.5.0.

## Critical Concepts

### The `min_h_0()` / `min_w_0()` Rule

**This is the most important thing to understand about GPUI scrolling.**

In flexbox, the default `min-height: auto` and `min-width: auto` prevent flex items from shrinking below their content size. This causes scroll containers to **grow to fit their content** instead of constraining content and enabling scrolling.

**Solution**: Add `min_h_0()` (and/or `min_w_0()`) to ALL flex containers in the parent chain of a scroll area.

```rust
// BAD - scroll container grows to fit content, no scrolling
div()
    .flex()
    .flex_col()
    .flex_1()        // Tries to fill space
    // Missing min_h_0() - container expands instead of scrolling
    .child(scroll_area)

// GOOD - scroll container is constrained, scrolling works
div()
    .flex()
    .flex_col()
    .flex_1()
    .min_h_0()       // Allows shrinking - CRITICAL for scrolling
    .child(scroll_area)
```

### Layout Hierarchy Example

For a typical app with header + main content:

```rust
div()
    .flex()
    .flex_col()
    .size_full()
    .child(header)  // Fixed height
    .child(
        // Main content - MUST have min_h_0 to allow scrolling in children
        div()
            .flex()
            .flex_row()
            .flex_1()
            .min_h_0()  // Critical!
            .w_full()
            .child(
                // Left panel with scroll
                div()
                    .w(px(300.0))
                    .flex_1()
                    .min_h_0()  // Critical!
                    .overflow_y_scroll()
                    .child(content)
            )
            .child(
                // Right panel
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_w_0()  // For horizontal content
                    .min_h_0()  // For vertical scrolling
                    .child(scroll_area)
            )
    )
```

## GPUI Native Scrolling

GPUI's native scrolling is enabled via methods on `Div`:

```rust
div()
    .overflow_scroll()      // Both axes
    .overflow_x_scroll()    // Horizontal only
    .overflow_y_scroll()    // Vertical only
```

**Important**: These methods only enable scroll *behavior* (via mouse wheel, trackpad). They do NOT render visible scrollbars.

### Scroll Handle

For programmatic scroll control and to connect visible scrollbars, use a `ScrollHandle`:

```rust
let scroll_handle = ScrollHandle::default();

div()
    .id("my_scroll_area")
    .overflow_scroll()
    .track_scroll(&scroll_handle)  // Connect handle to scroll area
    .child(content)
```

## gpui-component Visible Scrollbars

The `gpui-component` crate provides visible, interactive scrollbars.

### Required Initialization

**Critical**: You MUST call `gpui_component::init(cx)` at application startup:

```rust
Application::new().run(|cx: &mut App| {
    gpui_component::init(cx);  // Required for scrollbars!

    // Force scrollbars always visible (optional, overrides macOS auto-hide)
    use gpui_component::theme::Theme;
    use gpui_component::scroll::ScrollbarShow;
    Theme::global_mut(cx).scrollbar_show = ScrollbarShow::Always;

    // ... rest of initialization
});
```

### ScrollbarShow Modes

| Mode | Behavior |
|------|----------|
| `Scrolling` | Shows during scroll, fades out after 2-3 seconds (macOS default) |
| `Hover` | Shows only when hovering over scroll area |
| `Always` | Always visible |

**macOS Note**: By default, `gpui_component::init()` syncs with system preferences.

### Manual Scrollbar Setup (Recommended)

The `overflow_scrollbar()` convenience method has quirks. For reliable results, set up scrollbars manually:

```rust
use gpui_component::scroll::Scrollbar;

// In your struct
struct MyView {
    scroll_handle: ScrollHandle,
}

// In render()
fn render(&mut self, ...) -> impl IntoElement {
    // Calculate content dimensions
    let content_width = /* calculate based on content */;
    let content_height = /* calculate based on content */;

    let scroll_handle = &self.scroll_handle;

    div()
        .id("container")
        .size_full()
        .relative()  // Required for absolute overlay positioning
        .child(
            // Scroll area
            div()
                .id("scroll_area")
                .size_full()
                .overflow_scroll()
                .track_scroll(scroll_handle)
                .child(content)
        )
        .child(
            // Scrollbar overlay
            div()
                .absolute()
                .top_0()
                .left_0()
                .right_0()
                .bottom_0()
                .child(
                    Scrollbar::new(scroll_handle)
                        .id("my_scrollbar")
                        .scroll_size(size(px(content_width), px(content_height)))
                )
        )
}
```

### Why Manual Setup?

The `overflow_scrollbar()` wrapper:
1. Applies `.flex_1()` to your content, which can interfere with sizing
2. Extracts styles from your element and applies them to a wrapper
3. May not properly detect content size for both axes

Manual setup gives you full control over the scroll area and scrollbar behavior.

## Content Sizing

For scrolling to work, content must exceed container size. Key techniques:

### Calculate Content Dimensions

```rust
// For text content (~8px per char for monospace, ~20px per line)
let max_chars = lines.iter().map(|l| l.len()).max().unwrap_or(0);
let content_width = (max_chars as f32 * 8.0) + 64.0;  // + padding
let content_height = (lines.len() as f32 * 20.0) + 64.0;
```

### Apply to Content

```rust
div()
    .flex()
    .flex_col()
    .flex_shrink_0()           // Don't shrink below natural size
    .items_start()             // Don't stretch children horizontally
    .min_w(px(content_width))  // Minimum width for horizontal scroll
    .min_h(px(content_height)) // Minimum height for vertical scroll
    .children(lines.iter().map(|line| {
        div()
            .whitespace_nowrap()   // Prevent text wrapping
            .flex_shrink_0()       // Don't shrink
            .child(line)
    }))
```

## Scrollbar Component Details

The `Scrollbar` component checks if scrollbars should display:

```rust
// From gpui-component source:
// Hide scrollbar if scroll area is smaller than container
if scroll_area_size <= container_size {
    continue;  // Skip this axis
}
```

This means:
- **scroll_size** (content dimensions) must exceed **container bounds**
- Use `.scroll_size()` to explicitly set content dimensions
- The Scrollbar uses its own layout bounds as container size

### Scrollbar Dimensions

- Total scrollbar width: 16px
- Thumb width (inactive): 6px
- Thumb width (active/hovered): 8px
- Minimum thumb length: 48px

## Theme Colors

Scrollbar colors come from the gpui-component theme:

| Property | Description |
|----------|-------------|
| `scrollbar` | Background color (usually transparent) |
| `scrollbar_thumb` | Thumb color (inactive state) |
| `scrollbar_thumb_hover` | Thumb color (hovered/active state) |

### Debug Theme Colors

```rust
use gpui_component::theme::ActiveTheme;

let theme = cx.theme();
eprintln!("scrollbar_show: {:?}", theme.scrollbar_show);
eprintln!("scrollbar_thumb: {:?}", theme.scrollbar_thumb);
eprintln!("scrollbar_thumb_hover: {:?}", theme.scrollbar_thumb_hover);
```

If colors are `Hsla { h: 0.0, s: 0.0, l: 0.0, a: 0.0 }` (transparent), scrollbars won't be visible.

## Troubleshooting Checklist

### Scrolling doesn't work at all

1. **Check `min_h_0()` / `min_w_0()`** - Add to ALL flex parents in the chain
2. **Check overflow setting** - Ensure `.overflow_scroll()` is set
3. **Check container constraints** - Container needs bounded size

### Vertical scrolling doesn't work

1. **Add `min_h_0()`** to all flex column parents
2. **Check content height** - Content must exceed container height
3. **Set `min_h()` on content** to ensure minimum height

### Horizontal scrolling doesn't work

1. **Add `min_w_0()`** to all flex row parents
2. **Use `whitespace_nowrap()`** on text to prevent wrapping
3. **Use `flex_shrink_0()`** on content to prevent shrinking
4. **Set `min_w()` on content** based on calculated width

### Scrollbars not visible

1. **Check initialization** - `gpui_component::init(cx)` must be called
2. **Check show mode** - Set `ScrollbarShow::Always` to test
3. **Check content exceeds container** - Both dimensions must be larger
4. **Pass explicit `scroll_size()`** to the Scrollbar component
5. **Check theme colors** - Ensure thumb colors aren't transparent

### Content extends beyond window

1. **Missing `min_h_0()` somewhere** - Check entire parent chain
2. **Using `size_full()` instead of `flex_1()`** - Use flex_1 + min_h_0 for flexible sizing
3. **Fixed-height parents** - Ensure parents properly constrain children

## Complete Working Example

```rust
use gpui::prelude::*;
use gpui::*;
use gpui_component::scroll::{Scrollbar, ScrollbarShow};
use gpui_component::theme::Theme;

fn main() {
    Application::new().run(|cx: &mut App| {
        gpui_component::init(cx);
        Theme::global_mut(cx).scrollbar_show = ScrollbarShow::Always;

        cx.open_window(WindowOptions::default(), |_window, cx| {
            cx.new(|_cx| MyView::new())
        }).unwrap();

        cx.activate(true);
    });
}

struct MyView {
    scroll_handle: ScrollHandle,
    lines: Vec<String>,
}

impl MyView {
    fn new() -> Self {
        Self {
            scroll_handle: ScrollHandle::default(),
            lines: (0..100).map(|i| format!("Line {} - some content here that might be long", i)).collect(),
        }
    }
}

impl Render for MyView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let max_chars = self.lines.iter().map(|l| l.len()).max().unwrap_or(0);
        let content_width = (max_chars as f32 * 8.0) + 64.0;
        let content_height = (self.lines.len() as f32 * 20.0) + 64.0;
        let scroll_handle = &self.scroll_handle;

        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(rgb(0x1e1e1e))
            .child(
                // Header (fixed height)
                div()
                    .p_4()
                    .bg(rgb(0x2d2d2d))
                    .child("Header")
            )
            .child(
                // Main content - MUST have min_h_0 for child scrolling
                div()
                    .flex_1()
                    .min_h_0()  // CRITICAL!
                    .child(
                        // Scroll container
                        div()
                            .id("scroll_container")
                            .size_full()
                            .relative()
                            .child(
                                div()
                                    .id("scroll_area")
                                    .size_full()
                                    .overflow_scroll()
                                    .track_scroll(scroll_handle)
                                    .child(
                                        div()
                                            .flex()
                                            .flex_col()
                                            .flex_shrink_0()
                                            .items_start()
                                            .min_w(px(content_width))
                                            .min_h(px(content_height))
                                            .p_4()
                                            .gap_1()
                                            .children(self.lines.iter().map(|line| {
                                                div()
                                                    .whitespace_nowrap()
                                                    .flex_shrink_0()
                                                    .text_color(rgb(0xcccccc))
                                                    .child(line.clone())
                                            }))
                                    )
                            )
                            .child(
                                div()
                                    .absolute()
                                    .top_0()
                                    .left_0()
                                    .right_0()
                                    .bottom_0()
                                    .child(
                                        Scrollbar::new(scroll_handle)
                                            .id("scrollbar")
                                            .scroll_size(size(px(content_width), px(content_height)))
                                    )
                            )
                    )
            )
    }
}
```

## Common Pitfall: Recreating Scrollable Components on Every Render

### The Problem

When using `adabraka_ui::scrollable_vertical` (or similar scroll wrappers), if you create the component fresh on every render, the scroll state is lost:

```rust
// BAD - Creates new HelpOverlay (and new ScrollHandle) on every render
// Scroll position resets, scroll events may not work properly
.when(self.show_help, |el| {
    el.child(cx.new(|_cx| HelpOverlay::new(...)))
})
```

**Symptoms:**
- Scrollbar appears but doesn't respond to mouse wheel
- Clicking/dragging the scrollbar does nothing
- Scroll position resets unexpectedly

### The Solution

Store the scrollable component as a persistent `Entity<T>` and reuse it:

```rust
// In your App struct
struct App {
    help_overlay: Entity<HelpOverlay>,
    // ...
}

// Create once during initialization
let help_overlay = cx.new(|_cx| {
    HelpOverlay::new(overlay_transparency, font_size_scale)
});

// Reuse the stored entity in render
.when(self.show_help, |el| {
    el.child(self.help_overlay.clone())  // Reuse, don't recreate
})
```

### For adabraka_ui Scrollable Components

When using `scrollable_vertical` from adabraka_ui, also ensure you:

1. **Add a `ScrollHandle` field** to your component:
   ```rust
   pub struct HelpOverlay {
       scroll_handle: ScrollHandle,
       // ...
   }
   ```

2. **Initialize it in the constructor**:
   ```rust
   impl HelpOverlay {
       pub fn new(...) -> Self {
           Self {
               scroll_handle: ScrollHandle::new(),
               // ...
           }
       }
   }
   ```

3. **Connect it to the scrollable**:
   ```rust
   scrollable_vertical(content)
       .with_scroll_handle(self.scroll_handle.clone())
       .always_show_scrollbars()
       .id("my-scrollable")
   ```

### Why This Matters

- `ScrollHandle` maintains scroll position and connects to GPUI's event system
- Recreating components loses the handle, breaking event routing
- GPUI entities are designed to be persistent and reused across renders

## Summary of Key Points

1. **`min_h_0()` is essential** - Add to all flex parents for vertical scrolling
2. **`min_w_0()` is essential** - Add to all flex parents for horizontal scrolling
3. **Use manual scrollbar setup** - More reliable than `overflow_scrollbar()`
4. **Calculate and pass `scroll_size()`** - Helps scrollbar determine when to show
5. **Content must exceed container** - Use `min_w()` and `min_h()` on content
6. **Initialize gpui-component** - Call `gpui_component::init(cx)` at startup
7. **Set `ScrollbarShow::Always`** - For debugging or to override macOS auto-hide
8. **Persist scrollable components** - Store as `Entity<T>` and reuse; don't recreate on every render

## Version Information

- Tested with: gpui 0.2.2, gpui-component 0.5.0, adabraka-ui 0.2.3
- Platform: macOS (Darwin)
- Date: 2026-01-06
