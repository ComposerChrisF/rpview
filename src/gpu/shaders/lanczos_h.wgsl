// Horizontal Lanczos-3 resampling.  Reads sRGB source via `textureLoad`
// (which auto-decodes Rgba8UnormSrgb → linear), filters along x, writes
// linear values to an `Rgba16Float` storage texture at (`dst_w`, src_h).
//
// `filter_scale` widens the kernel for downscaling: it's `max(1, src_w/dst_w)`.
// At `dst_w == src_w` it collapses to 1 and the kernel reaches 3 source
// pixels in each direction (7 taps including the zero endpoints).

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

  let src_x_center = (f32(x) + 0.5) * (u.src_filter_dim / u.dst_w) - 0.5;
  let radius = LANCZOS_A * u.filter_scale;
  let s_min = i32(ceil(src_x_center - radius));
  let s_max = i32(floor(src_x_center + radius));
  let src_max = i32(u.src_filter_dim) - 1;

  var color = vec4<f32>(0.0);
  var weight_sum = 0.0;
  for (var s = s_min; s <= s_max; s = s + 1) {
    let s_clamped = clamp(s, 0, src_max);
    let dx = (f32(s) - src_x_center) / u.filter_scale;
    let w = lanczos(dx);
    let texel = textureLoad(Input, vec2<i32>(s_clamped, y), 0);
    color = color + texel * w;
    weight_sum = weight_sum + w;
  }
  if (weight_sum > 0.0) { color = color / weight_sum; }
  textureStore(Output, vec2<i32>(x, y), color);
}
