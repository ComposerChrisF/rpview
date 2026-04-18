use super::*;
use crate::utils::debug_eprintln;

impl Render for App {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Auto-dismiss toast after 2.5 seconds
        if let Some(ref toast) = self.toast {
            if toast.created_at.elapsed() >= Duration::from_millis(2500) {
                self.toast = None;
            } else {
                window.request_animation_frame();
            }
        }

        // Check if async image loading has completed
        if self.viewer.check_async_load() {
            // Image loaded successfully or failed - load state and setup animation
            if let Some(path) = self.app_state.current_image().cloned() {
                // Re-apply Local Contrast on every new image (session-global
                // sliders). Done before filter-state restore so the async LC
                // worker kicks off as early as possible.
                self.reapply_local_contrast_if_active(cx);

                // Load cached state if available and enabled in settings
                if self.settings.viewer_behavior.remember_per_image_state
                    && self.app_state.image_states.contains_key(&path)
                {
                    self.load_current_image_state(cx);
                } else {
                    // Apply default zoom mode from settings for new images
                    use crate::state::settings::ZoomMode;
                    match self.settings.viewer_behavior.default_zoom_mode {
                        ZoomMode::FitToWindow => {
                            self.viewer.fit_to_window();
                        }
                        ZoomMode::OneHundredPercent => {
                            self.viewer.set_one_hundred_percent();
                        }
                    }

                    // Reset filter controls to default (no filters)
                    let default_filters = state::image_state::FilterSettings {
                        brightness: 0.0,
                        contrast: 0.0,
                        gamma: 1.0,
                    };
                    self.filter_controls.update(cx, |controls, cx| {
                        controls.update_from_filters(default_filters, cx);
                    });
                }

                // Apply animation auto-play setting
                if let Some(ref mut anim_state) = self.viewer.image_state.animation {
                    // Set is_playing based on settings (unless we loaded cached state)
                    if !self.settings.viewer_behavior.remember_per_image_state
                        || !self.app_state.image_states.contains_key(&path)
                    {
                        anim_state.is_playing = self.settings.viewer_behavior.animation_auto_play;
                    }

                    if anim_state.is_playing {
                        self.last_frame_update = Instant::now();
                    }
                }

                // Tell the LC controls whether this image is animated (for batch UI).
                let is_animated = self
                    .viewer
                    .current_image
                    .as_ref()
                    .map(|img| img.animation_data.is_some())
                    .unwrap_or(false);
                self.local_contrast_controls.update(cx, |c, cx| {
                    c.set_is_animated(is_animated, cx);
                    c.set_batch_progress(None, cx);
                });
            }

            // Request re-render to show the loaded image
            cx.notify();
        }

        // Check if filter processing has completed; install the in-memory result immediately.
        if self.viewer.check_filter_processing() {
            cx.notify();
        }

        // Same for local-contrast processing.
        if self.viewer.check_lc_processing() {
            self.local_contrast_controls.update(cx, |c, cx| {
                c.set_status("Ready", cx);
                c.set_progress(None, cx);
            });
            cx.notify();
        }
        // While LC is running, keep requesting animation frames so we poll
        // the channel promptly when the worker finishes — otherwise nothing
        // re-wakes the render loop after the user stops moving the slider.
        if self.viewer.is_processing_lc() {
            if let Some(pct) = self.viewer.lc_progress_percent() {
                self.local_contrast_controls.update(cx, |c, cx| {
                    c.set_status(format!("Processing… {:.0}%", pct), cx);
                    c.set_progress(Some(pct / 100.0), cx);
                });
            }
            window.request_animation_frame();
        }

        // Poll batch LC processing (all animation frames).
        if let Some((current, total)) = self.viewer.check_lc_batch_processing() {
            self.local_contrast_controls.update(cx, |c, cx| {
                c.set_batch_progress(Some((current, total)), cx);
                c.set_status(format!("Processing frame {}/{}…", current + 1, total), cx);
            });
            window.request_animation_frame();
            cx.notify();
        } else if self.viewer.lc_batch_job.is_none() {
            // Batch just finished (or was never running). Clear progress
            // if the controls still show a batch in progress.
            let was_batching = self
                .local_contrast_controls
                .read(cx)
                .batch_progress
                .is_some();
            if was_batching {
                let all_done = self.viewer.all_frames_lc_processed();
                self.local_contrast_controls.update(cx, |c, cx| {
                    c.set_batch_progress(None, cx);
                    c.set_status(
                        if all_done {
                            "All frames processed"
                        } else {
                            "Batch cancelled"
                        },
                        cx,
                    );
                });
                cx.notify();
            }
        }
        // Keep render loop alive during batch.
        if self.viewer.lc_batch_job.is_some() {
            window.request_animation_frame();
        }

