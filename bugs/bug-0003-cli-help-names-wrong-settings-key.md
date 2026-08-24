# `--help` names a nonexistent settings key: `appearance.window_title_template` (real key: `window_title_format`)

**Severity:** low (one-word fix; but it silently defeats a documented workflow)
**Type:** CODE bug (doc string in `cli.rs`)
**Where:** `src/cli.rs:23` (“set via settings.json appearance.window_title_template”); the real field: `src/state/settings.rs:202` (`window_title_format`)
**Verified:** both lines read; grep confirms `window_title_template` appears nowhere in the code

## Description

The long help’s CONFIGURATION section tells users (and agents — the portfolio treats `--help` as the machine-readable contract) to set `appearance.window_title_template`.  A user who adds that key to settings.json sees no effect: serde ignores unknown fields, the real `window_title_format` keeps its default, and nothing warns.  A silently-dead documented knob.

## Reproduction

`rpview --help | grep window_title` → shows `window_title_template`.  Add `"window_title_template": "{filename}"` under `appearance` in settings.json → title format unchanged.

## Suggested Fix

Change the string in `src/cli.rs:23` to `appearance.window_title_format`.

## Why This Fix

Makes the help text name the key the code reads; the placeholders list in the same help block is already correct.
