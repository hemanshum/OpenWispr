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
static TRANSCRIBE_KEY: Lazy<Mutex<String>> = Lazy::new(|| Mutex::new("Control".to_string()));
#[cfg(target_os = "windows")]
static NOTES_KEY: Lazy<Mutex<String>> = Lazy::new(|| Mutex::new("Control + Win".to_string()));

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
pub fn update_hotkeys(transcribe: &str, notes: &str) {
    if let Ok(mut t) = TRANSCRIBE_KEY.lock() {
        *t = transcribe.to_string();
    }
    if let Ok(mut n) = NOTES_KEY.lock() {
        *n = notes.to_string();
    }
}

#[cfg(not(target_os = "windows"))]
pub fn update_hotkeys(_transcribe: &str, _notes: &str) {}

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
fn get_current_modifier_string() -> String {
    let ctrl = CTRL_HELD.load(Ordering::SeqCst);
    let win = WIN_HELD.load(Ordering::SeqCst);
    let shift = SHIFT_HELD.load(Ordering::SeqCst);
    let alt = ALT_HELD.load(Ordering::SeqCst);

    let mut parts = Vec::new();
    if ctrl { parts.push("Control"); }
    if win { parts.push("Win"); }
    if shift { parts.push("Shift"); }
    if alt { parts.push("Alt"); }

    parts.join(" + ")
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn low_level_keyboard_proc(
    n_code: i32,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    if n_code >= 0 {
        let kbd_struct = *(l_param as *const KBDLLHOOKSTRUCT);

        // Ignore synthetic/injected keystroke events (like the ones we send via SendInput)
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

            // Construct current combined held key caps
            let current_mods = get_current_modifier_string();
            let target_transcribe = if let Ok(t) = TRANSCRIBE_KEY.lock() { t.clone() } else { "Control".to_string() };
            let target_notes = if let Ok(n) = NOTES_KEY.lock() { n.clone() } else { "Control + Win".to_string() };

            let matches_notes = current_mods == target_notes;
            let matches_transcribe = current_mods == target_transcribe;

            let mut active = ACTIVE_RECORDING.lock().unwrap();

            if matches_notes {
                if *active == None {
                    *active = Some(RecordingType::Notes);
                    if let Some(ref tx) = SENDER {
                        let _ = tx.send(HotkeyEvent::Pressed(RecordingType::Notes));
                    }
                } else if *active == Some(RecordingType::Transcribe) {
                    // Transition to Note recording
                    if let Some(ref tx) = SENDER {
                        let _ = tx.send(HotkeyEvent::Released(RecordingType::Transcribe));
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
                    // Transition to Normal dictation
                    if let Some(ref tx) = SENDER {
                        let _ = tx.send(HotkeyEvent::Released(RecordingType::Notes));
                        let _ = tx.send(HotkeyEvent::Pressed(RecordingType::Transcribe));
                    }
                    *active = Some(RecordingType::Transcribe);
                }
            } else {
                // Modified keys combination no longer matches either trigger
                if let Some(rec_type) = *active {
                    if let Some(ref tx) = SENDER {
                        let _ = tx.send(HotkeyEvent::Released(rec_type));
                    }
                    *active = None;
                }
            }
        } else {
            // Typing an arbitrary regular key
            if is_down {
                let mut active = ACTIVE_RECORDING.lock().unwrap();
                if let Some(rec_type) = *active {
                    // Cancel recording
                    if let Some(ref tx) = SENDER {
                        let _ = tx.send(HotkeyEvent::Cancelled(rec_type));
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
