use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::Manager;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RecordingState {
    pub is_recording: bool,
    pub is_paused: bool,
    pub output_path: String,
    pub duration_secs: f64,
    pub started_at: Option<f64>, // unix timestamp when recording started
    pub paused_at: Option<f64>,  // unix timestamp when paused (accumulated pause time)
}

pub struct RecordingManager {
    process: Mutex<Option<tokio::process::Child>>,
    pub state: Mutex<RecordingState>,
    /// Serializes start/stop/pause/resume transitions. The state check and
    /// ffmpeg spawn must be one operation, otherwise two concurrent invokes
    /// can both pass the check and orphan the first child.
    operation_lock: tokio::sync::Mutex<()>,
}

impl RecordingManager {
    pub fn new() -> Self {
        Self {
            process: Mutex::new(None),
            state: Mutex::new(RecordingState {
                is_recording: false,
                is_paused: false,
                output_path: String::new(),
                duration_secs: 0.0,
                started_at: None,
                paused_at: None,
            }),
            operation_lock: tokio::sync::Mutex::new(()),
        }
    }
}

fn parse_first_dshow_audio_device(text: &str) -> Option<String> {
    for line in text.lines() {
        if !line.contains("(audio)") {
            continue;
        }
        if let Some(start) = line.find('"') {
            if let Some(end_rel) = line[start + 1..].find('"') {
                let name = line[start + 1..start + 1 + end_rel].trim();
                if !name.is_empty() {
                    return Some(name.to_string());
                }
            }
        }
    }
    None
}

/// Return the first DirectShow audio input reported by FFmpeg. Device names are
/// machine-specific, so never hard-code "Stereo Mix" or a localized microphone
/// name into the command line.
async fn first_dshow_audio_device() -> Result<String, String> {
    let output = tokio::process::Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-list_devices",
            "true",
            "-f",
            "dshow",
            "-i",
            "dummy",
        ])
        .output()
        .await
        .map_err(|e| format!("无法枚举音频设备: {}", e))?;
    let text = String::from_utf8_lossy(&output.stderr);
    parse_first_dshow_audio_device(&text)
        .ok_or_else(|| "未找到可用的 DirectShow 音频输入设备".to_string())
}

#[tauri::command]
pub async fn start_recording(
    app: tauri::AppHandle,
    output_path: Option<String>,
    region: Option<(i32, i32, i32, i32)>, // x, y, w, h
    with_audio: Option<bool>,
) -> Result<String, String> {
    let state = app.state::<RecordingManager>();
    let _operation_guard = state.operation_lock.lock().await;

    // Check if already recording
    {
        let s = state.state.lock().map_err(|e| e.to_string())?;
        if s.is_recording {
            return Err("Already recording".to_string());
        }
    }

    // Check ffmpeg availability
    let check = tokio::process::Command::new("ffmpeg")
        .arg("-version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .await;
    if check.is_err() {
        return Err(
            "FFmpeg 未安装或不在 PATH 中。请安装 FFmpeg: https://ffmpeg.org/download.html"
                .to_string(),
        );
    }

    // Determine output path
    let out = output_path.unwrap_or_else(|| {
        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
        let dir = std::env::var("USERPROFILE")
            .map(|p| PathBuf::from(p).join("Videos"))
            .unwrap_or_else(|_| PathBuf::from("."));
        dir.join(format!("CapPix_{}.mp4", timestamp))
            .to_string_lossy()
            .to_string()
    });

    // Build ffmpeg command
    // Use gdigrab for screen capture on Windows
    let mut cmd = tokio::process::Command::new("ffmpeg");

    cmd.arg("-y") // overwrite output
        .arg("-f")
        .arg("gdigrab")
        .arg("-framerate")
        .arg("30");

    // Add region if specified
    if let Some((x, y, w, h)) = region {
        cmd.arg("-offset_x")
            .arg(x.to_string())
            .arg("-offset_y")
            .arg(y.to_string())
            .arg("-video_size")
            .arg(format!("{}x{}", w, h));
    }

    cmd.arg("-i").arg("desktop");

    // Optional microphone capture through DirectShow. FFmpeg builds vary in
    // WASAPI support, so enumerate a real device instead of using unsupported
    // loopback flags or the machine-specific "Stereo Mix" name.
    if with_audio.unwrap_or(false) {
        let audio_device = first_dshow_audio_device().await?;
        cmd.arg("-f")
            .arg("dshow")
            .arg("-i")
            .arg(format!("audio={}", audio_device))
            .arg("-c:a")
            .arg("aac")
            .arg("-b:a")
            .arg("128k");
    }

    cmd.arg("-c:v")
        .arg("libx264")
        .arg("-preset")
        .arg("ultrafast")
        .arg("-crf")
        .arg("23")
        .arg("-pix_fmt")
        .arg("yuv420p")
        .arg(&out);

    cmd.stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped());

    let child = cmd
        .spawn()
        .map_err(|e| format!("Failed to start ffmpeg: {}", e))?;

    // Update state
    {
        let state = app.state::<RecordingManager>();
        let mut s = state.state.lock().map_err(|e| e.to_string())?;
        s.is_recording = true;
        s.is_paused = false;
        s.output_path = out.clone();
        s.duration_secs = 0.0;
        s.started_at = Some(chrono::Local::now().timestamp_millis() as f64 / 1000.0);
        s.paused_at = None;
        let mut p = state.process.lock().map_err(|e| e.to_string())?;
        *p = Some(child);
    }

    Ok(out)
}

