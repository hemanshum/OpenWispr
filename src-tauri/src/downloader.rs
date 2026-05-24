use std::fs;
use std::path::PathBuf;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use reqwest::Client;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

#[derive(Clone, Serialize)]
struct DownloadProgress {
    model_id: String,
    file_index: usize,
    total_files: usize,
    file_name: String,
    progress: f32, // 0.0 to 100.0
    status: String,
}

pub struct ModelFileInfo {
    pub name: &'static str,
    pub url: &'static str,
}

pub fn get_model_files(model_id: &str) -> Result<Vec<ModelFileInfo>, String> {
    match model_id {
        "parakeet_v3" => Ok(vec![
            ModelFileInfo {
                name: "encoder.int8.onnx",
                url: "https://huggingface.co/csukuangfj/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8/resolve/main/encoder.int8.onnx",
            },
            ModelFileInfo {
                name: "decoder.int8.onnx",
                url: "https://huggingface.co/csukuangfj/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8/resolve/main/decoder.int8.onnx",
            },
            ModelFileInfo {
                name: "joiner.int8.onnx",
                url: "https://huggingface.co/csukuangfj/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8/resolve/main/joiner.int8.onnx",
            },
            ModelFileInfo {
                name: "tokens.txt",
                url: "https://huggingface.co/csukuangfj/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8/resolve/main/tokens.txt",
            },
        ]),
        "whisper_tiny" => Ok(vec![
            ModelFileInfo {
                name: "tiny-encoder.int8.onnx",
                url: "https://huggingface.co/csukuangfj/sherpa-onnx-whisper-tiny/resolve/main/tiny-encoder.int8.onnx",
            },
            ModelFileInfo {
                name: "tiny-decoder.int8.onnx",
                url: "https://huggingface.co/csukuangfj/sherpa-onnx-whisper-tiny/resolve/main/tiny-decoder.int8.onnx",
            },
            ModelFileInfo {
                name: "tiny-tokens.txt",
                url: "https://huggingface.co/csukuangfj/sherpa-onnx-whisper-tiny/resolve/main/tiny-tokens.txt",
            },
        ]),
        "whisper_base" => Ok(vec![
            ModelFileInfo {
                name: "base-encoder.int8.onnx",
                url: "https://huggingface.co/csukuangfj/sherpa-onnx-whisper-base/resolve/main/base-encoder.int8.onnx",
            },
            ModelFileInfo {
                name: "base-decoder.int8.onnx",
                url: "https://huggingface.co/csukuangfj/sherpa-onnx-whisper-base/resolve/main/base-decoder.int8.onnx",
            },
            ModelFileInfo {
                name: "base-tokens.txt",
                url: "https://huggingface.co/csukuangfj/sherpa-onnx-whisper-base/resolve/main/base-tokens.txt",
            },
        ]),
        "whisper_small" => Ok(vec![
            ModelFileInfo {
                name: "small-encoder.int8.onnx",
                url: "https://huggingface.co/csukuangfj/sherpa-onnx-whisper-small/resolve/main/small-encoder.int8.onnx",
            },
            ModelFileInfo {
                name: "small-decoder.int8.onnx",
                url: "https://huggingface.co/csukuangfj/sherpa-onnx-whisper-small/resolve/main/small-decoder.int8.onnx",
            },
            ModelFileInfo {
                name: "small-tokens.txt",
                url: "https://huggingface.co/csukuangfj/sherpa-onnx-whisper-small/resolve/main/small-tokens.txt",
            },
        ]),
        _ => Err(format!("Unsupported model ID: {}", model_id)),
    }
}

pub fn get_model_dir(app_handle: &AppHandle, model_id: &str) -> PathBuf {
    let mut path = app_handle
        .path()
        .app_config_dir()
        .unwrap_or_else(|_| PathBuf::from("."));
    path.push("models");
    path.push(model_id);
    let _ = fs::create_dir_all(&path);
    path
}

