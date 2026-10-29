# Plan: Reconcile the GitHub Repo Name With the Folder Name

## Problem

The folder is `~/Chris/App/Rust/rpview-gpui` (and the portfolio row is `rpview-gpui`), but `origin` is `git@github.com:ComposerChrisF/rpview.git`.  Anything that maps folder to remote by name will miss it.  `~/Chris/Proj/Coding/TODO.md` flagged this on 9-Sep-2026, beside safesync’s mismatch, and asked that each be confirmed as deliberate before a sweep is scripted over them.  safesync’s turned out to be an oversight and was fixed by renaming the GitHub repo (safesync’s plan-0006, 29-Oct-2026).  rpview’s has never been decided.

## Evidence on Whether It Is Deliberate

- **The name was chosen at creation.**  `gh repo create` defaults a new repo’s name to its folder’s, and the folder was then `~/Chris/App/Rust/rpviews/rpview-gpui` (the old permission entries in `.claude/settings.local.json` record both the path and a `gh repo create` call).  A repo called `rpview` was therefore most likely named explicitly when it was created, on 8-Jan-2026 (UTC).
- **Everything public says `rpview`:** the crate and binary (`Cargo.toml` `name = "rpview"`), the `.app` bundle, `homepage` and `repository` in `Cargo.toml`, the Windows installer’s `MyAppURL` (`packaging/windows/rpview.iss`), and the clone URLs in `README.md` and `CONTRIBUTING.md`.
- **Only the folder says `-gpui`.**  It dates from the `rpviews/` parent, which looks as though it was meant to hold more than one implementation.  Only this one exists, and its ancestor was `Dev/Util/PView` (portfolio.toml), not another `rpview-*`.
- The repo is public, with no stars and no forks, so nobody outside depends on the URL.

## Options

- **A: Rename the GitHub repo to `rpview-gpui`.**  Cheap: GitHub redirects the old URL.  Update `origin`, the two `Cargo.toml` URLs, `rpview.iss`, `README.md`, and `CONTRIBUTING.md`.  The cost is that the public name then carries the toolkit, which a user of the app has no reason to see.
- **B: Rename the folder to `rpview`.**  Makes every name agree, but it is the expensive direction: Claude Code memory is keyed by project path, and the portfolio row, effort-coord row paths (`App/Rust/rpview-gpui`), ccf-gpui-widgets’ notes on its consumers, and the `.cargo/config.toml` patch all name the folder.
- **C: Keep both and record that the mismatch is deliberate.**  The repo carries the product’s name and the folder carries the lineage.  Record it where a folder-to-remote mapper will read it (the Coding `TODO.md` item, and the portfolio row if it grows a remote field), and close the TODO line.

**Recommendation: C**, since the evidence says the public name was chosen on purpose, and A would trade a good public name for an internal convention.  If Chris would rather every repo match its folder as a rule, A is the cheap way to do it.  B is not worth its cost.

## Implementation Notes

- A: `gh repo rename rpview-gpui -R ComposerChrisF/rpview`, then `git remote set-url origin git@github.com:ComposerChrisF/rpview-gpui.git` in every checkout, then the five file edits above in one doc-only commit.
- C: no code change; a line in `~/Chris/Proj/Coding/TODO.md` closing the item with the reason.  This plan is then retired as rejected, with that reason in the commit message.

## Why Not a Workaround

A sweep script could carry an exception table for this one repo, but that only moves the undecided question into the script.  Deciding once and recording the answer is the cheaper end state.
