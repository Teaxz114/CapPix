use serde::Serialize;
use std::ptr::null_mut;
use windows::Win32::Foundation::{HWND, POINT, RECT};
use windows::Win32::UI::WindowsAndMessaging::{
    GetAncestor, GetWindowRect, GetWindowTextW, WindowFromPoint, GA_ROOT,
};

#[derive(Debug, Serialize, Clone)]
pub struct WindowRegion {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub title: String,
    pub hwnd: u64,
}

/// Detect the top-level window at the given screen coordinates.
/// Uses `WindowFromPoint` to find the window, then walks up to the root ancestor
/// with `GetAncestor(GA_ROOT)` to get the top-level window (not a child control).
pub fn get_window_at_point(x: i32, y: i32) -> Option<WindowRegion> {
    unsafe {
        let point = POINT { x, y };
        let hwnd = WindowFromPoint(point);

        if hwnd.is_invalid() || hwnd == HWND(null_mut()) {
            return None;
        }

        // Walk up to top-level window (not child controls like buttons, text fields, etc.)
        let top_hwnd = GetAncestor(hwnd, GA_ROOT);

        if top_hwnd.is_invalid() || top_hwnd == HWND(null_mut()) {
            return None;
        }

        // Get window rectangle
        let mut rect = RECT::default();
        if GetWindowRect(top_hwnd, &mut rect).is_err() {
            return None;
        }

        let width = rect.right - rect.left;
        let height = rect.bottom - rect.top;

        // Skip zero-size or absurdly large windows
        if width <= 0 || height <= 0 || width > 10000 || height > 10000 {
            return None;
        }

        // Get window title
        let mut title_buf = [0u16; 512];
        let title_len = GetWindowTextW(top_hwnd, &mut title_buf);
        let title = String::from_utf16_lossy(&title_buf[..title_len as usize]);

        // Skip the desktop window (empty title, usually "Program Manager")
        if title.is_empty() || title == "Program Manager" {
            return None;
        }

        Some(WindowRegion {
            x: rect.left,
            y: rect.top,
            width,
            height,
            title,
            hwnd: top_hwnd.0 as u64,
        })
    }
}
