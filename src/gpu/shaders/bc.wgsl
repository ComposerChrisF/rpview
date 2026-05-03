// Brightness / Contrast — OKLab variant.
//   Contrast:   scales L around `Midpoint` (defaults to 0.5 = perceptual
//               middle-grey).  The same Midpoint is used by the LC stage's
//               shadow/highlight blend; the BC and LC sliders share one knob
//               surfaced in the LC section UI.
//   Brightness: additive shift on L.
// (Saturation moved into the Vibrance stage — see vibrance.wgsl.)

struct Uniforms {
  Brightness: f32,
  Contrast:   f32,
  Midpoint:   f32,
  _pad:       f32,
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
  var L = (lab.r - u.Midpoint) * max(u.Contrast + 1.0, 0.0) + u.Midpoint;
  L = L + u.Brightness;
  textureStore(Output, pixel, vec4<f32>(L, lab.g, lab.b, lab.a));
}