        // --- SVG dynamic re-rasterization ---
        let svg_just_finished = self.viewer.check_svg_reraster_processing();
        if svg_just_finished {
            self.viewer.pending_svg_reraster_preload_frames = 0;
            window.request_animation_frame();
            cx.notify();
        }

        if self.viewer.pending_svg_reraster_path.is_some() {
            if !svg_just_finished {
                self.viewer.pending_svg_reraster_preload_frames += 1;
            }
            if self.viewer.pending_svg_reraster_preload_frames >= 3 {
                self.viewer.apply_pending_svg_reraster();
                cx.notify();
            } else {
                window.request_animation_frame();
            }
        }

        if self.viewer.last_zoom_pan_change.is_some() {
            if self.viewer.check_svg_reraster_needed() {
                window.request_animation_frame();
            } else {
                // Debounce timer hasn't elapsed yet, keep polling
                window.request_animation_frame();
            }
        }

        if self.viewer.is_svg_rerastering {
            window.request_animation_frame();
        }

        // If still loading or processing filters, request another render to check again
        if self.viewer.is_loading || self.viewer.is_processing_filters {
            window.request_animation_frame();
        }

        // Update viewer's viewport size from window's drawable content area
        let viewport_size = window.viewport_size();
        self.viewer.update_viewport_size(viewport_size);

        // Set preload paths for next/previous images to prime GPU cache
        // This must happen in render() so images are preloaded BEFORE navigation occurs
        // This eliminates black flashing by ensuring textures are already in GPU memory
        self.viewer.preload_paths.clear();
        if let Some(next_path) = self.app_state.next_image_path() {
            self.viewer.preload_paths.push(next_path.clone());
        }
        if let Some(prev_path) = self.app_state.previous_image_path() {
            self.viewer.preload_paths.push(prev_path.clone());
        }

        // Update animation frame if playing (GPUI's suggested pattern).
        // Block playback while LC is active unless all frames are processed.
        let should_update_animation = self
            .viewer
            .image_state
            .animation
            .as_ref()
            .map(|a| a.is_playing && a.frame_count > 0)
            .unwrap_or(false)
            && (!self.viewer.lc_enabled || self.viewer.all_frames_lc_processed());

        if should_update_animation {
            // Progressive frame caching: cache next 3 frames ahead of playback
            if let Some(ref anim_state) = self.viewer.image_state.animation {
                let current = anim_state.current_frame;
                let total = anim_state.frame_count;

                // Cache next 3 frames ahead (look-ahead caching)
                for offset in 1..=3 {
                    let frame_to_cache = (current + offset) % total;
                    self.viewer.cache_frame(frame_to_cache);
                }
            }

            if let Some(ref mut anim_state) = self.viewer.image_state.animation {
                let now = Instant::now();
                let elapsed = now.duration_since(self.last_frame_update).as_millis() as u32;

                // Get current frame duration
                let frame_duration = anim_state
                    .frame_durations
                    .get(anim_state.current_frame)
                    .copied()
                    .unwrap_or(100);

                // Advance to next frame when duration has elapsed
                if elapsed >= frame_duration {
                    let next_frame = (anim_state.current_frame + 1) % anim_state.frame_count;
                    debug_eprintln!(
                        "[ANIMATION] Advancing from frame {} to frame {}",
                        anim_state.current_frame,
                        next_frame
                    );
                    anim_state.current_frame = next_frame;
                    self.last_frame_update = now;
                }
            }

            // Request next animation frame (GPUI's pattern for continuous animation)
            window.request_animation_frame();
        }

        // Update Z-drag state based on z_key_held
        if self.z_key_held && self.viewer.z_drag_state.is_none() {
            self.viewer.z_drag_state = Some(None);
        } else if !self.z_key_held && self.viewer.z_drag_state.is_some() {
            self.viewer.z_drag_state = None;
        }

