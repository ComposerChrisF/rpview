// Local Contrast — operates directly on OKLab rgba16float (input + output).
// L-only modification; a/b are passed through unchanged. Two-pass separable
// design: pass 1 is horizontal blur (Axis < 0.5), pass 2 is vertical.
//
// Algorithm from PSP3 (`local-contrast.wgsl`); the unified pipeline performs
// the sRGB↔OKLab conversions at the boundary so this shader doesn't need
// `linear_srgb_to_oklab`/`oklab_to_linear_srgb`.

const TILE: u32 = 16u;
const MAX_SAMPLES: u32 = 30u;
const STRIP: u32 = TILE + 2u * MAX_SAMPLES;

var<workgroup> cache: array<f32, 1216>;

struct Uniforms {
  Radius:          f32,
  Strength:        f32,
  ShadowLift:      f32,
  HighlightDarken: f32,
  Midpoint:        f32,
  Axis:            f32,
  ImageWidth:      f32,
  ImageHeight:     f32,
}

@group(0) @binding(0) var Input: texture_2d<f32>;
@group(0) @binding(1) var Output: texture_storage_2d<rgba16float, write>;
@group(0) @binding(2) var<uniform> u: Uniforms;

fn gaussian(x: f32, sigma: f32) -> f32 {
  return exp(-0.5 * (x * x) / (sigma * sigma));
}

fn loadL(coord: vec2<i32>, dims: vec2<i32>) -> f32 {
  let clamped = clamp(coord, vec2<i32>(0), dims - vec2<i32>(1));
  return textureLoad(Input, clamped, 0).r;
}

@compute @workgroup_size(16, 16, 1)
fn main(
  @builtin(global_invocation_id) gid: vec3<u32>,
  @builtin(local_invocation_id) lid: vec3<u32>,
  @builtin(workgroup_id) wid: vec3<u32>,
) {
  let dims = vec2<i32>(textureDimensions(Input));
  let horizontal = u.Axis < 0.5;

  let stride = max(1u, u32(u.Radius / f32(MAX_SAMPLES)));
  let samples = min(MAX_SAMPLES, u32(ceil(u.Radius / f32(stride))));
  let sigma = max(f32(samples) / 3.0, 0.3);

  let tile_origin = vec2<i32>(wid.xy) * i32(TILE);

  // Phase 1: cooperative load of L into shared memory.
  if (horizontal) {
    let base_x = tile_origin.x - i32(samples) * i32(stride);
    let y = tile_origin.y + i32(lid.y);
    let cache_row_offset = lid.y * STRIP;
    for (var i: u32 = lid.x; i < STRIP; i = i + TILE) {
      let src_x = base_x + i32(i) * i32(stride);
      cache[cache_row_offset + i] = loadL(vec2<i32>(src_x, y), dims);
    }
  } else {
    let base_y = tile_origin.y - i32(samples) * i32(stride);
    let x = tile_origin.x + i32(lid.x);
    let cache_col_offset = lid.x * STRIP;
    for (var i: u32 = lid.y; i < STRIP; i = i + TILE) {
      let src_y = base_y + i32(i) * i32(stride);
      cache[cache_col_offset + i] = loadL(vec2<i32>(x, src_y), dims);
    }
  }

  workgroupBarrier();

  // Phase 2: Gaussian blur of L from shared memory.
  let pixel = vec2<i32>(gid.xy);
  if (pixel.x >= dims.x || pixel.y >= dims.y) { return; }

  var blurred_L = 0.0;
  var weight_sum = 0.0;
  let range = min(2u * samples + u32(ceil(f32(TILE) / f32(stride))), STRIP);

  if (horizontal) {
    let cache_row_offset = lid.y * STRIP;
    let center_f = f32(samples) + f32(lid.x) / f32(stride);
    for (var d: u32 = 0u; d < range; d = d + 1u) {
      let w = gaussian(f32(d) - center_f, sigma);
      blurred_L = blurred_L + w * cache[cache_row_offset + d];
      weight_sum = weight_sum + w;
    }
  } else {
    let cache_col_offset = lid.x * STRIP;
    let center_f = f32(samples) + f32(lid.y) / f32(stride);
    for (var d: u32 = 0u; d < range; d = d + 1u) {
      let w = gaussian(f32(d) - center_f, sigma);
      blurred_L = blurred_L + w * cache[cache_col_offset + d];
      weight_sum = weight_sum + w;
    }
  }
  blurred_L = blurred_L / weight_sum;

  // Phase 3: deviation-from-mean enhancement on L.
  let lab = textureLoad(Input, pixel, 0);
  let L = lab.r;
  let deviation = L - blurred_L;
  var L_new = L + u.Strength * deviation;

  if (u.ShadowLift > 0.0) {
    let shadow_blend = (1.0 - smoothstep(0.0, u.Midpoint, blurred_L)) * u.ShadowLift;
    L_new = mix(L_new, u.Midpoint, shadow_blend);
  }
  if (u.HighlightDarken > 0.0) {
    let highlight_blend = smoothstep(u.Midpoint, 1.0, blurred_L) * u.HighlightDarken;
    L_new = mix(L_new, u.Midpoint, highlight_blend);
  }
  L_new = clamp(L_new, 0.0, 1.0);

  textureStore(Output, pixel, vec4<f32>(L_new, lab.g, lab.b, lab.a));
}
