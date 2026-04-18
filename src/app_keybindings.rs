use super::*;

pub(crate) fn setup_key_bindings(cx: &mut gpui::App) {
    cx.bind_keys([
        KeyBinding::new("cmd-w", CloseWindow, None),
        KeyBinding::new("cmd-q", Quit, None),
        KeyBinding::new("escape", EscapePressed, None),
        KeyBinding::new("right", NextImage, Some("ImageViewer")),
        KeyBinding::new("left", PreviousImage, Some("ImageViewer")),
        // Animation controls
        KeyBinding::new("o", ToggleAnimationPlayPause, None),
        KeyBinding::new("]", NextFrame, None),
        KeyBinding::new("[", PreviousFrame, None),
        KeyBinding::new("shift-cmd-a", SortAlphabetical, None),
        KeyBinding::new("shift-cmd-m", SortByModified, None),
        KeyBinding::new("shift-cmd-t", SortByTypeToggle, None),
        KeyBinding::new("shift-cmd-l", ToggleLocalContrast, None),
        KeyBinding::new("cmd-p", ApplyLocalContrast, None),
        // Zoom controls - base (normal speed)
        KeyBinding::new("=", ZoomIn, None), // = key (same as +)
        KeyBinding::new("+", ZoomIn, None),
        KeyBinding::new("-", ZoomOut, None),
        KeyBinding::new("0", ZoomReset, None),
        KeyBinding::new("cmd-0", ZoomResetAndCenter, None),
        // Zoom controls - fast (with Shift)
        KeyBinding::new("shift-=", ZoomInFast, None),
        KeyBinding::new("shift-+", ZoomInFast, None),
        KeyBinding::new("shift--", ZoomOutFast, None),
        KeyBinding::new("_", ZoomOutFast, None), // Shift+- produces _ on US keyboard
        // Zoom controls - slow (with Cmd/Ctrl)
        KeyBinding::new("cmd-=", ZoomInSlow, None),
        KeyBinding::new("cmd-+", ZoomInSlow, None),
        KeyBinding::new("cmd--", ZoomOutSlow, None),
        // Zoom controls - incremental (with Shift+Cmd/Ctrl)
        KeyBinding::new("shift-cmd-=", ZoomInIncremental, None),
        KeyBinding::new("shift-cmd-+", ZoomInIncremental, None),
        KeyBinding::new("shift-cmd--", ZoomOutIncremental, None),
        KeyBinding::new("cmd-_", ZoomOutIncremental, None), // Shift+Cmd+- produces Cmd+_ on US keyboard
        // Pan controls with WASD (base speed: 10px)
        KeyBinding::new("w", PanUp, None),
        KeyBinding::new("a", PanLeft, None),
        KeyBinding::new("s", PanDown, None),
        KeyBinding::new("d", PanRight, None),
        // Pan controls with IJKL (base speed: 10px)
        KeyBinding::new("i", PanUp, None),
        KeyBinding::new("j", PanLeft, None),
        KeyBinding::new("k", PanDown, None),
        KeyBinding::new("l", PanRight, None),
        // Fast pan with Shift (3x speed: 30px)
        KeyBinding::new("shift-w", PanUpFast, None),
        KeyBinding::new("shift-a", PanLeftFast, None),
        KeyBinding::new("shift-s", PanDownFast, None),
        KeyBinding::new("shift-d", PanRightFast, None),
        KeyBinding::new("shift-i", PanUpFast, None),
        KeyBinding::new("shift-j", PanLeftFast, None),
        KeyBinding::new("shift-k", PanDownFast, None),
        KeyBinding::new("shift-l", PanRightFast, None),
        // Slow pan with Alt (1px) - using Alt to avoid conflicts with Cmd/Ctrl shortcuts
        KeyBinding::new("alt-w", PanUpSlow, None),
        KeyBinding::new("alt-a", PanLeftSlow, None),
        KeyBinding::new("alt-s", PanDownSlow, None),
        KeyBinding::new("alt-d", PanRightSlow, None),
        KeyBinding::new("alt-i", PanUpSlow, None),
        KeyBinding::new("alt-j", PanLeftSlow, None),
        KeyBinding::new("alt-k", PanDownSlow, None),
        KeyBinding::new("alt-l", PanRightSlow, None),
        // Help and debug overlays
        KeyBinding::new("h", ToggleHelp, None),
        KeyBinding::new("?", ToggleHelp, None),
        KeyBinding::new("f1", ToggleHelp, None),
        KeyBinding::new("f12", ToggleDebug, None),
        KeyBinding::new("t", ToggleZoomIndicator, None),
        KeyBinding::new("b", ToggleBackground, None),
        // Settings window
        KeyBinding::new("cmd-,", ToggleSettings, None),
        KeyBinding::new("escape", CloseSettings, Some("SettingsWindow")),
        KeyBinding::new("cmd-enter", CloseSettings, Some("SettingsWindow")),
        // Filter controls
        KeyBinding::new("cmd-f", ToggleFilters, None),
        KeyBinding::new("1", DisableFilters, Some("ImageViewer")),
        KeyBinding::new("2", EnableFilters, Some("ImageViewer")),
        KeyBinding::new("3", RecallSlot3, Some("ImageViewer")),
        KeyBinding::new("4", RecallSlot4, Some("ImageViewer")),
        KeyBinding::new("5", RecallSlot5, Some("ImageViewer")),
        KeyBinding::new("6", RecallSlot6, Some("ImageViewer")),
        KeyBinding::new("7", RecallSlot7, Some("ImageViewer")),
        KeyBinding::new("8", RecallSlot8, Some("ImageViewer")),
        KeyBinding::new("9", RecallSlot9, Some("ImageViewer")),
        KeyBinding::new("ctrl-3", StoreSlot3, Some("ImageViewer")),
        KeyBinding::new("ctrl-4", StoreSlot4, Some("ImageViewer")),
        KeyBinding::new("ctrl-5", StoreSlot5, Some("ImageViewer")),
        KeyBinding::new("ctrl-6", StoreSlot6, Some("ImageViewer")),
        KeyBinding::new("ctrl-7", StoreSlot7, Some("ImageViewer")),
        KeyBinding::new("ctrl-8", StoreSlot8, Some("ImageViewer")),
        KeyBinding::new("ctrl-9", StoreSlot9, Some("ImageViewer")),
        KeyBinding::new("shift-cmd-r", ResetFilters, None),
        // File operations
        KeyBinding::new("cmd-o", OpenFile, None),
        KeyBinding::new("cmd-s", SaveFile, None),
        KeyBinding::new("cmd-alt-s", SaveFileToDownloads, None),
        KeyBinding::new("cmd-r", RevealInFinder, None),
        // External viewer
        KeyBinding::new("cmd-alt-v", OpenInExternalViewer, None),
        KeyBinding::new("shift-cmd-alt-v", OpenInExternalViewerAndQuit, None),
        // External editor
        KeyBinding::new("cmd-e", OpenInExternalEditor, None),
        // Delete operations
        KeyBinding::new("cmd-backspace", RequestDelete, None),
        KeyBinding::new("shift-cmd-backspace", RequestPermanentDelete, None),
        // Windows/Linux explicit Ctrl bindings (GPUI 0.2.2 doesn't translate cmd to ctrl)
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-w", CloseWindow, None),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-q", Quit, None),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("shift-ctrl-a", SortAlphabetical, None),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("shift-ctrl-m", SortByModified, None),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("shift-ctrl-t", SortByTypeToggle, None),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("shift-ctrl-l", ToggleLocalContrast, None),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-p", ApplyLocalContrast, None),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-0", ZoomResetAndCenter, None),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-=", ZoomInSlow, None),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-+", ZoomInSlow, None),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl--", ZoomOutSlow, None),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("shift-ctrl-=", ZoomInIncremental, None),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("shift-ctrl-+", ZoomInIncremental, None),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("shift-ctrl--", ZoomOutIncremental, None),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-_", ZoomOutIncremental, None),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-,", ToggleSettings, None),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-enter", CloseSettings, Some("SettingsWindow")),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-f", ToggleFilters, None),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("shift-ctrl-r", ResetFilters, None),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-o", OpenFile, None),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-s", SaveFile, None),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-alt-s", SaveFileToDownloads, None),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-r", RevealInFinder, None),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-alt-v", OpenInExternalViewer, None),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("shift-ctrl-alt-v", OpenInExternalViewerAndQuit, None),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-e", OpenInExternalEditor, None),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-backspace", RequestDelete, None),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("shift-ctrl-backspace", RequestPermanentDelete, None),
    ]);
}

