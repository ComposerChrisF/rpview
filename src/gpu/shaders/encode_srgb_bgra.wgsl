// Encode pass: OKLab rgba16float → sRGB BGRA bytes (rgba8unorm).
// Output bytes are in BGRA order so the result wraps directly into
// `gpui::RenderImage` without further conversion.

@group(0) @binding(0) var Input: texture_2d<f32>;
@group(0) @binding(1) var Output: texture_storage_2d<rgba8unorm, write>;

@compute @workgroup_size(16, 16, 1)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let dims = vec2<i32>(textureDimensions(Output));
  let pixel = vec2<i32>(gid.xy);
  if (pixel.x >= dims.x || pixel.y >= dims.y) { return; }
  let lab = textureLoad(Input, pixel, 0);
  let linear = clamp(oklab_to_linear_srgb(lab.rgb), vec3(0.0), vec3(1.0));
  let r = linear_to_srgb_component(linear.r);
  let g = linear_to_srgb_component(linear.g);
  let b = linear_to_srgb_component(linear.b);
  // BGRA byte order: stored R = source B, stored B = source R.
  textureStore(Output, pixel, vec4<f32>(b, g, r, lab.a));
}
