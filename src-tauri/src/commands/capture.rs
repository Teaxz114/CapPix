use crate::capture::window_detect::WindowRegion;
use crate::capture::{self, CaptureResult, ScreenInfo, WindowInfo};

#[tauri::command]
pub fn get_screens() -> Result<Vec<ScreenInfo>, String> {
    capture::screen::get_screen_list().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn capture_fullscreen() -> Result<CaptureResult, String> {
    capture::screen::capture_virtual_screen().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn capture_region(x: i32, y: i32, width: i32, height: i32) -> Result<CaptureResult, String> {
    capture::screen::capture_rect(x, y, width, height).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_windows() -> Result<Vec<WindowInfo>, String> {
    capture::window::get_window_list().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_window_at_point(x: i32, y: i32) -> Result<Option<WindowRegion>, String> {
    Ok(capture::window_detect::get_window_at_point(x, y))
}
