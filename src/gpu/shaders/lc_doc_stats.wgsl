// Local Contrast — Document-Style Contrast pass 2: derive a per-tile gray-point
// (mean AND median L) from the tile histogram built by `lc_clahe_histogram.wgsl`.
// One thread per tile (256 serial bins — trivial next to the per-pixel passes).
//
// The gray-point is what makes Document-Style Contrast *adaptive*: contrast_doc
// crushes toward black below it and pushes toward white above it.  We compute
// both statistics so the apply pass can switch between them via the "Use Median
// Gray-Point" flag.  Both are written in [0, 1] (bin index / 255) so the apply
// shader can feed them straight into contrast_std / contrast_doc, which expect
// the gray-point on the same 0..1 scale as L.
//
// The median uses the exact sub-bin formula from the CPU reference
// (`Histogram::finalize` in `src/utils/local_contrast.rs`) for parity.

struct Uniforms {
  nx:         u32,
  ny:         u32,
  image_w:    u32,
  image_h:    u32,
  contrast:   f32,  // unused here; shared uniform across the 2 doc passes
  mix:        f32,  // unused here
  tilt_black: f32,  // unused here
  tilt_white: f32,  // unused here
  flags:      u32,  // unused here
  _pad0:      f32,
  _pad1:      f32,
  _pad2:      f32,
}

@group(0) @binding(0) var<storage, read> hist: array<u32>;
@group(0) @binding(1) var<storage, read_write> stats: array<f32>;
@group(0) @binding(2) var<uniform> u: Uniforms;

@compute @workgroup_size(64, 1, 1)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let tile = gid.x;
  let ntiles = u.nx * u.ny;
  if (tile >= ntiles) { return; }
  let base = tile * 256u;
  let out = tile * 2u;

  // Total weight + weighted-sum for the mean.
  var total: f32 = 0.0;
  var wsum: f32 = 0.0;
  for (var i = 0u; i < 256u; i = i + 1u) {
    let c = f32(hist[base + i]);
    total = total + c;
    wsum = wsum + c * f32(i);
  }
  if (total <= 0.0) {
    // Empty tile (shouldn't happen for interior tiles): neutral mid-gray so
    // contrast_doc stays benign there.
    stats[out] = 0.5;
    stats[out + 1u] = 0.5;
    return;
  }

  // Mean (bin index / 255 → [0, 1]).
  stats[out] = (wsum / total) / 255.0;

  // Median via cumulative walk to half-weight, with the CPU sub-bin formula.
  let half = total * 0.5;
  var ibin = 0u;
  var sum = 0.0;
  loop {
    if (ibin >= 256u) { break; }
    let c = f32(hist[base + ibin]);
    if (sum + c >= half) { break; }
    sum = sum + c;
    ibin = ibin + 1u;
  }
  let after = sum + f32(hist[base + min(ibin, 255u)]);
  var median: f32;
  if (ibin > 0u && f32(hist[base + ibin - 1u]) != 0.0) {
    median = f32(ibin) - (after - half) / f32(hist[base + ibin - 1u]);
  } else {
    median = f32(ibin) - 0.5;
  }
  stats[out + 1u] = median / 255.0;
}
