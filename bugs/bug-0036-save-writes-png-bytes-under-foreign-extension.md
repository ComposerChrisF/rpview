# Saving a processed GIF/SVG/ICO writes PNG bytes under the foreign extension and reports success

**Severity:** high — silent wrong output on a default configuration
**Type:** CODE bug
**Where:** `src/app_handlers.rs:621-646` (suggested-name construction) and `src/app_handlers.rs:766-784` (`save_dynamic_image_to_path`, the `_ =>` fallback arm)
**Verified:** code path read in full; `SaveFormat::SameAsLoaded` confirmed as the default (`src/state/settings.rs:138`)

## Description

Two halves that combine into a silent wrong-format save:

1. When any processing is active, `handle_save_file_impl` builds the suggested filename from `default_save_format`.  The default is `SameAsLoaded`, which reuses the loaded file’s extension verbatim — including `gif`, `ico`, and `svg`, formats the save encoder does not support.
2. `save_dynamic_image_to_path` matches on extension: `png`, `jpg/jpeg`, `bmp`, `tiff/tif`, `webp` — and the catch-all arm silently encodes **PNG**: `_ => image_data.save_with_format(&temp_path, image::ImageFormat::Png)`.  It then reports success.

So out of the box: open `anim.gif`, nudge brightness (`filters_active` becomes true), press `Cmd+S`, accept the suggested `anim_filtered.gif` — the app writes a PNG bitstream into a `.gif` file and prints “Image saved”.  Same for `.svg` (a raster PNG in a `.svg` file) and `.ico`.  Downstream tools that trust the extension then fail or misbehave, far from the cause.  The save-dialog format filters do not prevent this: the suggested name is pre-filled and `rfd` will happily return it.

## Reproduction (Rust test)

`save_dynamic_image_to_path` is a private free function; make it `pub(crate)` for the test or test via the file result:

```rust
#[test]
fn save_to_gif_extension_must_not_silently_write_png() {
    let dir = tempfile::TempDir::new().unwrap();
    let out = dir.path().join("x_filtered.gif");
    let img = image::DynamicImage::new_rgba8(4, 4);
    let result = save_dynamic_image_to_path(&img, &out);

    // Today: result is Ok and the file starts with the PNG magic bytes.
    let bytes = std::fs::read(&out).unwrap_or_default();
    let is_png = bytes.starts_with(&[0x89, b'P', b'N', b'G']);
    assert!(
        result.is_err() || !is_png,
        "must not write PNG bytes under a .gif name and claim success"
    );  // FAILS today
}
```

Manual repro: `rpview anim.gif` → press `=` on brightness (or any filter) → `Cmd+S` → keep suggested name → `file anim_filtered.gif` reports PNG image data.

## Suggested Fix

Two coordinated changes:

1. **Suggested name:** in `handle_save_file_impl`, when `SameAsLoaded` resolves to an extension outside the writable set (`png/jpg/jpeg/bmp/tiff/tif/webp`), fall back to `png` for the suggested extension.  Loaded-as-GIF, saved-as-PNG is the honest default; the user can still pick another format in the dialog.
2. **Encoder:** replace the `_ =>` arm in `save_dynamic_image_to_path` with an error naming the extension and the supported set (`Err(format!("Unsupported save format: .{extension} (supported: png, jpg, bmp, tiff, webp)"))`).  GIF could alternatively be added as a real single-frame `ImageFormat::Gif` encode — worth doing since GIF sources are common here — but the catch-all must error either way.

(See the companion bug on save-error visibility: today this error would only reach stderr.  Both fixes are needed for the user to actually see it.)

## Why This Fix

Fix 1 removes the trap from the default path (the pre-filled name is what users accept); fix 2 turns the remaining reachable cases (user types `foo.svg` by hand) from silent wrong output into a loud error — the portfolio’s standard posture: a tool must never claim success for an artifact it could not actually produce.  The test passes because the `.gif` write becomes either a real GIF encode (option) or an error, never PNG-bytes-as-`.gif`.
