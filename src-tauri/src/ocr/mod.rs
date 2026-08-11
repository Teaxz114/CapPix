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
pub async fn ocr_image(
    app: tauri::AppHandle,
    image_base64: String,
    language: Option<String>,
    allow_online_fallback: Option<bool>,
) -> Result<OcrResult, String> {
    // Try local OCR worker first (cappix_ocr.exe or Python).
    match ocr_image_local(&app, &image_base64, language.as_deref()).await {
        Ok(result) => return Ok(result),
        Err(local_err) => {
            if !allow_online_fallback.unwrap_or(false) {
                log::warn!("[OCR] Local OCR failed; online fallback is disabled: {}", local_err);
                return Err(format!(
                    "本地 OCR 识别失败：{}。为保护截图隐私，未将截图上传到云端。可在“设置 > OCR”中明确开启“本地失败时允许云端 OCR 回退”后重试。",
                    local_err
                ));
            }

            log::info!("[OCR] Local OCR failed ({}), using user-approved online fallback", local_err);
        }
    }

    // Fallback: online OCR API (ocr.space free tier), explicitly approved by the caller.
    ocr_image_online(&image_base64, language.as_deref()).await
}

/// Local OCR via bundled cappix_ocr.exe or Python worker
async fn ocr_image_local(app: &tauri::AppHandle, image_base64: &str, _language: Option<&str>) -> Result<OcrResult, String> {
    let (program, args) = find_ocr_worker(app)?;

    log::info!("[OCR] Spawning local worker: {} {:?}", program, args);

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

    log::info!("[OCR] Local result: {} blocks, elapsed: {:?}s", result.blocks.len(), result.elapsed);
    Ok(result)
}

/// Online OCR fallback using ocr.space free API (no key required, 25K/month)
async fn ocr_image_online(image_base64: &str, language: Option<&str>) -> Result<OcrResult, String> {
    use base64::Engine;
    use base64::engine::general_purpose::STANDARD;

    let image_bytes = STANDARD.decode(image_base64)
        .map_err(|e| format!("Invalid base64: {}", e))?;

    // ocr.space free tier: 1MB limit per file, 25K/month
    if image_bytes.len() > 1_000_000 {
        return Err("图片过大（>1MB），在线 OCR 免费版不支持。请缩小截图后重试，或安装本地 OCR 引擎。".to_string());
    }

    // Determine format from magic bytes
    let filetype = if image_bytes.starts_with(b"\x89PNG") {
        "PNG"
    } else if image_bytes.starts_with(b"\xFF\xD8") {
        "JPG"
    } else {
        "PNG"
    };

    let start = std::time::Instant::now();

    // Map language code to ocr.space format
    let lang_code = match language.unwrap_or("ch_en") {
        "en" => "eng",
        "ch" => "chs",
        _ => "chs", // default: Chinese Simplified
    };

    let client = reqwest::Client::new();
    let body = reqwest::multipart::Form::new()
        .text("language", lang_code.to_string())
        .text("isOverlayRequired", "true".to_string())
        .part("file", reqwest::multipart::Part::bytes(image_bytes)
            .file_name(format!("image.{}", filetype.to_lowercase()))
            .mime_str(format!("image/{}", filetype.to_lowercase()).as_str())
            .unwrap_or_else(|_| reqwest::multipart::Part::bytes(vec![])
                .file_name("image.png")
                .mime_str("image/png").unwrap()));

    let response = client
        .post("https://api.ocr.space/parse/image")
        .multipart(body)
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await
        .map_err(|e| format!("Online OCR request failed: {}", e))?;

    let body: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse online OCR response: {}", e))?;

    let elapsed = start.elapsed().as_secs_f64();

    // Check for API errors
    if let Some(exit_code) = body.get("OCRExitCode").and_then(|v| v.as_i64()) {
        if exit_code != 1 {
            let error_msg = body.get("ErrorMessage")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown OCR API error");
            return Err(format!("Online OCR error: {}", error_msg));
        }
    }

    let mut all_text = String::new();
    let mut blocks = Vec::new();

    if let Some(parsed_results) = body.get("ParsedResults").and_then(|v| v.as_array()) {
        for parsed in parsed_results {
            if let Some(text) = parsed.get("ParsedText").and_then(|v| v.as_str()) {
                all_text.push_str(text);
                all_text.push('\n');
            }

            // Extract block-level info if available
            if let Some(overlay) = parsed.get("TextOverlay").and_then(|v| v.get("Lines")).and_then(|v| v.as_array()) {
                for line in overlay {
                    let line_text = line.get("LineText")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let mut bbox = Vec::new();
                    if let Some(words) = line.get("Words").and_then(|v| v.as_array()) {
                        for word in words {
                            let left = word.get("Left").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                            let top = word.get("Top").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                            let width = word.get("Width").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                            let height = word.get("Height").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                            bbox.push(vec![left, top]);
                            bbox.push(vec![left + width, top + height]);
                        }
                    }
                    blocks.push(OcrBlock {
                        text: line_text,
                        confidence: 0.9, // ocr.space doesn't provide per-block confidence
                        bbox,
                    });
                }
            }
        }
    }

    if all_text.trim().is_empty() {
        return Err("Online OCR returned empty result".to_string());
    }

    log::info!("[OCR] Online result: {} blocks, elapsed: {:.2}s", blocks.len(), elapsed);

    Ok(OcrResult {
        text: all_text.trim().to_string(),
        blocks,
        elapsed: Some(elapsed),
        error: None,
    })
}

#[tauri::command]
pub async fn ocr_translate(
    text: String,
    target_lang: Option<String>,
) -> Result<TranslateResult, String> {
    if text.trim().is_empty() {
        return Err("没有可翻译的文本".to_string());
    }

    let target = target_lang.unwrap_or_else(|| "zh".to_string());

    // Determine source language heuristically
    let source_lang = detect_language(&text);

    // If source and target are the same, swap target so translation is meaningful
    let effective_target = if source_lang == target {
        if target == "en" { "zh".to_string() } else { "en".to_string() }
    } else {
        target
    };

    // Map short codes to MyMemory-friendly codes for better accuracy
    let map_lang = |l: &str| -> String {
        match l {
            "zh" => "zh-CN".to_string(),
            other => other.to_string(),
        }
    };
    let src_code = map_lang(source_lang);
    let tgt_code = map_lang(&effective_target);

    // Use MyMemory free translation API (no key required, 5000 chars/day)
    let encoded_text = urlencoding::encode(&text);
    let lang_pair = format!("{}|{}", src_code, tgt_code);
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
        .map_err(|e| format!("翻译请求失败: {}", e))?;

    let body: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("翻译响应解析失败: {}", e))?;

    let translated = body["responseData"]["translatedText"]
        .as_str()
        .unwrap_or("")
        .to_string();

    if translated.is_empty() {
        // Surface MyMemory's error detail if present
        let detail = body["responseDetails"].as_str().unwrap_or("未知错误");
        return Err(format!("翻译失败: {}", detail));
    }

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
