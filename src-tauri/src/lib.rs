mod audio;
mod hotkey;
mod injector;
mod api;
mod config;
mod history;
mod downloader;
pub mod voice_notes;

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Emitter, Manager, State};
use tauri::menu::{MenuBuilder, MenuItemBuilder};
use tauri::tray::{TrayIconBuilder, TrayIconEvent};

use crate::audio::AudioRecorder;
use crate::hotkey::{HotkeyEvent, HotkeyListener, RecordingType};
use crate::config::AppConfig;
use crate::voice_notes::{VoiceNote, create_voice_note, get_voice_notes, get_voice_note, update_voice_note, delete_voice_note, get_audio_file_url};

pub struct AppState {
    pub recorder: Mutex<AudioRecorder>,
    pub is_recording: AtomicBool,
    pub status: Mutex<String>,
    pub is_transcribe_cancelled: AtomicBool,
}

#[tauri::command]
fn get_status(state: State<'_, AppState>) -> String {
    state.status.lock().unwrap().clone()
}

#[tauri::command]
fn get_app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}


#[tauri::command]
fn load_config(app_handle: AppHandle) -> AppConfig {
    let config = AppConfig::load(&app_handle);
    crate::hotkey::update_hotkeys(&config.transcribe_key, &config.notes_key, &config.cancel_key);
    config
}

#[tauri::command]
fn save_config(config: AppConfig, app_handle: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    if let Ok(recorder) = state.recorder.lock() {
        recorder.noise_gate_enabled.store(config.noise_gate, std::sync::atomic::Ordering::SeqCst);
    }
    crate::hotkey::update_hotkeys(&config.transcribe_key, &config.notes_key, &config.cancel_key);
    config.save(&app_handle)
}

fn update_status(app_handle: &AppHandle, state: &AppState, new_status: &str) {
    if let Ok(mut status) = state.status.lock() {
        *status = new_status.to_string();
    }
    let _ = app_handle.emit("status-changed", new_status);

    if new_status == "Idle" || new_status.starts_with("Error") {
        if let Some(overlay) = app_handle.get_webview_window("overlay") {
            let _ = overlay.hide();
        }
    }
}

#[tauri::command]
fn manual_trigger_start(state: State<'_, AppState>, app_handle: AppHandle) -> Result<(), String> {
    start_recording_internal(&app_handle, &state)
}

#[tauri::command]
fn manual_trigger_stop(state: State<'_, AppState>, app_handle: AppHandle) -> Result<(), String> {
    stop_recording_internal(&app_handle, &state)
}

// Voice note recording commands
#[tauri::command]
fn start_voice_note_recording(state: State<'_, AppState>, app_handle: AppHandle) -> Result<(), String> {
    start_recording_internal(&app_handle, &state)
}

async fn stop_voice_note_recording_internal(
    state: &AppState,
    app_handle: &AppHandle,
    title: String,
) -> Result<VoiceNote, String> {
    if !state.is_recording.load(Ordering::SeqCst) {
        return Err("Not recording".to_string());
    }

    state.is_recording.store(false, Ordering::SeqCst);

    // Stop recording and save to temp file
    let temp_file_str = {
        let mut recorder = state.recorder.lock().map_err(|e| format!("Lock error: {}", e))?;
        let api_config = AppConfig::load(&app_handle);
        let use_noise_gate = api_config.noise_gate;

        let temp_dir = std::env::temp_dir();
        let temp_file_path = temp_dir.join("murmur_voice_note.wav");
        let temp_file_str = temp_file_path.to_string_lossy().to_string();

        recorder.stop_recording(&temp_file_str, use_noise_gate)?;
        temp_file_str
    }; // Lock is dropped here

    // Transcribe the audio
    let transcription = transcribe_for_note(app_handle, &temp_file_str).await?;

    // Create the voice note
    let note = create_voice_note(app_handle.clone(), title, transcription, temp_file_str)?;

    Ok(note)
}

#[tauri::command]
async fn stop_voice_note_recording(
    state: State<'_, AppState>,
    app_handle: AppHandle,
    title: String,
) -> Result<VoiceNote, String> {
    stop_voice_note_recording_internal(&state, &app_handle, title).await
}

