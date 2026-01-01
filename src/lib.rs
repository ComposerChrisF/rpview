// Library modules for rpview-gpui

pub mod cli;
pub mod components;
pub mod error;
pub mod state;
pub mod utils;

// Define actions that need to be shared between main and components
use gpui::actions;

actions!(app, [
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
    ApplySettings,
    CancelSettings,
    ResetSettingsToDefaults,
]);