#[tauri::command]
pub async fn stop_recording(app: tauri::AppHandle) -> Result<String, String> {
    let state = app.state::<RecordingManager>();
    let _operation_guard = state.operation_lock.lock().await;

    // Take the child process out of the mutex to avoid holding MutexGuard across await
    let mut child_opt = {
        let mut p = state.process.lock().map_err(|e| e.to_string())?;
        p.take()
    };

    if let Some(ref mut child) = child_opt {
        // Send 'q' to ffmpeg to gracefully stop
        if let Some(stdin) = child.stdin.as_mut() {
            use tokio::io::AsyncWriteExt;
            let _ = stdin.write_all(b"q").await;
        }

        // Graceful stop must be bounded. A wedged ffmpeg process must not
        // leave the UI in an endless "stopping" state.
        let stopped = matches!(
            tokio::time::timeout(std::time::Duration::from_secs(5), child.wait(),).await,
            Ok(Ok(_))
        );
        if !stopped {
            log::warn!("[Recording] ffmpeg did not stop within 5s; killing it");
            let _ = child.kill().await;
            let _ = child.wait().await;
        }
    }

    // The child has exited (or was killed); never put a dead handle back into
    // the manager, otherwise later starts can be misclassified as active.
    {
        let mut p = state.process.lock().map_err(|e| e.to_string())?;
        *p = None;
    }

    let output_path = {
        let mut s = state.state.lock().map_err(|e| e.to_string())?;
        s.is_recording = false;
        s.is_paused = false;
        s.started_at = None;
        s.paused_at = None;
        let path = s.output_path.clone();
        path
    };

    Ok(output_path)
}

#[tauri::command]
pub fn get_recording_state(app: tauri::AppHandle) -> Result<RecordingState, String> {
    let state = app.state::<RecordingManager>();
    let mut s = state.state.lock().map_err(|e| e.to_string())?;

    // Calculate actual duration from started_at
    if s.is_recording {
        if s.is_paused {
            // While paused, duration stays at the value when paused
            s.duration_secs = s.paused_at.unwrap_or(0.0);
        } else if let Some(started) = s.started_at {
            let now = chrono::Local::now().timestamp_millis() as f64 / 1000.0;
            s.duration_secs = s.paused_at.unwrap_or(0.0) + (now - started);
        }
    }

    Ok(s.clone())
}

#[tauri::command]
pub async fn pause_recording(app: tauri::AppHandle) -> Result<(), String> {
    let state = app.state::<RecordingManager>();
    let _operation_guard = state.operation_lock.lock().await;
    let mut s = state.state.lock().map_err(|e| e.to_string())?;
    if !s.is_recording || s.is_paused {
        return Err("Not recording or already paused".to_string());
    }

    // Suspend the ffmpeg process threads on Windows
    let process = state.process.lock().map_err(|e| e.to_string())?;
    if let Some(ref child) = *process {
        if let Some(pid) = child.id() {
            if let Some(handle) = get_process_handle(pid) {
                let suspended = unsafe { suspend_process_threads(handle) };
                unsafe {
                    let _ = windows::Win32::Foundation::CloseHandle(handle);
                }
                if suspended {
                    s.is_paused = true;
                    // Store accumulated duration at pause time
                    s.paused_at = Some(s.duration_secs);
                    log::info!("[Recording] Paused ffmpeg process (PID {})", pid);
                    return Ok(());
                }
            }
        }
    }

    Err("Failed to pause ffmpeg process".to_string())
}