/// Stop recording and process transcription in the background.
/// Returns the note immediately with placeholder transcription so the user can
/// continue using the default transcribing feature without waiting.
#[tauri::command]
async fn stop_voice_note_recording_bg(
    state: State<'_, AppState>,
    app_handle: AppHandle,
    title: String,
    refine_mode: String,
) -> Result<VoiceNote, String> {
    if !state.is_recording.load(Ordering::SeqCst) {
        return Err("Not recording".to_string());
    }

    state.is_recording.store(false, Ordering::SeqCst);
    update_status(&app_handle, &state, "Idle");

    // Stop recording and save to temp file
    let temp_file_str = {
        let mut recorder = state.recorder.lock().map_err(|e| format!("Lock error: {}", e))?;
        let api_config = AppConfig::load(&app_handle);
        let use_noise_gate = api_config.noise_gate;

        let temp_dir = std::env::temp_dir();
        // Use a unique filename to avoid conflicts with concurrent recordings
        let ts = chrono::Local::now().format("%Y%m%d%H%M%S").to_string();
        let temp_file_path = temp_dir.join(format!("murmur_vn_{}.wav", ts));
        let temp_file_str = temp_file_path.to_string_lossy().to_string();

        recorder.stop_recording(&temp_file_str, use_noise_gate)?;
        temp_file_str
    };

    // Create voice note immediately with placeholder text
    let note = create_voice_note(
        app_handle.clone(),
        title.clone(),
        "Transcribing...".to_string(),
        temp_file_str.clone(),
    )?;

    let note_id = note.id;
    let note_title = note.title.clone();
    let app_clone = app_handle.clone();

    // Spawn background transcription task
    tokio::spawn(async move {
        match transcribe_for_note(&app_clone, &temp_file_str).await {
            Ok(transcription) => {
                let final_text = if refine_mode == "polished" {
                    let api_config = AppConfig::load(&app_clone);
                    match refine_text_internal(&transcription, &api_config).await {
                        Ok(refined) => refined,
                        Err(e) => {
                            eprintln!("Note refinement failed, falling back to raw transcription: {}", e);
                            transcription
                        }
                    }
                } else {
                    transcription
                };

                // Update the note with the real transcription
                match update_voice_note(app_clone.clone(), note_id, note_title.clone(), final_text) {
                    Ok(updated_note) => {
                        let _ = app_clone.emit("note-transcription-complete", updated_note);
                    }
                    Err(e) => {
                        let _ = app_clone.emit("note-transcription-failed", format!("Failed to save transcription: {}", e));
                    }
                }
            }
            Err(e) => {
                // Update note with error message
                let _ = update_voice_note(app_clone.clone(), note_id, note_title, format!("Transcription failed: {}", e));
                let _ = app_clone.emit("note-transcription-failed", e);
            }
        }
        // Clean up the temp audio file
        let _ = std::fs::remove_file(&temp_file_str);
    });

    Ok(note)
}

#[tauri::command]
async fn polish_voice_note_text(
    app_handle: AppHandle,
    text: String,
) -> Result<String, String> {
    let api_config = AppConfig::load(&app_handle);
    refine_text_internal(&text, &api_config).await
}

async fn transcribe_for_note(app_handle: &AppHandle, audio_path: &str) -> Result<String, String> {
    let api_config = AppConfig::load(app_handle);
    let tx_provider = api_config.transcription_provider.to_string();
    let transcription_language = api_config.transcription_language.to_string();

    let result = match tx_provider.as_str() {
        "local_whisper" | "local_parakeet" => {
            let model_id = if tx_provider == "local_parakeet" {
                "parakeet_v3".to_string()
            } else {
                format!("whisper_{}", api_config.local_whisper_model)
            };

            let model_type = if tx_provider == "local_parakeet" {
                "parakeet"
            } else {
                "whisper"
            };

            let is_downloaded = crate::downloader::check_model_downloaded(app_handle.clone(), model_id.clone());
            if is_downloaded {
                crate::api::transcribe_local_sherpa(
                    app_handle,
                    audio_path,
                    model_type,
                    &model_id,
                    &transcription_language,
                ).await
            } else {
                if tx_provider == "local_whisper" {
                    crate::api::transcribe_local_whisper(
                        audio_path,
                        &api_config.local_whisper_model,
                        &transcription_language,
                    ).await
                } else {
                    Err("Model files are not downloaded. Please download the Nvidia Parakeet model in Settings.".to_string())
                }
            }
        }
        "openai" => {
            crate::api::transcribe_openai(
                audio_path,
                &api_config.openai_api_key,
                &api_config.openai_model,
                &transcription_language,
            ).await
        }
        "lm_studio" => {
            crate::api::transcribe_lm_studio(
                audio_path,
                &api_config.lm_studio_url,
                &transcription_language,
            ).await
        }
        _ => { // "gemini"
            let verbatim_prompt = "Transcribe the audio verbatim. Keep all original words, sounds, and filler sounds.";
            crate::api::transcribe_and_clean_gemini(
                audio_path,
                &api_config.api_key,
                verbatim_prompt,
                "gemini-2.0-flash",
                &transcription_language,
            ).await
        }
    };

    result
}

