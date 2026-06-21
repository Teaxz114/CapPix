use std::ptr::null_mut;
use tauri::{AppHandle, Emitter};
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    HOT_KEY_MODIFIERS, MOD_CONTROL, MOD_SHIFT, RegisterHotKey, UnregisterHotKey,
};
use windows::Win32::UI::WindowsAndMessaging::{GetMessageW, MSG, WM_HOTKEY};

const HOTKEY_CAPTURE_REGION: i32 = 1;
const HOTKEY_CAPTURE_FULLSCREEN: i32 = 2;
const HOTKEY_CAPTURE_WINDOW: i32 = 3;

struct HotkeyDef {
    id: i32,
    modifiers: HOT_KEY_MODIFIERS,
    vk: u32,
    name: &'static str,
}

const DEFAULT_HOTKEYS: [HotkeyDef; 3] = [
    HotkeyDef {
        id: HOTKEY_CAPTURE_REGION,
        modifiers: HOT_KEY_MODIFIERS(MOD_CONTROL.0 | MOD_SHIFT.0),
        vk: 0x41,
        name: "capture_region",
    },
    HotkeyDef {
        id: HOTKEY_CAPTURE_FULLSCREEN,
        modifiers: HOT_KEY_MODIFIERS(MOD_CONTROL.0 | MOD_SHIFT.0),
        vk: 0x53,
        name: "capture_fullscreen",
    },
    HotkeyDef {
        id: HOTKEY_CAPTURE_WINDOW,
        modifiers: HOT_KEY_MODIFIERS(MOD_CONTROL.0 | MOD_SHIFT.0),
        vk: 0x51,
        name: "capture_window",
    },
];

pub fn register_hotkeys(app_handle: &AppHandle) -> anyhow::Result<()> {
    let hwnd = HWND(null_mut());

    for hk in &DEFAULT_HOTKEYS {
        unsafe {
            RegisterHotKey(hwnd, hk.id, hk.modifiers, hk.vk)?;
        }
    }

    let app = app_handle.clone();
    std::thread::spawn(move || {
        let mut msg = MSG::default();
        unsafe {
            while GetMessageW(&mut msg, HWND(null_mut()), 0, 0).0 > 0 {
                if msg.message == WM_HOTKEY {
                    let id = msg.wParam.0 as i32;
                    if let Some(hk) = DEFAULT_HOTKEYS.iter().find(|h| h.id == id) {
                        let _ = app.emit("hotkey", hk.name);
                    }
                }
            }
        }
    });

    Ok(())
}

#[allow(dead_code)]
pub fn unregister_hotkeys() {
    let hwnd = HWND(null_mut());
    for hk in &DEFAULT_HOTKEYS {
        unsafe {
            let _ = UnregisterHotKey(hwnd, hk.id);
        }
    }
}
