# ICO File Creation — Security Review & Approach

## Goal

Create `packaging/windows/rpview.ico` from `packaging/macos/icon.png` (1024x1024 PNG) with embedded sizes: 16, 32, 48, 64, 128, 256.

The `.ico` file is needed to embed a Windows application icon and version resource into `rpview.exe` via the `winres` crate in `build.rs`.

## ImageMagick — Security Assessment (Rejected)

ImageMagick was the originally documented approach (`convert icon.png -define icon:auto-resize=... rpview.ico`). A full security review found it unsuitable for this narrow use case:

- **723 lifetime CVEs** — one of the highest counts of any open-source project
- **70 security advisories in just 3.5 months** (Jan–Mar 2026), including heap/stack overflows, integer overflows, policy bypasses, and use-after-free bugs
- **Written in C** with manual memory management, 200+ format parsers, and a delegate system (external program invocation) that was the source of the ImageTragick RCE (CVE-2016-3714)
- **29 transitive Homebrew dependencies** including Ghostscript (which has its own extensive CVE history)
- **Effectively 2 maintainers** despite 214 listed contributors
- **Default security policy is fully permissive** (`open`); hardening requires manual configuration
- **AI fuzzers** (Google BigSleep) are actively finding new vulnerabilities at scale

**Verdict**: Massive overkill and unacceptable attack surface for a simple PNG-to-ICO conversion.

## Recommended Approach — `ico` Rust Crate

Use the `ico` Rust crate to write a small one-time conversion script that generates the `.ico` file, which is then committed to the repository. No runtime or build-time dependency on external tools.

**Why this is better**:
- Memory-safe (Rust) — eliminates buffer overflows, use-after-free, etc.
- Only 3 dependencies: `byteorder`, `png`, `serde`
- 11.4M downloads — well-established
- ICO format is trivially simple (header + embedded PNG/BMP data)
- Minimal attack surface: only ICO and PNG formats, no shell execution, no scripting

**Security review of `ico` crate**: See separate review (pending).

## Other Alternatives Considered

| Tool | Risk Level | Notes |
|------|-----------|-------|
| ImageMagick | High | 723 CVEs, massive attack surface |
| Python Pillow | Medium | Large dependency, may not be installed |
| `png2ico` CLI | Low | 1 dependency (libpng), but only 193 installs/year on Homebrew |
| `ico` Rust crate | Very Low | Memory-safe, 3 deps, 11.4M downloads |