fn start_recording_internal(app_handle: &AppHandle, state: &AppState) -> Result<(), String> {
    if state.is_recording.load(Ordering::SeqCst) {
        return Ok(());
    }

    #[cfg(target_os = "windows")]
    {
        let hwnd = unsafe { windows_sys::Win32::UI::WindowsAndMessaging::GetForegroundWindow() };
        let mut pid = 0;
        if hwnd != 0 {
            unsafe {
                windows_sys::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId(hwnd, &mut pid);
            }
        }
        let current_pid = unsafe { windows_sys::Win32::System::Threading::GetCurrentProcessId() };
        crate::injector::set_was_focused_on_murmur(pid == current_pid);
    }

    let mut recorder = state.recorder.lock().map_err(|e| format!("Lock error: {}", e))?;
    
    let api_config = AppConfig::load(app_handle);
    let device_name = if api_config.audio_device == "Default" || api_config.audio_device.is_empty() {
        None
    } else {
        Some(api_config.audio_device.as_str())
    };

    recorder.start_recording(app_handle.clone(), device_name)?;
    state.is_recording.store(true, Ordering::SeqCst);

    if let Some(overlay) = app_handle.get_webview_window("overlay") {
        let overlay_clone = overlay.clone();
        tauri::async_runtime::spawn(async move {
            if let Some(monitor) = overlay_clone.primary_monitor().unwrap_or(None) {
                let size = monitor.size();
                let scale_factor = monitor.scale_factor();
                
                let overlay_width = 380.0;
                let overlay_height = 64.0;
                
                let phys_width = (overlay_width * scale_factor) as u32;
                let phys_height = (overlay_height * scale_factor) as u32;
                
                let x = (size.width - phys_width) / 2;
                let y = size.height - phys_height - (60.0 * scale_factor) as u32;
                
                let _ = overlay_clone.set_size(tauri::Size::Physical(tauri::PhysicalSize::new(phys_width, phys_height)));
                let _ = overlay_clone.set_position(tauri::Position::Physical(tauri::PhysicalPosition::new(x as i32, y as i32)));
            }
            let _ = overlay_clone.show();
        });
    }

    update_status(app_handle, state, "Recording");
    Ok(())
}

#[tauri::command]
fn get_audio_devices() -> Result<Vec<String>, String> {
    use cpal::traits::{HostTrait, DeviceTrait};
    let host = cpal::default_host();
    let devices = host.input_devices()
        .map_err(|e| format!("Failed to list input devices: {}", e))?;
    let mut names = vec!["Default".to_string()];
    for d in devices {
        if let Ok(name) = d.name() {
            if !names.contains(&name) {
                names.push(name);
            }
        }
    }
    Ok(names)
}

#[tauri::command]
fn start_mic_test(device_name: Option<String>, state: State<'_, AppState>, app_handle: AppHandle) -> Result<(), String> {
    if state.is_recording.load(Ordering::SeqCst) {
        return Err("Cannot test microphone while recording is in progress".to_string());
    }
    let mut recorder = state.recorder.lock().map_err(|e| format!("Lock error: {}", e))?;
    let dev = device_name.as_deref();
    recorder.start_mic_test(app_handle, dev)
}

