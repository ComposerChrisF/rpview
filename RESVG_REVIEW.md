# resvg 0.47.0 — Security & Licensing Review

Reviewed: 2026-02-19

## Crate Metadata

| Field | Value |
|---|---|
| Repository | https://github.com/linebender/resvg |
| Total downloads | ~9.94 million |
| Latest version | 0.47.0 (published 2026-02-09) |
| Rust edition | 2024 |
| MSRV | 1.87.0 |
| Publisher | Nico Burns (@nicoburns), Linebender org |
| Original author | Yevhenii Reizner (@RazrFalcon) |

### Maintainer History

RazrFalcon created and maintained resvg from December 2017 through v0.44. In October 2024 he transferred stewardship to the [Linebender organization](https://linebender.org/), which focuses on 2D rendering, vector graphics, and text in Rust (also maintains Vello, Parley, etc.). The first Linebender release was v0.45.0 (February 2025), which included relicensing from MPL-2.0 to Apache-2.0 OR MIT.

---

## Licensing: PASS

All dependencies use permissive licenses. No copyleft licenses anywhere in the tree.

### Core Crates

| Crate | License |
|---|---|
| resvg 0.47.0 | Apache-2.0 OR MIT |
| usvg 0.47.0 | Apache-2.0 OR MIT |
| tiny-skia 0.12.0 | BSD-3-Clause |
| tiny-skia-path 0.12.0 | BSD-3-Clause |
| roxmltree 0.21.1 | MIT OR Apache-2.0 |
| svgtypes 0.16.1 | Apache-2.0 OR MIT |
| kurbo 0.13.0 | Apache-2.0 OR MIT |
| fontdb 0.23.0 | MIT |
| rustybuzz 0.20.1 | MIT |
| ttf-parser 0.25.1 | MIT OR Apache-2.0 |

### Other Licenses in Dependency Tree

| License | Crates |
|---|---|
| BSD-2-Clause | arrayref |
| Zlib | slotmap |
| Zlib OR Apache-2.0 OR MIT | bytemuck, tinyvec |
| Unlicense OR MIT | byteorder-lite, memchr |
| 0BSD OR MIT OR Apache-2.0 | adler2 |

### Attribution Notes

- **tiny-skia (BSD-3-Clause)**: Requires attribution in binary distributions.
- **arrayref (BSD-2-Clause)**: Requires attribution in binary distributions.
- These are standard requirements and are satisfied by including license files in source/distribution.

---

## Security: PASS (with caveats)

### Known Vulnerabilities

**None.** No CVEs or RustSec advisories exist for resvg, usvg, tiny-skia, roxmltree, svgtypes, or kurbo. Verified against the RustSec Advisory Database, NVD, OSV, and `cargo audit`.

### Code Safety

- **Pure Rust** — no C/C++ code in the final binary. This eliminates memory corruption (buffer overflows, use-after-free, etc.).
- **roxmltree** (the XML parser) explicitly forbids `unsafe` code (`#![forbid(unsafe_code)]`).
- Nearly no `unsafe` elsewhere in the resvg tree.

### XML Parsing (roxmltree)

| Threat | Status |
|---|---|
| XXE (external entity injection) | **Not possible** — no DTD/external entity support |
| Billion laughs (entity expansion) | **Mitigated** — no full DTD support; usvg caps at 1M elements |
| Encoding attacks | **Not possible** — UTF-8 only, rejects other encodings |
| Mutation attacks | **Not possible** — parsed tree is immutable |

### SVG Processing (usvg)

| Threat | Status |
|---|---|
| Element count DoS | **Mitigated** — hard limit of 1,000,000 elements (since v0.15.0) |
| Deep nesting DoS | **Mitigated** — hard limit of 1,024 nesting levels (since v0.15.0) |
| Stack overflow | **Mitigated** — explicit recursion guards (since v0.9.1 / v0.13.0) |
| JavaScript / `<script>` | **Not supported** — no scripting engine |
| SMIL animations | **Not supported** |
| Interactive elements | **Not supported** — no `<a>`, `<view>`, `<cursor>`, events |

### External Resource Loading

| Threat | Status |
|---|---|
| Network requests / SSRF | **Not possible** — no network I/O; URLs in `xlink:href` are ignored |
| Local file disclosure | **Possible by default** — `ImageHrefResolver` reads local file paths |
| Data URLs | **Supported** — `data:image/...;base64,...` are processed |

> **Important caveat**: The default `ImageHrefResolver` will read local files referenced by `<image xlink:href="...">`. For untrusted SVGs, replace the resolver to block local file access. In rpview's use case (viewing user-selected local files), this is acceptable behavior — the user already has filesystem access.

### Font Handling

fontdb, ttf-parser, and rustybuzz parse font data. No known vulnerabilities exist. When processing untrusted SVGs that reference custom fonts, this is a theoretical attack surface.

### Historical Security-Related Fixes

| Version | Fix |
|---|---|
| v0.15.0 (2021-06) | Added 1M element limit and 1024 nesting limit |
| v0.13.0 (2020-12) | Stack overflow fix in XML parser; moved to pure Rust |
| v0.12.0 (2020-12) | Memory leak fix in harfbuzz_rs |
| v0.35.0 (2023-06) | Panic fix for elements outside viewbox |
| v0.34.0 (2023-05) | Memory usage improvements for large paths |

---

## Dependency Tree

```
resvg v0.47.0
├── gif v0.14.1
│   ├── color_quant v1.1.0
│   └── weezl v0.1.12
├── image-webp v0.2.4
│   ├── byteorder-lite v0.1.0
│   └── quick-error v2.0.1
├── log v0.4.29
├── pico-args v0.5.0
├── rgb v0.8.52
│   └── bytemuck v1.24.0
├── svgtypes v0.16.1
│   ├── kurbo v0.13.0
│   │   ├── arrayvec v0.7.6
│   │   └── smallvec v1.15.1
│   └── siphasher v1.0.1
├── tiny-skia v0.12.0
│   ├── arrayref v0.3.9
│   ├── arrayvec v0.7.6
│   ├── bytemuck v1.24.0
│   ├── cfg-if v1.0.4
│   ├── log v0.4.29
│   ├── png v0.18.0
│   │   ├── bitflags v2.10.0
│   │   ├── crc32fast v1.5.0
│   │   ├── fdeflate v0.3.7
│   │   ├── flate2 v1.1.5
│   │   └── miniz_oxide v0.8.9
│   └── tiny-skia-path v0.12.0
│       ├── arrayref v0.3.9
│       ├── bytemuck v1.24.0
│       └── strict-num v0.1.1
├── usvg v0.47.0
│   ├── base64 v0.22.1
│   ├── data-url v0.3.2
│   ├── flate2 v1.1.5
│   ├── fontdb v0.23.0
│   │   ├── log v0.4.29
│   │   ├── memmap2 v0.9.9
│   │   ├── slotmap v1.1.1
│   │   ├── tinyvec v1.10.0
│   │   └── ttf-parser v0.25.1
│   ├── imagesize v0.14.0
│   ├── kurbo v0.13.0
│   ├── log v0.4.29
│   ├── pico-args v0.5.0
│   ├── roxmltree v0.21.1
│   │   └── memchr v2.7.6
│   ├── rustybuzz v0.20.1
│   │   ├── bitflags v2.10.0
│   │   ├── bytemuck v1.24.0
│   │   ├── core_maths v0.1.1
│   │   ├── log v0.4.29
│   │   ├── smallvec v1.15.1
│   │   ├── ttf-parser v0.25.1
│   │   ├── unicode-bidi-mirroring v0.4.0
│   │   ├── unicode-ccc v0.4.0
│   │   ├── unicode-properties v0.1.4
│   │   └── unicode-script v0.5.8
│   ├── simplecss v0.2.2
│   ├── siphasher v1.0.1
│   ├── strict-num v0.1.1
│   ├── svgtypes v0.16.1
│   ├── tiny-skia-path v0.12.0
│   ├── ttf-parser v0.25.1
│   ├── unicode-bidi v0.3.18
│   ├── unicode-script v0.5.8
│   ├── unicode-vo v0.1.0
│   └── xmlwriter v0.1.0
└── zune-jpeg v0.5.8
    └── zune-core v0.5.0
```

---

## Summary

| Area | Verdict |
|---|---|
| **Licensing** | PASS — all permissive (MIT, Apache-2.0, BSD-3-Clause, Zlib). No copyleft. |
| **Known CVEs** | PASS — none for any crate in the tree |
| **Code safety** | PASS — pure Rust, near-zero `unsafe` |
| **XML attacks** | PASS — no XXE, no billion laughs, UTF-8 only |
| **Scripting/XSS** | PASS — no JavaScript, SMIL, or interactivity |
| **Network access** | PASS — no network I/O |
| **Local file access** | CAVEAT — default resolver reads local files (acceptable for rpview's local-file viewer use case) |
| **DoS resistance** | PASS — element count and nesting depth limits enforced |
| **Maintenance** | PASS — actively maintained by Linebender, latest release 10 days ago, ~10M downloads |
