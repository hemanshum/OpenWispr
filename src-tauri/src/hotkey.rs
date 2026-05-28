#[cfg(target_os = "windows")]
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(target_os = "windows")]
use std::sync::{mpsc, Mutex};
#[cfg(target_os = "windows")]
use std::thread;
#[cfg(target_os = "windows")]
use once_cell::sync::Lazy;
#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, GetMessageW, PostThreadMessageW, SetWindowsHookExW, UnhookWindowsHookEx,
    HHOOK, KBDLLHOOKSTRUCT, MSG, WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP, WM_QUIT,
    WM_SYSKEYDOWN, WM_SYSKEYUP,
};

#[cfg(target_os = "windows")]
static mut HOOK_HANDLE: HHOOK = 0 as HHOOK;
#[cfg(target_os = "windows")]
static mut HOOK_THREAD_ID: u32 = 0;
#[cfg(target_os = "windows")]
static mut SENDER: Option<std::sync::mpsc::Sender<HotkeyEvent>> = None;

// Active trigger key cache updated on saves/loads (avoids slow disk access in hook thread)
#[cfg(target_os = "windows")]
#[cfg(target_os = "windows")]
static TRANSCRIBE_KEY: Lazy<Mutex<String>> = Lazy::new(|| Mutex::new("Control".to_string()));
#[cfg(target_os = "windows")]
static NOTES_KEY: Lazy<Mutex<String>> = Lazy::new(|| Mutex::new("Control + Win".to_string()));
#[cfg(target_os = "windows")]
static CANCEL_KEY: Lazy<Mutex<String>> = Lazy::new(|| Mutex::new("Escape".to_string()));

// Modifier key tracking states
#[cfg(target_os = "windows")]
static CTRL_HELD: AtomicBool = AtomicBool::new(false);
#[cfg(target_os = "windows")]
static SHIFT_HELD: AtomicBool = AtomicBool::new(false);
#[cfg(target_os = "windows")]
static ALT_HELD: AtomicBool = AtomicBool::new(false);
#[cfg(target_os = "windows")]
static WIN_HELD: AtomicBool = AtomicBool::new(false);

#[cfg(target_os = "windows")]
static ACTIVE_RECORDING: Lazy<Mutex<Option<RecordingType>>> = Lazy::new(|| Mutex::new(None));

#[cfg(target_os = "windows")]
pub fn update_hotkeys(transcribe: &str, notes: &str, cancel: &str) {
    if let Ok(mut t) = TRANSCRIBE_KEY.lock() {
        *t = transcribe.to_string();
    }
    if let Ok(mut n) = NOTES_KEY.lock() {
        *n = notes.to_string();
    }
    if let Ok(mut c) = CANCEL_KEY.lock() {
        *c = cancel.to_string();
    }
}

#[cfg(target_os = "windows")]
fn get_cancel_vk(key_name: &str) -> u32 {
    let lower = key_name.to_lowercase();
    match lower.as_str() {
        "escape" | "esc" => 0x1B,
        "backquote" | "`" | "~" => 0xC0,
        "f1" => 0x70,
        "f2" => 0x71,
        "f3" => 0x72,
        "f4" => 0x73,
        "f5" => 0x74,
        "f6" => 0x75,
        "f7" => 0x76,
        "f8" => 0x77,
        "f9" => 0x78,
        "f10" => 0x79,
        "f11" => 0x7A,
        "f12" => 0x7B,
        "scrolllock" | "scroll lock" => 0x91,
        "space" => 0x20,
        "tab" => 0x09,
        "backspace" => 0x08,
        "enter" | "return" => 0x0D,
        "delete" | "del" => 0x2E,
        "insert" | "ins" => 0x2D,
        "home" => 0x24,
        "end" => 0x23,
        "pageup" | "page up" => 0x21,
        "pagedown" | "page down" => 0x22,
        _ => {
            // Check if it's a single letter or digit
            if key_name.len() == 1 {
                let c = key_name.chars().next().unwrap().to_ascii_uppercase();
                if c >= 'A' && c <= 'Z' {
                    return c as u32;
                }
                if c >= '0' && c <= '9' {
                    return c as u32;
                }
            }
            0x1B // Default fallback to Escape
        }
    }
}

#[cfg(not(target_os = "windows"))]
pub fn update_hotkeys(_transcribe: &str, _notes: &str, _cancel: &str) {}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RecordingType {
    Transcribe,
    Notes,
}

#[derive(Debug)]
pub enum HotkeyEvent {
    Pressed(RecordingType),
    Released(RecordingType),
    Cancelled(RecordingType),
    GlobalCancel,
}

#[cfg(target_os = "windows")]
pub struct HotkeyListener {
    _thread: thread::JoinHandle<()>,
}

