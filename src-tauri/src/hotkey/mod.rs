use std::collections::HashMap;
use std::ptr::null_mut;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter};
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    HOT_KEY_MODIFIERS, MOD_ALT, MOD_CONTROL, MOD_SHIFT, MOD_WIN, RegisterHotKey, UnregisterHotKey,
};
use windows::Win32::UI::WindowsAndMessaging::{GetMessageW, MSG, WM_HOTKEY};

/// Game mode state — when active, all global hotkeys are disabled
static GAME_MODE: Mutex<bool> = Mutex::new(false);

/// A single hotkey definition
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HotkeyConfig {
    pub id: String,
    pub name: String,
    pub shortcut: String,
}

/// Parse shortcut string like "Ctrl+Shift+A" into (modifiers, vk_code)
fn parse_shortcut(shortcut: &str) -> Option<(HOT_KEY_MODIFIERS, u32)> {
    let parts: Vec<&str> = shortcut.split('+').map(|s| s.trim()).collect();
    if parts.is_empty() {
        return None;
    }

    let mut modifiers = HOT_KEY_MODIFIERS(0);
    let mut vk = 0u32;

    for part in &parts {
        match part.to_lowercase().as_str() {
            "ctrl" | "control" => modifiers.0 |= MOD_CONTROL.0,
            "shift" => modifiers.0 |= MOD_SHIFT.0,
            "alt" => modifiers.0 |= MOD_ALT.0,
            "win" | "super" | "meta" => modifiers.0 |= MOD_WIN.0,
            // Letters
            s if s.len() == 1 && s.chars().next().unwrap().is_ascii_alphabetic() => {
                vk = s.chars().next().unwrap().to_ascii_uppercase() as u32;
            }
            // Function keys
            s if s.starts_with('f') || s.starts_with('F') => {
                if let Ok(n) = s[1..].parse::<u32>() {
                    if n >= 1 && n <= 24 {
                        vk = 0x70 + n - 1; // VK_F1 = 0x70
                    }
                }
            }
            // Number keys
            s if s.len() == 1 && s.chars().next().unwrap().is_ascii_digit() => {
                vk = s.chars().next().unwrap() as u32;
            }
            // Special keys
            "space" => vk = 0x20,
            "tab" => vk = 0x09,
            "enter" | "return" => vk = 0x0D,
            "escape" | "esc" => vk = 0x1B,
            "backspace" => vk = 0x08,
            "delete" | "del" => vk = 0x2E,
            "insert" | "ins" => vk = 0x2D,
            "home" => vk = 0x24,
            "end" => vk = 0x23,
            "pageup" | "pgup" => vk = 0x21,
            "pagedown" | "pgdn" => vk = 0x22,
            "left" => vk = 0x25,
            "up" => vk = 0x26,
            "right" => vk = 0x27,
            "down" => vk = 0x28,
            "printscreen" | "prtsc" => vk = 0x2C,
            "scrolllock" => vk = 0x91,
            "pause" => vk = 0x13,
            // Numpad
            "num0" => vk = 0x60,
            "num1" => vk = 0x61,
            "num2" => vk = 0x62,
            "num3" => vk = 0x63,
            "num4" => vk = 0x64,
            "num5" => vk = 0x65,
            "num6" => vk = 0x66,
            "num7" => vk = 0x67,
            "num8" => vk = 0x68,
            "num9" => vk = 0x69,
            _ => {
                log::warn!("[Hotkey] Unknown key part: {}", part);
                return None;
            }
        }
    }

    if vk == 0 {
        return None;
    }

    Some((modifiers, vk))
}

/// Default hotkeys
fn default_hotkeys() -> Vec<HotkeyConfig> {
    vec![
        HotkeyConfig {
            id: "capture_region".into(),
            name: "区域截图".into(),
            shortcut: "Ctrl+Shift+A".into(),
        },
        HotkeyConfig {
            id: "capture_fullscreen".into(),
            name: "全屏截图".into(),
            shortcut: "Ctrl+Shift+S".into(),
        },
        HotkeyConfig {
            id: "capture_window".into(),
            name: "窗口截图".into(),
            shortcut: "Ctrl+Shift+Q".into(),
        },
    ]
}

