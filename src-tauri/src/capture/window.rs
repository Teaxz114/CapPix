use super::WindowInfo;
use anyhow::Result;
use windows::Win32::Foundation::{BOOL, HWND, LPARAM, RECT};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetClassNameW, GetWindowRect, GetWindowTextW, IsWindowVisible,
};

pub fn get_window_list() -> Result<Vec<WindowInfo>> {
    let mut windows: Vec<WindowInfo> = Vec::new();

    unsafe {
        EnumWindows(
            Some(enum_windows_callback),
            LPARAM(&mut windows as *mut _ as isize),
        )?;
    }

    windows.retain(|w| w.is_visible && !w.title.is_empty());
    Ok(windows)
}

unsafe extern "system" fn enum_windows_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let windows = &mut *(lparam.0 as *mut Vec<WindowInfo>);

    let is_visible = IsWindowVisible(hwnd).as_bool();

    let mut title_buf = [0u16; 512];
    let title_len = GetWindowTextW(hwnd, &mut title_buf);
    let title = String::from_utf16_lossy(&title_buf[..title_len as usize]);

    let mut class_buf = [0u16; 256];
    let class_len = GetClassNameW(hwnd, &mut class_buf);
    let class_name = String::from_utf16_lossy(&class_buf[..class_len as usize]);

    let mut rect = RECT::default();
    let _ = GetWindowRect(hwnd, &mut rect);

    windows.push(WindowInfo {
        hwnd: hwnd.0 as u64,
        title,
        class_name,
        x: rect.left,
        y: rect.top,
        width: rect.right - rect.left,
        height: rect.bottom - rect.top,
        is_visible,
    });

    BOOL(1)
}
