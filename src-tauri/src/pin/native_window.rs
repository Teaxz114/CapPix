use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use tauri::Manager;

use windows::core::PCWSTR;
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, BitBlt, ClientToScreen, CreateCompatibleDC, CreateDIBSection,
    CreateSolidBrush, DeleteDC, DeleteObject, EndPaint, FillRect, GetDC, GetStockObject,
    InvalidateRect, LineTo, MoveToEx, ScreenToClient, SelectObject, BITMAPINFO,
    BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HBITMAP, PAINTSTRUCT, SRCCOPY, WHITE_PEN,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu, DestroyWindow,
    DispatchMessageW, GetMessageW, GetWindowLongPtrW, GetWindowRect, HWND_NOTOPMOST, HWND_TOPMOST,
    IDC_ARROW, IDC_SIZENESW, IDC_SIZENS, IDC_SIZENWSE, IDC_SIZEWE, LoadCursorW, LWA_ALPHA, MF_STRING,
    MSG, MoveWindow, RegisterClassExW, SetCursor, SetLayeredWindowAttributes, SetWindowLongPtrW,
    SetWindowPos, ShowWindow, SWP_NOMOVE, SWP_NOSIZE, SW_SHOW, TPM_LEFTBUTTON, TPM_RIGHTBUTTON,
    TrackPopupMenu, TranslateMessage, GWL_EXSTYLE, GWLP_USERDATA, WNDCLASSEXW, WS_EX_LAYERED,
    WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_OVERLAPPED, WS_VISIBLE,
};
use windows::Win32::System::SystemServices::{MK_CONTROL, MK_SHIFT};

/// A native Win32 window that displays a pinned image without any webview.
pub struct NativePinWindow {
    hwnd: HWND,
    bitmap: HBITMAP,
    width: i32,
    height: i32,
    opacity: u8,
    clickthrough: bool,
    hover_close: bool,
    pin_id: Option<String>,
    app_handle: Option<tauri::AppHandle>,
}

const CLOSE_BTN_SIZE: i32 = 24;
const TITLE_BAR_H: i32 = 28;
const BORDER_SIZE: i32 = 4;

const CMD_CLOSE: usize = 1001;
const CMD_OPACITY_INC: usize = 1002;
const CMD_OPACITY_DEC: usize = 1003;
const CMD_CLICKTHROUGH: usize = 1004;
const CMD_TOPMOST: usize = 1007;

fn wide(s: &str) -> Vec<u16> {
    OsStr::new(s).encode_wide().chain(std::iter::once(0)).collect()
}

static mut PIN_WINDOW_CLASS_REGISTERED: bool = false;
const PIN_WINDOW_CLASS: &str = "CapPixPinWindow";

fn register_pin_class() {
    unsafe {
        if PIN_WINDOW_CLASS_REGISTERED {
            return;
        }
        let class_name = wide(PIN_WINDOW_CLASS);
        let wnd_class = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: Default::default(),
            lpfnWndProc: Some(pin_wnd_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: windows::Win32::Foundation::HINSTANCE(std::ptr::null_mut()),
            hIcon: Default::default(),
            hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
            hbrBackground: Default::default(),
            lpszMenuName: PCWSTR::null(),
            lpszClassName: PCWSTR(class_name.as_ptr()),
            hIconSm: Default::default(),
        };
        let _ = RegisterClassExW(&wnd_class);
        PIN_WINDOW_CLASS_REGISTERED = true;
    }
}

fn get_xy(lparam: LPARAM) -> (i32, i32) {
    let x = (lparam.0 & 0xFFFF) as i16 as i32;
    let y = ((lparam.0 >> 16) & 0xFFFF) as i16 as i32;
    (x, y)
}

fn in_close_btn(x: i32, y: i32, window_w: i32) -> bool {
    x >= window_w - CLOSE_BTN_SIZE && y >= 0 && y <= CLOSE_BTN_SIZE
}

