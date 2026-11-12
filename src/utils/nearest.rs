//! Nearest-neighbor magnification for the viewer.
//!
//! GPUI 0.2.2 samples every image through a hard-coded bilinear sampler
//! (`atlas_texture_sampler` in its `shaders.metal`), so a magnified image
//! blurs and there is no sampler to switch.  To show hard pixel edges, the
//! viewer overlays the visible part of the image already magnified on the
//! CPU to screen resolution: one texel per device pixel, which the bilinear
//! sampler then draws unchanged.  This module holds the geometry and the
//! pixel replication; `ImageViewer` owns the overlay's lifecycle.

/// Extra area rendered beyond each edge of the viewport, as a fraction of
/// the viewport's size, so a short pan stays covered before the next render.
pub const PAD_FRACTION: f32 = 0.125;

/// Upper bound on an overlay's size in device pixels (~ 4 bytes each).
/// Viewport-sized overlays sit far below it; it guards against a bogus
/// scale factor or viewport turning one render into a huge allocation.
pub const MAX_OVERLAY_PIXELS: u64 = 64 * 1024 * 1024;

/// The view an overlay is rendered for.  All lengths are logical pixels
/// except where named otherwise.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct View {
    /// Displayed image size before zoom (the size the layout scales by `zoom`).
    pub display_size: (u32, u32),
    pub zoom: f32,
    /// Position of the image's top-left corner within the viewer.
    pub pan: (f32, f32),
    pub viewport: (f32, f32),
    /// Device pixels per logical pixel.
    pub scale_factor: f32,
}

/// A rectangle in device pixels, relative to the viewer's top-left corner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeviceRect {
    pub x: i64,
    pub y: i64,
    pub width: u32,
    pub height: u32,
}

/// Whether the view magnifies the image (more than one device pixel per
/// image pixel), the only case where nearest-neighbor differs visibly from
/// bilinear.
pub fn magnifies(view: &View) -> bool {
    view.zoom * view.scale_factor > 1.0 + 1e-3
}

/// The device-pixel rectangle the overlay should cover: the part of the
/// image inside the viewport grown by `pad` (a fraction of the viewport per
/// edge).  `None` when the image is entirely off screen or the overlay
/// would exceed `MAX_OVERLAY_PIXELS`.
pub fn plan_rect(view: &View, pad: f32) -> Option<DeviceRect> {
    let (pan_x, pan_y) = view.pan;
    let (vp_w, vp_h) = view.viewport;
    let img_w = view.display_size.0 as f32 * view.zoom;
    let img_h = view.display_size.1 as f32 * view.zoom;
    let left = pan_x.max(-vp_w * pad);
    let right = (pan_x + img_w).min(vp_w * (1.0 + pad));
    let top = pan_y.max(-vp_h * pad);
    let bottom = (pan_y + img_h).min(vp_h * (1.0 + pad));
    if right <= left || bottom <= top {
        return None;
    }
    let s = view.scale_factor;
    let x0 = (left * s).floor() as i64;
    let x1 = (right * s).ceil() as i64;
    let y0 = (top * s).floor() as i64;
    let y1 = (bottom * s).ceil() as i64;
    let (width, height) = ((x1 - x0) as u64, (y1 - y0) as u64);
    if width == 0 || height == 0 || width * height > MAX_OVERLAY_PIXELS {
        return None;
    }
    Some(DeviceRect {
        x: x0,
        y: y0,
        width: width as u32,
        height: height as u32,
    })
}

/// Pixel layout of a source buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layout {
    Rgba,
    Bgra,
}

/// For each device pixel along one axis of `rect`, the source pixel it
/// shows, or `None` where it falls outside the image.
fn axis_map(
    start: i64,
    len: u32,
    pan: f32,
    zoom: f32,
    scale_factor: f32,
    display_len: u32,
    source_len: u32,
) -> Vec<Option<u32>> {
    let source_per_display = source_len as f32 / display_len as f32;
    (0..len as i64)
        .map(|i| {
            // Centre of the device pixel, in display-image coordinates.
            let logical = (start + i) as f32 / scale_factor + 0.5 / scale_factor;
            let u = (logical - pan) / zoom;
            if u < 0.0 || u >= display_len as f32 {
                return None;
            }
            Some(((u * source_per_display) as u32).min(source_len - 1))
        })
        .collect()
}

