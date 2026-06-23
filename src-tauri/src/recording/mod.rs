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
}

pub struct RecordingManager {
    process: Mutex<Option<tokio::process::Child>>,
    state: Mutex<RecordingState>,
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
            }),
        }
    }
}

#[tauri::command]
pub async fn start_recording(
    app: tauri::AppHandle,
    output_path: Option<String>,
    region: Option<(i32, i32, i32, i32)>, // x, y, w, h
    with_audio: Option<bool>,
) -> Result<String, String> {
    // Check if already recording
    {
        let state = app.state::<RecordingManager>();
        let s = state.state.lock().map_err(|e| e.to_string())?;
        if s.is_recording {
            return Err("Already recording".to_string());
        }
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
        .arg("-f").arg("gdigrab")
        .arg("-framerate").arg("30");

    // Add region if specified
    if let Some((x, y, w, h)) = region {
        cmd.arg("-offset_x").arg(x.to_string())
            .arg("-offset_y").arg(y.to_string())
            .arg("-video_size").arg(format!("{}x{}", w, h));
    }

    cmd.arg("-i").arg("desktop");

    // Audio capture: optional system audio (WASAPI loopback) or microphone (dshow)
    // System audio uses WASAPI loopback: -f wasapi -i "Stereo Mix (设备名)"
    // Microphone uses DirectShow: -f dshow -i "audio=麦克风设备名"
    if with_audio.unwrap_or(false) {
        // Try system audio (WASAPI loopback) first — captures what you hear
        // FFmpeg on Windows supports: -f wasapi -i "{GUID}"
        // The loopback device name can be enumerated; for now use a common default
        cmd.arg("-f").arg("wasapi")
            .arg("-i").arg("audio=Stereo Mix")
            .arg("-c:a").arg("aac")
            .arg("-b:a").arg("128k");
    }

    cmd.arg("-c:v").arg("libx264")
        .arg("-preset").arg("ultrafast")
        .arg("-crf").arg("23")
        .arg("-pix_fmt").arg("yuv420p")
        .arg(&out);

    cmd.stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped());

    let child = cmd.spawn().map_err(|e| format!("Failed to start ffmpeg: {}", e))?;

    // Update state
    {
        let state = app.state::<RecordingManager>();
        let mut s = state.state.lock().map_err(|e| e.to_string())?;
        s.is_recording = true;
        s.is_paused = false;
        s.output_path = out.clone();
        s.duration_secs = 0.0;
        let mut p = state.process.lock().map_err(|e| e.to_string())?;
        *p = Some(child);
    }

    Ok(out)
}

#[tauri::command]
pub async fn stop_recording(app: tauri::AppHandle) -> Result<String, String> {
    let state = app.state::<RecordingManager>();

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

        // Wait for process to exit
        let _ = child.wait().await;
    }

    // Put it back (or leave None)
    {
        let mut p = state.process.lock().map_err(|e| e.to_string())?;
        *p = child_opt;
    }

    let output_path = {
        let mut s = state.state.lock().map_err(|e| e.to_string())?;
        s.is_recording = false;
        s.is_paused = false;
        let path = s.output_path.clone();
        path
    };

    Ok(output_path)
}

#[tauri::command]
pub fn get_recording_state(app: tauri::AppHandle) -> Result<RecordingState, String> {
    let state = app.state::<RecordingManager>();
    let s = state.state.lock().map_err(|e| e.to_string())?;
    Ok(s.clone())
}

#[tauri::command]
pub fn pause_recording(app: tauri::AppHandle) -> Result<(), String> {
    // FFmpeg gdigrab does not support pause/resume natively.
    // Mark the state as paused for UI purposes; actual frame capture continues.
    let state = app.state::<RecordingManager>();
    let mut s = state.state.lock().map_err(|e| e.to_string())?;
    if !s.is_recording {
        return Err("Not recording".to_string());
    }
    s.is_paused = true;
    Ok(())
}

#[tauri::command]
pub fn resume_recording(app: tauri::AppHandle) -> Result<(), String> {
    let state = app.state::<RecordingManager>();
    let mut s = state.state.lock().map_err(|e| e.to_string())?;
    if !s.is_recording {
        return Err("Not recording".to_string());
    }
    s.is_paused = false;
    Ok(())
}

#[tauri::command]
pub async fn record_to_gif(
    _app: tauri::AppHandle,
    output_path: Option<String>,
    region: Option<(i32, i32, i32, i32)>,
    duration_secs: Option<f64>,
) -> Result<String, String> {
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
        .arg("-f").arg("gdigrab")
        .arg("-framerate").arg("15")
        .arg("-t").arg(duration.to_string());

    if let Some((x, y, w, h)) = region {
        cmd.arg("-offset_x").arg(x.to_string())
            .arg("-offset_y").arg(y.to_string())
            .arg("-video_size").arg(format!("{}x{}", w, h));
    }

    cmd.arg("-i").arg("desktop")
        .arg("-c:v").arg("libx264")
        .arg("-preset").arg("ultrafast")
        .arg("-pix_fmt").arg("yuv420p")
        .arg(temp_mp4.to_string_lossy().to_string());

    let output = cmd.output().await.map_err(|e| format!("ffmpeg record failed: {}", e))?;
    if !output.status.success() {
        return Err("Failed to record for GIF".to_string());
    }

    // Convert to GIF with palette for better quality
    let palette_path = dir.join(format!("palette_{}.png", timestamp));
    let palette_output = tokio::process::Command::new("ffmpeg")
        .arg("-y")
        .arg("-i").arg(temp_mp4.to_string_lossy().to_string())
        .arg("-vf").arg("palettegen")
        .arg(palette_path.to_string_lossy().to_string())
        .output().await.map_err(|e| format!("Palette gen failed: {}", e))?;

    if !palette_output.status.success() {
        // Fallback: simple GIF conversion
        let simple_out = tokio::process::Command::new("ffmpeg")
            .arg("-y")
            .arg("-i").arg(temp_mp4.to_string_lossy().to_string())
            .arg("-vf").arg("fps=10,scale=800:-1:flags=lanczos")
            .arg(&gif_out)
            .output().await.map_err(|e| format!("GIF conversion failed: {}", e))?;

        if !simple_out.status.success() {
            return Err("Failed to convert to GIF".to_string());
        }
    } else {
        // Use palette for high quality GIF
        let gif_output = tokio::process::Command::new("ffmpeg")
            .arg("-y")
            .arg("-i").arg(temp_mp4.to_string_lossy().to_string())
            .arg("-i").arg(palette_path.to_string_lossy().to_string())
            .arg("-lavfi").arg("paletteuse")
            .arg("-vf").arg("fps=10,scale=800:-1:flags=lanczos")
            .arg(&gif_out)
            .output().await.map_err(|e| format!("GIF conversion failed: {}", e))?;

        if !gif_output.status.success() {
            return Err("Failed to convert to GIF with palette".to_string());
        }
    }

    // Clean up temp files
    let _ = std::fs::remove_file(&temp_mp4);
    let _ = std::fs::remove_file(&palette_path);

    Ok(gif_out)
}
