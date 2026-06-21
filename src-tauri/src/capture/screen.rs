use super::{CaptureResult, ScreenInfo};
use anyhow::Result;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use image::RgbaImage;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};

pub fn get_screen_list() -> Result<Vec<ScreenInfo>> {
    let width = unsafe { GetSystemMetrics(SM_CXSCREEN) };
    let height = unsafe { GetSystemMetrics(SM_CYSCREEN) };

    Ok(vec![ScreenInfo {
        id: 0,
        x: 0,
        y: 0,
        width,
        height,
        is_primary: true,
    }])
}

pub fn capture_screen(_screen_id: u32) -> Result<CaptureResult> {
    let width = unsafe { GetSystemMetrics(SM_CXSCREEN) };
    let height = unsafe { GetSystemMetrics(SM_CYSCREEN) };
    capture_rect(0, 0, width, height)
}

pub fn capture_rect(x: i32, y: i32, width: i32, height: i32) -> Result<CaptureResult> {
    unsafe {
        let hdc_screen = GetDC(None);
        let hdc_mem = CreateCompatibleDC(hdc_screen);

        let h_bitmap = CreateCompatibleBitmap(hdc_screen, width, height);
        let old_bitmap = SelectObject(hdc_mem, h_bitmap);

        let _ = BitBlt(hdc_mem, 0, 0, width, height, hdc_screen, x, y, SRCCOPY);

        let _ = SelectObject(hdc_mem, old_bitmap);

        // Get bitmap data
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

        let pixel_count = (width * height) as usize;
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