/// Global hotkey state
static HOTKEY_STATE: Mutex<Option<HotkeyState>> = Mutex::new(None);

struct HotkeyState {
    /// Maps hotkey atom ID (1-based) to HotkeyConfig
    hotkeys: HashMap<i32, HotkeyConfig>,
    /// Next available atom ID
    next_id: i32,
}

impl HotkeyState {
    fn new() -> Self {
        Self {
            hotkeys: HashMap::new(),
            next_id: 1,
        }
    }
}

/// Load hotkey configs from app data, falling back to defaults
fn load_hotkey_configs(app: &AppHandle) -> Vec<HotkeyConfig> {
    use tauri::Manager;

    let config_path = app
        .path()
        .app_config_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
        .join("hotkeys.json");

    if config_path.exists() {
        if let Ok(data) = std::fs::read_to_string(&config_path) {
            if let Ok(configs) = serde_json::from_str::<Vec<HotkeyConfig>>(&data) {
                log::info!("[Hotkey] Loaded {} hotkey configs from {:?}", configs.len(), config_path);
                return configs;
            }
        }
    }

    log::info!("[Hotkey] Using default hotkey configs");
    default_hotkeys()
}

/// Save hotkey configs to app data
fn save_hotkey_configs(app: &AppHandle, configs: &[HotkeyConfig]) -> anyhow::Result<()> {
    use tauri::Manager;

    let config_dir = app
        .path()
        .app_config_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."));

    std::fs::create_dir_all(&config_dir)?;
    let config_path = config_dir.join("hotkeys.json");
    let data = serde_json::to_string_pretty(configs)?;
    std::fs::write(&config_path, data)?;
    log::info!("[Hotkey] Saved {} hotkey configs to {:?}", configs.len(), config_path);
    Ok(())
}

/// Register all hotkeys and start the message loop thread
pub fn register_hotkeys(app_handle: &AppHandle) -> anyhow::Result<()> {
    let configs = load_hotkey_configs(app_handle);

    let mut state = HOTKEY_STATE.lock().unwrap();
    *state = Some(HotkeyState::new());
    let state_ref = state.as_mut().unwrap();

    let hwnd = HWND(null_mut());

    for config in &configs {
        if let Some((modifiers, vk)) = parse_shortcut(&config.shortcut) {
            let id = state_ref.next_id;
            state_ref.next_id += 1;

            match unsafe { RegisterHotKey(hwnd, id, modifiers, vk) } {
                Ok(()) => {
                    log::info!("[Hotkey] Registered: {} = {} (id={})", config.id, config.shortcut, id);
                    state_ref.hotkeys.insert(id, config.clone());
                }
                Err(e) => {
                    log::warn!(
                        "[Hotkey] Failed to register {} = {}: {}",
                        config.id,
                        config.shortcut,
                        e
                    );
                }
            }
        } else {
            log::warn!("[Hotkey] Cannot parse shortcut: {} = {}", config.id, config.shortcut);
        }
    }

    let hotkey_map: HashMap<i32, String> = state_ref
        .hotkeys
        .iter()
        .map(|(id, config)| (*id, config.id.clone()))
        .collect();

    // Start message loop thread
    let app = app_handle.clone();
    std::thread::spawn(move || {
        let mut msg = MSG::default();
        unsafe {
            while GetMessageW(&mut msg, HWND(null_mut()), 0, 0).0 > 0 {
                if msg.message == WM_HOTKEY {
                    let id = msg.wParam.0 as i32;
                    if let Some(name) = hotkey_map.get(&id) {
                        log::info!("[Hotkey] Triggered: {}", name);
                        let _ = app.emit("hotkey", name.as_str());
                    }
                }
            }
        }
    });

    Ok(())
}