#[cfg(target_os = "windows")]
impl HotkeyListener {
    pub fn start<F>(event_handler: F) -> Self
    where
        F: Fn(HotkeyEvent) + Send + 'static,
    {
        let (tx, rx) = mpsc::channel();
        unsafe {
            SENDER = Some(tx);
        }

        // Spawn listener handler
        thread::spawn(move || {
            for event in rx {
                event_handler(event);
            }
        });

        // Spawn hook thread running Windows message loop
        let thread = thread::spawn(move || {
            unsafe {
                HOOK_THREAD_ID = windows_sys::Win32::System::Threading::GetCurrentThreadId();
                let h_instance = windows_sys::Win32::System::LibraryLoader::GetModuleHandleW(std::ptr::null());
                HOOK_HANDLE = SetWindowsHookExW(
                    WH_KEYBOARD_LL,
                    Some(low_level_keyboard_proc),
                    h_instance,
                    0,
                );

                if HOOK_HANDLE == 0 {
                    eprintln!("Failed to register low-level keyboard hook!");
                    return;
                }

                let mut msg: MSG = std::mem::zeroed();
                while GetMessageW(&mut msg, 0, 0, 0) > 0 {
                    // Process messages
                }

                UnhookWindowsHookEx(HOOK_HANDLE);
                HOOK_HANDLE = 0;
                HOOK_THREAD_ID = 0;
            }
        });

        Self { _thread: thread }
    }
}

#[cfg(target_os = "windows")]
impl Drop for HotkeyListener {
    fn drop(&mut self) {
        unsafe {
            if HOOK_THREAD_ID != 0 {
                PostThreadMessageW(HOOK_THREAD_ID, WM_QUIT, 0, 0);
            }
        }
    }
}


#[cfg(target_os = "windows")]
fn parse_hotkey(target: &str) -> (bool, u32, bool, bool, bool, bool) {
    let parts: Vec<&str> = target.split(" + ").collect();
    let mut ctrl = false;
    let mut win = false;
    let mut shift = false;
    let mut alt = false;
    
    let mut has_base_key = false;
    let mut base_vk = 0;
    
    if let Some(&last_part) = parts.last() {
        let last_lower = last_part.to_lowercase();
        if ["control", "ctrl", "win", "meta", "shift", "alt"].contains(&last_lower.as_str()) {
            for part in &parts {
                match part.to_lowercase().as_str() {
                    "control" | "ctrl" => ctrl = true,
                    "win" | "meta" => win = true,
                    "shift" => shift = true,
                    "alt" => alt = true,
                    _ => {}
                }
            }
        } else {
            has_base_key = true;
            base_vk = get_cancel_vk(last_part);
            
            for &part in parts.iter().take(parts.len() - 1) {
                match part.to_lowercase().as_str() {
                    "control" | "ctrl" => ctrl = true,
                    "win" | "meta" => win = true,
                    "shift" => shift = true,
                    "alt" => alt = true,
                    _ => {}
                }
            }
        }
    }
    (has_base_key, base_vk, ctrl, win, shift, alt)
}

#[cfg(target_os = "windows")]
fn match_hotkey_state(target: &str, pressed_vk: u32, is_modifier: bool) -> bool {
    let (has_base, base_vk, req_ctrl, req_win, req_shift, req_alt) = parse_hotkey(target);
    
    let actual_ctrl = CTRL_HELD.load(Ordering::SeqCst);
    let actual_win = WIN_HELD.load(Ordering::SeqCst);
    let actual_shift = SHIFT_HELD.load(Ordering::SeqCst);
    let actual_alt = ALT_HELD.load(Ordering::SeqCst);
    
    let mods_match = actual_ctrl == req_ctrl &&
                     actual_win == req_win &&
                     actual_shift == req_shift &&
                     actual_alt == req_alt;
                     
    if has_base {
        !is_modifier && pressed_vk == base_vk && mods_match
    } else {
        is_modifier && mods_match
    }
}

