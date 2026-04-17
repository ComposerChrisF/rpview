//! Platform-specific helper to elevate a window to always-on-top (utility-window) level.
//!
//! GPUI 0.2.2's `WindowKind::Floating` maps to `NSNormalWindowLevel` on macOS, so we
//! raise the level ourselves via the standard `raw_window_handle` exposure.

use gpui::Window;

/// Raise `window` above ordinary windows so it behaves like a tool palette.
#[allow(unused_variables)]
pub fn set_always_on_top(window: &Window) {
    #[cfg(target_os = "macos")]
    macos::set_always_on_top(window);

    #[cfg(target_os = "windows")]
    windows_impl::set_always_on_top(window);

    // Linux: WindowKind::Floating already applies the correct WM hints.
}

#[cfg(target_os = "macos")]
mod macos {
    use gpui::Window;
    use objc2::msg_send;
    use objc2::runtime::AnyObject;
    use objc2_app_kit::NSFloatingWindowLevel;
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};

    pub fn set_always_on_top(window: &Window) {
        let Ok(handle) = HasWindowHandle::window_handle(window) else {
            return;
        };
        let RawWindowHandle::AppKit(h) = handle.as_raw() else {
            return;
        };
        unsafe {
            let ns_view: *mut AnyObject = h.ns_view.as_ptr().cast();
            let ns_window: *mut AnyObject = msg_send![ns_view, window];
            if ns_window.is_null() {
                return;
            }
            let _: () = msg_send![ns_window, setLevel: NSFloatingWindowLevel];
        }
    }
}

#[cfg(target_os = "windows")]
mod windows_impl {
    use gpui::Window;
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{
        HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SetWindowPos,
    };

    pub fn set_always_on_top(window: &Window) {
        let Ok(handle) = HasWindowHandle::window_handle(window) else {
            return;
        };
        let RawWindowHandle::Win32(h) = handle.as_raw() else {
            return;
        };
        let hwnd = HWND(h.hwnd.get() as _);
        unsafe {
            let _ = SetWindowPos(
                hwnd,
                Some(HWND_TOPMOST),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            );
        }
    }
}
