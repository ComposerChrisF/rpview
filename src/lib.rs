//! rpview — a GPU-accelerated image viewer built on GPUI.
//!
//! # Module structure
//!
//! - [`cli`] — Command-line argument parsing (clap).
//! - [`components`] — GPUI UI components (viewer, overlays, settings, floating panels).
//! - [`error`] — Application error types ([`error::AppError`], [`error::AppResult`]).
//! - [`state`] — Application and per-image state, settings.
//! - [`utils`] — Image loading, filters, color math, SVG, zoom, file scanning, and more.

pub mod cli;
pub mod components;
pub mod error;
pub mod gpu;
pub mod state;
pub mod utils;

// Define actions that need to be shared between main and components
use gpui::actions;

actions!(
    app,
    [
        CloseWindow,
        Quit,
        EscapePressed,
        NextImage,
        PreviousImage,
        ToggleAnimationPlayPause,
        NextFrame,
        PreviousFrame,
        SortAlphabetical,
        SortByModified,
        SortByTypeToggle,
        ToggleLocalContrast,
        ApplyLocalContrast,
        ApplyLocalContrastAll,
        ResetLocalContrast,
        ToggleGpuPipeline,
        ResetGpuPipeline,
        ZoomIn,
        ZoomOut,
        ZoomReset,
        ZoomResetAndCenter,
        ZoomInFast,
        ZoomOutFast,
        ZoomInSlow,
        ZoomOutSlow,
        ZoomInIncremental,
        ZoomOutIncremental,
        PanUp,
        PanDown,
        PanLeft,
        PanRight,
        PanUpFast,
        PanDownFast,
        PanLeftFast,
        PanRightFast,
        PanUpSlow,
        PanDownSlow,
        PanLeftSlow,
        PanRightSlow,
        ToggleHelp,
        ToggleDebug,
        ToggleSettings,
        ToggleFilters,
        DisableFilters,
        EnableFilters,
        ResetFilters,
        BrightnessUp,
        BrightnessDown,
        ContrastUp,
        ContrastDown,
        GammaUp,
        GammaDown,
        OpenFile,
        SaveFile,
        SaveFileToDownloads,
        OpenInExternalViewer,
        OpenInExternalViewerAndQuit,
        OpenInExternalEditor,
        RevealInFinder,
        CloseSettings,
        ResetSettingsToDefaults,
        LoadOversizedImageAnyway,
        ToggleZoomIndicator,
        ToggleBackground,
        RequestDelete,
        RequestPermanentDelete,
        ConfirmDelete,
        RecallSlot3,
        RecallSlot4,
        RecallSlot5,
        RecallSlot6,
        RecallSlot7,
        RecallSlot8,
        RecallSlot9,
        StoreSlot3,
        StoreSlot4,
        StoreSlot5,
        StoreSlot6,
        StoreSlot7,
        StoreSlot8,
        StoreSlot9,
    ]
);
