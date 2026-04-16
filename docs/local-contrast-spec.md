# Spec: Port FraleyMusic-ImageDsp → rpview-gpui

## Context

rpview-gpui currently has three simple per-channel filters (brightness, contrast, gamma) implemented as a 256-entry RGB LUT in `src/utils/filters.rs`. The user has a mature C# image-processing library at `/Users/chris/Chris/Dev/Util/PView/FraleyMusic-ImageDsp` whose flagship algorithm is **local-contrast luminance normalization** in a perceptual color space. Goal: port that algorithm to Rust, with an optional upgrade from HSL to a modern perceptual model (Oklab / OkLCh).

This document is a **language-neutral spec** derived by reading the three C# source files. It describes what the algorithm does in enough detail to reimplement without needing the C# code open. It does *not* prescribe a Rust module layout — that's a follow-up design step after you've reviewed and tweaked this spec.

**Source files analyzed** (all in `/Users/chris/Chris/Dev/Util/PView/FraleyMusic-ImageDsp`):
- `HslSupport.cs` (192 lines) — RGB↔HSL conversion.
- `FloatMap.cs` (579 lines) — planar float32 bitmap with RGB[A]+HSL channels.
- `AdaptiveContrastDsp.cs` (662 lines) — `LocallyNormalizeLuminance` and helpers.
- `ImageDsp-Tests/TestRgbToHslToRgb.cs` (60 lines) — only test: HSL roundtrip to 1e-6.

---

## 1. Data model: FloatMap

A `FloatMap` is a planar float32 bitmap. Each channel is a separate `float[width, height]` array, values in **[0.0, 1.0]** (255 is the integer conversion factor). Channels stored:

- `RGB_r`, `RGB_g`, `RGB_b` — always present.
- `Alpha` — present iff the source had an alpha channel.
- `HSL_h`, `HSL_s`, `HSL_l` — populated on demand.

Integer round-trip: `float_to_byte(f) = clamp(round(f * 255), 0, 255)`. Byte-to-float is `b / 255.0`.

**Rust port note:** rpview already has `image::RgbaImage` as interleaved `u8[4]`. We'll need a planar-float equivalent (e.g. `Vec<f32>` per channel or a struct of `Box<[f32]>` plane handles) for the perceptual pipeline — interleaved u8 loses precision across stages. Could live in a new `src/utils/float_map.rs`.

---

## 2. Perceptual transform: RGB ↔ HSL

Reference: `HslSupport.cs:34-175`. Standard HSL ("lightness" variant, not HSV/value).

### 2.1 RGB → HSL (per pixel)

Given `r, g, b ∈ [0, 1]`:

```
max    = max(r, g, b)
min    = min(r, g, b)
chroma = max - min
L      = (max + min) / 2

if max == min:
    H = 0    (undefined, conventionally zero)
else if max == r:
    H = (1/6) * ((g - b) / chroma)          if g >= b
    H = (1/6) * ((g - b) / chroma) + 1       if g <  b
else if max == g:
    H = (1/6) * ((b - r) / chroma) + 1/3
else (max == b):
    H = (1/6) * ((r - g) / chroma) + 2/3

if L == 0 or L == 1:
    S = 0
else:
    S = chroma / (1 - |2*L - 1|)
```

Note: the C# file carries commented-out alternates for intensity/value/luma computation; the implementation uses **lightness** (HSL, not HSV).

### 2.2 HSL → RGB (per pixel)