#[tauri::command]
fn stop_mic_test(state: State<'_, AppState>) -> Result<(), String> {
    let mut recorder = state.recorder.lock().map_err(|e| format!("Lock error: {}", e))?;
    recorder.stop_mic_test()
}

fn stop_recording_internal(app_handle: &AppHandle, state: &AppState) -> Result<(), String> {
    if !state.is_recording.load(Ordering::SeqCst) {
        return Ok(());
    }

    state.is_recording.store(false, Ordering::SeqCst);
    state.is_transcribe_cancelled.store(false, Ordering::SeqCst);
    update_status(app_handle, state, "Transcribing");

    let mut recorder = state.recorder.lock().map_err(|e| format!("Lock error: {}", e))?;
    
    let api_config = AppConfig::load(app_handle);
    let use_noise_gate = api_config.noise_gate;

    let temp_dir = std::env::temp_dir();
    let temp_file_path = temp_dir.join("murmur_recording.wav");
    let temp_file_str = temp_file_path.to_string_lossy().to_string();

    recorder.stop_recording(&temp_file_str, use_noise_gate)?;

    let app_handle_clone = app_handle.clone();

    tauri::async_runtime::spawn(async move {
        let app_state = app_handle_clone.state::<AppState>();
        
        let tx_provider = api_config.transcription_provider.to_string();
        let openai_api_key = api_config.openai_api_key.trim().to_string();
        let api_key = api_config.api_key.trim().to_string();
        let provider = api_config.provider.to_string(); // provider maps to refinement provider
        let openrouter_api_key = api_config.openrouter_api_key.trim().to_string();
        let custom_api_url = api_config.custom_api_url.trim().to_string();

        if (tx_provider == "openai" || provider == "openai") && openai_api_key.is_empty() {
            update_status(&app_handle_clone, &app_state, "Error: Missing OpenAI Key");
            tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
            update_status(&app_handle_clone, &app_state, "Idle");
            return;
        }

        if (tx_provider == "gemini" || provider == "gemini") && api_key.is_empty() {
            update_status(&app_handle_clone, &app_state, "Error: Missing Gemini Key");
            tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
            update_status(&app_handle_clone, &app_state, "Idle");
            return;
        }

        if provider == "openrouter" && openrouter_api_key.is_empty() {
            update_status(&app_handle_clone, &app_state, "Error: Missing OpenRouter Key");
            tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
            update_status(&app_handle_clone, &app_state, "Idle");
            return;
        }

        if provider == "custom" && custom_api_url.is_empty() {
            update_status(&app_handle_clone, &app_state, "Error: Missing Custom URL");
            tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
            update_status(&app_handle_clone, &app_state, "Idle");
            return;
        }

        let lm_studio_url = api_config.lm_studio_url.trim().to_string();
        if (tx_provider == "lm_studio" || provider == "lm_studio") && lm_studio_url.is_empty() {
            update_status(&app_handle_clone, &app_state, "Error: Missing LM Studio URL");
            tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
            update_status(&app_handle_clone, &app_state, "Idle");
            return;
        }

        let prompt = api_config.prompt.to_string();
        let model = api_config.model.to_string();
        let local_whisper_model = api_config.local_whisper_model.to_string();
        let openai_model = api_config.openai_model.to_string();
        let transcription_language = api_config.transcription_language.to_string();

        let result = match tx_provider.as_str() {
            "local_whisper" | "local_parakeet" => {
                let model_id = if tx_provider == "local_parakeet" {
                    "parakeet_v3".to_string()
                } else {
                    format!("whisper_{}", local_whisper_model)
                };
                
                let model_type = if tx_provider == "local_parakeet" {
                    "parakeet"
                } else {
                    "whisper"
                };

                let is_downloaded = crate::downloader::check_model_downloaded(app_handle_clone.clone(), model_id.clone());
                if is_downloaded {
                    match crate::api::transcribe_local_sherpa(
                        &app_handle_clone,
                        &temp_file_str,
                        model_type,
                        &model_id,
                        &transcription_language,
                    ).await {
                        Ok(raw_text) => {
                            refine_text_internal(&raw_text, &api_config).await
                        }
                        Err(e) => Err(e),
                    }
                } else {
                    if tx_provider == "local_whisper" {
                        // Fall back to legacy local_whisper CLI logic if it's whisper
                        match crate::api::transcribe_local_whisper(
                            &temp_file_str,
                            &local_whisper_model,
                            &transcription_language,
                        ).await {
                            Ok(raw_text) => {
                                refine_text_internal(&raw_text, &api_config).await
                            }
                            Err(e) => Err(e),
                        }
                    } else {
                        Err("Model files are not downloaded. Please download the Nvidia Parakeet model in Settings.".to_string())
                    }
                }
            }
            "openai" => {
                match crate::api::transcribe_openai(
                    &temp_file_str,
                    &openai_api_key,
                    &openai_model,
                    &transcription_language,
                ).await {
                    Ok(raw_text) => {
                        refine_text_internal(&raw_text, &api_config).await
                    }
                    Err(e) => Err(e),
                }
            }
            "lm_studio" => {
                match crate::api::transcribe_lm_studio(
                    &temp_file_str,
                    &lm_studio_url,
                    &transcription_language,
                ).await {
                    Ok(raw_text) => {
                        refine_text_internal(&raw_text, &api_config).await
                    }
                    Err(e) => Err(e),
                }
            }
            _ => { // "gemini"
                if provider != "gemini" && provider != "none" {
                    // Refinement is Ollama, OpenAI, OpenRouter, Custom
                    // We must transcribe verbatim first
                    let verbatim_prompt = "Transcribe the audio verbatim. Keep all original words, sounds, and filler sounds.";
                    match crate::api::transcribe_and_clean_gemini(
                        &temp_file_str,
                        &api_key,
                        verbatim_prompt,
                        "gemini-2.0-flash",
                        &transcription_language,
                    ).await {
                        Ok(raw_text) => {
                            refine_text_internal(&raw_text, &api_config).await
                        }
                        Err(e) => Err(e),
                    }
                } else if provider == "gemini" {
                    // We can let Gemini do both transcription and refinement in a single call!
                    crate::api::transcribe_and_clean_gemini(
                        &temp_file_str,
                        &api_key,
                        &prompt,
                        &model,
                        &transcription_language,
                    ).await
                } else {
                    // Refinement is "none"
                    let verbatim_prompt = "Transcribe the audio verbatim. Keep all original words, sounds, and filler sounds.";
                    crate::api::transcribe_and_clean_gemini(
                        &temp_file_str,
                        &api_key,
                        verbatim_prompt,
                        "gemini-2.0-flash",
                        &transcription_language,
                    ).await
                }
            }
        };

        if app_state.is_transcribe_cancelled.load(Ordering::SeqCst) {
            update_status(&app_handle_clone, &app_state, "Idle");
            let _ = std::fs::remove_file(temp_file_str);
            return;
        }

        match result {
            Ok(transcribed_text) => {
                if transcribed_text.is_empty() {
                    update_status(&app_handle_clone, &app_state, "Idle");
                    return;
                }

                // Save to history database
                let _ = crate::history::add_history_entry(&app_handle_clone, &transcribed_text);

                let _ = app_handle_clone.emit("text-prepared", transcribed_text.clone());

                update_status(&app_handle_clone, &app_state, "Pasting");
                match crate::injector::inject_text(&transcribed_text) {
                    Ok(_) => {
                        update_status(&app_handle_clone, &app_state, "Idle");
                    }
                    Err(e) => {
                        eprintln!("Failed to inject text: {}", e);
                        update_status(&app_handle_clone, &app_state, &format!("Error: {}", e));
                        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
                        update_status(&app_handle_clone, &app_state, "Idle");
                    }
                }
            }
            Err(e) => {
                eprintln!("Transcription failed: {}", e);
                update_status(&app_handle_clone, &app_state, &format!("Error: {}", e));
                tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
                update_status(&app_handle_clone, &app_state, "Idle");
            }
        }

        let _ = std::fs::remove_file(temp_file_str);
    });

    Ok(())
}

