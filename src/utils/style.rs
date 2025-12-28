use gpui::*;

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

    /// Success/info color
    pub fn info() -> Hsla {
        rgb(0x50fa7b).into()
    }

    /// Overlay background (semi-transparent)
    pub fn overlay_bg() -> Hsla {
        let mut color: Hsla = rgb(0x000000).into();
        color.a = 0.85;
        color
    }

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

    pub fn xxl() -> Pixels {
        px(24.0)
    }
}