Given `H ∈ [0,1), S, L ∈ [0,1]` (Wikipedia's HSL reference formula):

```
if S == 0:
    R = G = B = L
else:
    t2 = L * (1 + S)              if L <  0.5
    t2 = L + S - L*S              if L >= 0.5
    t1 = 2*L - t2
    t3r = wrap(H + 1/3)
    t3g = wrap(H)
    t3b = wrap(H - 1/3)
    for each channel c ∈ {r, g, b} with t3c:
        if t3c < 1/6:    c = t1 + (t2 - t1) * 6 * t3c
        else if t3c < 1/2:    c = t2
        else if t3c < 2/3:    c = t1 + (t2 - t1) * (2/3 - t3c) * 6
        else:                  c = t1

wrap(d):  d + 1 if d < 0;  d - 1 if d > 1;  else d
```

Round-trip is accurate to ≈1e-6 (verified by `TestRgbToHslToRgb.cs`).

### 2.3 Optional: swap HSL for Oklab / OkLCh

HSL is not perceptually uniform — equal changes in L don't produce equal perceived lightness changes, and hue wraps non-linearly. Modern alternatives:

- **Oklab** (Björn Ottosson, 2020): perceptually uniform Lab, simple matrix + cube-root, fast. L ∈ [0,1], a/b roughly [-0.5, 0.5].
- **OkLCh**: polar form of Oklab (L, chroma, hue) — the drop-in equivalent for HSL.
- **CIE L\*a\*b\*** / **CIELCh**: older but still industry standard; more expensive (XYZ intermediate, sRGB gamma decode).

Recommended for this port: **OkLCh**, because:
1. Local luminance normalization fundamentally operates on perceived lightness — Oklab's L is closer to human perception than HSL's mechanical midpoint.
2. The algorithm's "desaturate when moving toward black/white" heuristic is a workaround for HSL's hue instability near extremes; Oklab chroma degrades more gracefully, possibly letting us drop that heuristic.
3. All of the LocallyNormalizeLuminance math operates on scalar L — swapping the color space doesn't require rewriting the histogram logic, only the conversion routines and the meaning of "L" (and "S" → chroma).

The port should keep the color-space transform behind a trait so we can A/B test HSL vs OkLCh.

---

## 3. Local contrast algorithm: `LocallyNormalizeLuminance`

Reference: `AdaptiveContrastDsp.cs:247-527`. This is the core deliverable.

### 3.1 Parameters

| Parameter | Default | Meaning |
|---|---|---|
| `cxyWindow` | `width/32` (if 0) | Radius of circular sampling window around each pixel, in pixels. Larger = slower + smoother local gray-point. |
| `cxyBlock` | `cxyWindow/4`, min 2 | Stride between histogram sample sites (for optimization — see §3.3). |
| `alphaBlack` | 0.04 (0.2 in doc) | Blend weight toward normalized value in dark regions. |
| `alphaWhite` | 0.04 | Blend weight toward normalized value in bright regions. |
| `contrast` | 0.04 | Strength of contrast adjustment, [0, 1]. 0 = no change. |
| `lightenShadows` | 0.50 | Lift shadows toward white when local gray ≤ middle gray. |
| `darkenHighlights` | 0.0 | Push highlights toward black when local gray ≥ middle gray. |
| `UseMedianForContrast` | false | Median vs mean as local gray-point reference. |
| `UseDocumentContrast` | false | Force near-black to black, near-white to white (for text documents). |
| `tiltBlackDocContrast` | −0.20 | Shift black threshold in doc mode. |
| `tiltWhiteDocContrast` | −0.05 | Shift white threshold in doc mode. |
| `ApplyContrastToBW` | true | Apply `contrast` amount to forced black/white pixels in doc mode. |
| `ApplyContrastToXition` | true | Apply `contrast` amount to mid-tone pixels in doc mode. |
| `mixDocumentContrast` | 1.0 | Linear mix between standard and document contrast. |
| `ReturnImage` | Dsp | Debug mode — return intermediate visualizations (see §3.6). |

### 3.2 Windowing kernel

Precomputed once: a `(2*cxyWindow + 1)² ` float array of weights with **squared linear falloff** from the center:

```
for x, y in kernel size:
    dist = sqrt((x - cxyWindow)² + (y - cxyWindow)²)
    w = (cxyWindow - dist) / cxyWindow        // linear
    if w < 0: w = 0                            // outside circle
    w = w²                                      // squared for softer falloff
    if w < 0.005: w = 0                        // sparsity threshold
    kernel[x, y] = w
```

### 3.3 Sparse histogram grid (the key optimization)

Instead of computing a histogram per pixel (O(W·H·window²)), compute one histogram every `cxyBlock` pixels, then bilinearly interpolate. With `cxyBlock = cxyWindow / 4`, the histogram count drops by **16×** with no visible artifacts. `cxyBlock = 2` is the minimum.

**Per-pixel bin map:** `binMap[x, y] = clamp(round(L[x, y] * 255), 0, 255)`. Precomputed once.

**Per grid-site `(xBlock, yBlock)` histogram** is a `float[259]`:
- `hist[0..=255]` — weighted luminance histogram, bucket `i` counts pixels with bin `i`, each contributing its kernel weight.
- `hist[256]` — total weight (sum of all bucket weights).
- `hist[257]` — fractional median bin (computed after filling).
- `hist[258]` — mean luminance × 255 (computed after filling).

**Window walk for site (xBlock·cxyBlock, yBlock·cxyBlock)**:
1. Compute window rectangle `[x-k, x+k] × [y-k, y+k]`, clipped to image bounds.
2. For each pixel `(wx, wy)` in window, with kernel weight `w`:
   - If `w == 0`: skip.
   - `hist[binMap[wx, wy]] += w`
   - `hist[256] += w`
   - `hist[258] += w * L_src[wx, wy]`
3. Median: walk cumulative weight until half of `hist[256]` reached. With `ibinMedian` the first bin that crosses the halfway point:
   ```
   if hist[ibinMedian - 1] != 0:
       hist[257] = ibinMedian - (sumTest - halfWeight) / hist[ibinMedian - 1]
   else:
       hist[257] = ibinMedian - 0.5
   ```
4. Mean: `hist[258] = (hist[258] / hist[256]) * 255`.

Parallelized with `Parallel.For` over `xBlock` rows. Progress is reported via `FeedbackCallback` every ~1% of blocks.

### 3.4 Per-pixel processing

Inner loop over each output pixel `(x, y)` inside block `(xBlock, yBlock)`:

1. **Bilinear weights within block:**
   ```
   xWeight = 1 - (xWithinBlock / cxyBlock)
   yWeight = 1 - (yWithinBlock / cxyBlock)
   ```
2. **Four-corner histograms** `hist1..hist4` = `histograms[xBlock+dx, yBlock+dy]` for `(dx, dy) ∈ {(0,0), (1,0), (0,1), (1,1)}`. Fall back to the adjacent histogram if one is null (edge of image).
3. **Compute `fracSum`** — the normalized value if the histogram were fully equalized — via `NormalizeValueVia4Histograms`:
   ```
   for each of the 4 histograms:
       sum = hist[0] + hist[1] + ... + hist[ibinCur]
       fracInBin = 1 - (ibinCur / 255)
       sum -= hist[ibinCur] * fracInBin       // sub-bin precision
       frac = sum / c[i]                       // c[i] = hist[256]
   bilinear blend: fracY = lerp(lerp(frac1, frac2, xw), lerp(frac3, frac4, xw), yw)
   return fracY
   ```
   Note the sub-bin adjustment: a bin of index 0 contributes 100% of its weight, a bin of 255 contributes 0% — this stretches the output to span fully [0, 1] rather than [1/512, 1 − 1/512].

4. **Compute `binMean`, `binMedian`** by bilinearly interpolating `hist[258]` and `hist[257]` across the four corners.

5. **Local gray-point:**
   ```
   binContrastReference = UseMedianForContrast ? binMedian : binMean
   ```

6. **Standard contrast** (`Contrast_Std(grayPoint, lCur, contrast)`, lines 628-644):
   ```
   gray = binContrastReference / 255       // normalized gray-point
   if lCur < gray:
       fracGray = lCur / gray
       fracGray = fracGray²                  // pull more strongly toward black
       lNew = fracGray * gray
       return contrast*lNew + (1-contrast)*lCur
   else:
       fracGray = (1 - lCur) / (1 - gray)    // symmetric toward white
       fracGray = fracGray²
       lNew = 1 - fracGray * (1 - gray)
       return contrast*lNew + (1-contrast)*lCur
   ```
   **Bug note in original**: `Contrast_Std` mixes unnormalized scale — the `/ 255` math on input uses `255.0` literally (`(255.0f - lCur)`) even though both values are in [0, 1]. In the original this produces a specific (non-symmetric) behavior. Port faithfully first, then consider whether to normalize the white-side branch.

7. **Document contrast** (if `UseDocumentContrast`), `Contrast_Doc(grayPoint, lCur, mix, tiltB, tiltW, ...)` (lines 609-620):
   ```
   lBlack = ApplyTilt(gray, tiltBlack)            // biases threshold
   if lCur < lBlack:
       return fContrastBW ? clamp01(lerp(0, 1, lCur; 0, 1) via contrast) : 0
   lWhite = ApplyTilt(gray, tiltWhite)
   if lCur > lWhite:
       return fContrastBW ? clamp01(lerp(lCur, 1, contrast)) : 1
   lNew = lerp(0, 1, lCur; lBlack, lWhite)       // linearly remap transition band to [0, 1]
   if !fContrastXition:
       return clamp01(lNew)
   return clamp01(lerp(lCur, lNew, contrast))

   ApplyTilt(gray, tilt):
       if tilt <= 0: return gray * (1 + tilt)       // pulls threshold toward black
       else:         return 1 - (1 - gray)*(1 - tilt)  // pulls toward white
   ```

8. **Lighten shadows** (if `lightenShadows != 0` and `grayPoint ≤ 127`):
   ```
   fracDark = 128 / (1 + binContrastReference)
   lContrast = lContrast * fracDark * lightenShadows + lContrast * (1 - lightenShadows)
   ```
9. **Darken highlights** (if `darkenHighlights != 0` and `grayPoint ≥ 127.5`):
   ```
   fracLight = 127.5 / binContrastReference
   lContrast = lContrast * fracLight * darkenHighlights + lContrast * (1 - darkenHighlights)
   ```
10. **Alpha-blend with the fully-normalized value:**
    ```
    fracAlpha = alphaWhite + (alphaBlack - alphaWhite) * (1 - lContrast)
    lFinal   = fracSum * fracAlpha + lContrast * (1 - fracAlpha)
    ```
    So darker regions pick up more of the raw equalization; brighter regions stay closer to the contrasted value.

11. **Desaturation near extremes:**
    ```
    fracBWOrig  = |0.5 - lCur|
    fracBWFinal = |0.5 - lFinal|
    if fracBWOrig > 0.4 AND fracBWFinal < fracBWOrig:
        fracDesaturate    = 1 - (fracBWOrig - 0.4) * 10   // linear fade in outer 10% of L range
        distColorableLum  = (fracBWOrig - fracBWFinal) * 2
        S_dest *= fracDesaturate * (1 - distColorableLum)
    ```
    Purpose: HSL hue is unstable when L → 0 or 1. Pixels that *started* near black/white but are *moved* toward midtones would otherwise gain spurious saturation.

12. Write `lFinal` to `HSL_l_dest[x, y]`, write possibly-modified `S` to `HSL_s_dest[x, y]`.

### 3.5 Final conversion

After per-pixel processing, call `HslToRgb(mapDest)` (unless `fSkipHslToRgb`) to produce the RGB output.

### 3.6 Debug visualizations (`ReturnImage`)

Alternate outputs for tuning:
- `Dsp` — normal output.
- `MedianGrayPoint_Colored` / `_Luminance` — L = binMedian/255; optionally force S = 0.
- `MeanGrayPoint_Colored` / `_Luminance` — L = binMean/255; optionally force S = 0.
- `NormalizedValue_Colored` / `_Luminance` — L = fracSum; optionally force S = 0.

These are substituted in step 12; the rest of the pipeline still runs (minor inefficiency the C# TODO flags).

### 3.7 Parallelism and cancellation

- Both the histogram building loop (step 3.3) and the per-pixel loop (step 3.4) are parallelized over `xBlock`. Rust equivalent: `rayon::prelude::ParallelIterator` over block rows.
- `FeedbackCallback: Fn(progress: f32, message: &str) -> bool` — returns `true` to cancel. Checked inside loops at ~1% granularity. Early-return produces a null result in C# (caller should handle).

---

## 4. What this spec deliberately omits

- **Gaussian weighting:** the C# windowing is squared-linear falloff within a disc, not a true Gaussian. The user's description in the task prompt mentioned "Gaussian-weighted local luminances" — this is an approximation, not a mathematically true Gaussian kernel. For the port, we could upgrade to an actual Gaussian (which would be cheaper via separable convolution for the mean, though histograms need per-site work either way).
- **No existing golden test vectors:** the only C# test is the HSL round-trip. We'll need to hand-build reference cases (small synthetic images, known-good outputs) for the port.
- **Integration with rpview's filter UI:** not decided. Options include:
  - Add to the existing `FilterSettings` struct with a new "local contrast" slider group.
  - Keep as a separate pipeline stage invoked via a menu item (it's much slower than the LUT filters).
  - Offer it as an "export-only" processing step (can't be real-time on megapixel images).
- **Progressive preview:** the algorithm can cost seconds on multi-megapixel images. For interactive use, we may want a downsampled preview pass first, then full-res on commit.

---

## 5. Decisions (from review)

1. **Color space: OkLCh only.** No HSL port path. The algorithm operates on a scalar L plus chroma/hue — Oklab's perceptual uniformity directly improves output vs HSL.
2. **UI surface: dedicated "Local Contrast" dialog.** Also hosts the parameters for the other per-pixel processing algorithms (future expansion). Separate from the existing B/C/G filter sliders.
3. **Preview strategy: background full-res.** Algorithm runs on a worker thread; UI stays live; result streams in when ready.
4. **Module structure (preliminary):**
   - `src/utils/float_map.rs` — planar f32 bitmap.
   - `src/utils/color.rs` — sRGB ↔ linear sRGB ↔ OkLCh.
   - `src/utils/local_contrast.rs` — the algorithm.
5. **Correctness tests: build reference images.** Use C# outputs for behavioral parity (not bit-exact — see §6 on intentional deviations).
6. **Phased implementation (approved):**
   - Phase A: FloatMap + sRGB↔OkLCh, round-trip tests.
   - Phase B: `LocallyNormalizeLuminance` with default params, single-threaded.
   - Phase C: Rayon parallelization + cancellation.
   - Phase D: UI dialog integration.
   - Phase E (new): fold in the efficiency gains from §6.

---

## 6. Intentional algorithmic deviations for speed

User guidance: "similar results, exact match not important." These deviations drop significant compute at negligible-to-zero perceptual cost. Each goes behind a feature flag during development so we can A/B against the faithful port.

### 6.1 Separable Gaussian instead of squared-linear disc kernel

Replace the `(2k+1)²` weight table with two 1D Gaussian passes (σ ≈ cxyWindow / 2.5, truncated at 3σ). The C# kernel is already a smooth monotonic falloff — not far from a Gaussian, and it matches the user's original description better than the disc does.

**Win:** O(window²) → O(2·window) weighting work per pixel. At default cxyWindow=120, ~40× less work in the sampling pass.

**Catch:** histograms don't decompose separably. Options:
- (a) Separable Gaussian for the *mean* path; keep the disc kernel for histograms (median + fracSum paths).
- (b) Per-column 1D histograms summed under a 1D Gaussian row-wise — reduces histogram work to O(cxyWindow) per site but needs careful cache layout.

Start with (a).

### 6.2 Integral image (summed-area table) for the mean

When `UseMedianForContrast = false` (default), skip histograms entirely for gray-point computation. Build a 2D prefix-sum of L once; each site's mean is a 4-lookup subtract.

**Win:** O(W·H) total gray-point work regardless of window size. Orders of magnitude faster than histogram-per-site at large windows.

**Approximation note:** a 3-pass repeated box blur gives a smooth ≈Gaussian at O(W·H) total — worth it if the crisp integral-image box edges show artifacts.

**Catch:** `fracSum` still needs histograms. If we keep it, this only replaces the mean derivation. See §6.4.

### 6.3 Downsample-and-upsample the gray-point map

Compute the gray-point map on a 4× downsampled luminance plane, then bilinearly upsample. Local gray varies slowly by construction (it's a low-pass filter), so 16× less compute with no visible artifacts.

**Win:** 16× on the gray-point stage. Stacks multiplicatively with §6.1/§6.2.

**Catch:** 4× is safe; 8× may work; beyond needs testing. Bilinear upsample is the right filter (area-average down, bilinear up).

### 6.4 Drop `fracSum` from the default path

`fracSum` (per-pixel histogram-equalized value) is alpha-blended in at only ~4% weight under default `alphaBlack = alphaWhite = 0.04`. Most of the visible effect comes from the contrast curve.

**Change:** expose `fracSum` as an opt-in "local equalization strength" slider; default it off. Histograms become optional (only computed if median mode or equalization is on).

**Behavioral change**, not purely optimization. Flag appropriately during A/B testing.

### 6.5 Fewer histogram bins (if we keep histograms)

64 bins instead of 256. Median estimate and cumulative-sum curves are indistinguishable at the noise level of photographic images.

**Win:** 4× less histogram memory, 4× faster median walk, fits in L1 cache better.

### 6.6 Drop the near-extremes desaturation heuristic

C# lines 494-500. The heuristic exists because HSL hue is unstable at L→0 and L→1. Oklab's chroma scales naturally with L — near-black pixels just have small chroma that doesn't blow up when L changes.

**Win:** eliminates one branch, two abs, and a couple of multiplies per pixel. Small per-pixel, meaningful in aggregate.

**Catch:** verify with test images containing saturated near-black pixels. If Oklab needs an analogue, it'll be a simpler one.

### 6.7 Expected combined speedup

4K image, default parameters, mean-based path: combining §6.1 + §6.2 + §6.3 + §6.5 + §6.6 should give **50-200×** vs a faithful port. Median path (keeps histograms) still gains ~5-10× from §6.3 and §6.5 alone.

Turns multi-second-per-image into tens of milliseconds — approaches interactive for single-slider edits on small previews.

### 6.8 Status of the deviations

All of §6 was initially scoped as opt-in behind a `use_fast_path` flag. An early iteration shipped only the mean-via-integral-image deviation (§6.2) plus a crude `fracSum` proxy for the alpha blend. In practice the proxy diverged enough from real histogram equalization (a piecewise-linear ramp centered on the local mean, vs a true local CDF) that the fast path looked like a different algorithm, and the `alpha_black` / `alpha_white` sliders didn't behave consistently between paths. The flag and all its supporting code were removed.

Current baseline: faithful histogram-based algorithm with OkLCh, parallelized across histogram-block rows via rayon. Observed perf is acceptable (hundreds of ms per compute on multi-megapixel images at default window size). §6 remains as a future roadmap — any future optimization should preserve the histogram-derived `fracSum` rather than approximate it.

One idea worth pursuing later: a **local-variance–shaped proxy** for `fracSum` using a second integral image over `L²`. A sigmoid anchored at the local mean with width proportional to local σ would approximate the real CDF's behavior in both narrow (σ→0, stay put) and wide (σ large, stretch toward endpoints) distributions. Unlike the piecewise-linear proxy, it would respect the uniform-region invariant (`fracSum ≈ l_cur`). Cost: O(W·H) for the second prefix sum + a few float ops per pixel.

---

## 7. Next step

Phase A: `FloatMap` + sRGB↔OkLCh conversion, with round-trip unit tests to ≤1e-5 tolerance. Small, self-contained, unblocks everything else.

---

## Verification of this spec

- Cross-referenced against `HslSupport.cs:34-175`, `AdaptiveContrastDsp.cs:247-527`, and the helper functions at lines 529-658.
- HSL round-trip lemma validated by the one existing test (`TestRgbToHslToRgb.cs`).
- Pseudocode above preserves the exact branching and clamping of the original — including the one known quirk in `Contrast_Std` (mixed normalized/unnormalized scale on the white branch).