fn cancel_recording_internal(app_handle: &AppHandle, state: &AppState) -> Result<(), String> {
    state.is_transcribe_cancelled.store(true, Ordering::SeqCst);

    if !state.is_recording.load(Ordering::SeqCst) {
        let status = state.status.lock().unwrap().clone();
        if status == "Transcribing" {
            update_status(app_handle, state, "Idle");
        }
        return Ok(());
    }

    state.is_recording.store(false, Ordering::SeqCst);
    update_status(app_handle, state, "Idle");

    let mut recorder = state.recorder.lock().map_err(|e| format!("Lock error: {}", e))?;
    let _ = recorder.cancel_recording();

    Ok(())
}

#[tauri::command]
fn cancel_recording(state: State<'_, AppState>, app_handle: AppHandle) -> Result<(), String> {
    cancel_recording_internal(&app_handle, &state)
}

#[tauri::command]
fn pause_recording(state: State<'_, AppState>) -> Result<(), String> {
    let mut recorder = state.recorder.lock().map_err(|e| format!("Lock error: {}", e))?;
    recorder.pause_recording()
}

#[tauri::command]
fn resume_recording(state: State<'_, AppState>) -> Result<(), String> {
    let mut recorder = state.recorder.lock().map_err(|e| format!("Lock error: {}", e))?;
    recorder.resume_recording()
}

