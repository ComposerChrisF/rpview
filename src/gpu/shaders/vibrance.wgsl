// Vibrance + Saturation — both operate on OKLab (a, b) chroma, so they fold
// into a single dispatch.
//   Vibrance: asymmetric, weighted by chroma magnitude (boosts low-chroma
//             pixels harder, cuts high-chroma pixels harder).
//   Saturation: uniform scale on (a, b), applied AFTER vibrance.
// L is passed through unchanged.

struct Uniforms {
  Amount:     f32,
  Saturation: f32,
  _pad0:      f32,
  _pad1:      f32,
}

@group(0) @binding(0) var Input: texture_2d<f32>;
@group(0) @binding(1) var Output: texture_storage_2d<rgba16float, write>;
@group(0) @binding(2) var<uniform> u: Uniforms;

@compute @workgroup_size(16, 16, 1)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let dims = vec2<i32>(textureDimensions(Output));
  let pixel = vec2<i32>(gid.xy);
  if (pixel.x >= dims.x || pixel.y >= dims.y) { return; }

  let lab = textureLoad(Input, pixel, 0);
  let a = lab.g;
  let b = lab.b;
  let C = sqrt(a * a + b * b);

  // Vibrance scale.
  var weight = clamp(1.0 - C * 4.0, 0.0, 1.0);
  if (u.Amount < 0.0) { weight = 1.0 - weight; }
  let vib_scale = 1.0 + u.Amount * weight;

  // Saturation scale, applied after vibrance.  −1 = greyscale, 0 = no change.
  let sat_scale = max(u.Saturation + 1.0, 0.0);

  let scale = vib_scale * sat_scale;
  textureStore(Output, pixel, vec4<f32>(lab.r, a * scale, b * scale, lab.a));
}
