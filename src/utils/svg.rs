use crate::error::{AppError, AppResult};
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

/// Cached font database loaded once and reused for all SVG operations.
/// System font discovery is slow (~50-100ms), so we only do it once.
static FONTDB: OnceLock<Arc<resvg::usvg::fontdb::Database>> = OnceLock::new();

fn fontdb() -> Arc<resvg::usvg::fontdb::Database> {
    FONTDB
        .get_or_init(|| {
            let mut db = resvg::usvg::fontdb::Database::new();
            db.load_system_fonts();
            eprintln!("[SVG] Loaded {} font faces from system", db.len());
            Arc::new(db)
        })
        .clone()
}

fn svg_options() -> resvg::usvg::Options<'static> {
    resvg::usvg::Options {
        fontdb: fontdb(),
        ..Default::default()
    }
}

/// Maximum total pixels for a full re-rasterization (4096x4096 = 16M pixels).
/// Beyond this, we use viewport-only rendering.
pub const MAX_FULL_RERASTER_PIXELS: u64 = 4096 * 4096;

/// Extra padding around the visible viewport for viewport-only re-renders.
/// 0.5 = 50% extra on each side, so the rendered area is 2x the viewport.
pub const VIEWPORT_PADDING_FACTOR: f32 = 0.5;

/// Region of an SVG that was rasterized for viewport-only rendering.
#[derive(Clone, Debug)]
pub struct SvgRerasterRegion {
    /// SVG-space origin X of the rendered area
    pub svg_x: f32,
    /// SVG-space origin Y of the rendered area
    pub svg_y: f32,
    /// SVG-space width of the rendered area
    pub svg_w: f32,
    /// SVG-space height of the rendered area
    pub svg_h: f32,
    /// Output pixmap width in pixels
    pub pixel_w: u32,
    /// Output pixmap height in pixels
    pub pixel_h: u32,
}

/// Generate a unique temp file path with the given prefix.
fn svg_temp_path(prefix: &str) -> Result<PathBuf, String> {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();

    let temp_dir = std::env::temp_dir()
        .canonicalize()
        .map_err(|e| format!("Failed to resolve temp dir: {}", e))?;

    Ok(temp_dir.join(format!(
        "rpview_{}_{}_{}.png",
        prefix,
        std::process::id(),
        timestamp
    )))
}

/// Parse an SVG file into a usvg::Tree. The tree is self-contained after parsing
/// (fontdb is not needed for subsequent renders).
pub fn parse_svg(path: &Path) -> AppResult<resvg::usvg::Tree> {
    let svg_data = std::fs::read(path).map_err(|e| {
        AppError::ImageLoadError(path.to_path_buf(), format!("Failed to read SVG file: {}", e))
    })?;

    let tree = resvg::usvg::Tree::from_data(&svg_data, &svg_options()).map_err(|e| {
        AppError::ImageLoadError(path.to_path_buf(), format!("Failed to parse SVG: {}", e))
    })?;

    Ok(tree)
}

/// Render an entire SVG tree at the given scale factor to a temp PNG.
/// Returns the path to the temp PNG file.
pub fn rerasterize_svg_full(
    tree: &resvg::usvg::Tree,
    scale: f32,
) -> Result<PathBuf, String> {
    let size = tree.size();
    let scaled_w = (size.width() * scale).ceil() as u32;
    let scaled_h = (size.height() * scale).ceil() as u32;

    if scaled_w == 0 || scaled_h == 0 {
        return Err("SVG has zero dimensions at this scale".to_string());
    }

    let mut pixmap = resvg::tiny_skia::Pixmap::new(scaled_w, scaled_h)
        .ok_or_else(|| format!("Failed to create pixmap ({}x{} may be too large)", scaled_w, scaled_h))?;

    let transform = resvg::tiny_skia::Transform::from_scale(scale, scale);
    resvg::render(tree, transform, &mut pixmap.as_mut());

    let temp_path = svg_temp_path("svg_reraster")?;
    pixmap
        .save_png(&temp_path)
        .map_err(|e| format!("Failed to save rasterized SVG: {}", e))?;

    eprintln!(
        "[SVG] Full re-raster at {:.1}x -> {} ({}x{})",
        scale,
        temp_path.display(),
        scaled_w,
        scaled_h
    );

    Ok(temp_path)
}

