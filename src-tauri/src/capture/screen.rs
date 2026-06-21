use super::{CaptureResult, ScreenInfo};
use anyhow::Result;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use image::RgbaImage;
use windows::Win32::Foundation::{BOOL, LPARAM, RECT};
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::UI::WindowsAndMessaging::{
    GetSystemMetrics, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN,
};

/// Monitor callback context
struct MonitorEnumCtx {
    monitors: Vec<ScreenInfo>,
    index: u32,
}

/// Enumerate all monitors with correct virtual screen coordinates
pub fn get_screen_list() -> Result<Vec<ScreenInfo>> {
    let mut ctx = MonitorEnumCtx {
        monitors: Vec::new(),
        index: 0,
    };

    unsafe {
        let _ = EnumDisplayMonitors(
            HDC(std::ptr::null_mut()),
            None,
            Some(monitor_enum_callback),
            LPARAM(&mut ctx as *mut _ as isize),
        );
    }

    Ok(ctx.monitors)
}

unsafe extern "system" fn monitor_enum_callback(
    hmonitor: HMONITOR,
    _hdc: HDC,
    _rect: *mut RECT,
    lparam: LPARAM,
) -> BOOL {
    let ctx = &mut *(lparam.0 as *mut MonitorEnumCtx);

    let mut info = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };

    if GetMonitorInfoW(hmonitor, &mut info).0 != 0 {
        let is_primary = info.dwFlags & 1 != 0; // MONITORINFOF_PRIMARY
        ctx.monitors.push(ScreenInfo {
            id: ctx.index,
            x: info.rcMonitor.left,
            y: info.rcMonitor.top,
            width: info.rcMonitor.right - info.rcMonitor.left,
            height: info.rcMonitor.bottom - info.rcMonitor.top,
            is_primary,
        });
        ctx.index += 1;
    }

    BOOL(1) // Continue enumeration
}

/// Capture a specific screen by index (0-based)
pub fn capture_screen(screen_id: u32) -> Result<CaptureResult> {
    let screens = get_screen_list()?;
    let screen = screens
        .into_iter()
        .find(|s| s.id == screen_id)
        .ok_or_else(|| anyhow::anyhow!("Screen {} not found", screen_id))?;

    capture_rect(screen.x, screen.y, screen.width, screen.height)
}

/// Capture a rectangular region of the virtual screen (coordinates in virtual screen space)
pub fn capture_rect(x: i32, y: i32, width: i32, height: i32) -> Result<CaptureResult> {
    // Clamp to virtual screen bounds
    let vx = unsafe { GetSystemMetrics(SM_XVIRTUALSCREEN) };
    let vy = unsafe { GetSystemMetrics(SM_YVIRTUALSCREEN) };
    let vw = unsafe { GetSystemMetrics(SM_CXVIRTUALSCREEN) };
    let vh = unsafe { GetSystemMetrics(SM_CYVIRTUALSCREEN) };

    let x = x.max(vx);
    let y = y.max(vy);
    let right = (x + width).min(vx + vw);
    let bottom = (y + height).min(vy + vh);
    let width = right - x;
    let height = bottom - y;

    if width <= 0 || height <= 0 {
        anyhow::bail!(
            "Invalid capture region: {}x{} at ({},{})",
            width,
            height,
            x,
            y
        );
    }

    capture_rect_gdi(x, y, width, height)
}

/// GDI-based screen capture (works on all Windows versions)
fn capture_rect_gdi(x: i32, y: i32, width: i32, height: i32) -> Result<CaptureResult> {
    unsafe {
        let hdc_screen = GetDC(None);
        let hdc_mem = CreateCompatibleDC(hdc_screen);

        let h_bitmap = CreateCompatibleBitmap(hdc_screen, width, height);
        let old_bitmap = SelectObject(hdc_mem, h_bitmap);

        // Use CAPTUREBLT to include layered windows
        let _ = BitBlt(
            hdc_mem,
            0,
            0,
            width,
            height,
            hdc_screen,
            x,
            y,
            ROP_CODE(SRCCOPY.0 | 0x40000000), // SRCCOPY | CAPTUREBLT
        );

        let _ = SelectObject(hdc_mem, old_bitmap);

        // Get bitmap data via GetDIBits
        let mut bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height, // top-down
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };

        let pixel_count = (width as usize) * (height as usize);
        let mut pixels: Vec<u8> = vec![0u8; pixel_count * 4];

        let result = GetDIBits(
            hdc_mem,
            h_bitmap,
            0,
            height as u32,
            Some(pixels.as_mut_ptr() as *mut _),
            &mut bmi,
            DIB_RGB_COLORS,
        );

        if result == 0 {
            let _ = DeleteObject(h_bitmap);
            let _ = DeleteDC(hdc_mem);
            let _ = ReleaseDC(None, hdc_screen);
            anyhow::bail!("GetDIBits failed");
        }

        // BGRA -> RGBA conversion
        for chunk in pixels.chunks_exact_mut(4) {
            chunk.swap(0, 2); // B <-> R
        }

        let img = RgbaImage::from_raw(width as u32, height as u32, pixels)
            .ok_or_else(|| anyhow::anyhow!("Failed to create image from pixels"))?;

        let mut png_data = Vec::new();
        img.write_to(
            &mut std::io::Cursor::new(&mut png_data),
            image::ImageFormat::Png,
        )?;

        let base64_str = STANDARD.encode(&png_data);

        let _ = DeleteObject(h_bitmap);
        let _ = DeleteDC(hdc_mem);
        let _ = ReleaseDC(None, hdc_screen);

        Ok(CaptureResult {
            image_base64: base64_str,
            width: width as u32,
            height: height as u32,
        })
    }
}
