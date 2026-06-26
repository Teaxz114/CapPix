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
        let is_primary = info.dwFlags & 1 != 0;
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

    BOOL(1)
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

    // GDI-only capture — DXGI Desktop Duplication returns all-black frames
    // on some GPU/driver configurations (e.g. hybrid GPU laptops where
    // the DXGI adapter doesn't match the display output).
    // GDI BitBlt with CAPTUREBLT is reliable for normal desktop capture.
    capture_rect_gdi(x, y, width, height)
}

/// DXGI Desktop Duplication capture (Win8+) — can capture DirectX fullscreen apps
fn capture_rect_dxgi(x: i32, y: i32, width: i32, height: i32) -> Result<CaptureResult> {
    use windows::Win32::Graphics::Direct3D::{D3D_DRIVER_TYPE_HARDWARE, D3D_DRIVER_TYPE_UNKNOWN};
    use windows::Win32::Graphics::Direct3D11::{
        D3D11_CREATE_DEVICE_FLAG, D3D11_SDK_VERSION, ID3D11Device, ID3D11DeviceContext,
        D3D11_TEXTURE2D_DESC, D3D11_RESOURCE_MISC_FLAG,
    };
    use windows::Win32::Graphics::Dxgi::{
        IDXGIOutputDuplication, DXGI_OUTDUPL_FRAME_INFO, DXGI_ERROR_ACCESS_LOST,
        DXGI_ERROR_WAIT_TIMEOUT,
    };
    use windows::Win32::Graphics::Dxgi::Common::{
        DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_SAMPLE_DESC,
    };
    use windows::core::Interface;

    unsafe {
        // 1. Create D3D11 device
        let mut device: Option<ID3D11Device> = None;
        let mut context: Option<ID3D11DeviceContext> = None;

        let hr = windows::Win32::Graphics::Direct3D11::D3D11CreateDevice(
            None,
            D3D_DRIVER_TYPE_HARDWARE,
            None,
            D3D11_CREATE_DEVICE_FLAG(0),
            None,
            D3D11_SDK_VERSION,
            Some(&mut device),
            None,
            Some(&mut context),
        );

        if hr.is_err() {
            anyhow::bail!("D3D11CreateDevice failed: {:?}", hr);
        }

        let device = device.ok_or_else(|| anyhow::anyhow!("No D3D11 device"))?;
        let context = context.ok_or_else(|| anyhow::anyhow!("No D3D11 context"))?;

        // 2. Get DXGI device → adapter → output
        let dxgi_device: windows::Win32::Graphics::Dxgi::IDXGIDevice =
            device.cast()?;
        let adapter = dxgi_device.GetAdapter()?;
        let output = adapter.EnumOutputs(0)?; // Primary output

        // 3. Get output description to determine which monitor
        let output_desc = output.GetDesc()?;

        // 4. Create Desktop Duplication
        let output1: windows::Win32::Graphics::Dxgi::IDXGIOutput1 = output.cast()?;
        let duplication: IDXGIOutputDuplication = output1.DuplicateOutput(&device)?;

        // 5. Acquire next frame (with short timeout)
        let mut frame_info = DXGI_OUTDUPL_FRAME_INFO::default();
        let mut resource: Option<windows::Win32::Graphics::Dxgi::IDXGIResource> = None;

        // Try to acquire frame — retry a few times for access-lost
        let acquired = (0..3).any(|attempt| {
            let result = duplication.AcquireNextFrame(
                if attempt == 0 { 100 } else { 500 }, // ms timeout
                &mut frame_info,
                &mut resource,
            );
            match result {
                Ok(()) => true,
                Err(e) if e.code() == DXGI_ERROR_WAIT_TIMEOUT => {
                    // No new frame — the desktop hasn't changed, that's OK
                    // We can still read the last frame, but we need to release first
                    false
                }
                Err(e) if e.code() == DXGI_ERROR_ACCESS_LOST => {
                    // Session locked or resolution changed — retry
                    let _ = duplication.ReleaseFrame();
                    false
                }
                Err(_) => false,
            }
        });

        if !acquired && resource.is_none() {
            // DXGI timeout — desktop hasn't changed, fall back to GDI
            let _ = duplication.ReleaseFrame();
            anyhow::bail!("DXGI: no frame available, falling back to GDI");
        }

        // 6. Get the surface from the frame resource
        let surface: windows::Win32::Graphics::Dxgi::IDXGISurface =
            resource.unwrap().cast()?;

        let desc = surface.GetDesc()?;
        let desktop_width = desc.Width as i32;
        let desktop_height = desc.Height as i32;

        // 7. Create a staging texture (CPU-readable) for the sub-region
        let staging_desc = D3D11_TEXTURE2D_DESC {
            Width: width as u32,
            Height: height as u32,
            MipLevels: 1,
            ArraySize: 1,
            Format: DXGI_FORMAT_B8G8R8A8_UNORM,
            SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
            Usage: windows::Win32::Graphics::Direct3D11::D3D11_USAGE_STAGING,
            BindFlags: 0,
            CPUAccessFlags: windows::Win32::Graphics::Direct3D11::D3D11_CPU_ACCESS_READ.0 as u32,
            MiscFlags: 0,
        };

        let mut staging_texture: Option<windows::Win32::Graphics::Direct3D11::ID3D11Texture2D> = None;
        device.CreateTexture2D(&staging_desc, None, Some(&mut staging_texture))?;
        let staging = staging_texture.ok_or_else(|| anyhow::anyhow!("Failed to create staging texture"))?;

        // 8. Copy sub-region from desktop surface to staging texture
        // The desktop texture coordinates are relative to the output's monitor
        // We need to map virtual screen coords to output-local coords
        let src_x = x - output_desc.DesktopCoordinates.left;
        let src_y = y - output_desc.DesktopCoordinates.top;

        let src_box = windows::Win32::Graphics::Direct3D11::D3D11_BOX {
            left: src_x.max(0) as u32,
            top: src_y.max(0) as u32,
            front: 0,
            right: (src_x.max(0) + width) as u32,
            bottom: (src_y.max(0) + height) as u32,
            back: 1,
        };

        // First copy full desktop to a GPU-readable texture, then sub-region to staging
        let full_desc = D3D11_TEXTURE2D_DESC {
            Width: desktop_width as u32,
            Height: desktop_height as u32,
            MipLevels: 1,
            ArraySize: 1,
            Format: DXGI_FORMAT_B8G8R8A8_UNORM,
            SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
            Usage: windows::Win32::Graphics::Direct3D11::D3D11_USAGE_DEFAULT,
            BindFlags: 0,
            CPUAccessFlags: 0,
            MiscFlags: 0,
        };

        let mut full_texture: Option<windows::Win32::Graphics::Direct3D11::ID3D11Texture2D> = None;
        device.CreateTexture2D(&full_desc, None, Some(&mut full_texture))?;
        let full_tex = full_texture.ok_or_else(|| anyhow::anyhow!("Failed to create full texture"))?;

        // Copy desktop surface → full GPU texture
        context.CopyResource(
            &full_tex,
            &surface.cast::<windows::Win32::Graphics::Direct3D11::ID3D11Resource>()?,
        );

        // Copy sub-region from full texture → staging (CPU-readable)
        context.CopySubresourceRegion(
            &staging,
            0,
            0, 0, 0,
            &full_tex,
            0,
            Some(&src_box),
        );

        // 9. Map staging texture and read pixels
        let mut mapped = windows::Win32::Graphics::Direct3D11::D3D11_MAPPED_SUBRESOURCE::default();
        context.Map(
            &staging,
            0,
            windows::Win32::Graphics::Direct3D11::D3D11_MAP_READ,
            0,
            Some(&mut mapped),
        )?;

        let row_pitch = mapped.RowPitch as usize;
        let pixel_count = (width as usize) * (height as usize);
        let mut pixels: Vec<u8> = Vec::with_capacity(pixel_count * 4);

        // Read row by row (row_pitch may have padding)
        for row in 0..height as usize {
            let src_offset = row * row_pitch;
            let src_row = std::slice::from_raw_parts(
                (mapped.pData as *const u8).add(src_offset),
                (width as usize) * 4,
            );
            pixels.extend_from_slice(src_row);
        }

        context.Unmap(&staging, 0);

        // 10. Release frame
        let _ = duplication.ReleaseFrame();

        // BGRA → RGBA conversion
        for chunk in pixels.chunks_exact_mut(4) {
            chunk.swap(0, 2); // B <-> R
        }

        let img = RgbaImage::from_raw(width as u32, height as u32, pixels)
            .ok_or_else(|| anyhow::anyhow!("Failed to create image from DXGI pixels"))?;

        let mut png_data = Vec::new();
        img.write_to(
            &mut std::io::Cursor::new(&mut png_data),
            image::ImageFormat::Png,
        )?;

        let base64_str = STANDARD.encode(&png_data);

        Ok(CaptureResult {
            image_base64: base64_str,
            width: width as u32,
            height: height as u32,
        })
    }
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

        // Overlay cursor icon onto the captured image
        overlay_cursor(hdc_mem, x, y, width, height);

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

/// Overlay the current cursor icon onto the captured image in the memory DC.
/// This draws the cursor at its current screen position (relative to the capture rect).
fn overlay_cursor(hdc_mem: HDC, capture_x: i32, capture_y: i32, _width: i32, _height: i32) {
    use windows::Win32::UI::WindowsAndMessaging::{
        DrawIconEx, GetCursorInfo, GetCursorPos, CURSORINFO, CURSORINFO_FLAGS, CURSOR_SHOWING,
        DI_COMPAT, DI_NORMAL,
    };

    unsafe {
        let mut ci = CURSORINFO {
            cbSize: std::mem::size_of::<CURSORINFO>() as u32,
            flags: CURSORINFO_FLAGS(0),
            hCursor: Default::default(),
            ptScreenPos: Default::default(),
        };

        if GetCursorInfo(&mut ci).is_ok() && ci.flags == CURSOR_SHOWING {
            let mut cursor_pos = Default::default();
            let _ = GetCursorPos(&mut cursor_pos);

            let local_x = cursor_pos.x - capture_x;
            let local_y = cursor_pos.y - capture_y;

            let _ = DrawIconEx(
                hdc_mem, local_x, local_y, ci.hCursor, 0, 0, 0, None,
                DI_NORMAL | DI_COMPAT,
            );
        }
    }
}
