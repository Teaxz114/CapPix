use serde::Serialize;
use std::ptr::null_mut;
use windows::Win32::Foundation::{HWND, POINT, RECT};
use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_INPROC_SERVER};
use windows::Win32::UI::Accessibility::*;
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

/// Detect the window region at the given screen coordinates.
/// Strategy: try UIA first (more precise for modern apps), fallback to Win32.
pub fn get_window_at_point(x: i32, y: i32) -> Option<WindowRegion> {
    // Try UIA first — better for modern apps (browsers, Electron, UWP)
    if let Some(region) = get_window_at_point_uia(x, y) {
        return Some(region);
    }

    // Fallback to Win32 — works for classic Win32 apps
    get_window_at_point_win32(x, y)
}

/// Win32 method: WindowFromPoint + GetAncestor(GA_ROOT)
fn get_window_at_point_win32(x: i32, y: i32) -> Option<WindowRegion> {
    unsafe {
        let point = POINT { x, y };
        let hwnd = WindowFromPoint(point);

        if hwnd.is_invalid() || hwnd == HWND(null_mut()) {
            return None;
        }

        let top_hwnd = GetAncestor(hwnd, GA_ROOT);

        if top_hwnd.is_invalid() || top_hwnd == HWND(null_mut()) {
            return None;
        }

        let mut rect = RECT::default();
        if GetWindowRect(top_hwnd, &mut rect).is_err() {
            return None;
        }

        let width = rect.right - rect.left;
        let height = rect.bottom - rect.top;

        if width <= 0 || height <= 0 || width > 10000 || height > 10000 {
            return None;
        }

        let mut title_buf = [0u16; 512];
        let title_len = GetWindowTextW(top_hwnd, &mut title_buf);
        let title = String::from_utf16_lossy(&title_buf[..title_len as usize]);

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

/// UIA method: uses IUIAutomation to find the element at the point.
/// This can detect sub-regions within a window (e.g., browser tabs, IDE panels)
/// that Win32 WindowFromPoint cannot distinguish.
fn get_window_at_point_uia(x: i32, y: i32) -> Option<WindowRegion> {
    unsafe {
        // Create UI Automation instance
        let automation: IUIAutomation =
            CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER).ok()?;

        // Get element at point
        let pt = POINT { x, y };
        let element: IUIAutomationElement = automation.ElementFromPoint(pt).ok()?;

        // Get the element's bounding rectangle
        let rect: RECT = element.CurrentBoundingRectangle().ok()?;
        let width = rect.right - rect.left;
        let height = rect.bottom - rect.top;

        // Skip zero-size or absurdly large regions
        if width <= 0 || height <= 0 || width > 10000 || height > 10000 {
            return None;
        }

        // Skip very small regions (likely individual controls, not meaningful sections)
        if width < 50 && height < 50 {
            return None;
        }

        // Get the element's name
        let title = element
            .CurrentName()
            .ok()
            .map(|bstr| {
                // BSTR -> String: read the wide chars directly
                let len = bstr.len();
                let ptr = bstr.as_wide().as_ptr();
                String::from_utf16_lossy(std::slice::from_raw_parts(ptr, len))
            })
            .unwrap_or_default();

        // Skip desktop
        if title == "Program Manager" || title == "Desktop" {
            return None;
        }

        // Get native window handle
        let hwnd_val = element
            .CurrentNativeWindowHandle()
            .ok()
            .map(|h| h.0 as u64)
            .unwrap_or(0);

        Some(WindowRegion {
            x: rect.left,
            y: rect.top,
            width,
            height,
            title,
            hwnd: hwnd_val,
        })
    }
}