/// Unregister all hotkeys
pub fn unregister_hotkeys() {
    let mut state = HOTKEY_STATE.lock().unwrap();
    if let Some(state_ref) = state.as_mut() {
        let hwnd = HWND(null_mut());
        for id in state_ref.hotkeys.keys() {
            unsafe {
                let _ = UnregisterHotKey(hwnd, *id);
            }
        }
        state_ref.hotkeys.clear();
        log::info!("[Hotkey] All hotkeys unregistered");
    }
}

/// Get all hotkey configs
pub fn get_hotkeys(app: AppHandle) -> Result<Vec<serde_json::Value>, String> {
    let configs = load_hotkey_configs(&app);
    Ok(configs
        .iter()
        .map(|c| {
            serde_json::json!({
                "id": c.id,
                "name": c.name,
                "shortcut": c.shortcut,
            })
        })
        .collect())
}

/// Update a hotkey shortcut (unregister old, register new)
pub fn set_hotkey(app: AppHandle, id: String, shortcut: String) -> Result<(), String> {
    // Parse the new shortcut first
    let (modifiers, vk) = parse_shortcut(&shortcut)
        .ok_or_else(|| format!("Cannot parse shortcut: {}", shortcut))?;

    let mut state = HOTKEY_STATE.lock().unwrap();
    let state_ref = state
        .as_mut()
        .ok_or_else(|| "Hotkey state not initialized".to_string())?;

    let hwnd = HWND(null_mut());

    // Find and unregister old hotkey with same id
    let old_id = state_ref
        .hotkeys
        .iter()
        .find(|(_, config)| config.id == id)
        .map(|(old_id, _)| *old_id);

    if let Some(old_id) = old_id {
        unsafe {
            let _ = UnregisterHotKey(hwnd, old_id);
        }
        state_ref.hotkeys.remove(&old_id);
    }

    // Register new hotkey
    let new_id = state_ref.next_id;
    state_ref.next_id += 1;

    unsafe {
        RegisterHotKey(hwnd, new_id, modifiers, vk)
            .map_err(|e| format!("Failed to register hotkey: {}", e))?;
    }

    let name = old_id
        .and_then(|oid| {
            // We already removed it, get name from default hotkeys
            default_hotkeys()
                .iter()
                .find(|c| c.id == id)
                .map(|c| c.name.clone())
        })
        .unwrap_or_else(|| id.clone());

    state_ref.hotkeys.insert(
        new_id,
        HotkeyConfig {
            id: id.clone(),
            name: name.clone(),
            shortcut: shortcut.clone(),
        },
    );

    log::info!("[Hotkey] Updated: {} = {} (old_id={:?}, new_id={})", id, shortcut, old_id, new_id);

    // Save to config
    let configs: Vec<HotkeyConfig> = state_ref.hotkeys.values().cloned().collect();
    drop(state); // Release lock before I/O
    save_hotkey_configs(&app, &configs)
        .map_err(|e| format!("Failed to save hotkey config: {}", e))?;

    Ok(())
}

/// Toggle game mode — when enabled, all global hotkeys are unregistered
/// to avoid conflicts with game keybindings
pub fn toggle_game_mode(app: AppHandle, enabled: bool) -> Result<bool, String> {
    let mut game_mode = GAME_MODE.lock().map_err(|e| e.to_string())?;
    let was_enabled = *game_mode;
    *game_mode = enabled;

    if enabled && !was_enabled {
        // Entering game mode — unregister all hotkeys
        unregister_hotkeys();
        log::info!("[Hotkey] Game mode ON — all hotkeys disabled");
    } else if !enabled && was_enabled {
        // Leaving game mode — re-register hotkeys
        register_hotkeys(&app).map_err(|e| e.to_string())?;
        log::info!("[Hotkey] Game mode OFF — hotkeys restored");
    }

    Ok(*game_mode)
}

/// Get current game mode state
pub fn get_game_mode() -> Result<bool, String> {
    GAME_MODE.lock().map_err(|e| e.to_string()).map(|g| *g)
}
