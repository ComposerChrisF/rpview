# bug-0062: The Committed `Cargo.lock` Carries the Local ccf-gpui-widgets Patch

**Severity:** medium.  CI and the GitHub release builds silently link a different ccf-gpui-widgets from the one in the installed app.

## Contract

The committed `Cargo.lock` resolves the same way on every machine, so the build CI checks, and the build `release.yml` ships, is the build Chris runs.  (`commit-common.md` C7b exists because a local-path dependency once broke that for three weeks in Obsidian-Brain.)

## Observed (HEAD `cccdf3b`, 29-Oct-2026)

- The main checkout has a gitignored `.cargo/config.toml`:

  ```toml
  [patch.crates-io]
  ccf-gpui-widgets = { path = "../ccf-gpui-widgets" }
  ```

- Every `cargo` command run there writes the patch into the lockfile.  The committed entry is `name = "ccf-gpui-widgets"`, `version = "0.1.2"`, with **no `source` line**: a path package.
- crates.io’s newest release is **0.1.0**, and the local checkout is at **0.1.3**.  Without the patch (CI, a fresh clone), cargo throws the entry away and re-resolves to crates.io: CI run 40952714662 (29-Oct-2026) logs `Checking ccf-gpui-widgets v0.1.0`.
- So CI tests, and the tag-triggered `release.yml` ships, rpview built against widgets 0.1.0.  The app `install-bin` puts in `/Applications` is built against the 0.1.3 working tree.  Neither build says so.
- The committed entry is stale even against the patch (0.1.2 against 0.1.3), so the next `cargo` command in any checkout rewrites it.  That rewrite then rides along in unrelated commits.
- Linked worktrees under `.claude/worktrees/` inherit the patch, because cargo looks for `.cargo/config.toml` in parent directories.

## Reproduction

Outside the rpview tree, with no patch in scope:

```
mkdir -p /tmp/lockcheck/src && cd /tmp/lockcheck
cp <rpview>/Cargo.toml . && git -C <rpview> show HEAD:Cargo.lock > Cargo.lock
touch src/main.rs src/lib.rs build.rs
cargo metadata --locked --offline --format-version 1 > /dev/null
```

It exits 101: `cannot update the lock file … because --locked was passed`.  A committed lockfile that cannot be used `--locked` without one machine’s config is the defect.

## Fix Directions (Not Decided)

- **Depend on what is published.**  Release ccf-gpui-widgets to crates.io when rpview needs a change from it, raise the requirement, and generate the committed lock without the patch.
- **Depend by git revision** (`git = …, rev = …`), so the lock records a source every machine can fetch, and keep the path patch for local work only.
- **Detection, whichever is chosen:** run CI’s cargo commands with `--locked`, so a lock that needs a local config fails loudly instead of being silently re-resolved.  On its own this turns CI red until one of the above lands.

Found while investigating the CI failures (effort-coord request `row:App/Rust/rpview-gpui/r0001`, effort `follow-ups-from-the-29-oct`).
