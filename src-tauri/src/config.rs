use std::fs;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use tauri::AppHandle;
use tauri::Manager;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AppConfig {
    pub api_key: String,
    pub prompt: String,
    #[serde(default = "default_model")]
    pub model: String,
    #[serde(default = "default_provider")]
    pub provider: String,
    #[serde(default = "default_ollama_url")]
    pub ollama_url: String,
    #[serde(default = "default_ollama_model")]
    pub ollama_model: String,
    #[serde(default = "default_audio_device")]
    pub audio_device: String,
    #[serde(default = "default_transcription_provider")]
    pub transcription_provider: String,
    #[serde(default = "default_openai_api_key")]
    pub openai_api_key: String,
    #[serde(default = "default_openai_model")]
    pub openai_model: String,
}

fn default_model() -> String {
    "gemini-2.0-flash".to_string()
}

fn default_provider() -> String {
    "gemini".to_string()
}

fn default_ollama_url() -> String {
    "http://localhost:11434".to_string()
}

fn default_ollama_model() -> String {
    "llama3".to_string()
}

fn default_audio_device() -> String {
    "Default".to_string()
}

fn default_transcription_provider() -> String {
    "gemini".to_string()
}

fn default_openai_api_key() -> String {
    "".to_string()
}

fn default_openai_model() -> String {
    "whisper-1".to_string()
}

impl AppConfig {
    pub fn default() -> Self {
        Self {
            api_key: "".to_string(),
            prompt: "Transcribe and clean up the audio. Remove filler words and format appropriately.".to_string(),
            model: "gemini-2.0-flash".to_string(),
            provider: "gemini".to_string(),
            ollama_url: "http://localhost:11434".to_string(),
            ollama_model: "llama3".to_string(),
            audio_device: "Default".to_string(),
            transcription_provider: "gemini".to_string(),
            openai_api_key: "".to_string(),
            openai_model: "whisper-1".to_string(),
        }
    }

    pub fn get_path(app_handle: &AppHandle) -> PathBuf {
        let mut path = app_handle
            .path()
            .app_config_dir()
            .unwrap_or_else(|_| PathBuf::from("."));
        let _ = fs::create_dir_all(&path);
        path.push("config.json");
        path
    }

    pub fn load(app_handle: &AppHandle) -> Self {
        let path = Self::get_path(app_handle);
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(mut config) = serde_json::from_str::<AppConfig>(&content) {
                // Self-healing migration for deprecated model
                if config.model == "gemini-1.5-flash" {
                    config.model = "gemini-2.0-flash".to_string();
                    let _ = config.save(app_handle);
                }
                return config;
            }
        }
        Self::default()
    }

    pub fn save(&self, app_handle: &AppHandle) -> Result<(), String> {
        let path = Self::get_path(app_handle);
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize config: {}", e))?;
        fs::write(path, content)
            .map_err(|e| format!("Failed to write config file: {}", e))?;
        Ok(())
    }
}
