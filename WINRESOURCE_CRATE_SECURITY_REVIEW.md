# Security Review: `winresource` Crate (v0.1.30)

## Project Overview

| Field | Value |
|---|---|
| **Crate name** | `winresource` |
| **Current version** | 0.1.30 |
| **Description** | Create and set Windows icons and metadata for executables |
| **License** | MIT |
| **Repository** | https://github.com/BenjaminRi/winresource |
| **Crates.io** | https://crates.io/crates/winresource |
| **Total downloads** | ~1,474,909 |
| **Code size** | 977 lines of Rust (single file: `lib.rs`) |
| **Rust edition** | 2021 |
| **Relationship** | Actively maintained fork of `winres` (abandoned since 2021, broken on Rust 1.61+) |

## Maintenance Activity

| Field | Value |
|---|---|
| **First release** | 2022-11-20 (v0.1.12) |
| **Latest release** | 2026-01-23 (v0.1.30) |
| **Total versions** | 19 (3 yanked) |
| **Last commit** | 2026-01-23 |
| **Total commits** | ~159 |
| **Open issues** | 8 |
| **Open PRs** | 3 |

Actively maintained with regular releases every 1–3 months. Recent commits include fixing MSVC resource linking and removing unnecessary `cargo:rustc-link-search` directives.

## Author / Maintainer

**Benjamin Richner** (`BenjaminRi` on GitHub). Forked `winres` when the original by Max Resch was abandoned. 10+ contributors have submitted patches.

## Dependencies

### Direct Dependencies (runtime)

| Dependency | Version | Downloads | License | RustSec Advisories |
|---|---|---|---|---|
| `version_check` | ^0.9 | 615M | MIT/Apache-2.0 | None |
| `toml` | ^0.9 (optional, default) | 527M | MIT/Apache-2.0 | None |

### Transitive Dependencies (via `toml`)

`serde`, `serde_spanned`, `toml_datetime`, `toml_parser`, `toml_writer`, `winnow` — all well-established, zero advisories.

**Total transitive dependency count:** ~6–8 crates (with `toml` feature), or just 1 (`version_check`) without.

## Security History

- **CVEs**: None
- **RustSec advisories**: None (crate and all dependencies)
- **GitHub Security Advisories**: None

## Code Quality

| Aspect | Finding |
|---|---|
| **Unsafe code** | **Zero** — no `unsafe` keyword anywhere |
| **Codebase size** | 977 lines — small, auditable |
| **CI/CD** | GitHub Actions (Windows tests, Linux cross-compilation) |
| **Tests** | Unit tests for string escaping, Windows SDK path resolution |
| **Documentation** | 96.97% coverage (docs.rs) |
| **Formal audit** | No known formal security audit |

## How It Works

Build-time only — does **not** execute at runtime:

1. Reads `Cargo.toml` for `[package.metadata.winresource]` values
2. Generates a `.rc` (Windows Resource) file in `OUT_DIR`
3. Invokes system resource compiler (`rc.exe` for MSVC, `windres` for GNU)
4. Emits `cargo:rustc-link-arg` directives to link the compiled resource

**Does NOT download anything.** No network access. Writes only to `OUT_DIR`. External tools invoked via `Command::new()` (no shell interpretation).

## Used By

- **Nushell** — popular cross-platform shell
- **Czkawka** — duplicate file finder
- Various Windows desktop applications (53 total dependents)

## Risk Rating: LOW

**Positive factors:**
- Zero `unsafe` code
- Zero CVEs and zero RustSec advisories
- Small, auditable codebase (977 lines, single file)
- Build-time only — not in final binary
- No network access
- Minimal, high-quality dependency tree
- Actively maintained with CI
- ~1.5M downloads

**Minor concerns:**
- No `#![forbid(unsafe_code)]` attribute (though no unsafe exists)
- No formal security audit
- 3 yanked versions (normal practice for quick bug fixes)
- Shells out to system tools (`rc.exe`, `windres`) — trusted but an implicit trust boundary

## Verdict

**Safe to use** as a Windows build dependency for embedding icons and version resources into executables.

### Note on `winres` vs `winresource`

The original `winres` crate (9.9M downloads) is **abandoned and broken on Rust 1.61+**. Use `winresource` instead — it is the maintained successor with a compatible API.

---

*Review date: 2026-03-13*
