use gpui::*;

/// Get the modifier key name for the current platform
/// Returns "Cmd" on macOS, "Ctrl" on Windows/Linux
pub fn modifier_key() -> &'static str {
    if cfg!(target_os = "macos") {
        "Cmd"
    } else {
        "Ctrl"
    }
}

/// Format a keyboard shortcut for the current platform
/// Example: format_shortcut("O") returns "Cmd+O" on macOS, "Ctrl+O" on Windows/Linux
pub fn format_shortcut(key: &str) -> String {
    format!("{}+{}", modifier_key(), key)
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
