use windows::Win32::Graphics::Gdi::{GetDC, GetPixel, ReleaseDC};

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct ColorInfo {
    pub hex: String,
    pub rgb: String,
    pub hsl: String,
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

#[tauri::command]
pub fn pick_color_at_point(x: i32, y: i32) -> Result<ColorInfo, String> {
    unsafe {
        let hdc = GetDC(None);
        if hdc.is_invalid() {
            return Err("Failed to get DC".to_string());
        }

        let color = GetPixel(hdc, x, y);
        let _ = ReleaseDC(None, hdc);

        // GetPixel returns BGR as COLORREF
        let color_val: u32 = color.0;
        let b = (color_val & 0xFF) as u8;
        let g = ((color_val >> 8) & 0xFF) as u8;
        let r = ((color_val >> 16) & 0xFF) as u8;

        // Convert to HSL
        let (h, s, l) = rgb_to_hsl(r, g, b);

        Ok(ColorInfo {
            hex: format!("#{:02X}{:02X}{:02X}", r, g, b),
            rgb: format!("rgb({}, {}, {})", r, g, b),
            hsl: format!("hsl({}, {}%, {}%)", h, s, l),
            r,
            g,
            b,
        })
    }
}

fn rgb_to_hsl(r: u8, g: u8, b: u8) -> (i32, i32, i32) {
    let r = r as f64 / 255.0;
    let g = g as f64 / 255.0;
    let b = b as f64 / 255.0;

    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;

    if (max - min).abs() < f64::EPSILON {
        return (0, 0, (l * 100.0).round() as i32);
    }

    let d = max - min;
    let s = if l > 0.5 { d / (2.0 - max - min) } else { d / (max + min) };

    let h = if (max - r).abs() < f64::EPSILON {
        (g - b) / d + if g < b { 6.0 } else { 0.0 }
    } else if (max - g).abs() < f64::EPSILON {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    };

    let h = (h * 60.0).round() as i32;
    let s = (s * 100.0).round() as i32;
    let l = (l * 100.0).round() as i32;

    (h, s, l)
}