async fn refine_text_internal(
    raw_text: &str,
    config: &AppConfig,
) -> Result<String, String> {
    if raw_text.is_empty() {
        return Ok("".to_string());
    }

    match config.provider.as_str() {
        "ollama" => {
            crate::api::refine_with_ollama(
                &config.ollama_url,
                &config.ollama_model,
                &config.prompt,
                raw_text,
                &config.transcription_language,
            ).await
        }
        "gemini" => {
            crate::api::refine_with_gemini(
                &config.api_key,
                &config.model,
                &config.prompt,
                raw_text,
                &config.transcription_language,
            ).await
        }
        "openai" => {
            crate::api::refine_with_openai_compatible(
                "https://api.openai.com/v1",
                &config.openai_api_key,
                &config.openai_refine_model,
                &config.prompt,
                raw_text,
                &config.transcription_language,
            ).await
        }
        "openrouter" => {
            crate::api::refine_with_openai_compatible(
                "https://openrouter.ai/api/v1",
                &config.openrouter_api_key,
                &config.openrouter_model,
                &config.prompt,
                raw_text,
                &config.transcription_language,
            ).await
        }
        "custom" => {
            crate::api::refine_with_openai_compatible(
                &config.custom_api_url,
                &config.custom_api_key,
                &config.custom_api_model,
                &config.prompt,
                raw_text,
                &config.transcription_language,
            ).await
        }
        "lm_studio" => {
            crate::api::refine_with_openai_compatible(
                &config.lm_studio_url,
                "",
                &config.lm_studio_model,
                &config.prompt,
                raw_text,
                &config.transcription_language,
            ).await
        }
        _ => Ok(raw_text.to_string()),
    }
}

