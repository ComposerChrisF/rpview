use crate::error::{AppError, AppResult};
use std::path::{Path, PathBuf};

/// Get the intrinsic dimensions of an SVG file
pub fn get_svg_dimensions(path: &Path) -> AppResult<(u32, u32)> {
    let svg_data = std::fs::read(path).map_err(|e| {
        AppError::ImageLoadError(path.to_path_buf(), format!("Failed to read SVG file: {}", e))
    })?;

    let tree = resvg::usvg::Tree::from_data(&svg_data, &resvg::usvg::Options::default())
        .map_err(|e| {
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
pub fn rasterize_svg(path: &Path, scale_factor: f32) -> AppResult<(PathBuf, u32, u32)> {
    let svg_data = std::fs::read(path).map_err(|e| {
        AppError::ImageLoadError(path.to_path_buf(), format!("Failed to read SVG file: {}", e))
    })?;

    let tree = resvg::usvg::Tree::from_data(&svg_data, &resvg::usvg::Options::default())
        .map_err(|e| {
            AppError::ImageLoadError(path.to_path_buf(), format!("Failed to parse SVG: {}", e))
        })?;

    let size = tree.size();
    let intrinsic_w = size.width().ceil() as u32;
    let intrinsic_h = size.height().ceil() as u32;

    if intrinsic_w == 0 || intrinsic_h == 0 {
        return Err(AppError::ImageLoadError(
            path.to_path_buf(),
            "SVG has zero dimensions".to_string(),
        ));
    }

    let scaled_w = (size.width() * scale_factor).ceil() as u32;
    let scaled_h = (size.height() * scale_factor).ceil() as u32;

    let mut pixmap = resvg::tiny_skia::Pixmap::new(scaled_w, scaled_h).ok_or_else(|| {
        AppError::ImageLoadError(
            path.to_path_buf(),
            format!(
                "Failed to create pixmap ({}x{} may be too large)",
                scaled_w, scaled_h
            ),
        )
    })?;

    let transform =
        resvg::tiny_skia::Transform::from_scale(scale_factor as f32, scale_factor as f32);
    resvg::render(&tree, transform, &mut pixmap.as_mut());

    // Build temp file path
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();

    let temp_dir = std::env::temp_dir().canonicalize().map_err(|e| {
        AppError::ImageLoadError(
            path.to_path_buf(),
            format!("Failed to resolve temp dir: {}", e),
        )
    })?;

    let temp_path = temp_dir.join(format!(
        "rpview_svg_{}_{}.png",
        std::process::id(),
        timestamp
    ));

    pixmap.save_png(&temp_path).map_err(|e| {
        AppError::ImageLoadError(
            path.to_path_buf(),
            format!("Failed to save rasterized SVG: {}", e),
        )
    })?;

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
