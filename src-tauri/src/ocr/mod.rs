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

/// OCR worker executable — tries in order:
/// 1. Bundled cappix_ocr.exe (PyInstaller standalone, no Python needed)
/// 2. Python + ocr_worker.py (dev mode, requires Python + rapidocr)
fn find_ocr_worker(app: &tauri::AppHandle) -> Result<(String, Vec<String>), String> {
    let exe_dir = std::env::current_exe()
        .map(|e| e.parent().map(|p| p.to_path_buf()).unwrap_or_default())
        .unwrap_or_default();

    // Search paths for cappix_ocr.exe (relative to main EXE)
    let bundled_candidates = vec![
        exe_dir.join("cappix_ocr.exe"),
        exe_dir.join("scripts").join("cappix_ocr.exe"),
        exe_dir.join("..").join("scripts").join("cappix_ocr.exe"),
    ];

    for candidate in &bundled_candidates {
        if candidate.exists() {
            log::info!("[OCR] Found bundled worker: {:?}", candidate);
            return Ok((candidate.to_string_lossy().to_string(), vec![]));
        }
    }

    // Also try resource_dir (for cargo tauri build)
    if let Ok(resource_dir) = app.path().resource_dir() {
        let candidate = resource_dir.join("scripts").join("cappix_ocr.exe");
        if candidate.exists() {
            log::info!("[OCR] Found bundled worker in resources: {:?}", candidate);
            return Ok((candidate.to_string_lossy().to_string(), vec![]));
        }
    }

    // Fallback: Python + ocr_worker.py (dev mode)
    if let Some(python) = find_python() {
        let script_candidates = vec![
            exe_dir.join("scripts").join("ocr_worker.py"),
            exe_dir.join("..").join("scripts").join("ocr_worker.py"),
        ];

        if let Ok(resource_dir) = app.path().resource_dir() {
            let mut with_resource = script_candidates;
            with_resource.push(resource_dir.join("scripts").join("ocr_worker.py"));
            for candidate in &with_resource {
                if candidate.exists() {
                    log::info!("[OCR] Using Python worker: {:?} {:?}", python, candidate);
                    return Ok((python, vec![candidate.to_string_lossy().to_string()]));
                }
            }
        } else {
            for candidate in &script_candidates {
                if candidate.exists() {
                    log::info!("[OCR] Using Python worker: {:?} {:?}", python, candidate);
                    return Ok((python, vec![candidate.to_string_lossy().to_string()]));
                }
            }
        }
    }

    Err("OCR worker not found. Install cappix_ocr.exe or Python + rapidocr_onnxruntime".to_string())
}

#[tauri::command]
pub async fn ocr_image(app: tauri::AppHandle, image_base64: String) -> Result<OcrResult, String> {
    let (program, args) = find_ocr_worker(&app)?;

    log::info!("[OCR] Spawning: {} {:?}", program, args);

    let mut cmd = tokio::process::Command::new(&program);
    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    for arg in &args {
        cmd.arg(arg);
    }

    let mut child = cmd
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

    let stdout_str = String::from_utf8_lossy(&output.stdout);
    let result: OcrResult = serde_json::from_str(stdout_str.trim())
        .map_err(|e| format!("Failed to parse OCR result: {} (raw: {})", e, &stdout_str[..stdout_str.len().min(200)]))?;

    log::info!("[OCR] Result: {} blocks, elapsed: {:?}s", result.blocks.len(), result.elapsed);

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
