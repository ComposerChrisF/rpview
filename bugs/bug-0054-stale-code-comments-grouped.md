# Grouped: stale code comments and misleading test names (no behavior change)

**Severity:** low (grouped hygiene — each item is a one-line edit; batched to keep the bug list navigable)
**Type:** CODE bug (comments/docs-in-code only)
**Where/what:** each item verified against the current code:

1. `src/app_keybindings.rs:57` — “Slow pan with Alt (1px)”: actual default is 3 px (`settings.rs:112`; DESIGN.md agrees on 3 px).
2. `src/app_keybindings.rs:48` and the help overlay’s “Fast pan (3x speed)” — actual behavior is 30 px × zoom (image pixels), per `fast_pan_speed()` (`src/app_handlers.rs:1403-1405`); DESIGN.md:130 is correct.  Reword to “30 px in image pixels (scales with zoom)”.
3. `src/app_handlers.rs:61`, `src/main.rs:417,456` — comments still name the removed “Local Contrast” window; the second floating window is the GPU Pipeline.
4. `src/state/settings.rs:94-95` — doc comment says `pan_speed_slow` is “with Cmd/Ctrl modifier”; the binding is Alt.
5. `src/utils/settings_io.rs:33` — doc comment says the last-resort fallback is `./rpview_settings.json`; the code produces `./settings.json` (`:45,53`).
6. `src/utils/settings_io.rs:303` — test `test_save_creates_parent_directory` asserts the save **fails** (the body comment says so); rename to `test_save_does_not_create_parent_directory`.
7. `src/gpu/mod.rs:16` — “Pipeline order: LC → SBC → Vibrance → Hue”: missing Equalize and Document Contrast; “SBC” is stale naming (authoritative order in `unified.rs:8-13`).
8. `src/gpu/unified.rs:34-36` — `LcParams::radius` documented “4–200, default 60”; controls map 4–1000 px × resize factor.
9. `src/gpu/unified.rs:185-188` — `HueParams::hue` documented “0.0–1.0 … 0.5 = 180°”; controls send −0.5…+0.5 (math wraps, doc misleads).
10. `src/gpu/mod.rs:239-243` — test header for `equalize_zero_runs_full_pipeline` claims the histogram pass runs; identity params take the CPU passthrough (`unified.rs:653-656`), as the inline comment at `mod.rs:258-260` correctly states.  Fix the header (or make the test set a non-identity param if exercising the encoder split is wanted).

**Verified:** every line above checked during the review passes

## Reproduction

Not applicable (comments only); each item is verifiable by reading the cited line against the cited behavior source.

## Suggested Fix

Apply the ten one-line edits.  None changes behavior; item 6 renames a test; item 10 may optionally strengthen a test.

## Why This Fix

Stale comments are how the next reader (or agent) re-learns wrong facts; each item names its authoritative source so the edit is mechanical.
