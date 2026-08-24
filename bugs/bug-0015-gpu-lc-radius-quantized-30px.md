# LC “Radius” response is quantized to 30 px steps by integer-truncated stride — dragging inside a band changes nothing, then jumps

**Severity:** low — visible UX staircase on the LC radius slider; faithful PSP3 port
**Type:** NEEDS-DECISION — fixing it here alone diverges from the PSP3 reference shader; Chris must choose parity vs quality (and possibly fix PSP3 too)
**Where:** `src/gpu/shaders/local_contrast.wgsl:48-49`; parity source: `~/Chris/Proj/PixelShaderPaint3/shaders-builtin/Color Adjustments/local-contrast.wgsl:112-113` (identical lines)
**Verified:** shader math reviewed against MAX_SAMPLES = 30; strip coverage and shared-memory bounds checked sound by the GPU review pass

## Description

```wgsl
let stride = max(1u, u32(u.Radius / f32(MAX_SAMPLES)));
let samples = min(MAX_SAMPLES, u32(ceil(u.Radius / f32(stride))));
```

For Radius ≥ 30 the effective Gaussian reach is `samples × stride = 30 × floor(Radius/30)` (sigma scales likewise).  Dragging Radius anywhere within a 30-px band changes nothing; at each multiple of 30 the reach jumps by 30 px.  Worst case Radius = 59 → actual reach 30 px (−49%); at 60 it doubles instantly.

## Reproduction

1000-px-wide gradient image, LC Strength 1.0: sweep Radius 31 → 59 (no visual change), then 59 → 60 (visible jump).  A compute-level test can render a 1-D ramp at Radius 31 and 59 and assert the outputs differ (fails today: byte-identical).

## Suggested Fix (pending the parity decision)

Fractional stride: sample at `stride_f = max(1.0, Radius / f32(MAX_SAMPLES))` with linear-filtered taps at fractional offsets (the pass already samples via a sampler-capable texture), making reach continuous in Radius.  A cheaper half-fix (`stride = ceil(...)`) overshoots instead of undershooting — not recommended.  If PSP3 parity is the priority, document the staircase in the panel (tooltip) and leave the shader as-is; if quality wins, apply the fractional-stride fix in both repos in the same pass so the reference and the port stay aligned.

## Why This Fix

Continuous stride removes the dead zones without changing the kernel’s shape at the exact multiples of 30 where today’s code is correct; doing it in both repos (or explicitly documenting the deviation) preserves the cross-repo parity that the port deliberately maintains.
