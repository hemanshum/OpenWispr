#[cfg(target_os = "windows")]
use std::thread;
#[cfg(target_os = "windows")]
use std::time::Duration;
#[cfg(target_os = "windows")]
use std::sync::atomic::{AtomicIsize, Ordering};
use arboard::Clipboard;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, KEYEVENTF_UNICODE, VK_CONTROL, VK_V,
};

#[cfg(target_os = "windows")]
static LAST_ACTIVE_HWND: AtomicIsize = AtomicIsize::new(0);

#[cfg(target_os = "windows")]
use std::sync::atomic::AtomicBool;

#[cfg(target_os = "windows")]
static WAS_FOCUSED_ON_MURMUR: AtomicBool = AtomicBool::new(false);

#[cfg(target_os = "windows")]
pub fn set_was_focused_on_murmur(val: bool) {
    WAS_FOCUSED_ON_MURMUR.store(val, Ordering::SeqCst);
}

#[cfg(target_os = "windows")]
pub fn start_focus_tracker() {
    thread::spawn(|| {
        unsafe {
            let current_pid = windows_sys::Win32::System::Threading::GetCurrentProcessId();
            loop {
                let hwnd = windows_sys::Win32::UI::WindowsAndMessaging::GetForegroundWindow();
                if hwnd != 0 {
                    let mut pid = 0;
                    windows_sys::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId(hwnd, &mut pid);
                    if pid != current_pid {
                        LAST_ACTIVE_HWND.store(hwnd, Ordering::SeqCst);
                    }
                }
                thread::sleep(Duration::from_millis(100));
            }
        }
    });
}

#[cfg(target_os = "windows")]
fn inject_via_unicode(text: &str) -> Result<(), String> {
    let utf16_chars: Vec<u16> = text.encode_utf16().collect();
    if utf16_chars.is_empty() {
        return Ok(());
    }

    let mut inputs: Vec<INPUT> = Vec::with_capacity(utf16_chars.len() * 2);
    for &ch in &utf16_chars {
        // Down
        let mut down: INPUT = unsafe { std::mem::zeroed() };
        down.r#type = INPUT_KEYBOARD;
        down.Anonymous.ki = KEYBDINPUT {
            wVk: 0,
            wScan: ch,
            dwFlags: KEYEVENTF_UNICODE,
            time: 0,
            dwExtraInfo: 0,
        };
        inputs.push(down);

        // Up
        let mut up: INPUT = unsafe { std::mem::zeroed() };
        up.r#type = INPUT_KEYBOARD;
        up.Anonymous.ki = KEYBDINPUT {
            wVk: 0,
            wScan: ch,
            dwFlags: KEYEVENTF_UNICODE | KEYEVENTF_KEYUP,
            time: 0,
            dwExtraInfo: 0,
        };
        inputs.push(up);
    }

    unsafe {
        let sent = SendInput(
            inputs.len() as u32,
            inputs.as_ptr(),
            std::mem::size_of::<INPUT>() as i32,
        );
        if sent == 0 {
            let err = windows_sys::Win32::Foundation::GetLastError();
            return Err(format!("SendInput returned 0. Last error: {}", err));
        }
        if sent != inputs.len() as u32 {
            return Err(format!("Failed to send all Unicode inputs. Sent {}/{}", sent, inputs.len()));
        }
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn inject_via_clipboard(text: &str) -> Result<(), String> {
    let mut clipboard = Clipboard::new().map_err(|e| format!("Failed to open clipboard: {}", e))?;

    // Save current clipboard content if it is text
    let original_text = clipboard.get_text().ok();

    // Set transcription text to clipboard
    clipboard
        .set_text(text.to_string())
        .map_err(|e| format!("Failed to set clipboard text: {}", e))?;

    // Simulate pressing Ctrl+V
    unsafe {
        let mut inputs: [INPUT; 4] = std::mem::zeroed();

        // 1. Ctrl Down
        inputs[0].r#type = INPUT_KEYBOARD;
        inputs[0].Anonymous.ki = KEYBDINPUT {
            wVk: VK_CONTROL,
            wScan: 0,
            dwFlags: 0,
            time: 0,
            dwExtraInfo: 0,
        };

        // 2. V Down
        inputs[1].r#type = INPUT_KEYBOARD;
        inputs[1].Anonymous.ki = KEYBDINPUT {
            wVk: VK_V,
            wScan: 0,
            dwFlags: 0,
            time: 0,
            dwExtraInfo: 0,
        };

        // 3. V Up
        inputs[2].r#type = INPUT_KEYBOARD;
        inputs[2].Anonymous.ki = KEYBDINPUT {
            wVk: VK_V,
            wScan: 0,
            dwFlags: KEYEVENTF_KEYUP,
            time: 0,
            dwExtraInfo: 0,
        };

        // 4. Ctrl Up
        inputs[3].r#type = INPUT_KEYBOARD;
        inputs[3].Anonymous.ki = KEYBDINPUT {
            wVk: VK_CONTROL,
            wScan: 0,
            dwFlags: KEYEVENTF_KEYUP,
            time: 0,
            dwExtraInfo: 0,
        };

        let sent = SendInput(4, inputs.as_ptr(), std::mem::size_of::<INPUT>() as i32);
        if sent != 4 {
            return Err("Failed to send Ctrl+V keyboard inputs".to_string());
        }
    }

    // Restore original clipboard content after a short delay
    if let Some(orig) = original_text {
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(150));
            if let Ok(mut cb) = Clipboard::new() {
                let _ = cb.set_text(orig);
            }
        });
    }

    Ok(())
}

#[cfg(target_os = "windows")]
pub fn inject_text(text: &str) -> Result<(), String> {
    // Restore focus to the last active window before injecting if we weren't focused on Murmur
    if !WAS_FOCUSED_ON_MURMUR.load(Ordering::SeqCst) {
        let target_hwnd = LAST_ACTIVE_HWND.load(Ordering::SeqCst);
        if target_hwnd != 0 {
            unsafe {
                windows_sys::Win32::UI::WindowsAndMessaging::SetForegroundWindow(target_hwnd);
                // Give Windows a tiny moment to switch focus
                thread::sleep(Duration::from_millis(100));
            }
        }
    }

    // Try clipboard injection first as it is much more reliable for modern Windows apps (like Notepad)
    let res = match inject_via_clipboard(text) {
        Ok(_) => Ok(()),
        Err(e) => {
            eprintln!("Clipboard injection failed: {}. Falling back to direct Unicode injection.", e);
            inject_via_unicode(text)
        }
    };

    // Reset the flag for the next dictation session
    WAS_FOCUSED_ON_MURMUR.store(false, Ordering::SeqCst);
    res
}

#[cfg(not(target_os = "windows"))]
pub fn start_focus_tracker() {}

#[cfg(not(target_os = "windows"))]
pub fn set_was_focused_on_murmur(_val: bool) {}

#[cfg(not(target_os = "windows"))]
pub fn inject_text(text: &str) -> Result<(), String> {
    let mut clipboard = Clipboard::new().map_err(|e| format!("Failed to open clipboard: {}", e))?;
    clipboard
        .set_text(text.to_string())
        .map_err(|e| format!("Failed to set clipboard text: {}", e))?;
    Ok(())
}
