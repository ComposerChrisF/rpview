// Hue Rotation in OKLCh: rotate the (a, b) chroma vector by `Hue` turns.
// L is passed through.

struct Uniforms {
  Hue: f32,
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
  let h = atan2(b, a);
  let hNew = h + u.Hue * 6.28318530717958647692;

  textureStore(Output, pixel, vec4<f32>(lab.r, C * cos(hNew), C * sin(hNew), lab.a));
}
