use crate::state::app_state::SortMode;
use crate::state::settings::AppSettings;

/// Format the window title for the given image state.
///
/// Returns `"rpview"` when `path` is `None`.
pub(crate) fn format_window_title(
    path: Option<&std::path::Path>,
    index: usize,
    total: usize,
    sort_mode: SortMode,
    settings: &AppSettings,
) -> String {
    match path {
        Some(p) => {
            let filename = p.file_name().and_then(|n| n.to_str()).unwrap_or("Unknown");
            let position = index + 1;
            if settings.sort_navigation.show_image_counter {
                expand_template(
                    &settings.appearance.window_title_format,
                    filename,
                    position,
                    total,
                    sort_mode,
                )
            } else {
                filename.to_string()
            }
        }
        None => "rpview".to_string(),
    }
}

/// Single-pass template expansion. Recognises `{filename}`, `{index}`,
/// `{total}`, `{sortmode}`, and `{sm}`. Unknown placeholders are left as-is.
fn expand_template(
    template: &str,
    filename: &str,
    position: usize,
    total: usize,
    sort_mode: SortMode,
) -> String {
    let mut result = String::with_capacity(template.len() + filename.len());
    let mut i = 0;

    while i < template.len() {
        if template.as_bytes()[i] == b'{' {
            // `i` is always a valid char boundary (ASCII '{')
            if let Some(end) = template[i..].find('}') {
                let key = &template[i + 1..i + end];
                match key {
                    "filename" => result.push_str(filename),
                    "index" => result.push_str(&position.to_string()),
                    "total" => result.push_str(&total.to_string()),
                    "sortmode" => result.push_str(sort_mode.long_label()),
                    "sm" => result.push_str(sort_mode.short_label()),
                    _ => {
                        // Unknown placeholder — preserve verbatim
                        result.push_str(&template[i..i + end + 1]);
                    }
                }
                i += end + 1;
            } else {
                result.push('{');
                i += 1;
            }
        } else {
            // Decode one full UTF-8 character at byte offset `i`.
            let ch = template[i..].chars().next().unwrap();
            result.push(ch);
            i += ch.len_utf8();
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::format_window_title;
    use crate::state::app_state::SortMode;
    use crate::state::settings::AppSettings;
    use std::path::Path;

    #[test]
    fn test_format_window_title_all_placeholders() {
        let settings = AppSettings::default();
        let path = Path::new("/photos/sunset.png");

        let title = format_window_title(Some(path), 2, 10, SortMode::Alphabetical, &settings);

        // Default template: "{filename} ({sm}, {index}/{total})"
        assert_eq!(title, "sunset.png (A, 3/10)");
    }

    #[test]
    fn test_format_window_title_modified_date_sort() {
        let settings = AppSettings::default();
        let path = Path::new("image.jpg");

        let title = format_window_title(Some(path), 0, 5, SortMode::ModifiedDate, &settings);

        assert_eq!(title, "image.jpg (M, 1/5)");
    }

    #[test]
    fn test_format_window_title_type_modified_sort() {
        let settings = AppSettings::default();
        let path = Path::new("photo.webp");

        let title = format_window_title(Some(path), 4, 20, SortMode::TypeModified, &settings);

        assert_eq!(title, "photo.webp (TM, 5/20)");
    }

    #[test]
    fn test_format_window_title_no_path_returns_rpview() {
        let settings = AppSettings::default();

        let title = format_window_title(None, 0, 0, SortMode::Alphabetical, &settings);

        assert_eq!(title, "rpview");
    }

    #[test]
    fn test_format_window_title_counter_disabled_returns_filename_only() {
        let mut settings = AppSettings::default();
        settings.sort_navigation.show_image_counter = false;
        let path = Path::new("/dir/photo.png");

        let title = format_window_title(Some(path), 3, 10, SortMode::Alphabetical, &settings);

        assert_eq!(title, "photo.png");
    }

    #[test]
    fn test_format_window_title_custom_template_with_sortmode() {
        let mut settings = AppSettings::default();
        settings.appearance.window_title_format =
            "{filename} [{sortmode}] {index} of {total}".to_string();
        let path = Path::new("pic.gif");

        let title = format_window_title(Some(path), 1, 3, SortMode::TypeAlpha, &settings);

        assert_eq!(title, "pic.gif [type+alphabetical] 2 of 3");
    }

    #[test]
    fn test_format_window_title_both_sm_and_sortmode() {
        let mut settings = AppSettings::default();
        settings.appearance.window_title_format =
            "{filename} [{sm}] {sortmode} {index}/{total}".to_string();
        let path = Path::new("photo.png");

        let title = format_window_title(Some(path), 0, 5, SortMode::ModifiedDate, &settings);

        assert_eq!(title, "photo.png [M] modified 1/5");
    }

    #[test]
    fn test_expand_template_unknown_placeholder_preserved() {
        let mut settings = AppSettings::default();
        settings.appearance.window_title_format =
            "{filename} {unknown} {index}".to_string();
        let path = Path::new("test.png");

        let title = format_window_title(Some(path), 0, 1, SortMode::Alphabetical, &settings);

        assert_eq!(title, "test.png {unknown} 1");
    }
}