/// Set up native application menus (macOS menu bar, Windows/Linux menus)
pub(crate) fn setup_menus(cx: &mut gpui::App) {
    cx.set_menus(vec![
        // Application menu (macOS only - shows as "RPView" menu)
        Menu {
            name: "RPView".into(),
            items: vec![
                #[cfg(target_os = "macos")]
                MenuItem::action("Preferences...", ToggleSettings),
                #[cfg(target_os = "macos")]
                MenuItem::separator(),
                MenuItem::action("Quit", Quit),
            ],
        },
        // Edit menu (for Windows/Linux settings)
        #[cfg(not(target_os = "macos"))]
        Menu {
            name: "Edit".into(),
            items: vec![MenuItem::action("Settings...", ToggleSettings)],
        },
        Menu {
            name: "File".into(),
            items: vec![
                MenuItem::action("Open File...", OpenFile),
                MenuItem::action("Save File...", SaveFile),
                MenuItem::action("Save to Downloads...", SaveFileToDownloads),
                MenuItem::separator(),
                MenuItem::action("Reveal in Finder", RevealInFinder),
                MenuItem::action("Open in External Viewer", OpenInExternalViewer),
                MenuItem::action("Open in Viewer and Quit", OpenInExternalViewerAndQuit),
                MenuItem::action("Open in External Editor", OpenInExternalEditor),
                MenuItem::separator(),
                MenuItem::action("Delete File...", RequestDelete),
                MenuItem::action("Permanently Delete File...", RequestPermanentDelete),
                MenuItem::separator(),
                MenuItem::action("Close Window", CloseWindow),
            ],
        },
        Menu {
            name: "View".into(),
            items: vec![
                MenuItem::action("Zoom In", ZoomIn),
                MenuItem::action("Zoom Out", ZoomOut),
                MenuItem::action("Reset Zoom", ZoomReset),
                MenuItem::separator(),
                MenuItem::action("Toggle Filters", ToggleFilters),
                MenuItem::action("Disable Filters", DisableFilters),
                MenuItem::action("Enable Filters", EnableFilters),
                MenuItem::action("Reset Filters", ResetFilters),
                MenuItem::separator(),
                MenuItem::action("Local Contrast...", ToggleLocalContrast),
                MenuItem::action("Apply Local Contrast", ApplyLocalContrast),
                MenuItem::action("Reset Local Contrast", ResetLocalContrast),
                MenuItem::separator(),
                MenuItem::action("Toggle Help", ToggleHelp),
                MenuItem::action("Toggle Debug", ToggleDebug),
                MenuItem::action("Toggle Zoom Indicator", ToggleZoomIndicator),
                MenuItem::action("Toggle Background", ToggleBackground),
            ],
        },
        Menu {
            name: "Navigate".into(),
            items: vec![
                MenuItem::action("Next Image", NextImage),
                MenuItem::action("Previous Image", PreviousImage),
                MenuItem::separator(),
                MenuItem::action("Sort Alphabetically", SortAlphabetical),
                MenuItem::action("Sort by Modified Date", SortByModified),
                MenuItem::action("Sort by Type (Toggle A/M)", SortByTypeToggle),
            ],
        },
        Menu {
            name: "Animation".into(),
            items: vec![
                MenuItem::action("Play/Pause", ToggleAnimationPlayPause),
                MenuItem::action("Next Frame", NextFrame),
                MenuItem::action("Previous Frame", PreviousFrame),
            ],
        },
    ]);
}
