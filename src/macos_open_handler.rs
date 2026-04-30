//! macOS-specific handler for "Open With" file events.
//!
//! GPUI only implements `application:openURLs:` (for URL schemes), not
//! `application:openFiles:` which macOS uses for "Open With" on files.
//! This module uses runtime method addition to add the missing handler.

use std::ffi::CStr;
use std::path::PathBuf;
use std::sync::Mutex;

use objc2::ffi::{class_addMethod, objc_getClass};
use objc2::msg_send;
use objc2::runtime::{AnyObject, Imp, Sel};

/// Global storage for file paths received via "Open With"
static OPEN_FILES_PATHS: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());

/// Get and clear pending file paths from "Open With" events
pub fn take_pending_paths() -> Vec<PathBuf> {
    let Ok(mut paths) = OPEN_FILES_PATHS.lock() else {
        return Vec::new();
    };
    std::mem::take(&mut *paths)
}

/// Check if there are pending file paths
pub fn has_pending_paths() -> bool {
    OPEN_FILES_PATHS
        .lock()
        .map(|p| !p.is_empty())
        .unwrap_or(false)
}

/// Store file paths for later processing
fn store_paths(paths: Vec<PathBuf>) {
    if paths.is_empty() {
        return;
    }

    if let Ok(mut pending) = OPEN_FILES_PATHS.lock() {
        pending.extend(paths);
    }
}

/// The handler function that will be called when application:openFiles: is invoked.
/// This has the signature: void (id self, SEL _cmd, NSApplication* app, NSArray<NSString*>* filenames)
///
/// Declared `unsafe extern "C-unwind"` to match objc2's `Imp` typedef
/// (`unsafe extern "C-unwind" fn()`); this avoids an ABI-tag mismatch in the
/// transmute below.
unsafe extern "C-unwind" fn handle_open_files(
    _this: &AnyObject,
    _cmd: Sel,
    _app: &AnyObject,
    filenames: *const AnyObject, // NSArray<NSString*>*
) {
    if filenames.is_null() {
        return;
    }

    unsafe {
        let filenames = &*filenames;
        let count: usize = msg_send![filenames, count];

        let mut paths = Vec::with_capacity(count);

        for i in 0..count {
            let filename: *const AnyObject = msg_send![filenames, objectAtIndex: i];
            if !filename.is_null() {
                let filename = &*filename;
                // filename is an NSString with the file path
                let utf8: *const i8 = msg_send![filename, UTF8String];
                if !utf8.is_null() {
                    if let Ok(path_str) = CStr::from_ptr(utf8).to_str() {
                        paths.push(PathBuf::from(path_str));
                    }
                }
            }
        }

        store_paths(paths);
    }
}

/// Register the application:openFiles: handler on GPUI's app delegate class.
/// Call this after `Application::new()` (which constructs the delegate class)
/// but before `application.run()` (which starts dispatching events through
/// it).
pub fn register_open_files_handler() {
    unsafe {
        // GPUI's app delegate class name
        let class_name = c"GPUIApplicationDelegate";
        let cls = objc_getClass(class_name.as_ptr());

        if cls.is_null() {
            // Class doesn't exist (or has been renamed). Surface this in dev
            // builds so a future GPUI rename of `GPUIApplicationDelegate`
            // produces a visible signal rather than silently breaking
            // "Open With".
            crate::debug_eprintln!(
                "[macos_open_handler] GPUIApplicationDelegate class not found; \
                 application:openFiles: handler NOT installed. \
                 Has GPUI's delegate class been renamed?"
            );
            return;
        }

        // Selector for application:openFiles:
        let sel = Sel::register(c"application:openFiles:");

        // Method signature: void (id, SEL, NSApplication*, NSArray*)
        // Encoding: v@:@@  (void, id, SEL, id, id)
        let types = c"v@:@@";

        // Cast the function pointer to Imp.  Both sides are
        // `unsafe extern "C-unwind" fn(...)`, so this transmutes only the
        // argument tuple — `Imp` is `fn()` (type-erased) and the typed
        // signature must be supplied at the call boundary.  No ABI
        // conversion happens here.
        let imp: Imp = std::mem::transmute::<
            unsafe extern "C-unwind" fn(&AnyObject, Sel, &AnyObject, *const AnyObject),
            Imp,
        >(handle_open_files);

        let _success = class_addMethod(cls as *mut _, sel, imp, types.as_ptr());
    }
}