        // Update spacebar-drag state based on spacebar_held
        if self.spacebar_held && self.viewer.spacebar_drag_state.is_none() {
            self.viewer.spacebar_drag_state = Some(None);
        } else if !self.spacebar_held && self.viewer.spacebar_drag_state.is_some() {
            self.viewer.spacebar_drag_state = None;
        }

        // Calculate background color once
        let active_bg = self.settings.appearance.active_background_color();
        let bg_color = rgb(((active_bg[0] as u32) << 16)
            | ((active_bg[1] as u32) << 8)
            | (active_bg[2] as u32));

        // Main content area (takes remaining space after menu bar)
        let content = div()
            .flex_1()
            .min_h_0() // Allow shrinking below content size
            .bg(bg_color)
            .when(self.drag_over, |div| {
                // Show highlighted border when dragging files over the window
                div.border_4().border_color(gpui::rgb(0x50fa7b)) // Green highlight
            })
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, event: &MouseDownEvent, _window, cx| {
                    this.mouse_button_down = true;

                    // Close menu bar when clicking on main content (Windows/Linux)
                    #[cfg(not(target_os = "macos"))]
                    this.menu_bar.update(cx, |mb, cx| mb.close_menu(cx));

                    // Start spacebar-drag pan if spacebar is being held
                    if this.viewer.spacebar_drag_state.is_some() {
                        let x: f32 = event.position.x.into();
                        let y: f32 = event.position.y.into();
                        // Store: (last_x, last_y) for 1:1 pixel movement
                        this.viewer.spacebar_drag_state = Some(Some((x, y)));
                        cx.notify();
                    }
                    // Start Z-drag zoom if Z key is being held (and spacebar is not)
                    else if this.viewer.z_drag_state.is_some() {
                        let y: f32 = event.position.y.into();
                        let x: f32 = event.position.x.into();
                        // Store: (last_x, last_y, center_x, center_y) for zoom centering
                        this.viewer.z_drag_state = Some(Some((x, y, x, y)));
                        cx.notify();
                    }
                }),
            )
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _event: &MouseUpEvent, _window, cx| {
                    this.mouse_button_down = false;

                    // End spacebar-drag pan (but keep spacebar state active if still held)
                    if this.viewer.spacebar_drag_state.is_some() {
                        // Save state after panning
                        this.save_current_image_state();
                        // Reset to held-but-not-dragging
                        this.viewer.spacebar_drag_state = Some(None);
                        cx.notify();
                    }
                    // End Z-drag zoom (but keep Z key state active if still held)
                    else if this.viewer.z_drag_state.is_some() {
                        // Reset to held-but-not-dragging
                        this.viewer.z_drag_state = Some(None);
                        cx.notify();
                    }
                }),
            )
            .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _window, cx| {
                // Check if mouse button is actually pressed (safety check for button released outside window)
                let button_actually_pressed = event.pressed_button.is_some();

                // If we think the button is down but the event says it's not, correct our state
                if this.mouse_button_down && !button_actually_pressed {
                    this.mouse_button_down = false;
                    // End spacebar-drag pan if active
                    if this.viewer.spacebar_drag_state.is_some() {
                        this.viewer.spacebar_drag_state = Some(None);
                    }
                    // End Z-drag zoom if active
                    if this.viewer.z_drag_state.is_some() {
                        this.viewer.z_drag_state = Some(None);
                    }
                }

                // Handle spacebar-drag pan (only if mouse button is down and we have valid drag data)
                if this.mouse_button_down && button_actually_pressed {
                    if let Some(Some((last_x, last_y))) = this.viewer.spacebar_drag_state {
                        let current_x: f32 = event.position.x.into();
                        let current_y: f32 = event.position.y.into();

                        // Calculate 1:1 pixel movement delta
                        let delta_x = current_x - last_x;
                        let delta_y = current_y - last_y;

                        // Apply pan directly (1:1 pixel movement)
                        this.viewer.pan(delta_x, delta_y);

                        // Update last position for next delta calculation
                        this.viewer.spacebar_drag_state = Some(Some((current_x, current_y)));

                        this.viewer.notify_svg_zoom_pan_changed();
                        cx.notify();
                        return; // Don't process Z-drag if we're spacebar-dragging
                    }
                }

                // Handle Z-drag zoom (only if mouse button is down and we have valid drag data)
                if this.mouse_button_down && button_actually_pressed {
                    if let Some(Some((last_x, last_y, center_x, center_y))) =
                        this.viewer.z_drag_state
                    {
                        let current_y: f32 = event.position.y.into();
                        let current_x: f32 = event.position.x.into();

                        // Calculate INCREMENTAL delta from LAST position (not initial)
                        let delta_y = last_y - current_y; // Up is positive (zoom in)
                        let delta_x = current_x - last_x; // Right is positive (zoom in)
                        let combined_delta = delta_y + delta_x;

                        // Get the current zoom level (which changes during drag)
                        let current_zoom = this.viewer.image_state.zoom;

                        // Scale zoom change proportionally to CURRENT zoom level
                        let sensitivity = this.settings.keyboard_mouse.z_drag_sensitivity;
                        let zoom_change = combined_delta * sensitivity * current_zoom;
                        let new_zoom = utils::zoom::clamp_zoom(current_zoom + zoom_change);

                        // Apply zoom centered on initial click position
                        let old_zoom = this.viewer.image_state.zoom;
                        this.viewer.image_state.zoom = new_zoom;

                        // Adjust pan to keep the click position at the same location
                        let (pan_x, pan_y) = this.viewer.image_state.pan;
                        let cursor_in_image_x = (center_x - pan_x) / old_zoom;
                        let cursor_in_image_y = (center_y - pan_y) / old_zoom;
                        let new_pan_x = center_x - cursor_in_image_x * new_zoom;
                        let new_pan_y = center_y - cursor_in_image_y * new_zoom;

                        this.viewer.image_state.pan = (new_pan_x, new_pan_y);
                        this.viewer.image_state.is_fit_to_window = false;

                        // Update last position for next delta calculation
                        this.viewer.z_drag_state =
                            Some(Some((current_x, current_y, center_x, center_y)));

                        this.viewer.notify_svg_zoom_pan_changed();
                        cx.notify();
                    }
                }
            }))
            .on_scroll_wheel(cx.listener(|this, event: &ScrollWheelEvent, _window, cx| {
                // Only handle scroll if Ctrl/Cmd is held
                // Use 'platform' field which is Cmd on macOS, Ctrl on other platforms
                if event.modifiers.platform {
                    // Get scroll delta in pixels (use window line height for conversion if needed)
                    let line_height = px(16.0); // Standard line height
                    let delta_y: f32 = event.delta.pixel_delta(line_height).y.into();

                    // Positive delta_y means scrolling down (zoom out)
                    // Negative delta_y means scrolling up (zoom in)
                    let zoom_in = delta_y < 0.0;

                    // Get cursor position relative to the viewport
                    let cursor_x: f32 = event.position.x.into();
                    let cursor_y: f32 = event.position.y.into();

                    // Use scroll wheel sensitivity from settings
                    let zoom_step = this.settings.keyboard_mouse.scroll_wheel_sensitivity;
                    this.viewer
                        .zoom_toward_point(cursor_x, cursor_y, zoom_in, zoom_step);
                    this.save_current_image_state();
                    cx.notify();
                }
            }))
            .on_drag_move(cx.listener(
                |this, _event: &DragMoveEvent<ExternalPaths>, _window, cx| {
                    // Set drag-over state to show visual feedback
                    if !this.drag_over {
                        this.drag_over = true;
                        cx.notify();
                    }
                },
            ))
            .on_drop(cx.listener(|this, paths: &ExternalPaths, window, cx| {
                // Clear drag-over state
                this.drag_over = false;
                this.handle_dropped_files(paths, window, cx);
            }))
            .child(self.viewer.render_view(
                active_bg,
                self.settings.appearance.overlay_transparency,
                self.settings.appearance.font_size_scale,
                self.show_zoom_indicator,
                cx,
            ))
            // Render overlays on top with proper z-order
            .when(self.show_help, |el| el.child(self.help_overlay.clone()))
            .when(self.show_debug, |el| {
                let image_dimensions = self
                    .viewer
                    .current_image
                    .as_ref()
                    .map(|img| (img.width, img.height));
                self.debug_overlay.update(cx, |overlay, _cx| {
                    overlay.update_config(DebugOverlayConfig {
                        current_path: self.app_state.current_image().cloned(),
                        current_index: self.app_state.current_index,
                        total_images: self.app_state.image_paths.len(),
                        image_state: self.viewer.image_state.clone(),
                        image_dimensions,
                        viewport_size: self.viewer.viewport_size,
                        sort_mode: self.app_state.sort_mode,
                        overlay_transparency: self.settings.appearance.overlay_transparency,
                        font_size_scale: self.settings.appearance.font_size_scale,
                    });
                });
                el.child(self.debug_overlay.clone())
            })
            .when(self.show_settings, |el| {
                el.child(self.settings_window.clone())
            })
            // Delete confirmation card at bottom-center
            .when_some(self.pending_delete, |el, mode| {
                let current_path = self.app_state.current_image().cloned();
                let filename = current_path
                    .as_ref()
                    .and_then(|p| p.file_name())
                    .and_then(|n| n.to_str())
                    .unwrap_or("file")
                    .to_string();
                let full_path = current_path
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_default();
                let button_label = match mode {
                    DeleteMode::Trash => "Delete",
                    DeleteMode::Permanent => "Permanently Delete",
                };
                el.child(
                    div()
                        .absolute()
                        .bottom(px(48.0))
                        .w_full()
                        .flex()
                        .justify_center()
                        .child(
                            // Card background
                            div()
                                .bg(rgba(0x1e1e1eee))
                                .border_1()
                                .border_color(rgba(0xff555599))
                                .rounded(px(10.0))
                                .px(px(20.0))
                                .py(px(16.0))
                                .shadow_lg()
                                .max_w(px(500.0))
                                .flex()
                                .flex_col()
                                .items_center()
                                .gap(px(10.0))
                                // Filename
                                .child(
                                    div()
                                        .text_color(rgb(0xffffff))
                                        .text_size(px(14.0))
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_align(TextAlign::Center)
                                        .child(filename),
                                )
                                // Full path
                                .child(
                                    div()
                                        .text_color(rgb(0x888888))
                                        .text_size(px(11.0))
                                        .text_align(TextAlign::Center)
                                        .max_w(px(460.0))
                                        .overflow_x_hidden()
                                        .text_ellipsis()
                                        .child(full_path),
                                )
                                // Delete button
                                .child(
                                    div()
                                        .id("delete-confirm-btn")
                                        .cursor_pointer()
                                        .bg(rgba(0xff5555ff))
                                        .hover(|s| s.bg(rgba(0xff3333ff)))
                                        .rounded(px(6.0))
                                        .px(px(24.0))
                                        .py(px(8.0))
                                        .text_color(rgb(0xffffff))
                                        .font_weight(FontWeight::BOLD)
                                        .text_size(px(13.0))
                                        .child(button_label)
                                        .on_mouse_down(
                                            MouseButton::Left,
                                            cx.listener(
                                                |this, _event: &MouseDownEvent, window, cx| {
                                                    this.handle_confirm_delete(window, cx);
                                                },
                                            ),
                                        ),
                                )
                                // Esc hint
                                .child(
                                    div()
                                        .text_color(rgb(0x666666))
                                        .text_size(px(11.0))
                                        .child("Press Esc to cancel"),
                                ),
                        ),
                )
            })
            // Toast notification at bottom-center (near delete card position)
            .when_some(self.toast.clone(), |el, toast| {
                let border_color = if toast.is_error {
                    rgba(0xff5555ff)
                } else {
                    rgba(0x50fa7bff)
                };
                let mut toast_el = div()
                    .bg(rgba(0x1e1e1eee))
                    .border_1()
                    .border_color(border_color)
                    .rounded(px(8.0))
                    .px(px(16.0))
                    .py(px(10.0))
                    .shadow_lg()
                    .max_w(px(500.0))
                    .child(
                        div()
                            .text_color(rgb(0xffffff))
                            .text_size(px(13.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(toast.message),
                    );
                if let Some(detail) = toast.detail {
                    toast_el = toast_el.child(
                        div()
                            .text_color(rgb(0xaaaaaa))
                            .text_size(px(11.0))
                            .mt(px(2.0))
                            .overflow_x_hidden()
                            .text_ellipsis()
                            .child(detail),
                    );
                }
                el.child(
                    div()
                        .absolute()
                        .bottom(px(48.0))
                        .w_full()
                        .flex()
                        .justify_center()
                        .child(toast_el),
                )
            });

        // Outer container with menu bar (Windows/Linux) and content
        // Action handlers are registered here so they're available for menu items
        div()
            .track_focus(&self.focus_handle)
            .focus(|s| s)
            // Enable ImageViewer key context (for arrow key navigation) only when no modal is open
            .when(!self.show_settings, |div| div.key_context("ImageViewer"))
            .size_full()
            .flex()
            .flex_col()
            .bg(bg_color)
            // Add menu bar for Windows/Linux
            .when(cfg!(not(target_os = "macos")), |el| {
                #[cfg(not(target_os = "macos"))]
                {
                    el.child(self.menu_bar.clone())
                }
                #[cfg(target_os = "macos")]
                {
                    el
                }
            })
            .child(content)
            // Key handlers for Space/Z drag modes - must be on focused element
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _window, cx| {
                // Don't process keyboard events if modal overlays are open
                if this.is_modal_open() {
                    return;
                }

                // Check for spacebar press (without modifiers)
                if event.keystroke.key.as_str() == "space"
                    && !event.keystroke.modifiers.shift
                    && !event.keystroke.modifiers.control
                    && !event.keystroke.modifiers.platform
                    && !event.keystroke.modifiers.alt
                {
                    // Enable spacebar-drag pan mode
                    this.spacebar_held = true;
                    cx.notify();
                }
                // Check for Z key press (without modifiers)
                else if event.keystroke.key.as_str() == "z"
                    && !event.keystroke.modifiers.shift
                    && !event.keystroke.modifiers.control
                    && !event.keystroke.modifiers.platform
                    && !event.keystroke.modifiers.alt
                {
                    // Enable Z-drag zoom mode
                    this.z_key_held = true;
                    cx.notify();
                }
            }))
            .on_key_up(cx.listener(|this, event: &KeyUpEvent, _window, cx| {
                // Check for spacebar release
                if event.keystroke.key.as_str() == "space" {
                    // Disable spacebar-drag pan mode and save state
                    if this.spacebar_held {
                        this.spacebar_held = false;
                        this.save_current_image_state();
                        cx.notify();
                    }
                }
                // Check for Z key release
                else if event.keystroke.key.as_str() == "z" {
                    // Disable Z-drag zoom mode and save state
                    if this.z_key_held {
                        this.z_key_held = false;
                        this.save_current_image_state();
                        cx.notify();
                    }
                }
            }))
            // Action handlers - registered on focused element so menu items work
            .on_action(|_: &CloseWindow, window, _| {
                window.remove_window();
            })
            .on_action(cx.listener(|this, _: &EscapePressed, window, cx| {
                this.handle_escape(window, cx);
            }))
            .on_action(cx.listener(|this, _: &NextImage, window, cx| {
                this.handle_next_image(window, cx);
            }))
            .on_action(cx.listener(|this, _: &PreviousImage, window, cx| {
                this.handle_previous_image(window, cx);
            }))
            .on_action(
                cx.listener(|this, _: &ToggleAnimationPlayPause, window, cx| {
                    this.handle_toggle_animation(window, cx);
                }),
            )
            .on_action(cx.listener(|this, _: &NextFrame, window, cx| {
                this.handle_next_frame(window, cx);
            }))
            .on_action(cx.listener(|this, _: &PreviousFrame, window, cx| {
                this.handle_previous_frame(window, cx);
            }))
            .on_action(cx.listener(|this, _: &SortAlphabetical, window, cx| {
                this.handle_sort_alphabetical(window, cx);
            }))
            .on_action(cx.listener(|this, _: &SortByModified, window, cx| {
                this.handle_sort_by_modified(window, cx);
            }))
            .on_action(cx.listener(|this, _: &SortByTypeToggle, window, cx| {
                this.handle_sort_by_type_toggle(window, cx);
            }))
            .on_action(cx.listener(|this, _: &ToggleLocalContrast, window, cx| {
                this.handle_toggle_local_contrast(window, cx);
            }))
            .on_action(cx.listener(|this, _: &ResetLocalContrast, window, cx| {
                this.handle_reset_local_contrast(window, cx);
            }))
            .on_action(cx.listener(|this, _: &ZoomIn, window, cx| {
                this.handle_zoom_in(window, cx);
            }))
            .on_action(cx.listener(|this, _: &ZoomOut, window, cx| {
                this.handle_zoom_out(window, cx);
            }))
            .on_action(cx.listener(|this, _: &ZoomReset, window, cx| {
                this.handle_zoom_reset(window, cx);
            }))
            .on_action(cx.listener(|this, _: &ZoomResetAndCenter, window, cx| {
                this.handle_zoom_reset_and_center(window, cx);
            }))
            .on_action(cx.listener(|this, _: &ZoomInFast, window, cx| {
                this.handle_zoom_in_fast(window, cx);
            }))
            .on_action(cx.listener(|this, _: &ZoomOutFast, window, cx| {
                this.handle_zoom_out_fast(window, cx);
            }))
            .on_action(cx.listener(|this, _: &ZoomInSlow, window, cx| {
                this.handle_zoom_in_slow(window, cx);
            }))
            .on_action(cx.listener(|this, _: &ZoomOutSlow, window, cx| {
                this.handle_zoom_out_slow(window, cx);
            }))
            .on_action(cx.listener(|this, _: &ZoomInIncremental, window, cx| {
                this.handle_zoom_in_incremental(window, cx);
            }))
            .on_action(cx.listener(|this, _: &ZoomOutIncremental, window, cx| {
                this.handle_zoom_out_incremental(window, cx);
            }))
            .on_action(cx.listener(|this, _: &PanUp, window, cx| {
                this.handle_pan_up(window, cx);
            }))
            .on_action(cx.listener(|this, _: &PanDown, window, cx| {
                this.handle_pan_down(window, cx);
            }))
            .on_action(cx.listener(|this, _: &PanLeft, window, cx| {
                this.handle_pan_left(window, cx);
            }))
            .on_action(cx.listener(|this, _: &PanRight, window, cx| {
                this.handle_pan_right(window, cx);
            }))
            .on_action(cx.listener(|this, _: &PanUpFast, window, cx| {
                this.handle_pan_up_fast(window, cx);
            }))
            .on_action(cx.listener(|this, _: &PanDownFast, window, cx| {
                this.handle_pan_down_fast(window, cx);
            }))
            .on_action(cx.listener(|this, _: &PanLeftFast, window, cx| {
                this.handle_pan_left_fast(window, cx);
            }))
            .on_action(cx.listener(|this, _: &PanRightFast, window, cx| {
                this.handle_pan_right_fast(window, cx);
            }))
            .on_action(cx.listener(|this, _: &PanUpSlow, window, cx| {
                this.handle_pan_up_slow(window, cx);
            }))
            .on_action(cx.listener(|this, _: &PanDownSlow, window, cx| {
                this.handle_pan_down_slow(window, cx);
            }))
            .on_action(cx.listener(|this, _: &PanLeftSlow, window, cx| {
                this.handle_pan_left_slow(window, cx);
            }))
            .on_action(cx.listener(|this, _: &PanRightSlow, window, cx| {
                this.handle_pan_right_slow(window, cx);
            }))
            .on_action(cx.listener(|this, _: &ToggleHelp, window, cx| {
                this.handle_toggle_help(window, cx);
            }))
            .on_action(cx.listener(|this, _: &ToggleDebug, window, cx| {
                this.handle_toggle_debug(window, cx);
            }))
            .on_action(cx.listener(|this, _: &ToggleZoomIndicator, window, cx| {
                this.handle_toggle_zoom_indicator(window, cx);
            }))
            .on_action(cx.listener(|this, _: &ToggleBackground, window, cx| {
                this.handle_toggle_background(window, cx);
            }))
            .on_action(cx.listener(|this, _: &ToggleSettings, window, cx| {
                this.handle_toggle_settings(window, cx);
            }))
            .on_action(cx.listener(|this, _: &CloseSettings, window, cx| {
                this.handle_close_settings(window, cx);
            }))
            .on_action(
                cx.listener(|this, _: &ResetSettingsToDefaults, window, cx| {
                    this.handle_reset_settings_to_defaults(window, cx);
                }),
            )
            .on_action(
                cx.listener(|this, _: &rpview::LoadOversizedImageAnyway, window, cx| {
                    this.handle_load_oversized_image_anyway(window, cx);
                }),
            )
            .on_action(cx.listener(|this, _: &ToggleFilters, window, cx| {
                this.handle_toggle_filters(window, cx);
            }))
            .on_action(cx.listener(|this, _: &DisableFilters, window, cx| {
                this.handle_disable_filters(window, cx);
            }))
            .on_action(cx.listener(|this, _: &EnableFilters, window, cx| {
                this.handle_enable_filters(window, cx);
            }))
            .on_action(cx.listener(|this, _: &RecallSlot3, window, cx| {
                this.handle_recall_slot(3, window, cx);
            }))
            .on_action(cx.listener(|this, _: &RecallSlot4, window, cx| {
                this.handle_recall_slot(4, window, cx);
            }))
            .on_action(cx.listener(|this, _: &RecallSlot5, window, cx| {
                this.handle_recall_slot(5, window, cx);
            }))
            .on_action(cx.listener(|this, _: &RecallSlot6, window, cx| {
                this.handle_recall_slot(6, window, cx);
            }))
            .on_action(cx.listener(|this, _: &RecallSlot7, window, cx| {
                this.handle_recall_slot(7, window, cx);
            }))
            .on_action(cx.listener(|this, _: &RecallSlot8, window, cx| {
                this.handle_recall_slot(8, window, cx);
            }))
            .on_action(cx.listener(|this, _: &RecallSlot9, window, cx| {
                this.handle_recall_slot(9, window, cx);
            }))
            .on_action(cx.listener(|this, _: &StoreSlot3, window, cx| {
                this.handle_store_slot(3, window, cx);
            }))
            .on_action(cx.listener(|this, _: &StoreSlot4, window, cx| {
                this.handle_store_slot(4, window, cx);
            }))
            .on_action(cx.listener(|this, _: &StoreSlot5, window, cx| {
                this.handle_store_slot(5, window, cx);
            }))
            .on_action(cx.listener(|this, _: &StoreSlot6, window, cx| {
                this.handle_store_slot(6, window, cx);
            }))
            .on_action(cx.listener(|this, _: &StoreSlot7, window, cx| {
                this.handle_store_slot(7, window, cx);
            }))
            .on_action(cx.listener(|this, _: &StoreSlot8, window, cx| {
                this.handle_store_slot(8, window, cx);
            }))
            .on_action(cx.listener(|this, _: &StoreSlot9, window, cx| {
                this.handle_store_slot(9, window, cx);
            }))
            .on_action(cx.listener(|this, _: &ResetFilters, window, cx| {
                this.handle_reset_filters(window, cx);
            }))
            .on_action(cx.listener(|this, _: &BrightnessUp, window, cx| {
                this.handle_brightness_up(window, cx);
            }))
            .on_action(cx.listener(|this, _: &BrightnessDown, window, cx| {
                this.handle_brightness_down(window, cx);
            }))
            .on_action(cx.listener(|this, _: &ContrastUp, window, cx| {
                this.handle_contrast_up(window, cx);
            }))
            .on_action(cx.listener(|this, _: &ContrastDown, window, cx| {
                this.handle_contrast_down(window, cx);
            }))
            .on_action(cx.listener(|this, _: &GammaUp, window, cx| {
                this.handle_gamma_up(window, cx);
            }))
            .on_action(cx.listener(|this, _: &GammaDown, window, cx| {
                this.handle_gamma_down(window, cx);
            }))
            .on_action(cx.listener(|this, _: &OpenFile, window, cx| {
                this.handle_open_file(window, cx);
            }))
            .on_action(cx.listener(|this, _: &SaveFile, window, cx| {
                this.handle_save_file(window, cx);
            }))
            .on_action(cx.listener(|this, _: &SaveFileToDownloads, window, cx| {
                this.handle_save_file_to_downloads(window, cx);
            }))
            .on_action(cx.listener(|this, _: &OpenInExternalViewer, window, cx| {
                this.handle_open_in_external_viewer(window, cx);
            }))
            .on_action(
                cx.listener(|this, _: &OpenInExternalViewerAndQuit, window, cx| {
                    this.handle_open_in_external_viewer_and_quit(window, cx);
                }),
            )
            .on_action(cx.listener(|this, _: &OpenInExternalEditor, window, cx| {
                this.handle_open_in_external_editor(window, cx);
            }))
            .on_action(cx.listener(|this, _: &RevealInFinder, window, cx| {
                this.handle_reveal_in_finder(window, cx);
            }))
            .on_action(cx.listener(|this, _: &RequestDelete, window, cx| {
                this.handle_request_delete(window, cx);
            }))
            .on_action(cx.listener(|this, _: &RequestPermanentDelete, window, cx| {
                this.handle_request_permanent_delete(window, cx);
            }))
            .on_action(cx.listener(|this, _: &ConfirmDelete, window, cx| {
                this.handle_confirm_delete(window, cx);
            }))
    }
}
