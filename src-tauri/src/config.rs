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
    #[serde(default = "default_local_whisper_model")]
    pub local_whisper_model: String,
    #[serde(default = "default_transcription_language")]
    pub transcription_language: String,
    #[serde(default = "default_openai_refine_model")]
    pub openai_refine_model: String,
    #[serde(default = "default_openrouter_api_key")]
    pub openrouter_api_key: String,
    #[serde(default = "default_openrouter_model")]
    pub openrouter_model: String,
    #[serde(default = "default_custom_api_url")]
    pub custom_api_url: String,
    #[serde(default = "default_custom_api_key")]
    pub custom_api_key: String,
    #[serde(default = "default_custom_api_model")]
    pub custom_api_model: String,
    #[serde(default = "default_noise_gate")]
    pub noise_gate: bool,
    #[serde(default = "default_lm_studio_url")]
    pub lm_studio_url: String,
    #[serde(default = "default_lm_studio_model")]
    pub lm_studio_model: String,
    #[serde(default = "default_transcribe_key")]
    pub transcribe_key: String,
    #[serde(default = "default_notes_key")]
    pub notes_key: String,
}

fn default_transcribe_key() -> String {
    "Control".to_string()
}

fn default_notes_key() -> String {
    "Control + Win".to_string()
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

fn default_local_whisper_model() -> String {
    "base".to_string()
}

fn default_transcription_language() -> String {
    "auto".to_string()
}

fn default_openai_refine_model() -> String {
    "gpt-4o-mini".to_string()
}

fn default_openrouter_api_key() -> String {
    "".to_string()
}

fn default_openrouter_model() -> String {
    "google/gemini-2.5-flash".to_string()
}

fn default_custom_api_url() -> String {
    "".to_string()
}

fn default_custom_api_key() -> String {
    "".to_string()
}

fn default_custom_api_model() -> String {
    "".to_string()
}

fn default_noise_gate() -> bool {
    false
}

fn default_lm_studio_url() -> String {
    "http://localhost:1234".to_string()
}

fn default_lm_studio_model() -> String {
    "".to_string()
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
            local_whisper_model: "base".to_string(),
            transcription_language: "auto".to_string(),
            openai_refine_model: "gpt-4o-mini".to_string(),
            openrouter_api_key: "".to_string(),
            openrouter_model: "google/gemini-2.5-flash".to_string(),
            custom_api_url: "".to_string(),
            custom_api_key: "".to_string(),
            custom_api_model: "".to_string(),
            noise_gate: false,
            lm_studio_url: "http://localhost:1234".to_string(),
            lm_studio_model: "".to_string(),
            transcribe_key: "Control".to_string(),
            notes_key: "Control + Win".to_string(),
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
