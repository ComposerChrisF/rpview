use gpui::*;

/// Get the platform modifier key glyph/name.
/// Returns "⌘" on macOS, "Ctrl" on Windows/Linux.
pub fn modifier_key() -> &'static str {
    if cfg!(target_os = "macos") {
        "⌘"
    } else {
        "Ctrl"
    }
}

/// Shift modifier prefix for compound shortcuts.
/// Returns "⇧" on macOS, "Shift+" on Windows/Linux.
pub fn shift_prefix() -> &'static str {
    if cfg!(target_os = "macos") {
        "⇧"
    } else {
        "Shift+"
    }
}

/// Option/Alt modifier prefix for compound shortcuts.
/// Returns "⌥" on macOS, "Alt+" on Windows/Linux.
pub fn option_prefix() -> &'static str {
    if cfg!(target_os = "macos") {
        "⌥"
    } else {
        "Alt+"
    }
}

/// Format a keyboard shortcut for the current platform.
/// On macOS uses ⌥⇧⌘ glyphs without separators.
/// On Windows/Linux uses Ctrl+Shift+Alt+ with "+" separators.
/// The Cmd/Ctrl modifier is always included.
pub fn format_shortcut(key: &str, shift: bool, option: bool) -> String {
    if cfg!(target_os = "macos") {
        // macOS standard order: ⌃⌥⇧⌘
        let mut s = String::new();
        if option {
            s.push('⌥');
        }
        if shift {
            s.push('⇧');
        }
        s.push('⌘');
        s.push_str(key);
        s
    } else {
        let mut parts = Vec::new();
        parts.push("Ctrl");
        if shift {
            parts.push("Shift");
        }
        if option {
            parts.push("Alt");
        }
        parts.push(key);
        parts.join("+")
    }
}

/// Common color palette
pub struct Colors;

impl Colors {
    /// Background color for the main view
    pub fn background() -> Hsla {
        rgb(0x1e1e1e).into()
    }

    /// Text color
    pub fn text() -> Hsla {
        rgb(0xffffff).into()
    }

    /// Text color that contrasts with the given background RGB
    pub fn text_for_background(bg: [u8; 3]) -> Hsla {
        // Relative luminance using sRGB coefficients
        let luminance = 0.299 * bg[0] as f32 + 0.587 * bg[1] as f32 + 0.114 * bg[2] as f32;
        if luminance > 150.0 {
            rgb(0x1a1a1a).into() // dark text on light background
        } else {
            rgb(0xffffff).into() // white text on dark background
        }
    }

    /// Error text color
    pub fn error() -> Hsla {
        rgb(0xff5555).into()
    }

    #[allow(dead_code)]
    /// Success/info color
    pub fn info() -> Hsla {
        rgb(0x50fa7b).into()
    }

    #[allow(dead_code)]
    /// Overlay background (semi-transparent)
    pub fn overlay_bg() -> Hsla {
        let mut color: Hsla = rgb(0x000000).into();
        color.a = 0.85;
        color
    }

    /// Overlay background with custom alpha value
    /// alpha: 0-255, where 0 is fully transparent and 255 is fully opaque
    pub fn overlay_bg_alpha(alpha: u8) -> Hsla {
        let mut color: Hsla = rgb(0x000000).into();
        color.a = (alpha as f32) / 255.0;
        color
    }

    #[allow(dead_code)]
    /// Border color
    pub fn border() -> Hsla {
        rgb(0x444444).into()
    }
}

/// Common spacing values (in pixels)
pub struct Spacing;

impl Spacing {
    pub fn xs() -> Pixels {
        px(4.0)
    }

    pub fn sm() -> Pixels {
        px(8.0)
    }

    pub fn md() -> Pixels {
        px(16.0)
    }

    pub fn lg() -> Pixels {
        px(24.0)
    }

    pub fn xl() -> Pixels {
        px(32.0)
    }
}

/// Common text sizes
pub struct TextSize;

impl TextSize {
    pub fn sm() -> Pixels {
        px(12.0)
    }

    pub fn md() -> Pixels {
        px(14.0)
    }

    pub fn lg() -> Pixels {
        px(16.0)
    }

    pub fn xl() -> Pixels {
        px(20.0)
    }

    #[allow(dead_code)]
    pub fn xxl() -> Pixels {
        px(24.0)
    }
}

/// Apply font size scale to a base pixel size
pub fn scaled_text_size(base_size: f32, scale: f32) -> Pixels {
    px(base_size * scale)
}