#[tauri::command]
pub async fn resume_recording(app: tauri::AppHandle) -> Result<(), String> {
    let state = app.state::<RecordingManager>();
    let _operation_guard = state.operation_lock.lock().await;
    let mut s = state.state.lock().map_err(|e| e.to_string())?;
    if !s.is_recording || !s.is_paused {
        return Err("Not recording or not paused".to_string());
    }

    // Resume the ffmpeg process threads
    let process = state.process.lock().map_err(|e| e.to_string())?;
    if let Some(ref child) = *process {
        if let Some(pid) = child.id() {
            if let Some(handle) = get_process_handle(pid) {
                let resumed = unsafe { resume_process_threads(handle) };
                unsafe {
                    let _ = windows::Win32::Foundation::CloseHandle(handle);
                }
                if resumed {
                    s.is_paused = false;
                    // Reset started_at so duration = paused_at + (now - new started_at)
                    s.started_at = Some(chrono::Local::now().timestamp_millis() as f64 / 1000.0);
                    log::info!("[Recording] Resumed ffmpeg process (PID {})", pid);
                    return Ok(());
                }
            }
        }
    }

    Err("Failed to resume ffmpeg process".to_string())
}

/// Get a PROCESS handle for the given PID with PROCESS_SUSPEND_RESUME access
fn get_process_handle(pid: u32) -> Option<windows::Win32::Foundation::HANDLE> {
    use windows::Win32::System::Threading::{OpenProcess, PROCESS_SUSPEND_RESUME};
    unsafe { OpenProcess(PROCESS_SUSPEND_RESUME, false, pid).ok() }
}

/// Suspend all threads of the process via NtSuspendProcess (ntdll.dll)
unsafe fn suspend_process_threads(handle: windows::Win32::Foundation::HANDLE) -> bool {
    use std::ffi::CString;
    let ntdll = windows::Win32::System::LibraryLoader::GetModuleHandleA(windows::core::PCSTR(
        CString::new("ntdll.dll").unwrap().as_ptr() as *const u8,
    ))
    .ok();
    if let Some(module) = ntdll {
        let proc_name = CString::new("NtSuspendProcess").unwrap();
        let proc_addr = windows::Win32::System::LibraryLoader::GetProcAddress(
            module,
            windows::core::PCSTR(proc_name.as_ptr() as *const u8),
        );
        if let Some(addr) = proc_addr {
            let nt_suspend: extern "system" fn(windows::Win32::Foundation::HANDLE) -> i32 =
                std::mem::transmute(addr);
            nt_suspend(handle) == 0
        } else {
            false
        }
    } else {
        false
    }
}

/// Resume all threads of the process via NtResumeProcess (ntdll.dll)
unsafe fn resume_process_threads(handle: windows::Win32::Foundation::HANDLE) -> bool {
    use std::ffi::CString;
    let ntdll = windows::Win32::System::LibraryLoader::GetModuleHandleA(windows::core::PCSTR(
        CString::new("ntdll.dll").unwrap().as_ptr() as *const u8,
    ))
    .ok();
    if let Some(module) = ntdll {
        let proc_name = CString::new("NtResumeProcess").unwrap();
        let proc_addr = windows::Win32::System::LibraryLoader::GetProcAddress(
            module,
            windows::core::PCSTR(proc_name.as_ptr() as *const u8),
        );
        if let Some(addr) = proc_addr {
            let nt_resume: extern "system" fn(windows::Win32::Foundation::HANDLE) -> i32 =
                std::mem::transmute(addr);
            nt_resume(handle) == 0
        } else {
            false
        }
    } else {
        false
    }
}

