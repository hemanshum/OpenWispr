mod audio;
mod hotkey;
mod injector;
mod api;
mod config;

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Emitter, Manager, State};

use crate::audio::AudioRecorder;
use crate::hotkey::{HotkeyEvent, HotkeyListener};
use crate::config::AppConfig;

pub struct AppState {
    pub recorder: Mutex<AudioRecorder>,
    pub is_recording: AtomicBool,
    pub status: Mutex<String>,
}

#[tauri::command]
fn get_status(state: State<'_, AppState>) -> String {
    state.status.lock().unwrap().clone()
}

#[tauri::command]
fn load_config(app_handle: AppHandle) -> AppConfig {
    AppConfig::load(&app_handle)
}

#[tauri::command]
fn save_config(config: AppConfig, app_handle: AppHandle) -> Result<(), String> {
    config.save(&app_handle)
}

fn update_status(app_handle: &AppHandle, state: &AppState, new_status: &str) {
    if let Ok(mut status) = state.status.lock() {
        *status = new_status.to_string();
    }
    let _ = app_handle.emit("status-changed", new_status);
}

#[tauri::command]
fn manual_trigger_start(state: State<'_, AppState>, app_handle: AppHandle) -> Result<(), String> {
    start_recording_internal(&app_handle, &state)
}

#[tauri::command]
fn manual_trigger_stop(state: State<'_, AppState>, app_handle: AppHandle) -> Result<(), String> {
    stop_recording_internal(&app_handle, &state)
}

fn start_recording_internal(app_handle: &AppHandle, state: &AppState) -> Result<(), String> {
    if state.is_recording.load(Ordering::SeqCst) {
        return Ok(());
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
    update_status(app_handle, state, "Transcribing");

    let mut recorder = state.recorder.lock().map_err(|e| format!("Lock error: {}", e))?;
    
    let temp_dir = std::env::temp_dir();
    let temp_file_path = temp_dir.join("openwispr_recording.wav");
    let temp_file_str = temp_file_path.to_string_lossy().to_string();

    recorder.stop_recording(&temp_file_str)?;

    let app_handle_clone = app_handle.clone();
    let api_config = AppConfig::load(app_handle);

    tauri::async_runtime::spawn(async move {
        let app_state = app_handle_clone.state::<AppState>();
        
        let tx_provider = api_config.transcription_provider.to_string();
        let openai_api_key = api_config.openai_api_key.trim().to_string();
        let api_key = api_config.api_key.trim().to_string();
        let provider = api_config.provider.to_string(); // provider maps to refinement provider

        if tx_provider == "openai" && openai_api_key.is_empty() {
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

        let prompt = api_config.prompt.to_string();
        let model = api_config.model.to_string();
        let ollama_url = api_config.ollama_url.to_string();
        let ollama_model = api_config.ollama_model.to_string();
        let openai_model = api_config.openai_model.to_string();
        let local_whisper_model = api_config.local_whisper_model.to_string();
        let transcription_language = api_config.transcription_language.to_string();

        let result = match tx_provider.as_str() {
            "local_whisper" => {
                match crate::api::transcribe_local_whisper(
                    &temp_file_str,
                    &local_whisper_model,
                    &transcription_language,
                ).await {
                    Ok(raw_text) => {
                        if raw_text.is_empty() {
                            Ok("".to_string())
                        } else {
                            match provider.as_str() {
                                "ollama" => {
                                    crate::api::refine_with_ollama(
                                        &ollama_url,
                                        &ollama_model,
                                        &prompt,
                                        &raw_text,
                                        &transcription_language,
                                    ).await
                                }
                                "gemini" => {
                                    crate::api::refine_with_gemini(
                                        &api_key,
                                        &model,
                                        &prompt,
                                        &raw_text,
                                        &transcription_language,
                                    ).await
                                }
                                _ => Ok(raw_text),
                            }
                        }
                    }
                    Err(e) => Err(e),
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
                        if raw_text.is_empty() {
                            Ok("".to_string())
                        } else {
                            match provider.as_str() {
                                "ollama" => {
                                    crate::api::refine_with_ollama(
                                        &ollama_url,
                                        &ollama_model,
                                        &prompt,
                                        &raw_text,
                                        &transcription_language,
                                    ).await
                                }
                                "gemini" => {
                                    crate::api::refine_with_gemini(
                                        &api_key,
                                        &model,
                                        &prompt,
                                        &raw_text,
                                        &transcription_language,
                                    ).await
                                }
                                _ => Ok(raw_text),
                            }
                        }
                    }
                    Err(e) => Err(e),
                }
            }
            _ => { // "gemini"
                if provider == "ollama" {
                    let verbatim_prompt = "Transcribe the audio verbatim. Keep all original words, sounds, and filler sounds.";
                    match crate::api::transcribe_and_clean_gemini(
                        &temp_file_str,
                        &api_key,
                        verbatim_prompt,
                        "gemini-2.0-flash",
                        &transcription_language,
                    ).await {
                        Ok(raw_text) => {
                            if raw_text.is_empty() {
                                Ok("".to_string())
                            } else {
                                crate::api::refine_with_ollama(
                                    &ollama_url,
                                    &ollama_model,
                                    &prompt,
                                    &raw_text,
                                    &transcription_language,
                                ).await
                            }
                        }
                        Err(e) => Err(e),
                    }
                } else if provider == "gemini" {
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

        match result {
            Ok(transcribed_text) => {
                if transcribed_text.is_empty() {
                    update_status(&app_handle_clone, &app_state, "Idle");
                    return;
                }

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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState {
            recorder: Mutex::new(AudioRecorder::new()),
            is_recording: AtomicBool::new(false),
            status: Mutex::new("Idle".to_string()),
        })
        .setup(|app| {
            let app_handle = app.handle().clone();
            
            let listener = HotkeyListener::start(move |event| {
                let app_state = app_handle.state::<AppState>();
                match event {
                    HotkeyEvent::Pressed => {
                        let _ = start_recording_internal(&app_handle, &app_state);
                    }
                    HotkeyEvent::Released => {
                        let _ = stop_recording_internal(&app_handle, &app_state);
                    }
                }
            });

            app.manage(listener);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            get_status,
            load_config,
            save_config,
            manual_trigger_start,
            manual_trigger_stop,
            get_audio_devices,
            start_mic_test,
            stop_mic_test
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}
