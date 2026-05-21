use std::thread;
use std::time::Duration;
use arboard::Clipboard;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, VK_CONTROL, VK_V,
};

pub fn inject_text(text: &str) -> Result<(), String> {
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
