use base64::Engine;
use base64::engine::general_purpose::STANDARD;

#[tauri::command]
pub fn save_image_to_file(app: tauri::AppHandle, image_base64: String) -> Result<String, String> {
    let data = STANDARD.decode(&image_base64).map_err(|e| e.to_string())?;

    use tauri_plugin_dialog::DialogExt;
    let path = app.dialog()
        .file()
        .set_title("保存截图")
        .set_file_name("CapPix_screenshot.png")
        .add_filter("PNG 图片", &["png"])
        .add_filter("JPEG 图片", &["jpg", "jpeg"])
        .blocking_save_file();

    let path = path.ok_or_else(|| "用户取消了保存".to_string())?;
    let path_str = path.to_string();

    std::fs::write(&path_str, &data).map_err(|e| format!("写入文件失败: {}", e))?;

    Ok(path_str)
}