/// Render `rect` of the view by nearest-neighbor replication of `source`
/// (`width * height * 4` bytes in `layout`).  Returns BGRA bytes, as GPUI's
/// `RenderImage` wants, `rect.width * rect.height * 4` long; device pixels
/// outside the image are transparent.
pub fn render(
    source: &[u8],
    source_size: (u32, u32),
    layout: Layout,
    view: &View,
    rect: DeviceRect,
) -> Vec<u8> {
    let (src_w, src_h) = source_size;
    let out_w = rect.width as usize;
    let mut out = vec![0u8; out_w * rect.height as usize * 4];
    if src_w == 0 || src_h == 0 || source.len() < src_w as usize * src_h as usize * 4 {
        return out;
    }
    let cols = axis_map(
        rect.x,
        rect.width,
        view.pan.0,
        view.zoom,
        view.scale_factor,
        view.display_size.0,
        src_w,
    );
    let rows = axis_map(
        rect.y,
        rect.height,
        view.pan.1,
        view.zoom,
        view.scale_factor,
        view.display_size.1,
        src_h,
    );
    let row_bytes = out_w * 4;
    let mut previous: Option<(u32, usize)> = None;
    for (oy, src_row) in rows.iter().enumerate() {
        let Some(sy) = *src_row else { continue };
        let dst_start = oy * row_bytes;
        // Rows showing the same source row are identical: copy the last one.
        if let Some((prev_sy, prev_start)) = previous
            && prev_sy == sy
        {
            out.copy_within(prev_start..prev_start + row_bytes, dst_start);
            continue;
        }
        let src_row_start = sy as usize * src_w as usize * 4;
        let dst = &mut out[dst_start..dst_start + row_bytes];
        for (px, src_col) in dst.chunks_exact_mut(4).zip(&cols) {
            let Some(sx) = *src_col else { continue };
            let i = src_row_start + sx as usize * 4;
            let p = &source[i..i + 4];
            match layout {
                Layout::Bgra => px.copy_from_slice(p),
                Layout::Rgba => px.copy_from_slice(&[p[2], p[1], p[0], p[3]]),
            }
        }
        previous = Some((sy, dst_start));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const RED: [u8; 4] = [255, 0, 0, 255];
    const BLUE: [u8; 4] = [0, 0, 255, 255];
    const BGRA_RED: [u8; 4] = [0, 0, 255, 255];
    const BGRA_BLUE: [u8; 4] = [255, 0, 0, 255];
    const CLEAR: [u8; 4] = [0, 0, 0, 0];

    /// A 2×1 RGBA image: red, then blue.
    fn red_blue() -> Vec<u8> {
        [RED, BLUE].concat()
    }

    fn view(zoom: f32, pan: (f32, f32), viewport: (f32, f32), scale: f32) -> View {
        View {
            display_size: (2, 1),
            zoom,
            pan,
            viewport,
            scale_factor: scale,
        }
    }

    fn pixel(buf: &[u8], width: u32, x: u32, y: u32) -> [u8; 4] {
        let i = ((y * width + x) * 4) as usize;
        buf[i..i + 4].try_into().unwrap()
    }

    #[test]
    fn magnifies_only_above_one_device_pixel_per_image_pixel() {
        assert!(!magnifies(&view(1.0, (0.0, 0.0), (10.0, 10.0), 1.0)));
        assert!(magnifies(&view(1.0, (0.0, 0.0), (10.0, 10.0), 2.0)));
        assert!(magnifies(&view(4.0, (0.0, 0.0), (10.0, 10.0), 1.0)));
        assert!(!magnifies(&view(0.5, (0.0, 0.0), (10.0, 10.0), 2.0)));
    }

    #[test]
    fn plan_rect_clips_to_image_inside_viewport() {
        // Image 8×4 logical at pan (1,1), well inside a 100×100 viewport.
        let r = plan_rect(&view(4.0, (1.0, 1.0), (100.0, 100.0), 2.0), 0.0).unwrap();
        assert_eq!(
            r,
            DeviceRect {
                x: 2,
                y: 2,
                width: 16,
                height: 8
            }
        );
    }

    #[test]
    fn plan_rect_clips_to_padded_viewport() {
        // Image 2000 logical wide, viewport 100 wide, pad 10% → -10..110.
        let r = plan_rect(&view(1000.0, (-500.0, 0.0), (100.0, 100.0), 1.0), 0.1).unwrap();
        assert_eq!((r.x, r.width), (-10, 120));
    }

    #[test]
    fn plan_rect_none_when_off_screen() {
        assert_eq!(
            plan_rect(&view(1.0, (500.0, 0.0), (100.0, 100.0), 1.0), 0.0),
            None
        );
    }

    #[test]
    fn render_replicates_each_source_pixel_into_a_hard_edged_block() {
        // Zoom 3 at scale 1: red covers device columns 0..3, blue 3..6.
        let v = view(3.0, (0.0, 0.0), (100.0, 100.0), 1.0);
        let rect = plan_rect(&v, 0.0).unwrap();
        let out = render(&red_blue(), (2, 1), Layout::Rgba, &v, rect);
        assert_eq!((rect.width, rect.height), (6, 3));
        for y in 0..3 {
            for x in 0..3 {
                assert_eq!(pixel(&out, 6, x, y), BGRA_RED, "({x},{y})");
                assert_eq!(pixel(&out, 6, x + 3, y), BGRA_BLUE, "({},{y})", x + 3);
            }
        }
    }

    #[test]
    fn render_accounts_for_scale_factor_and_pan() {
        // Zoom 2, scale 2 → 4 device px per source px; pan 1.5 logical = 3
        // device px, so the image starts at device column 3.
        let v = view(2.0, (1.5, 0.0), (100.0, 100.0), 2.0);
        let rect = DeviceRect {
            x: 0,
            y: 0,
            width: 12,
            height: 1,
        };
        let out = render(&red_blue(), (2, 1), Layout::Rgba, &v, rect);
        let row: Vec<[u8; 4]> = (0..12).map(|x| pixel(&out, 12, x, 0)).collect();
        let mut expected = vec![CLEAR; 3];
        expected.extend([BGRA_RED; 4]);
        expected.extend([BGRA_BLUE; 4]);
        expected.push(CLEAR);
        assert_eq!(row, expected);
    }

    #[test]
    fn render_passes_bgra_through_unchanged() {
        let bgra = [BGRA_RED, BGRA_BLUE].concat();
        let v = view(2.0, (0.0, 0.0), (100.0, 100.0), 1.0);
        let rect = plan_rect(&v, 0.0).unwrap();
        let out = render(&bgra, (2, 1), Layout::Bgra, &v, rect);
        assert_eq!(pixel(&out, 4, 1, 0), BGRA_RED);
        assert_eq!(pixel(&out, 4, 2, 0), BGRA_BLUE);
    }

    #[test]
    fn render_maps_display_to_a_larger_source() {
        // Source 4×1 shown as a 2×1 display image: display px 0 → source 0.
        let src = [RED, RED, BLUE, BLUE].concat();
        let v = view(4.0, (0.0, 0.0), (100.0, 100.0), 1.0);
        let rect = plan_rect(&v, 0.0).unwrap();
        let out = render(&src, (4, 1), Layout::Rgba, &v, rect);
        assert_eq!(pixel(&out, 8, 0, 0), BGRA_RED);
        assert_eq!(pixel(&out, 8, 7, 0), BGRA_BLUE);
    }

    #[test]
    fn render_short_source_yields_transparent_not_panic() {
        let v = view(2.0, (0.0, 0.0), (100.0, 100.0), 1.0);
        let rect = plan_rect(&v, 0.0).unwrap();
        let out = render(&[0u8; 3], (2, 1), Layout::Rgba, &v, rect);
        assert!(out.iter().all(|&b| b == 0));
    }
}