/// Render only the visible region of an SVG tree (plus padding) to a temp PNG.
/// Returns `(temp_path, region)` describing what was rendered.
///
/// `viewport_in_svg` is `(x, y, w, h)` in SVG coordinate space describing the
/// currently visible area. `padding_factor` adds extra around each side
/// (e.g. 0.5 = 50% extra). `scale` is the zoom level.
pub fn rerasterize_svg_viewport(
    tree: &resvg::usvg::Tree,
    viewport_in_svg: (f32, f32, f32, f32),
    padding_factor: f32,
    scale: f32,
) -> Result<(PathBuf, SvgRerasterRegion), String> {
    let svg_size = tree.size();
    let (vx, vy, vw, vh) = viewport_in_svg;

    // Add padding around the visible region
    let pad_x = vw * padding_factor;
    let pad_y = vh * padding_factor;

    // Clip to SVG bounds
    let region_x = (vx - pad_x).max(0.0);
    let region_y = (vy - pad_y).max(0.0);
    let region_r = (vx + vw + pad_x).min(svg_size.width());
    let region_b = (vy + vh + pad_y).min(svg_size.height());
    let region_w = region_r - region_x;
    let region_h = region_b - region_y;

    if region_w <= 0.0 || region_h <= 0.0 {
        return Err("Viewport region is empty".to_string());
    }

    let pixel_w = (region_w * scale).ceil() as u32;
    let pixel_h = (region_h * scale).ceil() as u32;

    if pixel_w == 0 || pixel_h == 0 {
        return Err("Viewport region too small at this scale".to_string());
    }

    let mut pixmap = resvg::tiny_skia::Pixmap::new(pixel_w, pixel_h)
        .ok_or_else(|| format!("Failed to create pixmap ({}x{} may be too large)", pixel_w, pixel_h))?;

    // Scale then translate so that the region origin maps to pixel (0,0)
    let transform = resvg::tiny_skia::Transform::from_scale(scale, scale)
        .pre_translate(-region_x, -region_y);
    resvg::render(tree, transform, &mut pixmap.as_mut());

    let temp_path = svg_temp_path("svg_viewport")?;
    pixmap
        .save_png(&temp_path)
        .map_err(|e| format!("Failed to save viewport raster: {}", e))?;

    let region = SvgRerasterRegion {
        svg_x: region_x,
        svg_y: region_y,
        svg_w: region_w,
        svg_h: region_h,
        pixel_w,
        pixel_h,
    };

    eprintln!(
        "[SVG] Viewport re-raster at {:.1}x, region ({:.0},{:.0} {}x{}) -> {} ({}x{})",
        scale, region_x, region_y, region_w as u32, region_h as u32,
        temp_path.display(), pixel_w, pixel_h
    );

    Ok((temp_path, region))
}

/// Get the intrinsic dimensions of an SVG file
pub fn get_svg_dimensions(path: &Path) -> AppResult<(u32, u32)> {
    let svg_data = std::fs::read(path).map_err(|e| {
        AppError::ImageLoadError(path.to_path_buf(), format!("Failed to read SVG file: {}", e))
    })?;

    let tree = resvg::usvg::Tree::from_data(&svg_data, &svg_options()).map_err(|e| {
        AppError::ImageLoadError(path.to_path_buf(), format!("Failed to parse SVG: {}", e))
    })?;

    let size = tree.size();
    let width = size.width().ceil() as u32;
    let height = size.height().ceil() as u32;

    if width == 0 || height == 0 {
        return Err(AppError::ImageLoadError(
            path.to_path_buf(),
            "SVG has zero dimensions".to_string(),
        ));
    }

    Ok((width, height))
}

/// Rasterize an SVG file to a temporary PNG at the given scale factor.
///
/// Returns `(temp_png_path, intrinsic_width, intrinsic_height)` where the
/// width/height are the *intrinsic* SVG dimensions (not the scaled raster).
#[allow(dead_code)]
pub fn rasterize_svg(path: &Path, scale_factor: f32) -> AppResult<(PathBuf, u32, u32)> {
    let tree = parse_svg(path)?;

    let size = tree.size();
    let intrinsic_w = size.width().ceil() as u32;
    let intrinsic_h = size.height().ceil() as u32;

    if intrinsic_w == 0 || intrinsic_h == 0 {
        return Err(AppError::ImageLoadError(
            path.to_path_buf(),
            "SVG has zero dimensions".to_string(),
        ));
    }

    let temp_path = rerasterize_svg_full(&tree, scale_factor).map_err(|e| {
        AppError::ImageLoadError(path.to_path_buf(), e)
    })?;

    let scaled_w = (size.width() * scale_factor).ceil() as u32;
    let scaled_h = (size.height() * scale_factor).ceil() as u32;

    eprintln!(
        "[SVG] Rasterized {} ({}x{}) at {}x to {} ({}x{})",
        path.display(),
        intrinsic_w,
        intrinsic_h,
        scale_factor,
        temp_path.display(),
        scaled_w,
        scaled_h
    );

    Ok((temp_path, intrinsic_w, intrinsic_h))
}
