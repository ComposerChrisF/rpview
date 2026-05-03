// OKLab color space (Björn Ottosson, 2020) + canonical sRGB ↔ linear EOTF.
// Source: PixelShaderPaint3 shader-includes/oklab.wgsl.

fn srgb_to_linear_component(x: f32) -> f32 {
  let ax = max(x, 0.0);
  if (ax <= 0.04045) { return ax / 12.92; }
  return pow((ax + 0.055) / 1.055, 2.4);
}

fn linear_to_srgb_component(x: f32) -> f32 {
  let ax = max(x, 0.0);
  if (ax <= 0.0031308) { return ax * 12.92; }
  return 1.055 * pow(ax, 1.0 / 2.4) - 0.055;
}

fn srgb_to_linear(c: vec3<f32>) -> vec3<f32> {
  return vec3<f32>(
    srgb_to_linear_component(c.r),
    srgb_to_linear_component(c.g),
    srgb_to_linear_component(c.b),
  );
}

fn linear_to_srgb(c: vec3<f32>) -> vec3<f32> {
  return vec3<f32>(
    linear_to_srgb_component(c.r),
    linear_to_srgb_component(c.g),
    linear_to_srgb_component(c.b),
  );
}

fn signed_cbrt(x: f32) -> f32 {
  return sign(x) * pow(abs(x), 1.0 / 3.0);
}

fn linear_srgb_to_oklab(c: vec3<f32>) -> vec3<f32> {
  let l_ = 0.4122214708 * c.r + 0.5363325363 * c.g + 0.0514459929 * c.b;
  let m_ = 0.2119034982 * c.r + 0.6806995451 * c.g + 0.1073969566 * c.b;
  let s_ = 0.0883024619 * c.r + 0.2817188376 * c.g + 0.6299787005 * c.b;
  let l = signed_cbrt(l_);
  let m = signed_cbrt(m_);
  let s = signed_cbrt(s_);
  return vec3<f32>(
    0.2104542553 * l + 0.7936177850 * m - 0.0040720468 * s,
    1.9779984951 * l - 2.4285922050 * m + 0.4505937099 * s,
    0.0259040371 * l + 0.7827717662 * m - 0.8086757660 * s,
  );
}

fn oklab_to_linear_srgb(c: vec3<f32>) -> vec3<f32> {
  let l_ = c.x + 0.3963377774 * c.y + 0.2158037573 * c.z;
  let m_ = c.x - 0.1055613458 * c.y - 0.0638541728 * c.z;
  let s_ = c.x - 0.0894841775 * c.y - 1.2914855480 * c.z;
  let l = l_ * l_ * l_;
  let m = m_ * m_ * m_;
  let s = s_ * s_ * s_;
  return vec3<f32>(
     4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s,
    -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s,
    -0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s,
  );
}