fn focus_window(window: &tauri::WebviewWindow) {
    let _ = window.show();
    let _ = window.unminimize();
    let _ = window.set_always_on_top(true);
    let _ = window.set_always_on_top(false);
    let _ = window.set_focus();
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState {
            recorder: Mutex::new(AudioRecorder::new()),
            is_recording: AtomicBool::new(false),
            status: Mutex::new("Idle".to_string()),
            is_transcribe_cancelled: AtomicBool::new(false),
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .setup(|app| {
            let app_handle = app.handle().clone();
            
            // Write transcribe_sherpa.py dynamically to config dir
            if let Ok(config_dir) = app_handle.path().app_config_dir() {
                let _ = std::fs::create_dir_all(&config_dir);
                let script_path = config_dir.join("transcribe_sherpa.py");
                let script_content = include_str!("transcribe_sherpa.py");
                let _ = std::fs::write(&script_path, script_content);
            }

            // Initialize voice_notes table
            if let Ok(data_dir) = app_handle.path().app_data_dir() {
                let db_path = data_dir.join("history.db");
                if let Ok(conn) = rusqlite::Connection::open(&db_path) {
                    let _ = voice_notes::init_voice_notes_table(&conn);
                }
            }

            let config = AppConfig::load(&app_handle);
            crate::hotkey::update_hotkeys(&config.transcribe_key, &config.notes_key, &config.cancel_key);

            crate::injector::start_focus_tracker();

            let listener = HotkeyListener::start(move |event| {
                let app_state = app_handle.state::<AppState>();
                match event {
                    HotkeyEvent::Pressed(rec_type) => {
                        match rec_type {
                            RecordingType::Transcribe => {
                                let _ = start_recording_internal(&app_handle, &app_state);
                            }
                            RecordingType::Notes => {
                                // Bring the main window to the foreground and open the title dialog
                                if let Some(window) = app_handle.get_webview_window("main") {
                                    focus_window(&window);
                                }
                                let _ = app_handle.emit("note-hotkey-open-title-dialog", ());
                            }
                        }
                    }
                    HotkeyEvent::Released(rec_type) => {
                        match rec_type {
                            RecordingType::Transcribe => {
                                let _ = stop_recording_internal(&app_handle, &app_state);
                            }
                            RecordingType::Notes => {
                                // Notes recording lifecycle is controlled by the UI.
                                // No action needed on key release.
                            }
                        }
                    }
                    HotkeyEvent::Cancelled(rec_type) => {
                        match rec_type {
                            RecordingType::Transcribe => {
                                let _ = cancel_recording_internal(&app_handle, &app_state);
                            }
                            RecordingType::Notes => {
                                // Only cancel if we're actively recording a note
                                if app_state.is_recording.load(Ordering::SeqCst) {
                                    let _ = cancel_recording_internal(&app_handle, &app_state);
                                    let _ = app_handle.emit("note-recording-cancelled-from-hotkey", ());
                                }
                            }
                        }
                    }
                    HotkeyEvent::GlobalCancel => {
                        if app_state.is_recording.load(Ordering::SeqCst) {
                            let _ = cancel_recording_internal(&app_handle, &app_state);
                        }
                        let _ = app_handle.emit("note-recording-cancelled-from-hotkey", ());
                    }
                }
            });

            app.manage(listener);

            // System Tray Menu and Icon
            let home_i = MenuItemBuilder::new("Home").id("home").build(app)?;
            let settings_i = MenuItemBuilder::new("Settings").id("settings").build(app)?;
            let exit_i = MenuItemBuilder::new("Exit").id("exit").build(app)?;
            let menu = MenuBuilder::new(app)
                .item(&home_i)
                .item(&settings_i)
                .item(&exit_i)
                .build()?;

            let mut tray_builder = TrayIconBuilder::new()
                .menu(&menu)
                .on_menu_event(|app, event| {
                    match event.id().as_ref() {
                        "home" => {
                            if let Some(window) = app.get_webview_window("main") {
                                focus_window(&window);
                                let _ = app.emit("show-tab", "dashboard");
                            }
                        }
                        "settings" => {
                            if let Some(window) = app.get_webview_window("main") {
                                focus_window(&window);
                                let _ = app.emit("show-tab", "settings");
                            }
                        }
                        "exit" => {
                            app.exit(0);
                        }
                        _ => {}
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: tauri::tray::MouseButton::Left,
                        button_state: tauri::tray::MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            focus_window(&window);
                        }
                    }
                });

            if let Some(icon) = app.default_window_icon().cloned() {
                tray_builder = tray_builder.icon(icon.clone());
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.set_icon(icon);
                }
            }

            let _tray = tray_builder.build(app)?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            get_status,
            get_app_version,
            load_config,
            save_config,
            manual_trigger_start,
            manual_trigger_stop,
            cancel_recording,
            pause_recording,
            resume_recording,
            get_audio_devices,
            start_mic_test,
            stop_mic_test,
            set_window_focusable,
            history::get_history,
            history::delete_history_entry,
            history::clear_all_history,
            downloader::check_model_downloaded,
            downloader::download_model_files,
            downloader::delete_model_files,
            create_voice_note,
            get_voice_notes,
            get_voice_note,
            update_voice_note,
            delete_voice_note,
            get_audio_file_url,
            start_voice_note_recording,
            stop_voice_note_recording,
            stop_voice_note_recording_bg,
            polish_voice_note_text
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn set_window_focusable(window: tauri::Window, focusable: bool) -> Result<(), String> {
    window.set_focusable(focusable).map_err(|e| e.to_string())?;
    if focusable {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_always_on_top(true);
        let _ = window.set_always_on_top(false);
        let _ = window.set_focus();
    }
    Ok(())
}
