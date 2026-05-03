// Decode pass: sRGB-encoded RGBA8 → OKLab rgba16float.
// `Input` is `Rgba8UnormSrgb` so `textureLoad` auto-decodes to linear-light;
// we then convert linear sRGB → OKLab and store (L, a, b, alpha).

@group(0) @binding(0) var Input: texture_2d<f32>;
@group(0) @binding(1) var Output: texture_storage_2d<rgba16float, write>;

@compute @workgroup_size(16, 16, 1)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let dims = vec2<i32>(textureDimensions(Output));
  let pixel = vec2<i32>(gid.xy);
  if (pixel.x >= dims.x || pixel.y >= dims.y) { return; }
  let linear = textureLoad(Input, pixel, 0);
  let lab = linear_srgb_to_oklab(linear.rgb);
  textureStore(Output, pixel, vec4<f32>(lab, linear.a));
}
