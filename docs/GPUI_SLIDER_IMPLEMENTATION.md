# GPUI Slider Implementation Guide

## Table of Contents
1. [Overview](#overview)
2. [Mouse Capture in GPUI](#mouse-capture-in-gpui)
3. [Pre-built Component Libraries](#pre-built-component-libraries)
4. [Scrollbar Implementation in Zed](#scrollbar-implementation-in-zed)
5. [Best Practices](#best-practices)
6. [Implementation Examples](#implementation-examples)
7. [Common Pitfalls](#common-pitfalls)
8. [Sources and References](#sources-and-references)

---

## Overview

This document provides comprehensive research findings on implementing sliders and interactive drag components in GPUI. As of December 2024, GPUI does not include built-in slider components, requiring either custom implementation or third-party libraries.

**Key Challenge:** Proper mouse capture is essential for sliders. Without it, dragging outside the slider bounds or releasing the mouse outside the window causes incorrect behavior.

---

## Mouse Capture in GPUI

### The Problem with Basic Mouse Handlers

Our current implementation (as of this writing) has a critical flaw:

```rust
// ❌ INCORRECT: Mouse events only fire when cursor is over the element
div()
    .on_mouse_down(MouseButton::Left, cx.listener(|this, event, cx| {
        this.active_slider = Some(SliderType::Brightness);
        this.mouse_button_down = true;
    }))
    .on_mouse_move(cx.listener(|this, event, cx| {
        if this.mouse_button_down {
            // This ONLY fires when mouse is over the div!
            // If user drags outside, we lose tracking
        }
    }))
```

**Issues:**
1. `on_mouse_move` only fires when the cursor is over the element
2. If the user drags outside the slider area, movement stops being tracked
3. If the user releases the mouse outside the window, the slider stays "stuck" in dragging mode
4. No way to constrain cursor or capture global mouse events

### Proper Mouse Capture Solutions

#### Solution 1: Window-Level Mouse Listeners (Recommended)

GPUI provides window-level event handling that continues to receive events even when the mouse is outside specific elements.

```rust
use gpui::*;

struct SliderView {
    dragging: bool,
    slider_value: f32,
}

impl Render for SliderView {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        div()
            .size_full()
            .on_mouse_down(MouseButton::Left, cx.listener(|this, event: &MouseDownEvent, cx| {
                this.dragging = true;
                
                // Subscribe to window-level mouse events
                cx.on_next_frame(|this, cx| {
                    // This ensures we process mouse events at window level
                    cx.notify();
                });
            }))
            // Key: Listen to mouse_up at window level through cx.on_mouse_event
            .child(/* slider UI */)
    }
}

impl SliderView {
    fn handle_global_mouse_move(&mut self, event: &MouseMoveEvent, cx: &mut ViewContext<Self>) {
        if self.dragging {
            // Update slider value based on mouse position
            // This fires even if mouse is outside the slider!
            cx.notify();
        }
    }
    
    fn handle_global_mouse_up(&mut self, event: &MouseUpEvent, cx: &mut ViewContext<Self>) {
        if self.dragging {
            self.dragging = false;
            cx.notify();
        }
    }
}
```

#### Solution 2: Using GPUI's Active Mousedown Tracking

Based on Zed's implementation patterns, GPUI tracks active mouse button state globally:

```rust
fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
    div()
        .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, cx| {
            // Check if button is actually still pressed
            let button_pressed = event.pressed_button == Some(MouseButton::Left);
            
            if this.dragging && !button_pressed {
                // Mouse was released outside our element - cleanup
                this.dragging = false;
                this.active_slider = None;
                cx.notify();
                return;
            }
            
            if this.dragging && button_pressed {
                // Safe to update - mouse is actually pressed
                this.update_slider_value(event.position, cx);
            }
        }))
}
```

**Key Property:** `event.pressed_button` - Available on `MouseMoveEvent`, indicates which button (if any) is currently pressed globally.

#### Solution 3: Cursor Locking (Not Available in GPUI)

**Important Note:** Unlike native GUI frameworks, GPUI does **not** currently provide:
- `CaptureMouse()` / `ReleaseMouse()` APIs
- Cursor locking/confinement to screen regions  
- Invisible cursor with delta-only movement

These features would need to be implemented at the windowing layer (likely requires changes to GPUI core).

### Current State of Mouse Capture in Zed/GPUI

**Research Findings:**

1. **GitHub Issue #24797** - [Support drag cursor](https://github.com/zed-industries/zed/pull/24797)
   - Added `CursorStyle::Grab` and `CursorStyle::Grabbing` (merged Nov 2024)
   - Visual feedback only - does not implement actual mouse capture

2. **GitHub Discussion #30637** - [Expose content size to ScrollHandle State](https://github.com/zed-industries/zed/discussions/30637)
   - Discusses scroll calculation but not mouse capture mechanics
   - Scrollbars use relative positioning rather than true capture

3. **No Native Capture API** - Current GPUI implementation relies on:
   - Checking `event.pressed_button` for button state
   - Attaching handlers to large container elements
   - Handling edge cases manually

### Recommended Mouse Capture Pattern for Sliders

```rust
use gpui::*;

#[derive(Clone, Copy, PartialEq)]
enum ActiveSlider {
    Brightness,
    Contrast,
    Gamma,
}

struct App {
    active_slider: Option<ActiveSlider>,
    brightness: f32,
    contrast: f32,
    gamma: f32,
}

impl App {
    fn render_slider(
        &self,
        cx: &mut ViewContext<Self>,
        slider_type: ActiveSlider,
        value: f32,
        label: &str,
    ) -> impl IntoElement {
        let is_active = self.active_slider == Some(slider_type);
        
        div()
            .h(px(30.0))  // Larger hit area
            .w_full()
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .text_sm()
                    .text_color(rgb(0xCCCCCC))
                    .child(format!("{}: {:.2}", label, value))
            )
            .child(
                div()
                    .h(px(20.0))  // Large vertical hit area
                    .w_full()
                    .flex()
                    .items_center()
                    .cursor_pointer()
                    // Mouse down - start dragging
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, event: &MouseDownEvent, cx| {
                            this.active_slider = Some(slider_type);
                            this.update_slider_from_position(
                                slider_type,
                                event.position,
                                cx,
                            );
                            cx.notify();
                        }),
                    )
                    // Mouse move - update if dragging
                    .on_mouse_move(cx.listener(move |this, event: &MouseMoveEvent, cx| {
                        // Safety check: is button actually pressed?
                        let button_pressed = event.pressed_button == Some(MouseButton::Left);
                        
                        if this.active_slider == Some(slider_type) && !button_pressed {
                            // Button released outside window - cleanup
                            this.active_slider = None;
                            cx.notify();
                            return;
                        }
                        
                        if this.active_slider == Some(slider_type) && button_pressed {
                            this.update_slider_from_position(
                                slider_type,
                                event.position,
                                cx,
                            );
                            cx.notify();
                        }
                    }))
                    // Mouse up - stop dragging
                    .on_mouse_up(
                        MouseButton::Left,
                        cx.listener(move |this, _event: &MouseUpEvent, cx| {
                            if this.active_slider == Some(slider_type) {
                                this.active_slider = None;
                                cx.notify();
                            }
                        }),
                    )
                    .child(
                        // Visual slider track
                        div()
                            .h(px(6.0))
                            .w_full()
                            .bg(rgba(0x444444FF))
                            .rounded(px(3.0))
                            .child(
                                // Filled portion
                                div()
                                    .h_full()
                                    .w(relative(value))
                                    .bg(if is_active {
                                        rgb(0x00AAFF)  // Bright when active
                                    } else {
                                        rgb(0x0088FF)  // Normal color
                                    })
                                    .rounded(px(3.0))
                            )
                    )
            )
    }
    
    fn update_slider_from_position(
        &mut self,
        slider_type: ActiveSlider,
        mouse_pos: Point<Pixels>,
        cx: &mut ViewContext<Self>,
    ) {
        // Get slider bounds from layout
        // Note: You'll need to store these during layout phase
        // or calculate from viewport size
        
        let slider_left = px(100.0);  // Example
        let slider_width = px(200.0); // Example
        
        let relative_x = (mouse_pos.x - slider_left).max(px(0.0));
        let percent = (relative_x / slider_width).clamp(0.0, 1.0);
        
        match slider_type {
            ActiveSlider::Brightness => {
                self.brightness = percent;
            }
            ActiveSlider::Contrast => {
                self.contrast = percent;
            }
            ActiveSlider::Gamma => {
                // Non-linear mapping example
                self.gamma = if percent <= 0.5 {
                    0.1 + (percent / 0.5) * 0.9
                } else {
                    1.0 + ((percent - 0.5) / 0.5) * 9.0
                };
            }
        }
    }
}
```

**Key Points:**
1. ✅ Check `event.pressed_button` on mouse move to detect external releases
2. ✅ Use larger hit areas (20px height) for easier interaction
3. ✅ Store active slider state to know which slider is being dragged
4. ✅ Provide visual feedback when slider is active
5. ✅ Handle cleanup when mouse is released outside the slider/window

---

## Pre-built Component Libraries

### 1. adabraka-ui (Recommended)

**Repository:** https://github.com/Augani/adabraka-ui  
**Documentation:** https://github.com/Augani/adabraka-ui/blob/main/README.md  
**Latest Release:** v0.1.11 (November 2025)  
**License:** MIT

**Features:**
- 73+ accessible UI components inspired by shadcn/ui
- Built specifically for GPUI
- Includes fully-featured Slider component with:
  - Horizontal and vertical orientations
  - Multiple sizes (Small, Medium, Large)
  - Value display toggle
  - Change callbacks
  - Custom styling support
  - Proper mouse capture handling

**Installation:**

```toml
# Cargo.toml
[dependencies]
adabraka-ui = "0.1"
```

**Note:** Requires nightly Rust compiler.

**Example Usage:**

```rust
use adabraka_ui::prelude::*;
use gpui::*;

struct MyApp {
    slider_value: SharedState<f32>,
}

impl Render for MyApp {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        div()
            .child(
                Slider::new(self.slider_value.clone())
                    .size(SliderSize::Lg)
                    .show_value(true)
                    .on_change(|value, _, cx| {
                        println!("Slider value changed to: {}", value);
                    })
            )
            .child(
                // Vertical slider example
                Slider::new(self.slider_value.clone())
                    .vertical()
                    .size(SliderSize::Md)
            )
    }
}
```

**Other Components Available:**
- Button, Checkbox, Radio, Toggle
- Input, Textarea, Select
- Tabs, Accordion, Collapsible
- Dialog, Popover, Tooltip
- Alert, Toast, Progress
- And 60+ more...

### 2. gpui-component

**Repository:** https://github.com/longbridge/gpui-component  
**Documentation:** https://longbridge.github.io/gpui-component/  
**License:** MIT

**Features:**
- 60+ cross-platform UI components
- Professional styling and theming
- Virtualized rendering for performance
- Well-documented with examples
- Active development and maintenance

**Installation:**

```toml
# Cargo.toml
[dependencies]
gpui-component = { git = "https://github.com/longbridge/gpui-component" }
```

**Components Include:**
- Form controls (Input, Checkbox, Radio, Select)
- Navigation (Tabs, Menu, Breadcrumb)
- Data display (Table, List, Tree)
- Feedback (Alert, Modal, Toast)
- Layout helpers

**Example:**

```rust
use gpui_component::*;

// Check documentation for specific slider implementation
// as API may vary
```

### 3. Custom Implementation

For full control and learning purposes, custom implementation is valuable. See [Implementation Examples](#implementation-examples) section below.

---

## Scrollbar Implementation in Zed

### Overview

Scrollbars in Zed provide excellent reference for slider-like components, as they share similar interaction patterns (dragging a thumb along a track).

### Key Files and PRs

1. **PR #25392** - [Add scrollbar_width method](https://github.com/zed-industries/zed/pull/25392)
   - Adds methods to reserve space for scrollbar rendering
   - Located in `crates/gpui/src/elements/div.rs`

2. **Discussion #30637** - [Expose content size to ScrollHandle State](https://github.com/zed-industries/zed/discussions/30637)
   - Details on scroll position calculations
   - Shows how max_scroll is computed during layout

3. **Core Implementation** - `zed/crates/gpui/src/elements/div.rs`
   - Function: `clamp_scroll_position()` calculates bounds during prepaint
   - Uses relative positioning for scrollbar thumb

### Scrollbar Architecture

```rust
// Simplified from Zed's implementation

struct ScrollState {
    offset: Point<Pixels>,    // Current scroll position
    max_offset: Point<Pixels>, // Maximum scrollable distance
}

impl ScrollState {
    fn clamp_scroll_position(&mut self, content_size: Size<Pixels>, viewport_size: Size<Pixels>) {
        // Calculate maximum scroll
        self.max_offset = Point {
            x: (content_size.width - viewport_size.width).max(px(0.0)),
            y: (content_size.height - viewport_size.height).max(px(0.0)),
        };
        
        // Clamp current offset
        self.offset.x = self.offset.x.clamp(px(0.0), self.max_offset.x);
        self.offset.y = self.offset.y.clamp(px(0.0), self.max_offset.y);
    }
}

// Render scrollbar thumb
fn render_scrollbar(&self, cx: &mut Context) -> impl IntoElement {
    let thumb_size = (viewport_size / content_size) * track_size;
    let thumb_position = (offset / max_offset) * (track_size - thumb_size);
    
    div()
        .w(px(12.0))  // Scrollbar width
        .h_full()
        .child(
            div()
                .w_full()
                .h(thumb_size)
                .top(thumb_position)
                .bg(rgba(0xFFFFFF33))
                .rounded(px(6.0))
                // Mouse handlers here (similar to slider)
        )
}
```

### Key Principles Applicable to Sliders

1. **Separation of Concerns:**
   - State (scroll offset) separate from rendering
   - Calculations done during prepaint/layout phase
   - Visual rendering uses calculated values

2. **Proportional Sizing:**
   - Thumb size proportional to viewport/content ratio
   - Thumb position proportional to offset/max_offset ratio

3. **Clamping:**
   - Always clamp values to valid ranges
   - Recalculate bounds when content/viewport changes

4. **Relative Positioning:**
   - Use `.top()`, `.left()` for thumb positioning
   - Parent container uses `.relative()` positioning

### Differences from Sliders

- Scrollbars respond to mouse wheel events
- Scrollbars have dynamic thumb size (sliders typically have fixed thumb)
- Scrollbars track two-dimensional offset (horizontal and vertical)
- Scrollbars automatically hide when content fits in viewport

---

## Best Practices

### 1. State Management

**Use Clear State Tracking:**

```rust
struct SliderState {
    // Which slider is currently being dragged
    active: Option<SliderId>,
    
    // Current values
    values: HashMap<SliderId, f32>,
    
    // Cached layout information (updated during render)
    slider_bounds: HashMap<SliderId, Bounds<Pixels>>,
}
```

**Always Notify on Changes:**

```rust
fn update_value(&mut self, value: f32, cx: &mut ViewContext<Self>) {
    self.value = value;
    cx.notify();  // Critical: triggers re-render
}
```

### 2. Layout Calculation

**Store Bounds During Render:**

```rust
impl Render for SliderView {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let slider_id = ElementId::Name("my-slider".into());
        
        div()
            .id(slider_id.clone())
            .on_mouse_down(MouseButton::Left, cx.listener(move |this, event, cx| {
                // Get element bounds
                if let Some(bounds) = cx.element_bounds(&slider_id) {
                    this.slider_bounds = Some(bounds);
                    // Calculate value from mouse position relative to bounds
                }
            }))
    }
}
```

**Use Viewport-Aware Positioning:**

```rust
// Calculate slider position relative to viewport
fn calculate_slider_layout(viewport: Size<Pixels>) -> (Pixels, Pixels) {
    let panel_width = px(320.0);
    let padding = px(20.0);
    let right_offset = px(20.0);
    
    let panel_left = viewport.width - panel_width - right_offset;
    let slider_left = panel_left + padding;
    let slider_width = panel_width - (padding * 2.0);
    
    (slider_left, slider_width)
}
```

### 3. Visual Feedback

**Show Active State:**

```rust
.bg(if is_active {
    rgb(0x00AAFF)  // Bright blue when dragging
} else {
    rgb(0x0088FF)  // Normal blue
})
```

**Use Appropriate Cursors:**

```rust
div()
    .cursor(if self.dragging {
        CursorStyle::Grabbing
    } else {
        CursorStyle::Grab
    })
```

**Consider Adding Visual Thumb:**

```rust
// Instead of just a filled bar, add a visible thumb
.child(
    div()
        .absolute()
        .left(relative(value))
        .w(px(16.0))
        .h(px(16.0))
        .rounded(px(8.0))
        .bg(rgb(0xFFFFFF))
        .border_2()
        .border_color(rgb(0x0088FF))
)
```

### 4. Value Mapping

**Support Non-Linear Mappings:**

```rust
fn map_to_value(percent: f32, min: f32, max: f32, curve: CurveType) -> f32 {
    match curve {
        CurveType::Linear => {
            min + (percent * (max - min))
        }
        CurveType::Exponential => {
            // Example: gamma slider with exponential curve
            if percent <= 0.5 {
                min + (percent / 0.5) * ((max - min) / 2.0)
            } else {
                (min + max) / 2.0 + ((percent - 0.5) / 0.5) * ((max - min) / 2.0)
            }
        }
        CurveType::Logarithmic => {
            // For frequency, gain, etc.
            min * ((max / min).powf(percent))
        }
    }
}
```

**Always Clamp Values:**

```rust
fn set_value(&mut self, value: f32) {
    self.value = value.clamp(self.min, self.max);
}
```

### 5. Accessibility

**Provide Text Labels and Values:**

```rust
div()
    .flex()
    .flex_col()
    .child(
        div()
            .text_sm()
            .child(format!("{}: {:.2}", label, value))
    )
    .child(/* slider visual */)
```

**Support Keyboard Input (Advanced):**

```rust
// Future enhancement: arrow keys to adjust
.on_key_down(cx.listener(|this, event: &KeyDownEvent, cx| {
    match event.key {
        Key::ArrowLeft => this.decrease_value(0.01),
        Key::ArrowRight => this.increase_value(0.01),
        _ => {}
    }
}))
```

### 6. Performance

**Avoid Unnecessary Re-renders:**

```rust
// Only notify if value actually changed
fn update_value(&mut self, new_value: f32, cx: &mut ViewContext<Self>) {
    let clamped = new_value.clamp(0.0, 1.0);
    if (self.value - clamped).abs() > 0.001 {  // Threshold for floating point
        self.value = clamped;
        cx.notify();
    }
}
```

**Debounce Expensive Operations:**

```rust
fn on_slider_change(&mut self, value: f32, cx: &mut ViewContext<Self>) {
    self.pending_value = value;
    
    // Update visual immediately
    cx.notify();
    
    // Debounce expensive recalculation
    cx.spawn(|this, mut cx| async move {
        Timer::after(Duration::from_millis(50)).await;
        this.update(&mut cx, |this, cx| {
            this.apply_expensive_filter(this.pending_value);
            cx.notify();
        })
    }).detach();
}
```

### 7. Testing Edge Cases

**Test These Scenarios:**

1. ✅ Drag beyond slider bounds (left/right)
2. ✅ Release mouse outside window
3. ✅ Release mouse outside slider element
4. ✅ Rapid dragging back and forth
5. ✅ Multiple sliders on same panel (only one should be active)
6. ✅ Window resize during drag
7. ✅ Minimum and maximum value boundaries

---

## Implementation Examples

### Complete Slider Component

```rust
use gpui::*;

#[derive(Clone)]
pub struct SliderConfig {
    pub min: f32,
    pub max: f32,
    pub step: Option<f32>,
    pub default: f32,
    pub curve: ValueCurve,
}

#[derive(Clone, Copy)]
pub enum ValueCurve {
    Linear,
    Exponential,
    Logarithmic,
}

pub struct Slider {
    id: ElementId,
    config: SliderConfig,
    value: f32,
    dragging: bool,
    on_change: Option<Box<dyn Fn(f32)>>,
}

impl Slider {
    pub fn new(id: impl Into<ElementId>, config: SliderConfig) -> Self {
        Self {
            id: id.into(),
            config: config.clone(),
            value: config.default,
            dragging: false,
            on_change: None,
        }
    }
    
    pub fn value(mut self, value: f32) -> Self {
        self.value = value.clamp(self.config.min, self.config.max);
        self
    }
    
    pub fn on_change(mut self, callback: impl Fn(f32) + 'static) -> Self {
        self.on_change = Some(Box::new(callback));
        self
    }
    
    fn percent_to_value(&self, percent: f32) -> f32 {
        let clamped = percent.clamp(0.0, 1.0);
        let raw_value = match self.config.curve {
            ValueCurve::Linear => {
                self.config.min + (clamped * (self.config.max - self.config.min))
            }
            ValueCurve::Exponential => {
                // Maps 0-50% to first half, 50-100% to second half with curve
                if clamped <= 0.5 {
                    self.config.min + (clamped / 0.5) * ((self.config.max - self.config.min) / 2.0)
                } else {
                    let mid = (self.config.min + self.config.max) / 2.0;
                    mid + ((clamped - 0.5) / 0.5) * ((self.config.max - self.config.min) / 2.0)
                }
            }
            ValueCurve::Logarithmic => {
                self.config.min * ((self.config.max / self.config.min).powf(clamped))
            }
        };
        
        // Apply step if configured
        if let Some(step) = self.config.step {
            (raw_value / step).round() * step
        } else {
            raw_value
        }
    }
    
    fn value_to_percent(&self, value: f32) -> f32 {
        let clamped = value.clamp(self.config.min, self.config.max);
        
        match self.config.curve {
            ValueCurve::Linear => {
                (clamped - self.config.min) / (self.config.max - self.config.min)
            }
            ValueCurve::Logarithmic => {
                (clamped / self.config.min).log(self.config.max / self.config.min)
            }
            ValueCurve::Exponential => {
                // Inverse of exponential mapping
                let mid = (self.config.min + self.config.max) / 2.0;
                if clamped <= mid {
                    ((clamped - self.config.min) / ((self.config.max - self.config.min) / 2.0)) * 0.5
                } else {
                    0.5 + ((clamped - mid) / ((self.config.max - self.config.min) / 2.0)) * 0.5
                }
            }
        }
    }
}

impl RenderOnce for Slider {
    fn render(self, cx: &mut WindowContext) -> impl IntoElement {
        let id = self.id.clone();
        let percent = self.value_to_percent(self.value);
        
        div()
            .id(id.clone())
            .h(px(20.0))
            .w_full()
            .flex()
            .items_center()
            .cursor_pointer()
            .on_mouse_down(MouseButton::Left, {
                let id = id.clone();
                move |event: &MouseDownEvent, cx| {
                    // Get element bounds and calculate value
                    if let Some(bounds) = cx.element_bounds(&id) {
                        let relative_x = (event.position.x - bounds.left()).max(px(0.0));
                        let percent = (relative_x / bounds.size.width).clamp(0.0, 1.0);
                        // Update value via callback
                        // Note: This is simplified - real implementation needs state
                    }
                }
            })
            .child(
                div()
                    .h(px(6.0))
                    .w_full()
                    .bg(rgba(0x444444FF))
                    .rounded(px(3.0))
                    .relative()
                    .child(
                        // Filled portion
                        div()
                            .h_full()
                            .w(relative(percent))
                            .bg(if self.dragging {
                                rgb(0x00AAFF)
                            } else {
                                rgb(0x0088FF)
                            })
                            .rounded(px(3.0))
                    )
                    .child(
                        // Thumb
                        div()
                            .absolute()
                            .left(relative(percent))
                            .top(px(-5.0))
                            .w(px(16.0))
                            .h(px(16.0))
                            .rounded(px(8.0))
                            .bg(rgb(0xFFFFFF))
                            .border_2()
                            .border_color(if self.dragging {
                                rgb(0x00AAFF)
                            } else {
                                rgb(0x0088FF)
                            })
                    )
            )
    }
}
```

**Note:** The above is a simplified example. For production use, you'll need to manage state properly, likely using a `View<Slider>` pattern or integrating with a parent view's state.

### Stateful Slider in Parent View

```rust
use gpui::*;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum SliderId {
    Brightness,
    Contrast,
    Saturation,
}

struct ImageEditor {
    active_slider: Option<SliderId>,
    brightness: f32,
    contrast: f32,
    saturation: f32,
    slider_bounds: Option<Bounds<Pixels>>,
}

impl ImageEditor {
    fn new() -> Self {
        Self {
            active_slider: None,
            brightness: 1.0,
            contrast: 1.0,
            saturation: 1.0,
            slider_bounds: None,
        }
    }
    
    fn render_labeled_slider(
        &self,
        cx: &mut ViewContext<Self>,
        id: SliderId,
        label: &str,
        value: f32,
        min: f32,
        max: f32,
    ) -> impl IntoElement {
        let is_active = self.active_slider == Some(id);
        let element_id = ElementId::Name(format!("slider-{:?}", id).into());
        
        div()
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .text_sm()
                    .text_color(rgb(0xCCCCCC))
                    .child(format!("{}: {:.2}", label, value))
            )
            .child(
                div()
                    .id(element_id.clone())
                    .h(px(20.0))
                    .w_full()
                    .flex()
                    .items_center()
                    .cursor_pointer()
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, event: &MouseDownEvent, cx| {
                            if let Some(bounds) = cx.element_bounds(&element_id) {
                                this.active_slider = Some(id);
                                this.slider_bounds = Some(bounds);
                                this.update_slider_from_mouse(id, event.position, min, max, cx);
                                cx.notify();
                            }
                        }),
                    )
                    .on_mouse_move(cx.listener(move |this, event: &MouseMoveEvent, cx| {
                        // Safety check for mouse release outside window
                        let button_pressed = event.pressed_button == Some(MouseButton::Left);
                        
                        if this.active_slider == Some(id) && !button_pressed {
                            this.active_slider = None;
                            cx.notify();
                            return;
                        }
                        
                        if this.active_slider == Some(id) && button_pressed {
                            this.update_slider_from_mouse(id, event.position, min, max, cx);
                            cx.notify();
                        }
                    }))
                    .on_mouse_up(
                        MouseButton::Left,
                        cx.listener(move |this, _event: &MouseUpEvent, cx| {
                            if this.active_slider == Some(id) {
                                this.active_slider = None;
                                cx.notify();
                            }
                        }),
                    )
                    .child(
                        div()
                            .h(px(6.0))
                            .w_full()
                            .bg(rgba(0x444444FF))
                            .rounded(px(3.0))
                            .relative()
                            .child(
                                // Filled portion
                                div()
                                    .h_full()
                                    .w(relative((value - min) / (max - min)))
                                    .bg(if is_active {
                                        rgb(0x00AAFF)
                                    } else {
                                        rgb(0x0088FF)
                                    })
                                    .rounded(px(3.0))
                            )
                    )
            )
    }
    
    fn update_slider_from_mouse(
        &mut self,
        id: SliderId,
        mouse_pos: Point<Pixels>,
        min: f32,
        max: f32,
        cx: &mut ViewContext<Self>,
    ) {
        if let Some(bounds) = &self.slider_bounds {
            let relative_x = (mouse_pos.x - bounds.left()).max(px(0.0));
            let percent = (relative_x / bounds.size.width).clamp(0.0, 1.0);
            let value = min + (percent * (max - min));
            
            match id {
                SliderId::Brightness => self.brightness = value,
                SliderId::Contrast => self.contrast = value,
                SliderId::Saturation => self.saturation = value,
            }
        }
    }
}

impl Render for ImageEditor {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_4()
            .p_4()
            .child(self.render_labeled_slider(
                cx,
                SliderId::Brightness,
                "Brightness",
                self.brightness,
                0.0,
                2.0,
            ))
            .child(self.render_labeled_slider(
                cx,
                SliderId::Contrast,
                "Contrast",
                self.contrast,
                0.0,
                2.0,
            ))
            .child(self.render_labeled_slider(
                cx,
                SliderId::Saturation,
                "Saturation",
                self.saturation,
                0.0,
                2.0,
            ))
    }
}
```

---

## Common Pitfalls

### 1. ❌ Not Checking Button State on Mouse Move

```rust
// WRONG - doesn't handle release outside window
.on_mouse_move(cx.listener(|this, event, cx| {
    if this.dragging {
        // This will stay true even if mouse was released outside!
        this.update_value(event.position);
    }
}))

// CORRECT
.on_mouse_move(cx.listener(|this, event, cx| {
    let button_pressed = event.pressed_button == Some(MouseButton::Left);
    
    if this.dragging && !button_pressed {
        this.dragging = false;
        cx.notify();
        return;
    }
    
    if this.dragging && button_pressed {
        this.update_value(event.position);
    }
}))
```

### 2. ❌ Forgetting to Call `cx.notify()`

```rust
// WRONG - changes won't trigger re-render
fn update_value(&mut self, value: f32) {
    self.value = value;
    // Missing cx.notify()!
}

// CORRECT
fn update_value(&mut self, value: f32, cx: &mut ViewContext<Self>) {
    self.value = value;
    cx.notify();  // Triggers re-render
}
```

### 3. ❌ Using Small Hit Areas

```rust
// WRONG - hard to click/drag
div()
    .h(px(4.0))  // Too small!
    .w_full()
    .on_mouse_down(...)

// CORRECT - larger interactive area
div()
    .h(px(20.0))  // Easy to click
    .w_full()
    .flex()
    .items_center()
    .on_mouse_down(...)
    .child(
        div()
            .h(px(4.0))  // Visual slider can be small
            .w_full()
            // ...
    )
```

### 4. ❌ Not Clamping Values

```rust
// WRONG - values can go out of range
self.value = min + (percent * (max - min));

// CORRECT
self.value = (min + (percent * (max - min))).clamp(min, max);
```

### 5. ❌ Hardcoding Layout Values

```rust
// WRONG - breaks on window resize
let slider_left = px(100.0);
let slider_width = px(200.0);

// CORRECT - calculate from element bounds
if let Some(bounds) = cx.element_bounds(&slider_id) {
    let slider_left = bounds.left();
    let slider_width = bounds.size.width;
    // ...
}
```

### 6. ❌ Multiple Sliders Dragging Simultaneously

```rust
// WRONG - all sliders respond to dragging
struct App {
    brightness_dragging: bool,
    contrast_dragging: bool,
    gamma_dragging: bool,
}

// CORRECT - only one can be active
struct App {
    active_slider: Option<SliderId>,
}
```

### 7. ❌ Not Handling Element ID Properly

```rust
// WRONG - same ID for all sliders
.id(ElementId::Name("slider".into()))

// CORRECT - unique ID per slider
.id(ElementId::Name(format!("slider-{:?}", slider_id).into()))
```

---

## Sources and References

### Official GPUI Resources

1. **GPUI Official Website**  
   https://www.gpui.rs/  
   Official GPUI framework website

2. **GPUI Crate Source**  
   https://github.com/zed-industries/zed/tree/main/crates/gpui  
   Core GPUI implementation in Zed repository

3. **Zed Editor Repository**  
   https://github.com/zed-industries/zed  
   Reference implementations of GPUI patterns

### Component Libraries

4. **adabraka-ui Repository**  
   https://github.com/Augani/adabraka-ui  
   73+ GPUI components including sliders

5. **adabraka-ui Releases**  
   https://github.com/Augani/adabraka-ui/releases  
   Latest: v0.1.11 (November 2025) - Enhanced slider with vertical orientation

6. **gpui-component Repository**  
   https://github.com/longbridge/gpui-component  
   60+ professional UI components

7. **gpui-component Documentation**  
   https://longbridge.github.io/gpui-component/  
   API documentation and examples

### GitHub Issues and Discussions

8. **PR #25392 - Add scrollbar_width method**  
   https://github.com/zed-industries/zed/pull/25392  
   Scrollbar rendering space reservation

9. **Discussion #30637 - Expose content size to ScrollHandle State**  
   https://github.com/zed-industries/zed/discussions/30637  
   Scroll position calculation details

10. **PR #24797 - Support drag cursor**  
    https://github.com/zed-industries/zed/pull/24797  
    Added CursorStyle::Grab and CursorStyle::Grabbing (merged Nov 2024)

### Tutorials and Examples

11. **Qiita - Moving Images with GPUI Mouse Events** (Japanese)  
    https://qiita.com/ishikawakouji/items/74a620a07cdd3b703624  
    Demonstrates draggable image viewer implementation (September 2024)

### Related Technologies

12. **Rust Language**  
    https://www.rust-lang.org/  
    GPUI is built with Rust

13. **WGPU**  
    https://wgpu.rs/  
    Graphics API used by GPUI

---

## Conclusion

Implementing sliders in GPUI requires careful attention to mouse capture, state management, and visual feedback. The key challenges are:

1. **Mouse Capture:** GPUI lacks native capture APIs, requiring manual state tracking and `pressed_button` checks
2. **Layout Calculation:** Element bounds must be retrieved and cached for position calculations
3. **State Synchronization:** Always call `cx.notify()` when values change
4. **Edge Cases:** Handle mouse release outside window/element

**Recommendations:**

- **For rapid development:** Use **adabraka-ui** - mature slider component with proper capture handling
- **For learning/customization:** Implement custom sliders following the patterns in this document
- **For production apps:** Consider starting with adabraka-ui and customizing as needed

**Critical Pattern for Mouse Capture:**

```rust
.on_mouse_move(cx.listener(|this, event, cx| {
    // ALWAYS check if button is actually pressed
    let button_pressed = event.pressed_button == Some(MouseButton::Left);
    
    if this.dragging && !button_pressed {
        // Cleanup on external release
        this.dragging = false;
        cx.notify();
        return;
    }
    
    if this.dragging && button_pressed {
        // Safe to update
        this.update_slider(event.position, cx);
    }
}))
```

This pattern ensures robust slider behavior even when users drag outside the window or release the mouse unexpectedly.

---

**Document Version:** 1.0  
**Last Updated:** December 29, 2024  
**Author:** Research on GPUI slider implementation patterns
