use windows::core::PCWSTR;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, BitBlt, CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject,
    EndPaint, GetDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
    HBITMAP, PAINTSTRUCT, SRCCOPY,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, GetWindowLongPtrW, LoadCursorW,
    RegisterClassExW, SetWindowLongPtrW, ShowWindow, GWLP_USERDATA, IDC_ARROW, MSG,
    SW_SHOW, WNDCLASSEXW, WS_EX_LAYERED, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
    WS_OVERLAPPEDWINDOW, WS_VISIBLE,
};

use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;

/// A native Win32 window that displays a pinned image without any webview.
pub struct NativePinWindow {
    hwnd: HWND,
    bitmap: HBITMAP,
    width: i32,
    height: i32,
}

fn wide(s: &str) -> Vec<u16> {
    OsStr::new(s)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
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
            style: windows::Win32::UI::WindowsAndMessaging::WNDCLASS_STYLES(0),
            lpfnWndProc: Some(pin_wnd_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: windows::Win32::Foundation::HINSTANCE(std::ptr::null_mut()),
            hIcon: windows::Win32::UI::WindowsAndMessaging::HICON(std::ptr::null_mut()),
            hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
            hbrBackground: windows::Win32::Graphics::Gdi::HBRUSH(std::ptr::null_mut()),
            lpszMenuName: PCWSTR::null(),
            lpszClassName: PCWSTR(class_name.as_ptr()),
            hIconSm: windows::Win32::UI::WindowsAndMessaging::HICON(std::ptr::null_mut()),
        };
        let _ = RegisterClassExW(&wnd_class);
        PIN_WINDOW_CLASS_REGISTERED = true;
    }
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
            let data_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const NativePinWindow;
            if !data_ptr.is_null() {
                let pin = &*data_ptr;
                let mem_dc = CreateCompatibleDC(hdc);
                let old_bmp = SelectObject(mem_dc, pin.bitmap);
                let _ = BitBlt(hdc, 0, 0, pin.width, pin.height, mem_dc, 0, 0, SRCCOPY);
                let _ = SelectObject(mem_dc, old_bmp);
                let _ = DeleteDC(mem_dc);
            }
            let _ = EndPaint(hwnd, &ps);
            LRESULT(0)
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
                let pin = Box::from_raw(data_ptr);
                let _ = DeleteObject(pin.bitmap);
            }
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

impl NativePinWindow {
    /// Create a native pin window displaying the given image data (RGBA bytes).
    /// Returns the HWND handle as isize.
    pub fn create(
        image_rgba: &[u8],
        width: i32,
        height: i32,
        x: i32,
        y: i32,
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
                WS_VISIBLE | WS_OVERLAPPEDWINDOW,
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
            });
            let pin_ptr = Box::into_raw(pin);
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, pin_ptr as isize);

            let _ = ShowWindow(hwnd, SW_SHOW);

            Ok(hwnd.0 as isize)
        }
    }

    fn create_bitmap_from_rgba(
        rgba: &[u8],
        width: i32,
        height: i32,
    ) -> Result<HBITMAP, String> {
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
                hdc,
                &bmi,
                DIB_RGB_COLORS,
                &mut ppv_bits as *mut _ as *mut _,
                None,
                0,
            )
        }
        .map_err(|e| format!("CreateDIBSection failed: {}", e))?;

        if !ppv_bits.is_null() {
            let pixel_count = (width * height) as usize;
            for i in 0..pixel_count {
                unsafe {
                    let src = i * 4;
                    let dst = i * 4;
                    *ppv_bits.add(dst) = rgba[src + 2]; // B
                    *ppv_bits.add(dst + 1) = rgba[src + 1]; // G
                    *ppv_bits.add(dst + 2) = rgba[src]; // R
                    *ppv_bits.add(dst + 3) = rgba[src + 3]; // A
                }
            }
        }

        Ok(hbitmap)
    }
}
