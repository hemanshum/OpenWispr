use base64::{engine::general_purpose, Engine as _};
use reqwest::Client;
use serde_json::json;
use std::fs;

fn get_language_name(code: &str) -> &str {
    match code {
        "en" => "English",
        "hi" => "Hindi",
        "es" => "Spanish",
        "fr" => "French",
        "de" => "German",
        "it" => "Italian",
        "pt" => "Portuguese",
        "zh" => "Chinese",
        "ja" => "Japanese",
        "ko" => "Korean",
        "ru" => "Russian",
        _ => "Auto-detect",
    }
}

pub async fn transcribe_and_clean_gemini(
    wav_path: &str,
    api_key: &str,
    prompt: &str,
    model: &str,
    language: &str,
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

    let system_instruction = "You are a professional voice dictation assistant. Your task is to transcribe the audio and clean it up. \
Keep all original meaning but remove filler words (like 'um', 'uh', 'like'), correct backtracking, and format it into clean text.

CRITICAL TRANSLITERATION, SCRIPT & TRANSLATION PRESERVATION RULES:
1. DO NOT TRANSLATE: Absolutely DO NOT translate any spoken words to English or any other language. Transcribe exactly the words spoken in their native language.
2. HINDI IN DEVANAGARI: If the speaker speaks in Hindi (or a mix of Hindi and English), you MUST write all Hindi words/phrases in Devanagari script.
3. ENGLISH IN ENGLISH SCRIPT: Keep all English words and phrases in English (Latin script).
4. MIXED LANGUAGE (HINGLISH) HANDLING: For code-mixed speech (mixing Hindi and English), write each word/phrase in its respective native script (e.g., Hindi in Devanagari script, English in English script).
   - Example Spoken: \"Mera naam Hemanshu hai and I am a software engineer\"
   - Expected Output: \"मेरा नाम हिमांशु है and I am a software engineer\"
   - (DO NOT output: \"My name is Hemanshu and I am a software engineer\")
5. NO META-TEXT: Do not add any conversational responses, notes, explanations, prefix, or suffix. Return ONLY the transcribed and cleaned text.";

    let language_name = get_language_name(language);
    let lang_instruction = if language_name == "Hindi" {
        " IMPORTANT SCRIPT & TRANSLATION RULES: The audio is in Hindi (or Hinglish, a mix of Hindi and English). You MUST transcribe Hindi words in Devanagari script (e.g. 'मेरा', 'नाम', 'है', 'कैसे', 'हो') and English words in English/Latin script. Absolutely DO NOT translate Hindi words to English (e.g. do NOT transcribe 'मेरा नाम' as 'My name').".to_string()
    } else if language_name != "Auto-detect" {
        format!(" IMPORTANT: The audio is spoken in {} (or a mix of {} and English). You must transcribe exactly what is spoken in that same language mix. Write {} words in native script and English words in English script. Do NOT translate non-English words to English.", language_name, language_name, language_name)
    } else {
        " IMPORTANT SCRIPT & TRANSLATION RULES: Detect the language spoken in the audio. If the audio is in Hindi or Hinglish (mixed Hindi-English), you MUST write Hindi words in Devanagari script and English words in English script. Absolutely DO NOT translate Hindi words to English (e.g. do NOT write 'My name' if the speaker said 'Mera naam'). Absolutely DO NOT translate English words to Hindi.".to_string()
    };
    
    let combined_prompt = format!(
        "User Instruction: {}\n\nCRITICAL SCRIPT & LANGUAGE DIRECTIVE: {}\n\nTranscribe and clean up the audio according to these instructions:",
        if prompt.is_empty() { "Transcribe and clean up the audio." } else { prompt },
        lang_instruction
    );

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
                        "text": combined_prompt
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
    language: &str,
) -> Result<String, String> {
    let client = Client::new();
    let url = format!("{}/api/generate", ollama_url.trim_end_matches('/'));

    let system_instruction = "You are a professional voice dictation assistant. Your task is to refine and clean up the raw transcription. \
You must carefully process the text to ensure the correct script and language are preserved.

CRITICAL TRANSLITERATION, SCRIPT & TRANSLATION PRESERVATION RULES:
1. DETECT HINDI WORDS: Identify all Hindi words and phrases, even if they are written in Roman/Latin script (Hinglish / transliterated script, e.g., 'mera', 'naam', 'hai', 'kaise', 'ho', 'main', 'aur').
2. WRITE HINDI IN DEVANAGARI: You MUST convert all Hindi words/phrases to Devanagari script (e.g., convert 'mera' to 'मेरा', 'naam' to 'नाम', 'hai' to 'है', 'kaise' to 'कैसे', 'ho' to 'हो').
3. KEEP ENGLISH IN ENGLISH SCRIPT: Keep all English words and phrases in English (Latin script). For example, 'software engineer', 'developer', 'meeting', 'call you later' must remain in English script. Do NOT translate English words to Hindi.
4. STRICTLY PROHIBIT TRANSLATION: Do NOT translate Hindi words to English (e.g., do NOT translate 'mera naam' or 'मेरा नाम' to 'my name'). Keep them in their original language, but written in Devanagari script.
5. REMOVE FILLER WORDS: Remove filler words (like 'um', 'uh', 'like', 'ah'), correct backtracking, and format into clean, readable text.
6. NO META-TEXT: Do not add any conversational responses, explanations, note, prefix, or suffix. Return ONLY the finalized refined text.

EXAMPLES:
- Input: \"mera naam hemanshu hai and I am a software engineer\"
  Output: \"मेरा नाम हिमांशु है and I am a software engineer\"
- Input: \"aaj weather bahut achha hai main office ja raha hoon\"
  Output: \"आज वेदर बहुत अच्छा है मैं ऑफिस जा रहा हूँ\"
- Input: \"hello team aaj ki meeting ka agenda kya hai\"
  Output: \"hello team आज की मीटिंग का एजेंडा क्या है\"
- Input: \"I will call you later, main abhi busy hoon\"
  Output: \"I will call you later, मैं अभी बिजी हूँ\"";

    let language_name = get_language_name(language);
    let lang_instruction = if language_name == "Hindi" {
        " IMPORTANT SCRIPT & TRANSLATION RULES: The text is in Hindi (or Hinglish, a mix of Hindi and English). You MUST refine and clean up the text in that same language mix. Write all Hindi words in Devanagari script (e.g. convert Romanized 'mera naam', 'kaise ho' to 'मेरा नाम', 'कैसे हो') and keep English words in English/Latin script. Absolutely DO NOT translate Hindi words to English (e.g. do NOT convert 'मेरा नाम' or 'mera naam' to 'My name').".to_string()
    } else if language_name != "Auto-detect" {
        format!(" IMPORTANT: The text is in {} (or a mix of {} and English). You must refine and clean up the text in that same language mix. Keep {} words in their native script and English words in English script. Do NOT translate non-English words to English.", language_name, language_name, language_name)
    } else {
        " IMPORTANT SCRIPT & TRANSLATION RULES: Detect the language of the raw text. If the raw text contains Hindi or Hinglish (mixed Hindi-English), you MUST write all Hindi words in Devanagari script and keep all English words in English script. Absolutely DO NOT translate Hindi words to English (e.g. do NOT convert 'मेरा नाम' or 'mera naam' to 'My name').".to_string()
    };

    let combined_prompt = format!(
        "User Instruction: {}\n\nCRITICAL SCRIPT & LANGUAGE DIRECTIVE: {}\n\nRaw Transcript to clean up:\n\"\"\"\n{}\n\"\"\"\n\nRefined and Cleaned transcript:",
        prompt,
        lang_instruction,
        raw_text
    );

    let request_body = json!({
        "model": model,
        "prompt": combined_prompt,
        "system": system_instruction,
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

pub async fn transcribe_openai(
    wav_path: &str,
    api_key: &str,
    model: &str,
    language: &str,
) -> Result<String, String> {
    let client = Client::new();
    let url = "https://api.openai.com/v1/audio/transcriptions";

    let file_bytes = fs::read(wav_path)
        .map_err(|e| format!("Failed to read recorded audio file: {}", e))?;

    let part = reqwest::multipart::Part::bytes(file_bytes)
        .file_name("audio.wav")
        .mime_str("audio/wav")
        .map_err(|e| format!("Failed to prepare audio multipart: {}", e))?;

    let mut form = reqwest::multipart::Form::new()
        .part("file", part)
        .text("model", model.to_string());

    if language != "auto" {
        form = form.text("language", language.to_string());
    }

    if language == "hi" || language == "auto" {
        form = form.text("prompt", "Mera naam Hemanshu hai and I am a software engineer. Hello, how are you? आप कैसे हो? मैं ठीक हूँ।");
    }

    let response = client
        .post(url)
        .bearer_auth(api_key)
        .multipart(form)
        .send()
        .await
        .map_err(|e| format!("OpenAI API request failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let err_text = response.text().await.unwrap_or_default();
        return Err(format!("OpenAI API returned error status {}: {}", status, err_text));
    }

    let json_resp: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse OpenAI response: {}", e))?;

    let transcribed_text = json_resp["text"]
        .as_str()
        .ok_or_else(|| {
            format!(
                "Unexpected OpenAI response structure. Response: {:?}",
                json_resp
            )
        })?;

    Ok(transcribed_text.trim().to_string())
}

pub async fn refine_with_gemini(
    api_key: &str,
    model: &str,
    prompt: &str,
    raw_text: &str,
    language: &str,
) -> Result<String, String> {
    let client = Client::new();
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
        model, api_key
    );

    let system_instruction = "You are a professional voice dictation assistant. Your task is to refine and clean up the raw transcription. \
You must carefully process the text to ensure the correct script and language are preserved.

CRITICAL TRANSLITERATION, SCRIPT & TRANSLATION PRESERVATION RULES:
1. DETECT HINDI WORDS: Identify all Hindi words and phrases, even if they are written in Roman/Latin script (Hinglish / transliterated script, e.g., 'mera', 'naam', 'hai', 'kaise', 'ho', 'main', 'aur').
2. WRITE HINDI IN DEVANAGARI: You MUST convert all Hindi words/phrases to Devanagari script (e.g., convert 'mera' to 'मेरा', 'naam' to 'नाम', 'hai' to 'है', 'kaise' to 'कैसे', 'ho' to 'हो').
3. KEEP ENGLISH IN ENGLISH SCRIPT: Keep all English words and phrases in English (Latin script). For example, 'software engineer', 'developer', 'meeting', 'call you later' must remain in English script. Do NOT translate English words to Hindi.
4. STRICTLY PROHIBIT TRANSLATION: Do NOT translate Hindi words to English (e.g., do NOT translate 'mera naam' or 'मेरा नाम' to 'my name'). Keep them in their original language, but written in Devanagari script.
5. REMOVE FILLER WORDS: Remove filler words (like 'um', 'uh', 'like', 'ah'), correct backtracking, and format into clean, readable text.
6. NO META-TEXT: Do not add any conversational responses, explanations, note, prefix, or suffix. Return ONLY the finalized refined text.

EXAMPLES:
- Input: \"mera naam hemanshu hai and I am a software engineer\"
  Output: \"मेरा नाम हिमांशु है and I am a software engineer\"
- Input: \"aaj weather bahut achha hai main office ja raha hoon\"
  Output: \"आज वेदर बहुत अच्छा है मैं ऑफिस जा रहा हूँ\"
- Input: \"hello team aaj ki meeting ka agenda kya hai\"
  Output: \"hello team आज की मीटिंग का एजेंडा क्या है\"
- Input: \"I will call you later, main abhi busy hoon\"
  Output: \"I will call you later, मैं अभी बिजी हूँ\"";

    let language_name = get_language_name(language);
    let lang_instruction = if language_name == "Hindi" {
        " IMPORTANT SCRIPT & TRANSLATION RULES: The text is in Hindi (or Hinglish, a mix of Hindi and English). You MUST refine and clean up the text in that same language mix. Write all Hindi words in Devanagari script (e.g. convert Romanized 'mera naam', 'kaise ho' to 'मेरा नाम', 'कैसे हो') and keep English words in English/Latin script. Absolutely DO NOT translate Hindi words to English (e.g. do NOT convert 'मेरा नाम' or 'mera naam' to 'My name').".to_string()
    } else if language_name != "Auto-detect" {
        format!(" IMPORTANT: The text is in {} (or a mix of {} and English). You must refine and clean up the text in that same language mix. Keep {} words in their native script and English words in English script. Do NOT translate non-English words to English.", language_name, language_name, language_name)
    } else {
        " IMPORTANT SCRIPT & TRANSLATION RULES: Detect the language of the raw text. If the raw text contains Hindi or Hinglish (mixed Hindi-English), you MUST write all Hindi words in Devanagari script and keep all English words in English script. Absolutely DO NOT translate Hindi words to English (e.g. do NOT convert 'मेरा नाम' or 'mera naam' to 'My name').".to_string()
    };

    let combined_prompt = format!(
        "User Instruction: {}\n\nCRITICAL SCRIPT & LANGUAGE DIRECTIVE: {}\n\nRaw Transcript to clean up:\n\"\"\"\n{}\n\"\"\"\n\nRefined and Cleaned transcript:",
        prompt,
        lang_instruction,
        raw_text
    );

    let request_body = json!({
        "contents": [
            {
                "parts": [
                    {
                        "text": combined_prompt
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

    let refined_text = json_resp["candidates"][0]["content"]["parts"][0]["text"]
        .as_str()
        .ok_or_else(|| {
            format!(
                "Unexpected Gemini response structure. Response: {:?}",
                json_resp
            )
        })?;

    Ok(refined_text.trim().to_string())
}

pub async fn transcribe_local_whisper(
    wav_path: &str,
    model: &str,
    language: &str,
) -> Result<String, String> {
    use std::path::Path;
    use tokio::process::Command;

    let wav_path_buf = Path::new(wav_path);
    let file_stem = wav_path_buf
        .file_stem()
        .ok_or_else(|| "Invalid WAV file path".to_string())?
        .to_string_lossy();
    
    let parent_dir = wav_path_buf
        .parent()
        .ok_or_else(|| "Invalid parent directory for WAV file".to_string())?;
    
    let output_txt_path = parent_dir.join(format!("{}.txt", file_stem));
    let parent_dir_str = parent_dir.to_string_lossy().to_string();

    let mut command = Command::new("whisper");
    command
        .arg(wav_path)
        .arg("--model")
        .arg(model)
        .arg("--output_dir")
        .arg(&parent_dir_str)
        .arg("--output_format")
        .arg("txt")
        .env("PYTHONIOENCODING", "utf-8");

    if language != "auto" {
        command.arg("--language").arg(language);
    }

    if language == "hi" || language == "auto" {
        command.arg("--initial_prompt").arg("Mera naam Hemanshu hai and I am a software engineer. Hello, how are you? आप कैसे हो? मैं ठीक हूँ।");
    }

    let output = command
        .output()
        .await
        .map_err(|e| format!("Failed to run local Whisper executable. Is it installed and on PATH? Error: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Local Whisper command failed: {}", stderr));
    }

    if !output_txt_path.exists() {
        return Err("Local Whisper transcription completed but the output text file was not found".to_string());
    }

    let transcribed_text = fs::read_to_string(&output_txt_path)
        .map_err(|e| format!("Failed to read local Whisper output file: {}", e))?;

    let _ = fs::remove_file(output_txt_path);

    Ok(transcribed_text.trim().to_string())
}