unsafe extern "system" fn pin_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        0x000F => {
            // WM_PAINT
            let mut ps = PAINTSTRUCT::default();
            let hdc = BeginPaint(hwnd, &mut ps);
            let data_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut NativePinWindow;
            if !data_ptr.is_null() {
                let pin = &*data_ptr;

                let mem_dc = CreateCompatibleDC(hdc);
                let old_bmp = SelectObject(mem_dc, pin.bitmap);
                let _ = BitBlt(hdc, 0, 0, pin.width, pin.height, mem_dc, 0, 0, SRCCOPY);
                let _ = SelectObject(mem_dc, old_bmp);
                let _ = DeleteDC(mem_dc);

                // Close button (top-right, only when hovered)
                if pin.hover_close {
                    let btn_x = pin.width - CLOSE_BTN_SIZE;
                    let btn_y = 0;
                    let rect = RECT {
                        left: btn_x + 2,
                        top: btn_y + 2,
                        right: btn_x + CLOSE_BTN_SIZE - 2,
                        bottom: btn_y + CLOSE_BTN_SIZE - 2,
                    };
                    let red_brush = CreateSolidBrush(COLORREF(0x4444FF));
                    FillRect(hdc, &rect, red_brush);
                    DeleteObject(red_brush);

                    let old_pen = SelectObject(hdc, GetStockObject(WHITE_PEN));
                    let inset = 7i32;
                    MoveToEx(hdc, btn_x + inset, btn_y + inset, None);
                    let _ = LineTo(hdc, btn_x + CLOSE_BTN_SIZE - inset, btn_y + CLOSE_BTN_SIZE - inset);
                    MoveToEx(hdc, btn_x + CLOSE_BTN_SIZE - inset, btn_y + inset, None);
                    let _ = LineTo(hdc, btn_x + inset, btn_y + CLOSE_BTN_SIZE - inset);
                    SelectObject(hdc, old_pen);
                }
            }
            let _ = EndPaint(hwnd, &ps);
            LRESULT(0)
        }

        0x0084 => {
            // WM_NCHITTEST
            let (x, y) = get_xy(lparam);
            let mut pt = POINT { x, y };
            let _ = ScreenToClient(hwnd, &mut pt);

            let data_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const NativePinWindow;
            let (win_w, win_h) = if !data_ptr.is_null() {
                (pin_width(data_ptr), pin_height(data_ptr))
            } else {
                (800, 600)
            };

            if in_close_btn(pt.x, pt.y, win_w) {
                return LRESULT(1); // HTCLIENT
            }

            // Border resize
            if pt.x < BORDER_SIZE && pt.y < BORDER_SIZE {
                return LRESULT(0xD);
            }
            if pt.x > win_w - BORDER_SIZE && pt.y < BORDER_SIZE {
                return LRESULT(0xE);
            }
            if pt.x < BORDER_SIZE && pt.y > win_h - BORDER_SIZE {
                return LRESULT(0x10);
            }
            if pt.x > win_w - BORDER_SIZE && pt.y > win_h - BORDER_SIZE {
                return LRESULT(0x11);
            }
            if pt.y < BORDER_SIZE {
                return LRESULT(0xC);
            }
            if pt.y > win_h - BORDER_SIZE {
                return LRESULT(0xF);
            }
            if pt.x < BORDER_SIZE {
                return LRESULT(0xA);
            }
            if pt.x > win_w - BORDER_SIZE {
                return LRESULT(0xB);
            }

            // Title bar → drag
            if pt.y < TITLE_BAR_H {
                return LRESULT(0x2);
            }

            LRESULT(1)
        }

        0x0020 => {
            // WM_SETCURSOR
            let hit_test = (lparam.0 & 0xFFFF) as u32;
            let cursor_id = match hit_test {
                0xA | 0xB => Some(IDC_SIZEWE),
                0xC | 0xF => Some(IDC_SIZENS),
                0xD | 0x11 => Some(IDC_SIZENWSE),
                0xE | 0x10 => Some(IDC_SIZENESW),
                _ => None,
            };
            if let Some(id) = cursor_id {
                if let Ok(cur) = LoadCursorW(None, id) {
                    SetCursor(cur);
                    return LRESULT(1);
                }
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }

        0x00A0 => {
            // WM_NCMOUSEMOVE
            let (x, y) = get_xy(lparam);
            let mut pt = POINT { x, y };
            let _ = ScreenToClient(hwnd, &mut pt);
            let data_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut NativePinWindow;
            if !data_ptr.is_null() {
                let pin = &mut *data_ptr;
                let was_hover = pin.hover_close;
                pin.hover_close = in_close_btn(pt.x, pt.y, pin.width);
                if pin.hover_close != was_hover {
                    let _ = InvalidateRect(hwnd, None, false);
                }
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }

        0x0200 => {
            // WM_MOUSEMOVE
            let (x, y) = get_xy(lparam);
            let data_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut NativePinWindow;
            if !data_ptr.is_null() {
                let pin = &mut *data_ptr;
                let was_hover = pin.hover_close;
                pin.hover_close = in_close_btn(x, y, pin.width);
                if pin.hover_close != was_hover {
                    let _ = InvalidateRect(hwnd, None, false);
                }
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }

        0x0202 => {
            // WM_LBUTTONUP
            let (x, y) = get_xy(lparam);
            let data_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const NativePinWindow;
            if !data_ptr.is_null() && in_close_btn(x, y, pin_width(data_ptr)) {
                let _ = DestroyWindow(hwnd);
                return LRESULT(0);
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }

        0x0205 => {
            // WM_RBUTTONUP — context menu
            let (x, y) = get_xy(lparam);
            let data_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut NativePinWindow;
            if data_ptr.is_null() {
                return DefWindowProcW(hwnd, msg, wparam, lparam);
            }
            let pin = &*data_ptr;

            let hmenu = CreatePopupMenu().unwrap_or_default();
            let opa_str = wide(&format!("增加透明度 ({:.0}%)", pin.opacity as f64 / 255.0 * 100.0));
            let opd_str = wide(&format!("降低透明度 ({:.0}%)", pin.opacity as f64 / 255.0 * 100.0));
            let ct_str = wide(if pin.clickthrough { "取消鼠标穿透" } else { "鼠标穿透" });
            let topmost_str = wide("取消置顶");
            let close_str = wide("关闭");

            AppendMenuW(hmenu, MF_STRING, CMD_OPACITY_INC, PCWSTR(opa_str.as_ptr()));
            AppendMenuW(hmenu, MF_STRING, CMD_OPACITY_DEC, PCWSTR(opd_str.as_ptr()));
            AppendMenuW(hmenu, MF_STRING, CMD_CLICKTHROUGH, PCWSTR(ct_str.as_ptr()));
            AppendMenuW(hmenu, MF_STRING, CMD_TOPMOST, PCWSTR(topmost_str.as_ptr()));
            AppendMenuW(hmenu, MF_STRING, CMD_CLOSE, PCWSTR(close_str.as_ptr()));

            let mut pt = POINT { x, y };
            let _ = ClientToScreen(hwnd, &mut pt);

            let cmd = TrackPopupMenu(
                hmenu,
                TPM_RIGHTBUTTON | TPM_LEFTBUTTON,
                pt.x, pt.y, 0, hwnd, None,
            );
            let _ = DestroyMenu(hmenu);

            let cmd_id = cmd.0 as usize;
            match cmd_id {
                CMD_CLOSE => { let _ = DestroyWindow(hwnd); }
                CMD_OPACITY_INC => {
                    let pin = &mut *data_ptr;
                    pin.opacity = (pin.opacity as u16).saturating_add(26).min(255) as u8;
                    let _ = SetLayeredWindowAttributes(hwnd, COLORREF(0), pin.opacity, LWA_ALPHA);
                }
                CMD_OPACITY_DEC => {
                    let pin = &mut *data_ptr;
                    pin.opacity = (pin.opacity as i16).saturating_sub(26).max(26) as u8;
                    let _ = SetLayeredWindowAttributes(hwnd, COLORREF(0), pin.opacity, LWA_ALPHA);
                }
                CMD_CLICKTHROUGH => {
                    let pin = &mut *data_ptr;
                    pin.clickthrough = !pin.clickthrough;
                    let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
                    if pin.clickthrough {
                        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, style | WS_EX_TRANSPARENT.0 as isize);
                    } else {
                        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, style & !(WS_EX_TRANSPARENT.0 as isize));
                    }
                }
                CMD_TOPMOST => {
                    let _ = SetWindowPos(hwnd, HWND_NOTOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE);
                }
                _ => {}
            }
            LRESULT(0)
        }

        0x020A => {
            // WM_MOUSEWHEEL — zoom (Ctrl+wheel) or opacity (wheel)
            let delta = (wparam.0 >> 16) as i16 as i32;
            let keys = (wparam.0 & 0xFFFF) as u32;
            let data_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut NativePinWindow;
            if data_ptr.is_null() {
                return DefWindowProcW(hwnd, msg, wparam, lparam);
            }

            if (keys & MK_CONTROL.0) != 0 {
                let scale = if delta > 0 { 1.1f64 } else { 1.0 / 1.1 };
                let mut rect = RECT::default();
                let _ = GetWindowRect(hwnd, &mut rect);
                let cur_w = (rect.right - rect.left) as f64;
                let cur_h = (rect.bottom - rect.top) as f64;
                let new_w = (cur_w * scale) as i32;
                let new_h = (cur_h * scale) as i32;
                let _ = MoveWindow(hwnd, rect.left, rect.top, new_w, new_h, true);
            } else {
                let pin = &mut *data_ptr;
                let step = 13i16;
                pin.opacity = if delta > 0 {
                    (pin.opacity as i16).saturating_add(step).min(255) as u8
                } else {
                    (pin.opacity as i16).saturating_sub(step).max(26) as u8
                };
                let _ = SetLayeredWindowAttributes(hwnd, COLORREF(0), pin.opacity, LWA_ALPHA);
            }
            LRESULT(0)
        }

        0x0005 => {
            // WM_SIZE
            let data_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut NativePinWindow;
            if !data_ptr.is_null() {
                let pin = &mut *data_ptr;
                pin.width = (lparam.0 & 0xFFFF) as i32;
                pin.height = ((lparam.0 >> 16) & 0xFFFF) as i32;
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }

        0x0010 => {
            // WM_CLOSE
            let _ = DestroyWindow(hwnd);
            LRESULT(0)
        }
        0x0002 => {
            // WM_DESTROY
            let data_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut NativePinWindow;
            if !data_ptr.is_null() {
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
                let pin = Box::from_raw(data_ptr);
                let _ = DeleteObject(pin.bitmap);
                // Clean up PinRegistry so stale HWNDs don't linger
                if let (Some(pid), Some(app)) = (&pin.pin_id, &pin.app_handle) {
                    if let Some(reg) = app.try_state::<super::PinRegistry>() {
                        if let Ok(mut map) = reg.0.lock() {
                            map.remove(pid);
                        }
                    }
                    log::info!("[Pin] WM_DESTROY: cleaned registry for {}", pid);
                }
            }
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

/// Helper to read pin.width through a raw pointer
unsafe fn pin_width(ptr: *const NativePinWindow) -> i32 {
    (*ptr).width
}
unsafe fn pin_height(ptr: *const NativePinWindow) -> i32 {
    (*ptr).height
}

impl NativePinWindow {
    /// Create a pin window and run its message loop on the calling thread.
    /// The HWND is sent via `tx` right after ShowWindow, BEFORE the message
    /// loop begins, so the caller gets the handle without blocking.
    pub fn create_with_channel(
        image_rgba: &[u8],
        width: i32,
        height: i32,
        x: i32,
        y: i32,
        pin_id: String,
        app_handle: tauri::AppHandle,
        tx: std::sync::mpsc::Sender<Result<isize, String>>,
    ) -> Result<(), String> {
        unsafe {
            register_pin_class();

            let bitmap = Self::create_bitmap_from_rgba(image_rgba, width, height)?;

            let class_name = wide(PIN_WINDOW_CLASS);
            let title = wide("CapPix Pin");

            let hwnd = match CreateWindowExW(
                WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_LAYERED,
                PCWSTR(class_name.as_ptr()),
                PCWSTR(title.as_ptr()),
                WS_VISIBLE | WS_OVERLAPPED,
                x, y, width, height,
                None, None,
                windows::Win32::Foundation::HMODULE(std::ptr::null_mut()),
                None,
            ) {
                Ok(h) => h,
                Err(e) => {
                    let _ = tx.send(Err(format!("CreateWindowExW failed: {}", e)));
                    return Err(format!("CreateWindowExW failed: {}", e));
                }
            };

            let pin = Box::new(NativePinWindow {
                hwnd,
                bitmap,
                width,
                height,
                opacity: 255,
                clickthrough: false,
                hover_close: false,
                pin_id: Some(pin_id),
                app_handle: Some(app_handle),
            });
            let pin_ptr = Box::into_raw(pin);
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, pin_ptr as isize);

            let _ = ShowWindow(hwnd, SW_SHOW);

            // Send HWND to caller NOW — before entering the blocking message loop
            let hwnd_val = hwnd.0 as isize;
            let _ = tx.send(Ok(hwnd_val));

            // Run the message loop on THIS thread for the lifetime of the window
            let mut msg = MSG::default();
            while GetMessageW(&mut msg, hwnd, 0, 0).0 > 0 {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            // Window destroyed — thread will exit

            Ok(())
        }
    }

    pub fn create(
        image_rgba: &[u8],
        width: i32,
        height: i32,
        x: i32,
        y: i32,
        pin_id: String,
        app_handle: tauri::AppHandle,
    ) -> Result<isize, String> {
        unsafe {
            register_pin_class();

            let bitmap = Self::create_bitmap_from_rgba(image_rgba, width, height)?;

            let class_name = wide(PIN_WINDOW_CLASS);
            let title = wide("CapPix Pin");

            let hwnd = CreateWindowExW(
                WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_LAYERED,
                PCWSTR(class_name.as_ptr()),
                PCWSTR(title.as_ptr()),
                WS_VISIBLE | WS_OVERLAPPED,
                x,
                y,
                width,
                height,
                None,
                None,
                windows::Win32::Foundation::HMODULE(std::ptr::null_mut()),
                None,
            )
            .map_err(|e| format!("CreateWindowExW failed: {}", e))?;

            let pin = Box::new(NativePinWindow {
                hwnd,
                bitmap,
                width,
                height,
                opacity: 255,
                clickthrough: false,
                hover_close: false,
                pin_id: Some(pin_id),
                app_handle: Some(app_handle),
            });
            let pin_ptr = Box::into_raw(pin);
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, pin_ptr as isize);

            let _ = ShowWindow(hwnd, SW_SHOW);

            Ok(hwnd.0 as isize)
        }
    }

    fn create_bitmap_from_rgba(rgba: &[u8], width: i32, height: i32) -> Result<HBITMAP, String> {
        let bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                biSizeImage: 0,
                biXPelsPerMeter: 0,
                biYPelsPerMeter: 0,
                biClrUsed: 0,
                biClrImportant: 0,
            },
            ..Default::default()
        };

        let mut ppv_bits: *mut u8 = std::ptr::null_mut();
        let hdc = unsafe { GetDC(HWND::default()) };

        let hbitmap = unsafe {
            CreateDIBSection(
                hdc, &bmi, DIB_RGB_COLORS,
                &mut ppv_bits as *mut _ as *mut _,
                None, 0,
            )
        }.map_err(|e| format!("CreateDIBSection failed: {}", e))?;

        if !ppv_bits.is_null() {
            let pixel_count = (width * height) as usize;
            for i in 0..pixel_count {
                unsafe {
                    let src = i * 4;
                    let dst = i * 4;
                    *ppv_bits.add(dst) = rgba[src + 2];     // B
                    *ppv_bits.add(dst + 1) = rgba[src + 1]; // G
                    *ppv_bits.add(dst + 2) = rgba[src];     // R
                    *ppv_bits.add(dst + 3) = rgba[src + 3]; // A
                }
            }
        }

        Ok(hbitmap)
    }
}
