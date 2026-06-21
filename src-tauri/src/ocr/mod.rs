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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TranslateResult {
    pub original: String,
    pub translated: String,
    pub source_lang: String,
    pub target_lang: String,
}

#[tauri::command]
pub async fn ocr_image(app: tauri::AppHandle, image_base64: String) -> Result<OcrResult, String> {
    let python = find_python().ok_or("Python not found")?;

    let script_path = app
        .path()
        .resource_dir()
        .map(|d| d.join("scripts").join("ocr_worker.py"))
        .unwrap_or_else(|_| {
            std::path::PathBuf::from("../scripts/ocr_worker.py")
        });

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
            std::path::PathBuf::from("scripts/ocr_worker.py")
        }
    };

    if !script_path.exists() {
        return Err(format!("OCR worker script not found: {:?}", script_path));
    }

    let mut child = tokio::process::Command::new(&python)
        .arg(&script_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn OCR process: {}", e))?;

    if let Some(mut stdin) = child.stdin.take() {
        use tokio::io::AsyncWriteExt;
        stdin
            .write_all(image_base64.as_bytes())
            .await
            .map_err(|e| format!("Failed to write to OCR process: {}", e))?;
        drop(stdin);
    }

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

#[tauri::command]
pub async fn ocr_translate(
    text: String,
    target_lang: Option<String>,
) -> Result<TranslateResult, String> {
    let target = target_lang.unwrap_or_else(|| "en".to_string());

    // Determine source language heuristically
    let source_lang = detect_language(&text);

    // If source and target are the same, swap target
    let effective_target = if source_lang == target {
        if target == "en" { "zh".to_string() } else { "en".to_string() }
    } else {
        target
    };

    // Use MyMemory free translation API (no key required, 5000 chars/day)
    let encoded_text = urlencoding::encode(&text);
    let lang_pair = format!("{}|{}", source_lang, effective_target);
    let url = format!(
        "https://api.mymemory.translated.net/get?q={}&langpair={}",
        encoded_text, lang_pair
    );

    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| format!("Translation request failed: {}", e))?;

    let body: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse translation response: {}", e))?;

    let translated = body["responseData"]["translatedText"]
        .as_str()
        .unwrap_or("")
        .to_string();

    Ok(TranslateResult {
        original: text,
        translated,
        source_lang: source_lang.to_string(),
        target_lang: effective_target,
    })
}

/// Simple heuristic language detection based on character ranges
fn detect_language(text: &str) -> &'static str {
    let mut cjk_count = 0;
    let mut latin_count = 0;
    let mut hiragana_katakana = 0;
    let mut hangul = 0;

    for ch in text.chars() {
        if ('\u{4E00}'..='\u{9FFF}').contains(&ch) {
            cjk_count += 1;
        } else if ch.is_ascii_alphabetic() {
            latin_count += 1;
        } else if ('\u{3040}'..='\u{30FF}').contains(&ch) {
            hiragana_katakana += 1;
        } else if ('\u{AC00}'..='\u{D7AF}').contains(&ch) {
            hangul += 1;
        }
    }

    if hiragana_katakana > cjk_count && hiragana_katakana > latin_count {
        "ja"
    } else if hangul > cjk_count && hangul > latin_count {
        "ko"
    } else if cjk_count > latin_count {
        "zh"
    } else {
        "en"
    }
}

fn find_python() -> Option<String> {
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
