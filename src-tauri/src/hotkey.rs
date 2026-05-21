use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::thread;
use windows_sys::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{VK_LCONTROL, VK_RCONTROL};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, GetMessageW, PostThreadMessageW, SetWindowsHookExW, UnhookWindowsHookEx,
    HHOOK, KBDLLHOOKSTRUCT, MSG, WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP, WM_QUIT,
    WM_SYSKEYDOWN, WM_SYSKEYUP,
};

static mut HOOK_HANDLE: HHOOK = 0 as HHOOK;
static mut HOOK_THREAD_ID: u32 = 0;
static mut SENDER: Option<mpsc::Sender<HotkeyEvent>> = None;
static KEY_PRESSED: AtomicBool = AtomicBool::new(false);
static CANCELLED: AtomicBool = AtomicBool::new(false);

#[derive(Debug)]
pub enum HotkeyEvent {
    Pressed,
    Released,
    Cancelled,
}

pub struct HotkeyListener {
    _thread: thread::JoinHandle<()>,
}

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

impl Drop for HotkeyListener {
    fn drop(&mut self) {
        unsafe {
            if HOOK_THREAD_ID != 0 {
                PostThreadMessageW(HOOK_THREAD_ID, WM_QUIT, 0, 0);
            }
        }
    }
}

unsafe extern "system" fn low_level_keyboard_proc(
    n_code: i32,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    if n_code >= 0 {
        let kbd_struct = *(l_param as *const KBDLLHOOKSTRUCT);

        let vk = kbd_struct.vkCode;
        let is_ctrl = vk == VK_LCONTROL as u32 || vk == VK_RCONTROL as u32;

        if is_ctrl {
            let is_down = w_param == WM_KEYDOWN as usize || w_param == WM_SYSKEYDOWN as usize;
            let is_up = w_param == WM_KEYUP as usize || w_param == WM_SYSKEYUP as usize;

            if is_down {
                let already_pressed = KEY_PRESSED.swap(true, Ordering::SeqCst);
                if !already_pressed {
                    CANCELLED.store(false, Ordering::SeqCst);
                    if let Some(ref tx) = SENDER {
                        let _ = tx.send(HotkeyEvent::Pressed);
                    }
                }
            } else if is_up {
                let was_cancelled = CANCELLED.swap(false, Ordering::SeqCst);
                KEY_PRESSED.store(false, Ordering::SeqCst);
                if !was_cancelled {
                    if let Some(ref tx) = SENDER {
                        let _ = tx.send(HotkeyEvent::Released);
                    }
                }
            }
        } else {
            let is_down = w_param == WM_KEYDOWN as usize || w_param == WM_SYSKEYDOWN as usize;
            if is_down && KEY_PRESSED.load(Ordering::SeqCst) {
                let already_cancelled = CANCELLED.swap(true, Ordering::SeqCst);
                if !already_cancelled {
                    if let Some(ref tx) = SENDER {
                        let _ = tx.send(HotkeyEvent::Cancelled);
                    }
                }
            }
        }
    }

    CallNextHookEx(HOOK_HANDLE, n_code, w_param, l_param)
}
