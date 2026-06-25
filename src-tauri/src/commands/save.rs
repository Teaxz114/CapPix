use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use std::path::PathBuf;
use std::sync::Mutex;

/// State to track the auto-increment sequence number per save directory
pub struct SaveSeqState {
    pub seq: Mutex<u32>,
}

#[tauri::command]
pub fn save_image_to_file(app: tauri::AppHandle, image_base64: String) -> Result<String, String> {
    let data = STANDARD.decode(&image_base64).map_err(|e| e.to_string())?;

    use tauri_plugin_dialog::DialogExt;
    let file_path = app.dialog()
        .file()
        .set_title("保存截图")
        .set_file_name("CapPix_screenshot.png")
        .add_filter("PNG 图片", &["png"])
        .add_filter("JPEG 图片", &["jpg", "jpeg"])
        .blocking_save_file();

    let file_path = file_path.ok_or_else(|| "用户取消了保存".to_string())?;
    let path_str = file_path.to_string();

    std::fs::write(&path_str, &data).map_err(|e| format!("写入文件失败: {}", e))?;

    Ok(path_str)
}

/// Save image directly to a specified path (no dialog)
#[tauri::command]
pub fn save_image_to_path(
    image_base64: String,
    save_path: String,
) -> Result<String, String> {
    let data = STANDARD.decode(&image_base64).map_err(|e| e.to_string())?;

    // Ensure parent directory exists
    let parent = PathBuf::from(&save_path)
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));
    std::fs::create_dir_all(&parent).map_err(|e| format!("创建目录失败: {}", e))?;

    std::fs::write(&save_path, &data).map_err(|e| format!("写入文件失败: {}", e))?;

    Ok(save_path)
}

/// Prepare the save directory and generate the full file path from a pattern.
/// Returns the full file path with the next sequence number.
/// 
/// Pattern variables:
///   {date} - current date in YYYY-MM-DD format
///   {time} - current time in HH-mm-ss format
///   {seq}  - auto-increment sequence number (resets on app restart)
///
/// If save_directory is empty, defaults to Pictures/CapPix in the user's home.
#[tauri::command]
pub fn prepare_save_path(
    app: tauri::AppHandle,
    state: tauri::State<SaveSeqState>,
    save_directory: String,
    filename_pattern: String,
    file_format: String,
) -> Result<String, String> {
    // Determine save directory
    let save_dir = if save_directory.is_empty() {
        // Default: Pictures/CapPix in user's home directory
        let pictures_dir = dirs::picture_dir()
            .unwrap_or_else(|| {
                let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
                home.join("Pictures")
            });
        pictures_dir.join("CapPix")
    } else {
        PathBuf::from(&save_directory)
    };

    // Ensure the directory exists
    std::fs::create_dir_all(&save_dir).map_err(|e| format!("创建保存目录失败: {}", e))?;

    // Get current date/time
    let now = chrono::Local::now();
    let date_str = now.format("%Y-%m-%d").to_string();
    let time_str = now.format("%H-%M-%S").to_string();

    // Get next sequence number
    let seq = {
        let mut seq_val = state.seq.lock().map_err(|e| e.to_string())?;
        *seq_val += 1;
        *seq_val
    };

    // Determine file extension
    let ext = match file_format.as_str() {
        "jpg" | "jpeg" => "jpg",
        "bmp" => "bmp",
        _ => "png",
    };

    // Replace pattern variables
    let filename = filename_pattern
        .replace("{date}", &date_str)
        .replace("{time}", &time_str)
        .replace("{seq}", &format!("{:04}", seq));

    let full_filename = format!("{}.{}", filename, ext);

    let full_path = save_dir.join(&full_filename);

    // If file already exists, increment seq until we find a free name
    let mut final_path = full_path.clone();
    let mut current_seq = seq;
    while final_path.exists() {
        current_seq += 1;
        let retry_filename = filename_pattern
            .replace("{date}", &date_str)
            .replace("{time}", &time_str)
            .replace("{seq}", &format!("{:04}", current_seq));
        let retry_full = format!("{}.{}", retry_filename, ext);
        final_path = save_dir.join(&retry_full);
    }

    // Update the sequence state to the last used value
    {
        let mut seq_val = state.seq.lock().map_err(|e| e.to_string())?;
        *seq_val = current_seq;
    }

    Ok(final_path.to_string_lossy().to_string())
}
