# Migrating to Longbridge‑Compatible GPUI

_Written 2026‑05‑20.  Captures a design discussion about moving rpview‑gpui and ccf‑gpui‑widgets off the published `gpui = "0.2.2"` crate and onto the Zed monorepo source that `longbridge/gpui-component` tracks._

---

## Background

### The state of GPUI as of May 2026

- **Zed 1.0 shipped April 29, 2026.**  The 1.0 announcement mentions GPUI exactly once, in passing — no commitment to extract it as a standalone framework, no roadmap.  A Zed team comment on Discussion #30515 explicitly states they don’t have resources to maintain GPUI standalone.
- **The published `gpui` crate on crates.io is stuck at `0.2.2`** (released October 22, 2025).  No update in ~7 months.  The crate works on macOS (Metal), Linux (Vulkan), and Windows (Vulkan) via Blade.
- **The Zed monorepo has drifted significantly from `0.2.2`.**  Windows now uses a custom DirectX 11 + DirectWrite backend, Linux switched to wgpu, and a wasm32 web platform landed in late February 2026 (PR #50228).
- **Community ecosystem is growing.**  `longbridge/gpui-component` jumped to 11.5k stars with v0.5.1 (Feb 5, 2026).  `gpui-ce` (community fork) is at 0.3.2.  Multiple community extraction forks exist (`EloiGG/gpui`, `Glass-HQ/gpui` at gpui.rs).

### Why migrate

Two reasons:

1. **The published crate is effectively unmaintained.**  Staying on `0.2.2` means missing seven months of GPUI improvements, the new web target, and any bug fixes that have landed since.
2. **The Longbridge signal.**  Longbridge is a Hong Kong fintech that ships a commercial trading desktop app (Longbridge Pro) built on GPUI.  They have a business incentive to keep their GPUI pin working — possibly the most credible “GPUI will outlive Zed’s direct involvement” signal currently available.  Mirroring their pin gives us implicit QA from a real production app.

### Why _not_ migrate

- **API breakage cost.**  Seven months of Zed monorepo changes will produce real compile failures across ccf‑gpui‑widgets’s ~20 widgets.
- **crates.io publish constraints.**  Switching ccf‑gpui‑widgets to a git source disallows publishing it to crates.io — see Gotcha #1.
- **Windows rendering backend change.**  Monorepo HEAD uses DirectX 11 on Windows, not Blade/Vulkan.  Different code path, possibly different bugs.
- **Compile time increases.**  Pulling GPUI from the Zed monorepo pulls in much of the Zed workspace.

If none of the above is acceptable, the alternative is to stay on `gpui = "0.2.2"` and extend ccf‑gpui‑widgets locally for anything missing.  Longer‑term, Iced and Dioxus are the more conservative Rust GUI bets if longevity is the dominant concern.

---

## The Longbridge Reference Point

Longbridge’s actual pinning, as of this writing:

**`Cargo.toml`** (no rev — tracks main):

```toml
gpui = { git = "https://github.com/zed-industries/zed" }
gpui_platform = { git = "https://github.com/zed-industries/zed", features = [...] }
gpui_web = { git = "https://github.com/zed-industries/zed" }
gpui_macros = { git = "https://github.com/zed-industries/zed" }
```

**`Cargo.lock`** (pins a specific commit):

```toml
[[package]]
name = "gpui"
version = "0.2.2"
source = "git+https://github.com/zed-industries/zed#14befe215158182be6b505b26bccf25538831213"
```

So the manifest tracks `main`, but the lockfile pins commit `14befe215158182be6b505b26bccf25538831213`.  Longbridge updates by bumping their lockfile when they’re ready.  We should mirror their _commit_, not their _branch tracking_, so our build is reproducible and we choose when to follow their bumps.

---

## Strategy: `[patch.crates-io]` in rpview, Untouched ccf‑gpui‑widgets

The cleanest expression of the migration is **patch‑based**, not by editing ccf‑gpui‑widgets’s published manifest.

In `rpview-gpui/Cargo.toml`:

```toml
[patch.crates-io]
gpui = { git = "https://github.com/zed-industries/zed", rev = "14befe215158182be6b505b26bccf25538831213" }
```

Cargo will substitute this everywhere in the dep graph — including inside ccf‑gpui‑widgets — as long as the patched version satisfies each consumer’s `gpui = "0.2.2"` requirement.  It does: the monorepo crate is still version‑tagged `0.2.2`, so Cargo treats this as a valid replacement.

**Why this is better than editing ccf‑gpui‑widgets’s `gpui` dep directly:**

- ccf‑gpui‑widgets’s published manifest stays crates.io‑compatible (git deps are forbidden in published crates).
- The pinning lives in the consuming binary, where it belongs — rpview decides which Zed commit it ships against.
- If we later have multiple downstream apps, each can patch to a different commit if needed.
- No fork of ccf‑gpui‑widgets is required.

---

## Five Gotchas

### Gotcha 1.  ccf‑gpui‑widgets publishability

crates.io does not allow git dependencies in published crates.  If we change ccf‑gpui‑widgets’s `Cargo.toml` to use `gpui = { git = "..." }`, we can no longer `cargo publish` it.

**Resolution:** don’t change ccf‑gpui‑widgets’s manifest at all.  Use `[patch.crates-io]` in rpview (and in any other consuming binary) instead.  ccf‑gpui‑widgets’s manifest continues to say `gpui = "0.2.2"`; the patch line redirects that to the monorepo commit at build time.  ccf‑gpui‑widgets remains publishable.

### Gotcha 2.  Pin the commit, not the branch

Longbridge writes `git = "..."` with no `rev` and lets their lockfile do the pinning.  That works for them because they control their own lockfile updates.  For us, putting `rev = "..."` directly in the `[patch.crates-io]` line is safer:

- Reproducible builds regardless of lockfile state.
- We bump explicitly when we want to follow Longbridge to a newer commit.
- Documents the version we’re shipping in source control, not buried in `Cargo.lock`.

To find the current commit Longbridge is on: open their `Cargo.lock` on GitHub, search for the `[[package]] name = "gpui"` block, copy the hash after the `#`.  Repeat whenever they cut a new release.

### Gotcha 3.  API breakage between published 0.2.2 and Zed HEAD

There’s ~7 months of drift in the Zed monorepo since the published `0.2.2`.  Expect:

- Renames in core GPUI types and traits.
- Signature changes in `Render`, `IntoElement`, `cx.spawn`, view creation patterns.
- New required parameters or trait methods.
- Possible behavior changes in styling and layout primitives.

Most of this will land as compile errors, which are mechanical to fix.  The bulk of the work is in ccf‑gpui‑widgets, which has roughly 20 widgets all calling GPUI APIs.  rpview’s direct GPUI usage is thinner and will break in fewer places.

### Gotcha 4.  Windows rendering backend switch

| Backend on Windows | Source |
|---|---|
| Blade / Vulkan | published `gpui = "0.2.2"` (crates.io) |
| Custom DirectX 11 + DirectWrite | Zed monorepo HEAD |

This is invisible to application code, but it’s a different rendering path.  Possible surface‑level differences:

- HiDPI scaling behavior.
- Text rendering (DirectWrite vs.  whatever Blade uses on Windows).
- Window event timing or focus semantics.
- GPU memory or driver compatibility on older Windows hardware.

**Resolution:** plan a dedicated Windows test pass after the retarget compiles on macOS.  Don’t assume macOS green means Windows green.

### Gotcha 5.  Compile times

Pulling GPUI from the Zed monorepo pulls a large subset of the Zed workspace as transitive dependencies.  Expect cold‑build times to go up noticeably.  Incremental builds remain fast.  CI configuration may need cache adjustments.

---

## Migration Sequence

1.  **Read Longbridge’s `Cargo.lock`.**  Grab the current `gpui` commit hash.  At time of writing it’s `14befe215158182be6b505b26bccf25538831213`.

2.  **Add the patch to rpview‑gpui’s `Cargo.toml`:**

    ```toml
    [patch.crates-io]
    gpui = { git = "https://github.com/zed-industries/zed", rev = "14befe215158182be6b505b26bccf25538831213" }
    ```

3.  **Run `cargo check` from rpview‑gpui.**  Read the failure list.  Most errors will originate in ccf‑gpui‑widgets, not in rpview itself.

4.  **Fix ccf‑gpui‑widgets first.**  Work on a feature branch — call it `feat/gpui-monorepo-compat`.  Don’t bump the published version yet.  Use a local path override in rpview while iterating:

    ```toml
    [patch.crates-io]
    gpui = { git = "https://github.com/zed-industries/zed", rev = "14befe21..." }
    ccf-gpui-widgets = { path = "../ccf-gpui-widgets" }
    ```

5.  **Once rpview compiles, run it.**  Most things should look right.  Visual regressions and behavioral differences are likely — work through them.

6.  **Test on Windows.**  This is the highest‑risk platform delta.  Verify HiDPI rendering, text appearance, window events, and any drag‑and‑drop or clipboard behavior rpview uses.

7.  **Once both platforms are green:**
    - Land the ccf‑gpui‑widgets compatibility work on its main branch.  Decide whether to ship a `0.1.3` release that still says `gpui = "0.2.2"` (since the actual GPUI version is determined by the patch in the consuming binary, ccf‑gpui‑widgets can keep its manifest unchanged and just adjust its code to be compatible with both the published 0.2.2 _and_ the monorepo HEAD — if that’s feasible).  If the API drift is too large to support both, mark `0.1.x` as the legacy line and start a `0.2.x` line that targets monorepo only, with the manifest still saying `gpui = "0.2.2"` since the patch handles the substitution.
    - Land the patch line in rpview’s main branch.  Document the chosen commit hash in this file or a comment near the patch.

8.  **Establish a bump cadence.**  Each time Longbridge cuts a new release (their cadence so far is quarterly: v0.5.1 was February 2026, no release since), pull their new `gpui` commit hash, update the `rev` in the patch line, run the full test pass, and ship.

---

## Optional Follow‑Up: Adopting gpui‑component Itself

Once on the same GPUI source, we _can_ adopt `gpui-component` widgets where they’re useful — they’ll resolve to the same GPUI instance and compose cleanly with ccf‑gpui‑widgets.  Overlap is minimal; the two libraries solve different problems:

- **ccf‑gpui‑widgets**: form inputs, file/directory pickers, color picker, repeatable lists, confirmation dialogs.
- **gpui‑component**: dock layout, virtualized tables/lists, 200K‑line code editor with LSP, charts, markdown rendering, i18n.

This is orthogonal to the migration itself.  Don’t adopt gpui‑component widgets as part of the retarget — do it later, in a separate pass, only where the heavy components are genuinely needed.

---

## Decision Record

- **2026‑05‑20:** Decision pending.  Document drafted to capture the strategy and gotchas so the migration can be undertaken deliberately, not improvised.
- _Next entry:_ when the migration actually begins or is deferred.

---

## References

- [longbridge/gpui-component on GitHub](https://github.com/longbridge/gpui-component) — the reference consumer
- [Longbridge’s `Cargo.toml`](https://github.com/longbridge/gpui-component/blob/main/Cargo.toml) — manifest pinning style
- [Longbridge’s `Cargo.lock`](https://github.com/longbridge/gpui-component/blob/main/Cargo.lock) — current commit reference
- [Please extract GPUI — Discussion #30515](https://github.com/zed-industries/zed/discussions/30515) — Zed team’s standalone framework position
- [Zed 1.0 announcement](https://zed.dev/blog/zed-1-0) — current Zed status
- [GPUI on the web — PR #50228](https://github.com/zed-industries/zed/pull/50228) — wasm32 target landing
- [Cargo `[patch]` documentation](https://doc.rust-lang.org/cargo/reference/overriding-dependencies.html) — how the patch mechanism works