#[tauri::command]
pub fn check_model_downloaded(app_handle: AppHandle, model_id: String) -> bool {
    let files = match get_model_files(&model_id) {
        Ok(f) => f,
        Err(_) => return false,
    };
    
    let model_dir = get_model_dir(&app_handle, &model_id);
    
    for file_info in files {
        let file_path = model_dir.join(file_info.name);
        if !file_path.exists() {
            return false;
        }
        // Verify size is not zero
        if let Ok(metadata) = fs::metadata(&file_path) {
            if metadata.len() == 0 {
                return false;
            }
        } else {
            return false;
        }
    }
    
    true
}

#[tauri::command]
pub async fn download_model_files(app_handle: AppHandle, model_id: String) -> Result<(), String> {
    let files = get_model_files(&model_id)?;
    let model_dir = get_model_dir(&app_handle, &model_id);
    
    let client = Client::new();
    let total_files = files.len();
    
    for (idx, file_info) in files.iter().enumerate() {
        let file_name = file_info.name.to_string();
        let dest_path = model_dir.join(&file_name);
        
        // Skip if already exists and has size > 0
        if dest_path.exists() {
            if let Ok(meta) = fs::metadata(&dest_path) {
                if meta.len() > 0 {
                    // Send skip progress event
                    let _ = app_handle.emit(
                        "model-download-progress",
                        DownloadProgress {
                            model_id: model_id.clone(),
                            file_index: idx + 1,
                            total_files,
                            file_name: file_name.clone(),
                            progress: 100.0,
                            status: format!("Skipped existing file {}/{} ({})", idx + 1, total_files, file_name),
                        },
                    );
                    continue;
                }
            }
        }
        
        let mut response = client.get(file_info.url)
            .send()
            .await
            .map_err(|e| format!("Failed to initiate download for {}: {}", file_name, e))?;
            
        if !response.status().is_success() {
            return Err(format!("Download request failed for {}: HTTP {}", file_name, response.status()));
        }
        
        let total_size = response.content_length().unwrap_or(0);
        let mut file = File::create(&dest_path)
            .await
            .map_err(|e| format!("Failed to create local model file {}: {}", file_name, e))?;
            
        let mut downloaded: u64 = 0;
        
        while let Some(chunk) = response.chunk().await.map_err(|e| format!("Error downloading chunk: {}", e))? {
            file.write_all(&chunk)
                .await
                .map_err(|e| format!("Failed to write chunk: {}", e))?;
                
            downloaded += chunk.len() as u64;
            
            // Calculate percentage
            let progress = if total_size > 0 {
                (downloaded as f32 / total_size as f32) * 100.0
            } else {
                0.0
            };
            
            // Emit progress event
            let _ = app_handle.emit(
                "model-download-progress",
                DownloadProgress {
                    model_id: model_id.clone(),
                    file_index: idx + 1,
                    total_files,
                    file_name: file_name.clone(),
                    progress,
                    status: format!(
                        "Downloading file {}/{} ({}): {:.1}%",
                        idx + 1,
                        total_files,
                        file_name,
                        progress
                    ),
                },
            );
        }
        
        file.flush().await.map_err(|e| format!("Failed to flush file: {}", e))?;
    }
    
    // final success event
    let _ = app_handle.emit(
        "model-download-progress",
        DownloadProgress {
            model_id: model_id.clone(),
            file_index: total_files,
            total_files,
            file_name: "".to_string(),
            progress: 100.0,
            status: "All files downloaded successfully!".to_string(),
        },
    );
    
    Ok(())
}

#[tauri::command]
pub fn delete_model_files(app_handle: AppHandle, model_id: String) -> Result<(), String> {
    let model_dir = get_model_dir(&app_handle, &model_id);
    if model_dir.exists() {
        fs::remove_dir_all(&model_dir)
            .map_err(|e| format!("Failed to delete model files: {}", e))?;
    }
    Ok(())
}
