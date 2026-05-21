use base64::{engine::general_purpose, Engine as _};
use reqwest::Client;
use serde_json::json;
use std::fs;

pub async fn transcribe_and_clean_gemini(
    wav_path: &str,
    api_key: &str,
    prompt: &str,
    model: &str,
) -> Result<String, String> {
    let client = Client::new();

    // Read WAV bytes and base64 encode
    let file_bytes = fs::read(wav_path)
        .map_err(|e| format!("Failed to read recorded audio file: {}", e))?;
    let base64_audio = general_purpose::STANDARD.encode(file_bytes);

    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
        model, api_key
    );

    let system_instruction = "You are a professional voice dictation assistant. Your task is to transcribe the audio and clean it up. Keep all original meaning but remove filler words (like 'um', 'uh', 'like'), correct backtracking (where the speaker corrects themselves mid-sentence), and format it into clean text. If lists are dictated, format them as bullet points or numbered lists. Do not add any conversational meta-text like 'Here is your transcript:' or 'Sure, here is the text.'. Return ONLY the cleaned, finalized text.";

    let request_body = json!({
        "contents": [
            {
                "parts": [
                    {
                        "inlineData": {
                            "mimeType": "audio/wav",
                            "data": base64_audio
                        }
                    },
                    {
                        "text": if prompt.is_empty() { "Transcribe and clean up the audio." } else { prompt }
                    }
                ]
            }
        ],
        "systemInstruction": {
            "parts": [
                {
                    "text": system_instruction
                }
            ]
        },
        "generationConfig": {
            "temperature": 0.2,
        }
    });

    let response = client
        .post(&url)
        .json(&request_body)
        .send()
        .await
        .map_err(|e| format!("Gemini API request failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let err_text = response.text().await.unwrap_or_default();
        return Err(format!("Gemini API returned error status {}: {}", status, err_text));
    }

    let json_resp: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse Gemini response: {}", e))?;

    // Extract text from the response structure
    let transcribed_text = json_resp["candidates"][0]["content"]["parts"][0]["text"]
        .as_str()
        .ok_or_else(|| {
            format!(
                "Unexpected Gemini response structure. Response: {:?}",
                json_resp
            )
        })?;

    Ok(transcribed_text.trim().to_string())
}

pub async fn refine_with_ollama(
    ollama_url: &str,
    model: &str,
    prompt: &str,
    raw_text: &str,
) -> Result<String, String> {
    let client = Client::new();
    let url = format!("{}/api/generate", ollama_url.trim_end_matches('/'));

    let system_instruction = "You are a professional voice dictation assistant. Your task is to refine and clean up the raw transcription. Keep all original meaning but remove filler words (like 'um', 'uh', 'like'), correct backtracking (where the speaker corrects themselves mid-sentence), and format it into clean text. If lists are dictated, format them as bullet points or numbered lists. Do not add any conversational meta-text like 'Here is your transcript:' or 'Sure, here is the text.'. Return ONLY the cleaned, finalized text.";

    let combined_prompt = format!(
        "System Instruction: {}\nUser Instruction: {}\n\nRaw Transcript to clean up:\n\"\"\"\n{}\n\"\"\"\n\nCleaned transcript:",
        system_instruction,
        prompt,
        raw_text
    );

    let request_body = json!({
        "model": model,
        "prompt": combined_prompt,
        "stream": false
    });

    let response = client
        .post(&url)
        .json(&request_body)
        .send()
        .await
        .map_err(|e| format!("Ollama API request failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let err_text = response.text().await.unwrap_or_default();
        return Err(format!("Ollama API returned error status {}: {}", status, err_text));
    }

    let json_resp: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse Ollama response: {}", e))?;

    let refined_text = json_resp["response"]
        .as_str()
        .ok_or_else(|| {
            format!(
                "Unexpected Ollama response structure. Response: {:?}",
                json_resp
            )
        })?;

    Ok(refined_text.trim().to_string())
}
