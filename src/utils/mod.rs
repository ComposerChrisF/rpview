pub mod animation;
pub mod file_scanner;
pub mod filters;
pub mod image_loader;
pub mod settings_io;
pub mod style;
pub mod svg;
pub mod zoom;

/// Like `eprintln!`, but only emits output in debug builds.
macro_rules! debug_eprintln {
    ($($arg:tt)*) => {
        #[cfg(debug_assertions)]
        eprintln!($($arg)*);
    };
}
pub(crate) use debug_eprintln;
