use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use tauri::{Emitter, Manager};

#[tauri::command]
pub async fn copy_image_to_clipboard(image_base64: String) -> Result<(), String> {
    let data = STANDARD.decode(&image_base64).map_err(|e| e.to_string())?;
    let img = image::load_from_memory(&data).map_err(|e| e.to_string())?;
    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();

    unsafe {
        use windows::Win32::Graphics::Gdi::*;
        use windows::Win32::System::DataExchange::*;
        use windows::Win32::System::Memory::{GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalUnlock};
        use windows::Win32::System::Ole::CF_DIB;
        use windows::Win32::Foundation::HANDLE;

        OpenClipboard(None).map_err(|e| format!("Failed to open clipboard: {}", e))?;
        let _ = EmptyClipboard();

        // Create DIB for clipboard
        let bmi_size = std::mem::size_of::<BITMAPINFOHEADER>();
        let pixel_data_size = (width * height * 4) as usize;
        let total_size = bmi_size + pixel_data_size;

        let h_mem = GlobalAlloc(GMEM_MOVEABLE, total_size)
            .map_err(|e| {
                let _ = CloseClipboard();
                format!("GlobalAlloc failed: {}", e)
            })?;

        let p_mem = GlobalLock(h_mem);
        if p_mem.is_null() {
            let _ = CloseClipboard();
            return Err("GlobalLock failed".to_string());
        }

        // Write BITMAPINFOHEADER
        let header = BITMAPINFOHEADER {
            biSize: bmi_size as u32,
            biWidth: width as i32,
            biHeight: -(height as i32),
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            biSizeImage: pixel_data_size as u32,
            ..Default::default()
        };
        std::ptr::write(p_mem as *mut BITMAPINFOHEADER, header);

        // Write pixel data (RGBA -> BGRA for clipboard)
        let pixels = rgba.as_raw();
        let p_pixels = (p_mem as *mut u8).add(bmi_size);
        for i in 0..(width * height) as usize {
            *p_pixels.add(i * 4) = pixels[i * 4 + 2]; // B
            *p_pixels.add(i * 4 + 1) = pixels[i * 4 + 1]; // G
            *p_pixels.add(i * 4 + 2) = pixels[i * 4]; // R
            *p_pixels.add(i * 4 + 3) = pixels[i * 4 + 3]; // A
        }

        let _ = GlobalUnlock(h_mem);
        let _ = SetClipboardData(CF_DIB.0 as u32, HANDLE(h_mem.0));
        let _ = CloseClipboard();
    }

    Ok(())
}

#[tauri::command]
pub fn crop_image(
    image_base64: String,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
) -> Result<String, String> {
    let data = STANDARD.decode(&image_base64).map_err(|e| e.to_string())?;
    let img = image::load_from_memory(&data).map_err(|e| e.to_string())?;
    let cropped = img.crop_imm(x, y, width, height);
    let mut buf = Vec::new();
    cropped
        .write_to(
            &mut std::io::Cursor::new(&mut buf),
            image::ImageFormat::Png,
        )
        .map_err(|e| e.to_string())?;
    Ok(STANDARD.encode(&buf))
}

#[tauri::command]
pub fn open_screenshot_overlay(app: tauri::AppHandle) -> Result<(), String> {
    use tauri::WebviewWindowBuilder;

    let monitor = app
        .primary_monitor()
        .map_err(|e| e.to_string())?
        .ok_or("No primary monitor")?;
    let pos = monitor.position();
    let size = monitor.size();

    // Close existing overlay if any
    if let Some(existing) = app.get_webview_window("screenshot-overlay") {
        let _ = existing.close();
    }

    WebviewWindowBuilder::new(
        &app,
        "screenshot-overlay",
        tauri::WebviewUrl::App("/#/screenshot".into()),
    )
    .title("CapPix Screenshot")
    .decorations(false)
    .always_on_top(true)
    .transparent(true)
    .skip_taskbar(true)
    .inner_size(size.width as f64, size.height as f64)
    .position(pos.x as f64, pos.y as f64)
    .resizable(false)
    .build()
    .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn open_annotate_window(app: tauri::AppHandle, image_base64: String) -> Result<(), String> {
    use tauri::WebviewWindowBuilder;

    // Close existing annotate window if any
    if let Some(existing) = app.get_webview_window("annotate") {
        let _ = existing.close();
    }

    let window = WebviewWindowBuilder::new(
        &app,
        "annotate",
        tauri::WebviewUrl::App("/annotate".into()),
    )
    .title("CapPix - 标注编辑")
    .decorations(true)
    .always_on_top(false)
    .inner_size(1200.0, 800.0)
    .center()
    .resizable(true)
    .build()
    .map_err(|e| e.to_string())?;

    // Emit the image data to the annotate window
    let _ = app.emit("annotate-image", image_base64);

    // Focus the window
    let _ = window.set_focus();

    Ok(())
}