#[tauri::command]
pub async fn record_to_gif(
    app: tauri::AppHandle,
    output_path: Option<String>,
    region: Option<(i32, i32, i32, i32)>,
    duration_secs: Option<f64>,
) -> Result<String, String> {
    let state = app.state::<RecordingManager>();
    let _operation_guard = state.operation_lock.lock().await;
    {
        let s = state.state.lock().map_err(|e| e.to_string())?;
        if s.is_recording {
            return Err("已有录屏正在进行，无法同时启动 GIF 录制".to_string());
        }
    }

    // Record a short clip and convert to GIF
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
    let dir = std::env::var("USERPROFILE")
        .map(|p| PathBuf::from(p).join("Videos"))
        .unwrap_or_else(|_| PathBuf::from("."));
    let temp_mp4 = dir.join(format!("CapPix_temp_{}.mp4", timestamp));
    let gif_out = output_path.unwrap_or_else(|| {
        dir.join(format!("CapPix_{}.gif", timestamp))
            .to_string_lossy()
            .to_string()
    });

    // Record with ffmpeg gdigrab
    let duration = duration_secs.unwrap_or(5.0);
    let mut cmd = tokio::process::Command::new("ffmpeg");
    cmd.arg("-y")
        .arg("-f")
        .arg("gdigrab")
        .arg("-framerate")
        .arg("15")
        .arg("-t")
        .arg(duration.to_string());

    if let Some((x, y, w, h)) = region {
        cmd.arg("-offset_x")
            .arg(x.to_string())
            .arg("-offset_y")
            .arg(y.to_string())
            .arg("-video_size")
            .arg(format!("{}x{}", w, h));
    }

    cmd.arg("-i")
        .arg("desktop")
        .arg("-c:v")
        .arg("libx264")
        .arg("-preset")
        .arg("ultrafast")
        .arg("-pix_fmt")
        .arg("yuv420p")
        .arg(temp_mp4.to_string_lossy().to_string());

    let output = cmd
        .output()
        .await
        .map_err(|e| format!("ffmpeg record failed: {}", e))?;
    if !output.status.success() {
        return Err("Failed to record for GIF".to_string());
    }

    // Convert to GIF with palette for better quality
    let palette_path = dir.join(format!("palette_{}.png", timestamp));
    let palette_output = tokio::process::Command::new("ffmpeg")
        .arg("-y")
        .arg("-i")
        .arg(temp_mp4.to_string_lossy().to_string())
        .arg("-vf")
        .arg("palettegen")
        .arg(palette_path.to_string_lossy().to_string())
        .output()
        .await
        .map_err(|e| format!("Palette gen failed: {}", e))?;

    if !palette_output.status.success() {
        // Fallback: simple GIF conversion
        let simple_out = tokio::process::Command::new("ffmpeg")
            .arg("-y")
            .arg("-i")
            .arg(temp_mp4.to_string_lossy().to_string())
            .arg("-vf")
            .arg("fps=10,scale=800:-1:flags=lanczos")
            .arg(&gif_out)
            .output()
            .await
            .map_err(|e| format!("GIF conversion failed: {}", e))?;

        if !simple_out.status.success() {
            return Err("Failed to convert to GIF".to_string());
        }
    } else {
        // Use palette for high quality GIF
        let gif_output = tokio::process::Command::new("ffmpeg")
            .arg("-y")
            .arg("-i")
            .arg(temp_mp4.to_string_lossy().to_string())
            .arg("-i")
            .arg(palette_path.to_string_lossy().to_string())
            .arg("-lavfi")
            .arg("paletteuse")
            .arg("-vf")
            .arg("fps=10,scale=800:-1:flags=lanczos")
            .arg(&gif_out)
            .output()
            .await
            .map_err(|e| format!("GIF conversion failed: {}", e))?;

        if !gif_output.status.success() {
            return Err("Failed to convert to GIF with palette".to_string());
        }
    }

    // Clean up temp files
    let _ = std::fs::remove_file(&temp_mp4);
    let _ = std::fs::remove_file(&palette_path);

    Ok(gif_out)
}

#[cfg(test)]
mod tests {
    use super::parse_first_dshow_audio_device;

    #[test]
    fn parses_first_audio_device_and_ignores_video() {
        let output = r#"
[dshow @ 1] "OBS Virtual Camera" (video)
[dshow @ 1] "Mic (MCHOSE G9 PRO)" (audio)
[dshow @ 1] "Other Mic" (audio)
"#;
        assert_eq!(
            parse_first_dshow_audio_device(output).as_deref(),
            Some("Mic (MCHOSE G9 PRO)")
        );
    }

    #[test]
    fn returns_none_without_audio_device() {
        assert_eq!(
            parse_first_dshow_audio_device("[dshow] \\\"Camera\\\" (video)"),
            None
        );
    }
}
