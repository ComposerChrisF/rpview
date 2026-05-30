// Local Contrast — Document-Style Contrast pass 3: apply contrast_std followed
// by contrast_doc per pixel, around a bilinearly-interpolated local gray-point.
//
// This is the GPU port of the CPU `apply_tone_curve` document path in
// `src/utils/local_contrast.rs` (contrast_std at line 143, contrast_doc at
// line 166).  Both are reproduced verbatim — including the documented
// `255.0`-scale quirk in contrast_std's white-side branch — so the GPU result
// tracks the CPU reference.  Only OKLab L is shaped; chroma (a, b) and alpha
// pass through unchanged.
//
// The gray-point is read from the per-tile mean/median stats buffer
// (`lc_doc_stats.wgsl`) and bilinearly interpolated between the four nearest
// tile centers, using the same tile-center-space mapping as the CLAHE apply
// pass — this is what removes blocky tile boundaries.

struct Uniforms {
  nx:         u32,
  ny:         u32,
  image_w:    u32,
  image_h:    u32,
  contrast:   f32,  // contrast_std amount
  mix:        f32,  // contrast_doc mix (mix_document_contrast)
  tilt_black: f32,
  tilt_white: f32,
  flags:      u32,  // bit0 = adjust BW, bit1 = adjust transition, bit2 = use median
  _pad0:      f32,
  _pad1:      f32,
  _pad2:      f32,
}

@group(0) @binding(0) var Input: texture_2d<f32>;
@group(0) @binding(1) var Output: texture_storage_2d<rgba16float, write>;
@group(0) @binding(2) var<storage, read> stats: array<f32>;
@group(0) @binding(3) var<uniform> u: Uniforms;

fn clamp01(v: f32) -> f32 { return clamp(v, 0.0, 1.0); }

fn lerp_remap(vo0: f32, vo1: f32, vin: f32, vin0: f32, vin1: f32) -> f32 {
  return mix(vo0, vo1, (vin - vin0) / (vin1 - vin0));
}

// Standard contrast around the gray-point.  Faithful port of CPU `contrast_std`,
// quirk preserved: the white-side branch uses 255.0-scale arithmetic even though
// l_cur / l_gray are in [0, 1].
fn contrast_std(l_gray: f32, l_cur: f32, c: f32) -> f32 {
  if (l_cur < l_gray) {
    var frac = l_cur / l_gray;
    frac = frac * frac;
    let l_new = frac * l_gray;
    return c * l_new + (1.0 - c) * l_cur;
  }
  var frac = (255.0 - l_cur) / (255.0 - l_gray);
  frac = frac * frac;
  let l_new = 255.0 - frac * (255.0 - l_gray);
  return c * l_new + (1.0 - c) * l_cur;
}

// Tilt the gray-point threshold toward black (tilt < 0) or white (tilt > 0).
fn apply_tilt(l_gray: f32, tilt: f32) -> f32 {
  if (tilt <= 0.0) {
    return l_gray * (1.0 + tilt);
  }
  return 1.0 - (1.0 - l_gray) * (1.0 - tilt);
}

// Document-mode contrast.  Faithful port of CPU `contrast_doc`.
fn contrast_doc(
  l_gray: f32,
  l_cur: f32,
  c: f32,
  tilt_black: f32,
  tilt_white: f32,
  adjust_xition: bool,
  adjust_bw: bool,
) -> f32 {
  let l_black = apply_tilt(l_gray, tilt_black);
  if (l_cur < l_black) {
    if (adjust_bw) {
      return clamp01(lerp_remap(l_cur, 0.0, c, 0.0, 1.0));
    }
    return 0.0;
  }
  let l_white = apply_tilt(l_gray, tilt_white);
  if (l_cur > l_white) {
    if (adjust_bw) {
      return clamp01(mix(l_cur, 1.0, c));
    }
    return 1.0;
  }
  let l_new = lerp_remap(0.0, 1.0, l_cur, l_black, l_white);
  if (!adjust_xition) {
    return clamp01(l_new);
  }
  return clamp01(lerp_remap(l_cur, l_new, c, 0.0, 1.0));
}

// Gray-point for `tile`: mean (stats[tile*2]) or median (stats[tile*2+1]).
fn gray_at(tile: u32, use_median: bool) -> f32 {
  if (use_median) {
    return stats[tile * 2u + 1u];
  }
  return stats[tile * 2u];
}

@compute @workgroup_size(16, 16, 1)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let dims = vec2<i32>(textureDimensions(Output));
  let p = vec2<i32>(gid.xy);
  if (p.x >= dims.x || p.y >= dims.y) { return; }

  let lab = textureLoad(Input, p, 0);
  let L = clamp(lab.r, 0.0, 1.0);

  let use_median = (u.flags & 4u) != 0u;
  let adjust_bw = (u.flags & 1u) != 0u;
  let adjust_xition = (u.flags & 2u) != 0u;

  // Bilinearly interpolate the gray-point across the four nearest tiles
  // (tile-center-space mapping identical to lc_clahe_apply.wgsl).
  let gx = (f32(p.x) + 0.5) / f32(u.image_w) * f32(u.nx) - 0.5;
  let gy = (f32(p.y) + 0.5) / f32(u.image_h) * f32(u.ny) - 0.5;
  let x0f = floor(gx);
  let y0f = floor(gy);
  let fx = gx - x0f;
  let fy = gy - y0f;

  let nx1 = f32(u.nx - 1u);
  let ny1 = f32(u.ny - 1u);
  let x0 = u32(clamp(x0f, 0.0, nx1));
  let x1 = u32(clamp(x0f + 1.0, 0.0, nx1));
  let y0 = u32(clamp(y0f, 0.0, ny1));
  let y1 = u32(clamp(y0f + 1.0, 0.0, ny1));

  let g00 = gray_at(y0 * u.nx + x0, use_median);
  let g10 = gray_at(y0 * u.nx + x1, use_median);
  let g01 = gray_at(y1 * u.nx + x0, use_median);
  let g11 = gray_at(y1 * u.nx + x1, use_median);
  let gray = mix(mix(g00, g10, fx), mix(g01, g11, fx), fy);

  // contrast_std then contrast_doc, both around the local gray-point.
  let l_std = contrast_std(gray, L, u.contrast);
  let l_doc = contrast_doc(
    gray, l_std, u.mix, u.tilt_black, u.tilt_white, adjust_xition, adjust_bw,
  );

  textureStore(Output, p, vec4<f32>(clamp01(l_doc), lab.g, lab.b, lab.a));
}
