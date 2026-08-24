# CLI.md’s error-handling contract contradicts the code (and the portfolio exit-code rule) — the SPEC is wrong, not the code

**Severity:** medium (documentation) — an agent “fixing the code to match CLI.md” would introduce a silent-continue anti-pattern
**Type:** SPEC bug — fix CLI.md; do NOT change the code’s fail-fast behavior
**Where:** `CLI.md:92-94` (“File Not Found: … display an error message and **continue with other valid files**”), `:85-88` (exit codes: only “0 / Non-zero”), `:170-180` (Implementation Notes struct with field `image_paths`); code truth: `src/cli.rs:94-121` (`collect_image_paths` returns `Err(FileNotFound)` on the first missing path → `main` exits 1), `src/cli.rs:13-28` (`after_long_help` documents the real 0/1/2 table)
**Verified:** both files read in full

## Description

Three drift items in CLI.md, the first one load-bearing:

1. **“Continue with other valid files” is not, and should not be, the behavior.**  The code fails the whole invocation with exit 1 when any explicitly named path is missing.  This is the portfolio contract (`cli-exit-codes.md`: a caller-asserted input path that is missing must exit 1, never silent success) — the world doesn’t match what the invocation asserted.  CLI.md predates that rule and describes a lenient behavior that was never implemented (git: `collect_image_paths` has returned `Err` on the first missing path in every version).
2. **Exit codes under-specified:** CLI.md says only “0 / Non-zero”; the binary’s `--help` documents the real table (0 success, 1 argument-resolution failure, 2 clap usage error).  Spec under-specification is exactly how divergent reimplementations happen (cited incident class in the portfolio rules).
3. **Stale implementation sketch:** the struct example shows field `image_paths`; the real field is `paths` with `value_name = "PATH"` (usage line `rpview [PATH]...`, not `[IMAGE_PATHS]...`).

## Reproduction

`rpview real.png missing.png` → exits 1 with “File not found: missing.png”, displaying nothing — CLI.md says it would show the valid file.  (This behavior is correct; the doc is wrong.)

## Suggested Fix (CLI.md only)

1. Rewrite “Error Handling → File Not Found” to: any explicitly named path that does not exist fails the invocation with exit 1, naming the path; nothing is opened.  (Files that disappear from a _scanned directory_ later are a different, in-app matter.)
2. Replace the exit-codes section with the 0/1/2 table from `after_long_help`, and state that `--help` is authoritative.
3. Fix the Implementation Notes struct/usage line, or better, drop the code sketch entirely (it duplicates the source and rots — this is the second time).

## Why This Fix

The code implements the portfolio-wide contract that exists precisely to prevent plausible-but-wrong silent output; aligning the spec to it (rather than vice versa) is mandated by `cli-exit-codes.md`, and recording the full exit table closes the under-specification that let the two drift apart unnoticed.
