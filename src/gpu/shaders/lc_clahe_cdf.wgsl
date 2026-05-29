// Local Contrast — CLAHE pass 2: turn each tile's histogram into an anchored,
// contrast-limited CDF.  One thread per tile (the work is 256 serial bins —
// trivial next to the per-pixel passes, so no parallel scan is needed).
//
// Steps per tile:
//   1. Total the bins.  Empty tile → identity ramp (pass becomes a no-op there).
//   2. Contrast-limit: clip every bin to `clip_limit × mean` and redistribute
//      the clipped excess uniformly across all bins.  This is the "CL" in
//      CLAHE — it caps how steep the CDF can get, which is what stops flat
//      shadow regions from turning to noise under aggressive equalization.
//   3. Prefix-sum the clipped+redistributed counts into a CDF, normalize.
//   4. Hard-anchor the endpoints (cdf[0]→0, cdf[255] stays 1) so pure black
//      stays black and only the occupied range between is stretched — same
//      reasoning as the global Equalize stage's `read_cdf`.

struct Uniforms {
  nx:         u32,
  ny:         u32,
  image_w:    u32,
  image_h:    u32,
  clip_limit: f32,
  shadows:    f32,  // unused here
  highlights: f32,  // unused here
  _pad:       f32,
}

@group(0) @binding(0) var<storage, read> hist: array<u32>;
@group(0) @binding(1) var<storage, read_write> cdf: array<f32>;
@group(0) @binding(2) var<uniform> u: Uniforms;

fn write_identity(base: u32) {
  for (var i = 0u; i < 256u; i = i + 1u) {
    cdf[base + i] = f32(i) / 255.0;
  }
}

@compute @workgroup_size(64, 1, 1)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let tile = gid.x;
  let ntiles = u.nx * u.ny;
  if (tile >= ntiles) { return; }
  let base = tile * 256u;

  // 1. Total.
  var total: f32 = 0.0;
  for (var i = 0u; i < 256u; i = i + 1u) {
    total = total + f32(hist[base + i]);
  }
  if (total <= 0.0) {
    write_identity(base);
    return;
  }

  // 2. Contrast limit: find the excess above the clip threshold.
  let limit = max(1.0, u.clip_limit * total / 256.0);
  var excess: f32 = 0.0;
  for (var i = 0u; i < 256u; i = i + 1u) {
    let c = f32(hist[base + i]);
    if (c > limit) { excess = excess + (c - limit); }
  }
  let add = excess / 256.0;

  // 3. Prefix-sum the clipped+redistributed counts.  The total is unchanged
  //    (clipped mass is added back via `add`), so normalize by `total`.
  var cum: f32 = 0.0;
  for (var i = 0u; i < 256u; i = i + 1u) {
    cum = cum + min(f32(hist[base + i]), limit) + add;
    cdf[base + i] = cum / total;
  }

  // 4. Anchor endpoints.
  let c0 = cdf[base];
  let denom = 1.0 - c0;
  if (denom < 1e-6) {
    write_identity(base);
    return;
  }
  for (var i = 0u; i < 256u; i = i + 1u) {
    cdf[base + i] = clamp((cdf[base + i] - c0) / denom, 0.0, 1.0);
  }
}
