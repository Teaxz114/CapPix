use serde::{Deserialize, Serialize};
use std::process::Stdio;
use tauri::Manager;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OcrBlock {
    pub text: String,
    pub confidence: f64,
    pub bbox: Vec<Vec<i32>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OcrResult {
    pub text: String,
    pub blocks: Vec<OcrBlock>,
    pub elapsed: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[tauri::command]
pub async fn ocr_image(app: tauri::AppHandle, image_base64: String) -> Result<OcrResult, String> {
    // Find the Python executable
    let python = find_python().ok_or("Python not found")?;

    // Find the OCR worker script
    let script_path = app
        .path()
        .resource_dir()
        .map(|d| d.join("scripts").join("ocr_worker.py"))
        .unwrap_or_else(|_| {
            std::path::PathBuf::from("../scripts/ocr_worker.py")
        });

    // Fallback: try relative to current exe
    let script_path = if script_path.exists() {
        script_path
    } else {
        let exe_dir = std::env::current_exe()
            .map(|e| e.parent().map(|p| p.to_path_buf()).unwrap_or_default())
            .unwrap_or_default();
        let candidate = exe_dir.join("../scripts/ocr_worker.py");
        if candidate.exists() {
            candidate
        } else {
            // Dev mode: relative to project root
            std::path::PathBuf::from("scripts/ocr_worker.py")
        }
    };

    if !script_path.exists() {
        return Err(format!("OCR worker script not found: {:?}", script_path));
    }

    // Spawn Python subprocess
    let mut child = tokio::process::Command::new(&python)
        .arg(&script_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn OCR process: {}", e))?;

    // Write base64 to stdin
    if let Some(mut stdin) = child.stdin.take() {
        use tokio::io::AsyncWriteExt;
        stdin
            .write_all(image_base64.as_bytes())
            .await
            .map_err(|e| format!("Failed to write to OCR process: {}", e))?;
        drop(stdin);
    }

    // Read output with timeout
    let output = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        child.wait_with_output(),
    )
    .await
    .map_err(|_| "OCR process timed out (30s)".to_string())?
    .map_err(|e| format!("OCR process failed: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("OCR process error: {}", stderr));
    }

    let result: OcrResult =
        serde_json::from_slice(&output.stdout).map_err(|e| format!("Failed to parse OCR result: {}", e))?;

    Ok(result)
}

fn find_python() -> Option<String> {
    // Try python3 first, then python
    for cmd in &["python", "python3"] {
        if which_python(cmd).is_some() {
            return Some(cmd.to_string());
        }
    }
    None
}

fn which_python(cmd: &str) -> Option<()> {
    std::process::Command::new(cmd)
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .ok()
        .map(|s| if s.success() { Some(()) } else { None })
        .flatten()
}
