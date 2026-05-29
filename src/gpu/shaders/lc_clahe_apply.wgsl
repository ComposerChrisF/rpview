// Local Contrast — CLAHE pass 3: apply the per-tile CDFs with bilinear
// interpolation between the four nearest tile centers, then blend by source
// luminance.
//
// Bilinear interpolation between neighboring tiles' CDFs is what removes the
// blocky tile boundaries you'd get from naive per-tile equalization — it's the
// "AHE→CLAHE" interpolation step.  Tile centers sit at the center of each grid
// cell, so a pixel's position in "tile-center space" is
// `(pixel + 0.5)/image * n - 0.5`; the fractional part is the lerp weight and
// edge cells clamp.
//
// The final blend mirrors the global Equalize stage's Shadows/Highlights: the
// equalized L is faded in weighted by the SOURCE pixel luminance — `shadows`
// toward black, `highlights` toward white — so detail is pulled out of the
// dark (or bright) regions while the opposite end and the endpoints are left
// alone.  This is the local-histogram analogue of the CPU local-contrast
// path's "Alpha of pure Black" / "Alpha of pure White".

struct Uniforms {
  nx:         u32,
  ny:         u32,
  image_w:    u32,
  image_h:    u32,
  clip_limit: f32,  // unused here
  shadows:    f32,
  highlights: f32,
  _pad:       f32,
}

@group(0) @binding(0) var Input: texture_2d<f32>;
@group(0) @binding(1) var Output: texture_storage_2d<rgba16float, write>;
@group(0) @binding(2) var<storage, read> cdf: array<f32>;
@group(0) @binding(3) var<uniform> u: Uniforms;

// Equalized L for `tile` at luminance `L`, linearly interpolated between bins.
fn lookup(tile: u32, L: f32) -> f32 {
  let f = L * 255.0;
  let i0 = u32(min(f, 254.0));
  let t = f - f32(i0);
  let base = tile * 256u;
  return mix(cdf[base + i0], cdf[base + i0 + 1u], t);
}

@compute @workgroup_size(16, 16, 1)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let dims = vec2<i32>(textureDimensions(Output));
  let p = vec2<i32>(gid.xy);
  if (p.x >= dims.x || p.y >= dims.y) { return; }

  let lab = textureLoad(Input, p, 0);
  let L = clamp(lab.r, 0.0, 1.0);

  // Position in tile-center space.
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

  let e00 = lookup(y0 * u.nx + x0, L);
  let e10 = lookup(y0 * u.nx + x1, L);
  let e01 = lookup(y1 * u.nx + x0, L);
  let e11 = lookup(y1 * u.nx + x1, L);
  let eq = mix(mix(e00, e10, fx), mix(e01, e11, fx), fy);

  // Source-luminance-weighted blend (no flat term — that's the Equalize
  // stage's job; here Shadows/Highlights are the only knobs).
  let amount = clamp(u.shadows * (1.0 - L) + u.highlights * L, 0.0, 1.0);
  let new_L = mix(lab.r, eq, amount);

  textureStore(Output, p, vec4<f32>(new_L, lab.g, lab.b, lab.a));
}