#[cfg(target_os = "windows")]
fn is_hotkey_held(target: &str, vk: u32, is_modifier: bool, is_up: bool) -> bool {
    let (has_base, base_vk, req_ctrl, req_win, req_shift, req_alt) = parse_hotkey(target);
    
    if is_up {
        if has_base && !is_modifier && vk == base_vk {
            return false;
        }
        
        let ctrl_released = vk == 0xA2 || vk == 0xA3;
        let win_released = vk == 0x5B || vk == 0x5C;
        let shift_released = vk == 0xA0 || vk == 0xA1;
        let alt_released = vk == 0xA4 || vk == 0xA5;
        
        if (req_ctrl && ctrl_released) ||
           (req_win && win_released) ||
           (req_shift && shift_released) ||
           (req_alt && alt_released) {
            return false;
        }
    }
    
    let actual_ctrl = CTRL_HELD.load(Ordering::SeqCst);
    let actual_win = WIN_HELD.load(Ordering::SeqCst);
    let actual_shift = SHIFT_HELD.load(Ordering::SeqCst);
    let actual_alt = ALT_HELD.load(Ordering::SeqCst);
    
    actual_ctrl == req_ctrl &&
    actual_win == req_win &&
    actual_shift == req_shift &&
    actual_alt == req_alt
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn low_level_keyboard_proc(
    n_code: i32,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    if n_code >= 0 {
        let kbd_struct = *(l_param as *const KBDLLHOOKSTRUCT);

        // Ignore synthetic/injected keystroke events
        let is_injected = (kbd_struct.flags & 0x10) != 0 || (kbd_struct.flags & 0x02) != 0;
        if is_injected {
            return CallNextHookEx(HOOK_HANDLE, n_code, w_param, l_param);
        }

        let vk = kbd_struct.vkCode;
        
        let is_ctrl = vk == 0xA2 || vk == 0xA3; // VK_LCONTROL, VK_RCONTROL
        let is_shift = vk == 0xA0 || vk == 0xA1; // VK_LSHIFT, VK_RSHIFT
        let is_alt = vk == 0xA4 || vk == 0xA5; // VK_LMENU, VK_RMENU
        let is_win = vk == 0x5B || vk == 0x5C; // VK_LWIN, VK_RWIN
        let is_modifier = is_ctrl || is_shift || is_alt || is_win;

        let is_down = w_param == WM_KEYDOWN as usize || w_param == WM_SYSKEYDOWN as usize;
        let is_up = w_param == WM_KEYUP as usize || w_param == WM_SYSKEYUP as usize;

        if is_modifier {
            if is_down {
                if is_ctrl { CTRL_HELD.store(true, Ordering::SeqCst); }
                if is_shift { SHIFT_HELD.store(true, Ordering::SeqCst); }
                if is_alt { ALT_HELD.store(true, Ordering::SeqCst); }
                if is_win { WIN_HELD.store(true, Ordering::SeqCst); }
            } else if is_up {
                if is_ctrl { CTRL_HELD.store(false, Ordering::SeqCst); }
                if is_shift { SHIFT_HELD.store(false, Ordering::SeqCst); }
                if is_alt { ALT_HELD.store(false, Ordering::SeqCst); }
                if is_win { WIN_HELD.store(false, Ordering::SeqCst); }
            }
        }

        let target_transcribe = if let Ok(t) = TRANSCRIBE_KEY.lock() { t.clone() } else { "Control".to_string() };
        let target_notes = if let Ok(n) = NOTES_KEY.lock() { n.clone() } else { "Control + Win".to_string() };
        let target_cancel = if let Ok(c) = CANCEL_KEY.lock() { c.clone() } else { "Escape".to_string() };

        let matches_cancel = match_hotkey_state(&target_cancel, vk, is_modifier) && is_down;

        if matches_cancel {
            if let Some(ref tx) = SENDER {
                let _ = tx.send(HotkeyEvent::GlobalCancel);
            }
            let mut active = ACTIVE_RECORDING.lock().unwrap();
            if let Some(rec_type) = *active {
                if let Some(ref tx) = SENDER {
                    let _ = tx.send(HotkeyEvent::Cancelled(rec_type));
                }
                *active = None;
            }
            return CallNextHookEx(HOOK_HANDLE, n_code, w_param, l_param);
        }

        let matches_notes = match_hotkey_state(&target_notes, vk, is_modifier) && is_down;
        let matches_transcribe = match_hotkey_state(&target_transcribe, vk, is_modifier) && is_down;

        let mut active = ACTIVE_RECORDING.lock().unwrap();

        if matches_notes {
            if *active == None {
                *active = Some(RecordingType::Notes);
                if let Some(ref tx) = SENDER {
                    let _ = tx.send(HotkeyEvent::Pressed(RecordingType::Notes));
                }
            } else if *active == Some(RecordingType::Transcribe) {
                if let Some(ref tx) = SENDER {
                    let _ = tx.send(HotkeyEvent::Cancelled(RecordingType::Transcribe));
                    let _ = tx.send(HotkeyEvent::Pressed(RecordingType::Notes));
                }
                *active = Some(RecordingType::Notes);
            }
        } else if matches_transcribe {
            if *active == None {
                *active = Some(RecordingType::Transcribe);
                if let Some(ref tx) = SENDER {
                    let _ = tx.send(HotkeyEvent::Pressed(RecordingType::Transcribe));
                }
            } else if *active == Some(RecordingType::Notes) {
                if let Some(ref tx) = SENDER {
                    let _ = tx.send(HotkeyEvent::Cancelled(RecordingType::Notes));
                    let _ = tx.send(HotkeyEvent::Pressed(RecordingType::Transcribe));
                }
                *active = Some(RecordingType::Transcribe);
            }
        } else {
            if let Some(rec_type) = *active {
                let target = match rec_type {
                    RecordingType::Transcribe => &target_transcribe,
                    RecordingType::Notes => &target_notes,
                };
                if !is_hotkey_held(target, vk, is_modifier, is_up) {
                    if let Some(ref tx) = SENDER {
                        let _ = tx.send(HotkeyEvent::Released(rec_type));
                    }
                    *active = None;
                }
            }
        }
    }

    CallNextHookEx(HOOK_HANDLE, n_code, w_param, l_param)
}

#[cfg(not(target_os = "windows"))]
pub struct HotkeyListener;

#[cfg(not(target_os = "windows"))]
impl HotkeyListener {
    pub fn start<F>(_event_handler: F) -> Self
    where
        F: Fn(HotkeyEvent) + Send + 'static,
    {
        Self
    }
}
