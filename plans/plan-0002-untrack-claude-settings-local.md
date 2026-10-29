# Plan: Stop Tracking `.claude/settings.local.json`

## Problem

`.claude/settings.local.json` is Claude Code’s machine-local permissions file, normally untracked.  This repo has tracked it since `8fc4a2f` (6-Jan-2026).  `.gitignore` gained a `.claude/settings.local.json` line in `c7a481f` (3-Feb-2026), but ignoring a file does not untrack it, so every change since has still been committed (`0d29131`, `6acfd24`, `07c453f`).

What it holds, and why that matters:

- **Dead entries.**  Most path-bearing rules name `/Users/chris/Chris/App/Rust/rpviews/rpview-gpui`, a path that no longer exists, including one-off commands such as `git push -u origin master`.
- **Broad grants that every checkout inherits.**  `Bash(git:*)`, `Bash(git push:*)`, and `Bash(git checkout:*)` reach every clone and every linked worktree, because the file arrives with the checkout.
- **It is published.**  `ComposerChrisF/rpview` is a public repo, so the file’s local paths and permission rules are on GitHub.

The ignore line already records the intent not to track the file; only the untracking step was never done.

## Proposed Change

Untrack the file and keep the local copy: `git rm --cached .claude/settings.local.json`, committed as doc-only.  The existing `.gitignore` line then takes effect.  Optionally, prune the dead `rpviews/` entries from the local copy afterwards; once untracked, that edit is local and never committed.

## Implementation Notes

- **Do it in the main checkout, not in a linked worktree.**  When a checkout fast-forwards over a commit that deletes a tracked file, git deletes that file from its working tree.  An untracking commit made in a worktree and then fast-forwarded into the main checkout would delete Chris’s live permissions file there.  Run `git rm --cached` in the main checkout itself, or copy the file aside before the fast-forward and restore it after.  Other clones and worktrees lose their copy when they pull the commit, which is the intended end state.
- The file stays in git history, which is public.  Rewriting history would need a force-push and is not proposed; nothing in the file is a secret.
- **A neighbour, not in this plan:** `.install-bin.toml` is tracked too and also holds machine paths, among them a Dropbox share destination.  It is the same question (is this machine-local?) and deserves its own answer.

## Why Not a Workaround

Pruning the stale entries while leaving the file tracked would leave the broad grants inherited by every checkout and the file still published.  The `.gitignore` line shows the decision was already made; this plan only finishes it.
