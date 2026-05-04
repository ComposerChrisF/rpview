// Vertical Lanczos-3 resampling + linear-sRGB → OKLab decode.  Reads the
// horizontal-pass intermediate (`Rgba16Float` linear at (dst_w, src_h)),
// filters along y, converts the filtered linear-RGB to OKLab, writes
// `Rgba16Float` OKLab at (dst_w, dst_h).
//
// Folding the OKLab decode into this pass replaces the standalone decode
// dispatch on the resize path — three passes (decode, lanczos_h, lanczos_v)
// collapse to two.  The non-resize path keeps using `decode_oklab.wgsl`
// since there's no intermediate to filter.
//
// Requires `oklab.wgsl` to be prepended for `linear_srgb_to_oklab`.

struct Uniforms {
  dst_w: f32,
  dst_h: f32,
  src_filter_dim: f32,
  filter_scale: f32,
};

@group(0) @binding(0) var Input: texture_2d<f32>;
@group(0) @binding(1) var Output: texture_storage_2d<rgba16float, write>;
@group(0) @binding(2) var<uniform> u: Uniforms;

const PI: f32 = 3.14159265358979;
const LANCZOS_A: f32 = 3.0;

fn sinc(x: f32) -> f32 {
  if (abs(x) < 1.0e-6) { return 1.0; }
  let p = PI * x;
  return sin(p) / p;
}

fn lanczos(x: f32) -> f32 {
  if (abs(x) >= LANCZOS_A) { return 0.0; }
  return sinc(x) * sinc(x / LANCZOS_A);
}

@compute @workgroup_size(16, 16, 1)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let x = i32(gid.x);
  let y = i32(gid.y);
  if (f32(x) >= u.dst_w || f32(y) >= u.dst_h) { return; }

  let src_y_center = (f32(y) + 0.5) * (u.src_filter_dim / u.dst_h) - 0.5;
  let radius = LANCZOS_A * u.filter_scale;
  let s_min = i32(ceil(src_y_center - radius));
  let s_max = i32(floor(src_y_center + radius));
  let src_max = i32(u.src_filter_dim) - 1;

  var color = vec4<f32>(0.0);
  var weight_sum = 0.0;
  for (var s = s_min; s <= s_max; s = s + 1) {
    let s_clamped = clamp(s, 0, src_max);
    let dy = (f32(s) - src_y_center) / u.filter_scale;
    let w = lanczos(dy);
    let texel = textureLoad(Input, vec2<i32>(x, s_clamped), 0);
    color = color + texel * w;
    weight_sum = weight_sum + w;
  }
  if (weight_sum > 0.0) { color = color / weight_sum; }

  // Lanczos can produce small negative ringing values; signed_cbrt in the
  // OKLab transform handles them but the encode-back roundtrip clamps, so
  // letting them ride matches the rest of the pipeline's "no clipping in
  // perceptual space" stance.
  let oklab = linear_srgb_to_oklab(color.rgb);
  textureStore(Output, vec2<i32>(x, y), vec4<f32>(oklab, color.a));
}
