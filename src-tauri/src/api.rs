use base64::{engine::general_purpose, Engine as _};
use once_cell::sync::Lazy;
use reqwest::Client;
use serde_json::json;
use std::fs;
use tauri::{AppHandle, Manager, Emitter};

/// Persistent HTTP client singleton — reuses TCP connections and TLS sessions
/// across requests via HTTP keep-alive, avoiding handshake overhead.
static HTTP_CLIENT: Lazy<Client> = Lazy::new(|| Client::new());

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
    let client = &*HTTP_CLIENT;

    // Read WAV bytes and base64 encode
    let file_bytes = fs::read(wav_path)
        .map_err(|e| format!("Failed to read recorded audio file: {}", e))?;
    let base64_audio = general_purpose::STANDARD.encode(file_bytes);

    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
        model, api_key
    );

    let language_name = get_language_name(language);
    let system_instruction = if language_name == "English" {
        "You are a professional voice dictation assistant. Your task is to transcribe the audio and clean it up. \
The spoken language is English. You must write everything strictly in English.

CRITICAL LANGUAGE RULES:
1. FILTER BACKGROUND NOISE: Ignore any ambient noise, background chatter, crosstalk, or non-speech sounds. Do not transcribe them.
2. WRITE ONLY IN ENGLISH: The spoken audio/text is in English. You must write everything strictly in English. Do NOT mix other languages, and do NOT use Devanagari script, Romanized Hindi, or any non-English script.
3. TRANSLATE/CORRECT NON-ENGLISH: If there are any non-English words, phrases, or scripts in the raw input (due to transcription errors, noise, accents, or code-switching), you MUST translate them to English or correct them to fit the English context. The entire output must be 100% standard English.
4. REMOVE FILLER WORDS: Remove filler words (like 'um', 'uh', 'like', 'ah'), correct backtracking, and format into clean, readable text.
5. NO META-TEXT: Do not add any conversational responses, notes, explanations, prefix, or suffix. Return ONLY the finalized text."
    } else if language_name == "Hindi" {
        "You are a professional voice dictation assistant. Your task is to transcribe the audio and clean it up. \
Keep all original meaning but remove filler words (like 'um', 'uh', 'like'), correct backtracking, and format it into clean text.

CRITICAL TRANSLITERATION, SCRIPT & TRANSLATION PRESERVATION RULES:
1. FILTER BACKGROUND NOISE: Ignore any ambient noise, background chatter, crosstalk, or non-speech sounds. Do not transcribe them.
2. DO NOT TRANSLATE: Absolutely DO NOT translate any spoken words to English or any other language. Transcribe exactly the words spoken in their native language.
3. HINDI IN DEVANAGARI: If the speaker speaks in Hindi (or a mix of Hindi and English), you MUST write all Hindi words/phrases in Devanagari script.
4. ENGLISH IN ENGLISH SCRIPT: Keep all English words and phrases in English (Latin script). For example, 'software engineer', 'developer', 'meeting', 'call you later' must remain in English script. Do NOT translate English words to Hindi.
5. MIXED LANGUAGE (HINGLISH) HANDLING: For code-mixed speech (mixing Hindi and English), write each word/phrase in its respective native script (e.g., Hindi in Devanagari script, English in English script).
   - Example Spoken: \"Mera naam Hemanshu hai and I am a software engineer\"
   - Expected Output: \"मेरा नाम हिमांशु है and I am a software engineer\"
   - (DO NOT output: \"My name is Hemanshu and I am a software engineer\")
6. NO META-TEXT: Do not add any conversational responses, notes, explanations, prefix, or suffix. Return ONLY the transcribed and cleaned text."
    } else {
        "You are a professional voice dictation assistant. Your task is to transcribe the audio and clean it up. \
Keep all original meaning but remove filler words (like 'um', 'uh', 'like'), correct backtracking, and format it into clean text.

CRITICAL SCRIPT & TRANSLATION PRESERVATION RULES:
1. FILTER BACKGROUND NOISE: Ignore any ambient noise, background chatter, crosstalk, or non-speech sounds. Do not transcribe them.
2. DO NOT TRANSLATE: Absolutely DO NOT translate non-English words to English. Transcribe exactly the words spoken in their native language.
3. NATIVE SCRIPT: Write the spoken words in their respective native script (e.g., Spanish words in Spanish/Latin script, Chinese in Chinese characters, Hindi in Devanagari script).
4. ENGLISH IN ENGLISH SCRIPT: Keep all English words and phrases in English (Latin script).
5. MIXED LANGUAGE HANDLING: For code-mixed speech (mixing the target language and English), write each word/phrase in its respective native script.
6. NO META-TEXT: Do not add any conversational responses, notes, explanations, prefix, or suffix. Return ONLY the transcribed and cleaned text."
    };

    let lang_instruction = if language_name == "English" {
        "IMPORTANT: The audio is spoken in English. You must transcribe everything strictly in English. Do NOT use any Hindi words, Devanagari script, Romanized Hindi, or non-English script. Translate or correct any non-English words to clean English.".to_string()
    } else if language_name == "Hindi" {
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
    let client = &*HTTP_CLIENT;
    let url = format!("{}/api/generate", ollama_url.trim_end_matches('/'));

    let language_name = get_language_name(language);
    let system_instruction = if language_name == "English" {
        "You are a professional voice dictation assistant. Your task is to refine and clean up the raw transcription. \
The spoken language is English. You must write everything strictly in English.

CRITICAL LANGUAGE RULES:
1. FILTER BACKGROUND NOISE: Ignore and remove any words or phrases that represent ambient noise, background chatter, crosstalk, or non-speech sounds. Do not include them in the refined text.
2. WRITE ONLY IN ENGLISH: The spoken audio/text is in English. You must write everything strictly in English. Do NOT mix other languages, and do NOT use Devanagari script, Romanized Hindi, or any non-English script.
3. TRANSLATE/CORRECT NON-ENGLISH: If there are any non-English words, phrases, or scripts in the raw input (due to transcription errors, noise, accents, or code-switching), you MUST translate them to English or correct them to fit the English context. The entire output must be 100% standard English.
4. REMOVE FILLER WORDS: Remove filler words (like 'um', 'uh', 'like', 'ah'), correct backtracking, and format into clean, readable text.
5. NO META-TEXT: Do not add any conversational responses, explanations, note, prefix, or suffix. Return ONLY the finalized refined text."
    } else if language_name == "Hindi" {
        "You are a professional voice dictation assistant. Your task is to refine and clean up the raw transcription. \
You must carefully process the text to ensure the correct script and language are preserved.

CRITICAL TRANSLITERATION, SCRIPT & TRANSLATION PRESERVATION RULES:
1. FILTER BACKGROUND NOISE: Ignore and remove any words or phrases that represent ambient noise, background chatter, crosstalk, or non-speech sounds. Do not include them in the refined text.
2. DETECT HINDI WORDS: Identify all Hindi words and phrases, even if they are written in Roman/Latin script (Hinglish / transliterated script, e.g., 'mera', 'naam', 'hai', 'kaise', 'ho', 'main', 'aur').
3. WRITE HINDI IN DEVANAGARI: You MUST convert all Hindi words/phrases to Devanagari script (e.g., convert 'mera' to 'मेरा', 'naam' to 'नाम', 'hai' to 'है', 'kaise' to 'कैसे', 'ho' to 'हो').
4. KEEP ENGLISH IN ENGLISH SCRIPT: Keep all English words and phrases in English (Latin script). For example, 'software engineer', 'developer', 'meeting', 'call you later' must remain in English script. Do NOT translate English words to Hindi.
5. STRICTLY PROHIBIT TRANSLATION: Do NOT translate Hindi words to English (e.g., do NOT translate 'mera naam' or 'मेरा नाम' to 'my name'). Keep them in their original language, but written in Devanagari script.
6. REMOVE FILLER WORDS: Remove filler words (like 'um', 'uh', 'like', 'ah'), correct backtracking, and format into clean, readable text.
7. NO META-TEXT: Do not add any conversational responses, explanations, note, prefix, or suffix. Return ONLY the finalized refined text.

EXAMPLES:
- Input: \"mera naam hemanshu hai and I am a software engineer\"
  Output: \"मेरा नाम हिमांशु है and I am a software engineer\"
- Input: \"aaj weather bahut achha hai main office ja raha hoon\"
  Output: \"आज weather बहुत अच्छा है मैं office जा रहा हूँ\"
- Input: \"hello team aaj ki meeting ka agenda kya hai\"
  Output: \"hello team आज की meeting का agenda क्या है\"
- Input: \"I will call you later, main abhi busy hoon\"
  Output: \"I will call you later, मैं अभी busy हूँ\""
    } else {
        "You are a professional voice dictation assistant. Your task is to refine and clean up the raw transcription. \
You must carefully process the text to ensure the correct script and language are preserved.

CRITICAL SCRIPT & TRANSLATION PRESERVATION RULES:
1. FILTER BACKGROUND NOISE: Ignore and remove any words or phrases that represent ambient noise, background chatter, crosstalk, or non-speech sounds. Do not include them in the refined text.
2. DO NOT TRANSLATE: Absolutely DO NOT translate non-English words to English. Keep non-English words in their native language.
3. NATIVE SCRIPT: Write all non-English words in their respective native script (e.g., Spanish words in Spanish/Latin script, Chinese in Chinese characters, Hindi in Devanagari script).
4. KEEP ENGLISH IN ENGLISH SCRIPT: Keep all English words and phrases in English (Latin script). Do NOT translate English words to the target language.
5. MIXED LANGUAGE HANDLING: For code-mixed text, keep each word/phrase in its native script.
6. REMOVE FILLER WORDS: Remove filler words (like 'um', 'uh', 'like', 'ah'), correct backtracking, and format into clean, readable text.
7. NO META-TEXT: Do not add any conversational responses, explanations, note, prefix, or suffix. Return ONLY the finalized refined text."
    };

    let lang_instruction = if language_name == "English" {
        "IMPORTANT: The text is in English. You must refine it strictly in English. Do NOT use any Hindi words, Devanagari script, Romanized Hindi, or non-English script. Translate or correct any non-English words to clean English.".to_string()
    } else if language_name == "Hindi" {
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
    let client = &*HTTP_CLIENT;
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

pub async fn transcribe_lm_studio(
    wav_path: &str,
    base_url: &str,
    language: &str,
) -> Result<String, String> {
    let client = &*HTTP_CLIENT;

    // Build the transcriptions endpoint from the base URL
    let mut endpoint = base_url.trim().trim_end_matches('/').to_string();
    if !endpoint.ends_with("/audio/transcriptions") {
        if endpoint.ends_with("/v1") {
            endpoint.push_str("/audio/transcriptions");
        } else {
            endpoint.push_str("/v1/audio/transcriptions");
        }
    }

    let file_bytes = fs::read(wav_path)
        .map_err(|e| format!("Failed to read recorded audio file: {}", e))?;

    let part = reqwest::multipart::Part::bytes(file_bytes)
        .file_name("audio.wav")
        .mime_str("audio/wav")
        .map_err(|e| format!("Failed to prepare audio multipart: {}", e))?;

    let mut form = reqwest::multipart::Form::new()
        .part("file", part)
        .text("model", "whisper-1".to_string());

    if language != "auto" {
        form = form.text("language", language.to_string());
    }

    let response = client
        .post(&endpoint)
        .multipart(form)
        .send()
        .await
        .map_err(|e| format!("LM Studio transcription request failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let err_text = response.text().await.unwrap_or_default();
        return Err(format!("LM Studio API returned error status {}: {}", status, err_text));
    }

    let json_resp: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse LM Studio response: {}", e))?;

    let transcribed_text = json_resp["text"]
        .as_str()
        .ok_or_else(|| {
            format!(
                "Unexpected LM Studio response structure. Response: {:?}",
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
    let client = &*HTTP_CLIENT;
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
        model, api_key
    );

    let language_name = get_language_name(language);
    let system_instruction = if language_name == "English" {
        "You are a professional voice dictation assistant. Your task is to refine and clean up the raw transcription. \
The spoken language is English. You must write everything strictly in English.

CRITICAL LANGUAGE RULES:
1. FILTER BACKGROUND NOISE: Ignore and remove any words or phrases that represent ambient noise, background chatter, crosstalk, or non-speech sounds. Do not include them in the refined text.
2. WRITE ONLY IN ENGLISH: The spoken audio/text is in English. You must write everything strictly in English. Do NOT mix other languages, and do NOT use Devanagari script, Romanized Hindi, or any non-English script.
3. TRANSLATE/CORRECT NON-ENGLISH: If there are any non-English words, phrases, or scripts in the raw input (due to transcription errors, noise, accents, or code-switching), you MUST translate them to English or correct them to fit the English context. The entire output must be 100% standard English.
4. REMOVE FILLER WORDS: Remove filler words (like 'um', 'uh', 'like', 'ah'), correct backtracking, and format into clean, readable text.
5. NO META-TEXT: Do not add any conversational responses, explanations, note, prefix, or suffix. Return ONLY the finalized refined text."
    } else if language_name == "Hindi" {
        "You are a professional voice dictation assistant. Your task is to refine and clean up the raw transcription. \
You must carefully process the text to ensure the correct script and language are preserved.

CRITICAL TRANSLITERATION, SCRIPT & TRANSLATION PRESERVATION RULES:
1. FILTER BACKGROUND NOISE: Ignore and remove any words or phrases that represent ambient noise, background chatter, crosstalk, or non-speech sounds. Do not include them in the refined text.
2. DETECT HINDI WORDS: Identify all Hindi words and phrases, even if they are written in Roman/Latin script (Hinglish / transliterated script, e.g., 'mera', 'naam', 'hai', 'kaise', 'ho', 'main', 'aur').
3. WRITE HINDI IN DEVANAGARI: You MUST convert all Hindi words/phrases to Devanagari script (e.g., convert 'mera' to 'मेरा', 'naam' to 'नाम', 'hai' to 'है', 'kaise' to 'कैसे', 'ho' to 'हो').
4. KEEP ENGLISH IN ENGLISH SCRIPT: Keep all English words and phrases in English (Latin script). For example, 'software engineer', 'developer', 'meeting', 'call you later' must remain in English script. Do NOT translate English words to Hindi.
5. STRICTLY PROHIBIT TRANSLATION: Do NOT translate Hindi words to English (e.g., do NOT translate 'mera naam' or 'मेरा नाम' to 'my name'). Keep them in their original language, but written in Devanagari script.
6. REMOVE FILLER WORDS: Remove filler words (like 'um', 'uh', 'like', 'ah'), correct backtracking, and format into clean, readable text.
7. NO META-TEXT: Do not add any conversational responses, explanations, note, prefix, or suffix. Return ONLY the finalized refined text.

EXAMPLES:
- Input: \"mera naam hemanshu hai and I am a software engineer\"
  Output: \"मेरा नाम हिमांशु है and I am a software engineer\"
- Input: \"aaj weather bahut achha hai main office ja raha hoon\"
  Output: \"आज weather बहुत अच्छा है मैं office जा रहा हूँ\"
- Input: \"hello team aaj ki meeting ka agenda kya hai\"
  Output: \"hello team आज की meeting का agenda क्या है\"
- Input: \"I will call you later, main abhi busy hoon\"
  Output: \"I will call you later, मैं अभी busy हूँ\""
    } else {
        "You are a professional voice dictation assistant. Your task is to refine and clean up the raw transcription. \
You must carefully process the text to ensure the correct script and language are preserved.

CRITICAL SCRIPT & TRANSLATION PRESERVATION RULES:
1. FILTER BACKGROUND NOISE: Ignore and remove any words or phrases that represent ambient noise, background chatter, crosstalk, or non-speech sounds. Do not include them in the refined text.
2. DO NOT TRANSLATE: Absolutely DO NOT translate non-English words to English. Keep non-English words in their native language.
3. NATIVE SCRIPT: Write all non-English words in their respective native script (e.g., Spanish words in Spanish/Latin script, Chinese in Chinese characters, Hindi in Devanagari script).
4. KEEP ENGLISH IN ENGLISH SCRIPT: Keep all English words and phrases in English (Latin script). Do NOT translate English words to the target language.
5. MIXED LANGUAGE HANDLING: For code-mixed text, keep each word/phrase in its native script.
6. REMOVE FILLER WORDS: Remove filler words (like 'um', 'uh', 'like', 'ah'), correct backtracking, and format into clean, readable text.
7. NO META-TEXT: Do not add any conversational responses, explanations, note, prefix, or suffix. Return ONLY the finalized refined text."
    };

    let lang_instruction = if language_name == "English" {
        "IMPORTANT: The text is in English. You must refine it strictly in English. Do NOT use any Hindi words, Devanagari script, Romanized Hindi, or non-English script. Translate or correct any non-English words to clean English.".to_string()
    } else if language_name == "Hindi" {
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
    #[cfg(windows)]
    command.creation_flags(0x08000000); // CREATE_NO_WINDOW
    command
        .arg(wav_path)
        .arg("--model")
        .arg(model)
        .arg("--output_dir")
        .arg(&parent_dir_str)
        .arg("--output_format")
        .arg("txt")
        .arg("--fp16")
        .arg("False")
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

pub async fn refine_with_openai_compatible(
    api_url: &str,
    api_key: &str,
    model: &str,
    prompt: &str,
    raw_text: &str,
    language: &str,
) -> Result<String, String> {
    let client = &*HTTP_CLIENT;
    
    // Normalize endpoint URL
    let mut endpoint = api_url.trim().to_string();
    if !endpoint.contains("/chat/completions") && !endpoint.contains("/completions") {
        if endpoint.ends_with('/') {
            endpoint.push_str("chat/completions");
        } else {
            endpoint.push_str("/chat/completions");
        }
    }

    let language_name = get_language_name(language);
    let system_instruction = if language_name == "English" {
        "You are a professional voice dictation assistant. Your task is to refine and clean up the raw transcription. \
The spoken language is English. You must write everything strictly in English.

CRITICAL LANGUAGE RULES:
1. FILTER BACKGROUND NOISE: Ignore and remove any words or phrases that represent ambient noise, background chatter, crosstalk, or non-speech sounds. Do not include them in the refined text.
2. WRITE ONLY IN ENGLISH: The spoken audio/text is in English. You must write everything strictly in English. Do NOT mix other languages, and do NOT use Devanagari script, Romanized Hindi, or any non-English script.
3. TRANSLATE/CORRECT NON-ENGLISH: If there are any non-English words, phrases, or scripts in the raw input (due to transcription errors, noise, accents, or code-switching), you MUST translate them to English or correct them to fit the English context. The entire output must be 100% standard English.
4. REMOVE FILLER WORDS: Remove filler words (like 'um', 'uh', 'like', 'ah'), correct backtracking, and format into clean, readable text.
5. NO META-TEXT: Do not add any conversational responses, explanations, note, prefix, or suffix. Return ONLY the finalized refined text."
    } else if language_name == "Hindi" {
        "You are a professional voice dictation assistant. Your task is to refine and clean up the raw transcription. \
You must carefully process the text to ensure the correct script and language are preserved.

CRITICAL TRANSLITERATION, SCRIPT & TRANSLATION PRESERVATION RULES:
1. FILTER BACKGROUND NOISE: Ignore and remove any words or phrases that represent ambient noise, background chatter, crosstalk, or non-speech sounds. Do not include them in the refined text.
2. DETECT HINDI WORDS: Identify all Hindi words and phrases, even if they are written in Roman/Latin script (Hinglish / transliterated script, e.g., 'mera', 'naam', 'hai', 'kaise', 'ho', 'main', 'aur').
3. WRITE HINDI IN DEVANAGARI: You MUST convert all Hindi words/phrases to Devanagari script (e.g., convert 'mera' to 'मेरा', 'naam' to 'नाम', 'hai' to 'है', 'kaise' to 'कैसे', 'ho' to 'हो').
4. KEEP ENGLISH IN ENGLISH SCRIPT: Keep all English words and phrases in English (Latin script). For example, 'software engineer', 'developer', 'meeting', 'call you later' must remain in English script. Do NOT translate English words to Hindi.
5. STRICTLY PROHIBIT TRANSLATION: Do NOT translate Hindi words to English (e.g., do NOT translate 'mera naam' or 'मेरा नाम' to 'my name'). Keep them in their original language, but written in Devanagari script.
6. REMOVE FILLER WORDS: Remove filler words (like 'um', 'uh', 'like', 'ah'), correct backtracking, and format into clean, readable text.
7. NO META-TEXT: Do not add any conversational responses, explanations, note, prefix, or suffix. Return ONLY the finalized refined text.

EXAMPLES:
- Input: \"mera naam hemanshu hai and I am a software engineer\"
  Output: \"मेरा नाम हिमांशु है and I am a software engineer\"
- Input: \"aaj weather bahut achha hai main office ja raha hoon\"
  Output: \"आज weather बहुत अच्छा है मैं office जा रहा हूँ\"
- Input: \"hello team aaj ki meeting ka agenda kya hai\"
  Output: \"hello team आज की meeting का agenda क्या है\"
- Input: \"I will call you later, main abhi busy hoon\"
  Output: \"I will call you later, मैं अभी busy हूँ\""
    } else {
        "You are a professional voice dictation assistant. Your task is to refine and clean up the raw transcription. \
You must carefully process the text to ensure the correct script and language are preserved.

CRITICAL SCRIPT & TRANSLATION PRESERVATION RULES:
1. FILTER BACKGROUND NOISE: Ignore and remove any words or phrases that represent ambient noise, background chatter, crosstalk, or non-speech sounds. Do not include them in the refined text.
2. DO NOT TRANSLATE: Absolutely DO NOT translate non-English words to English. Keep non-English words in their native language.
3. NATIVE SCRIPT: Write all non-English words in their respective native script (e.g., Spanish words in Spanish/Latin script, Chinese in Chinese characters, Hindi in Devanagari script).
4. KEEP ENGLISH IN ENGLISH SCRIPT: Keep all English words and phrases in English (Latin script). Do NOT translate English words to the target language.
5. MIXED LANGUAGE HANDLING: For code-mixed text, keep each word/phrase in its native script.
6. REMOVE FILLER WORDS: Remove filler words (like 'um', 'uh', 'like', 'ah'), correct backtracking, and format into clean, readable text.
7. NO META-TEXT: Do not add any conversational responses, explanations, note, prefix, or suffix. Return ONLY the finalized refined text."
    };

    let lang_instruction = if language_name == "English" {
        "IMPORTANT: The text is in English. You must refine it strictly in English. Do NOT use any Hindi words, Devanagari script, Romanized Hindi, or non-English script. Translate or correct any non-English words to clean English.".to_string()
    } else if language_name == "Hindi" {
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
        "messages": [
            {
                "role": "system",
                "content": system_instruction
            },
            {
                "role": "user",
                "content": combined_prompt
            }
        ],
        "temperature": 0.2
    });

    let mut req = client.post(&endpoint)
        .bearer_auth(api_key)
        .json(&request_body);

    if endpoint.contains("openrouter.ai") {
        req = req.header("HTTP-Referer", "https://github.com/hemanshum/Murmur")
                 .header("X-Title", "Murmur");
    }

    let response = req.send()
        .await
        .map_err(|e| format!("API request failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let err_text = response.text().await.unwrap_or_default();
        return Err(format!("API returned error status {}: {}", status, err_text));
    }

    let json_resp: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response JSON: {}", e))?;

    let refined_text = json_resp["choices"][0]["message"]["content"]
        .as_str()
        .ok_or_else(|| {
            format!(
                "Unexpected API response structure. Response: {:?}",
                json_resp
            )
        })?;

    Ok(refined_text.trim().to_string())
}

pub async fn transcribe_local_sherpa(
    app_handle: &AppHandle,
    wav_path: &str,
    model_type: &str, // "whisper" or "parakeet"
    model_id: &str,   // "whisper_tiny", "parakeet_v3", etc.
    language: &str,
) -> Result<String, String> {
    // 1. Find Python executable
    let python_cmd = find_python_cmd().ok_or_else(|| {
        "Python was not found on your system. Please install Python (3.8+) and make sure it is added to your PATH.".to_string()
    })?;

    // 2. Check if numpy and sherpa-onnx are installed and fully functional
    let mut check_cmd = tokio::process::Command::new(python_cmd);
    #[cfg(windows)]
    check_cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    let check_status = check_cmd
        .args(["-c", "import numpy, sherpa_onnx; sherpa_onnx.OfflineTransducerModelConfig"])
        .output()
        .await;

    let needs_install = match check_status {
        Ok(output) => !output.status.success(),
        Err(_) => true,
    };

    if needs_install {
        emit_status(app_handle, "Setting up Local Engine...");
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        emit_status(app_handle, "Installing dependencies...");
        
        let mut install_cmd = tokio::process::Command::new(python_cmd);
        #[cfg(windows)]
        install_cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
        let install_status = install_cmd
            .args(["-m", "pip", "install", "--user", "numpy", "sherpa-onnx", "--break-system-packages"])
            .output()
            .await
            .map_err(|e| format!("Failed to run pip install: {}", e))?;

        if !install_status.status.success() {
            let stderr = String::from_utf8_lossy(&install_status.stderr);
            emit_status(app_handle, "Install failed");
            return Err(format!("Failed to install Python dependencies (numpy, sherpa-onnx) via pip. Error: {}", stderr));
        }
        
        emit_status(app_handle, "Engine Ready");
    }

    // 3. Resolve paths
    let config_dir = app_handle
        .path()
        .app_config_dir()
        .map_err(|e| format!("Failed to get config directory: {}", e))?;
    let script_path = config_dir.join("transcribe_sherpa.py");
    let script_path_str = script_path.to_string_lossy().to_string();

    let model_dir = config_dir.join("models").join(model_id);
    if !model_dir.exists() {
        return Err(format!("Model files directory does not exist: {}. Please download the model first.", model_dir.display()));
    }
    let model_dir_str = model_dir.to_string_lossy().to_string();

    emit_status(app_handle, "Transcribing");

    // 4. Run python script
    let mut command = tokio::process::Command::new(python_cmd);
    #[cfg(windows)]
    command.creation_flags(0x08000000); // CREATE_NO_WINDOW
    command
        .arg(&script_path_str)
        .arg("--wav_path")
        .arg(wav_path)
        .arg("--model_type")
        .arg(model_type)
        .arg("--model_dir")
        .arg(&model_dir_str)
        .arg("--language")
        .arg(language);

    let output = command
        .output()
        .await
        .map_err(|e| format!("Failed to run transcription script: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Transcription script exited with error: {}", stderr));
    }

    let stdout_str = String::from_utf8_lossy(&output.stdout);
    
    let parsed: serde_json::Value = serde_json::from_str(&stdout_str)
        .map_err(|e| format!("Failed to parse transcription script output JSON: {}. Raw output: {}", e, stdout_str))?;

    if let Some(err_msg) = parsed["error"].as_str() {
        return Err(format!("Error from transcription engine: {}", err_msg));
    }

    let text = parsed["text"]
        .as_str()
        .ok_or_else(|| format!("Invalid response format from script. Raw output: {}", stdout_str))?;

    Ok(text.trim().to_string())
}

fn find_python_cmd() -> Option<&'static str> {
    #[cfg(windows)]
    use std::os::windows::process::CommandExt;
    for cmd in &["python", "python3", "py"] {
        let result = {
            let mut c = std::process::Command::new(cmd);
            #[cfg(windows)]
            c.creation_flags(0x08000000); // CREATE_NO_WINDOW
            c.arg("--version").output()
        };
        if let Ok(output) = result {
            if output.status.success() {
                return Some(cmd);
            }
        }
    }
    None
}

fn emit_status(app_handle: &AppHandle, status: &str) {
    if let Some(app_state) = app_handle.try_state::<crate::AppState>() {
        if let Ok(mut state_status) = app_state.status.lock() {
            *state_status = status.to_string();
        }
    }
    let _ = app_handle.emit("status-changed", status);
}

pub async fn refine_with_local_llm(
    app_handle: &AppHandle,
    model_id: &str,
    prompt: &str,
    raw_text: &str,
    language: &str,
    thinking: bool,
) -> Result<String, String> {
    // Resolve config dir and model path
    let config_dir = app_handle
        .path()
        .app_config_dir()
        .map_err(|e| format!("Failed to get config directory: {}", e))?;

    // 1. Ensure llama-cli binary is present (download on first use)
    let llama_cli = ensure_llama_cli(&config_dir, app_handle).await?;

    // 2. Find the .gguf file in the model directory
    let model_dir = config_dir.join("models").join(model_id);
    if !model_dir.exists() {
        return Err(format!(
            "Refinement model '{}' is not downloaded. Please download it from Settings > Models.",
            model_id
        ));
    }

    let gguf_file = std::fs::read_dir(&model_dir)
        .map_err(|e| format!("Failed to read model directory: {}", e))?
        .filter_map(|entry| entry.ok())
        .find(|entry| {
            entry.path().extension()
                .map(|ext| ext.to_string_lossy().to_lowercase() == "gguf")
                .unwrap_or(false)
        })
        .ok_or_else(|| format!("No .gguf model file found in {}", model_dir.display()))?;

    let model_path_str = gguf_file.path().to_string_lossy().to_string();

    emit_status(app_handle, "Refining");

    // 3. Build prompts — using system prompt and user prompt designed for chat/conversation mode
    let language_name = get_language_name(language);
    let system_prompt = if language_name == "English" {
        "You are an expert editor. Rewrite the transcript to be clear, clean, and grammatically correct. \
         Remove all filler words (uh, um, ah, like) and backtracking. \
         Write everything strictly in English. Do not translate. \
         Never reply with chat greetings, explanations, or notes. Return only the cleaned text."
            .to_string()
    } else if language_name == "Hindi" {
        "You are an expert editor. Rewrite the transcript to be clear, clean, and grammatically correct. \
         Remove all filler words (uh, um, ah, like) and backtracking. \
         Convert all Hindi words written in Roman script to Devanagari script. \
         Keep all English words in Latin script. Do not translate Hindi to English or English to Hindi. \
         Never reply with chat greetings, explanations, or notes. Return only the cleaned text."
            .to_string()
    } else {
        format!(
            "You are an expert editor. Rewrite the transcript to be clear, clean, and grammatically correct. \
             Remove all filler words (uh, um, ah, like) and backtracking. \
             Keep all original {} words in their native script. Keep English in Latin script. Do not translate. \
             Never reply with chat greetings, explanations, or notes. Return only the cleaned text.",
            language_name
        )
    };

    let user_prompt = if prompt.is_empty() {
        format!(
            "Clean up this text. Remove all filler words (like \"um\", \"uh\", \"ah\"), fix repetitions, and correct grammar and punctuation. Do not add any extra text or quotes around the response. Return only the cleaned text.\n\nText: \"{}\"",
            raw_text
        )
    } else {
        format!(
            "Clean up this text. Remove all filler words (like \"um\", \"uh\", \"ah\"), fix repetitions, and correct grammar and punctuation. Do not add any extra text or quotes around the response. Return only the cleaned text.\n\nUser Instruction: {}\n\nText: \"{}\"",
            prompt,
            raw_text
        )
    };

    // 4. Run llama-cli in chat/conversation mode (which formats prompt with model's native template)
    // -sys/--system-prompt  = sets the system prompt
    // -p/--prompt           = sets the user input
    // -st/--single-turn     = run for one turn then exit immediately (prevents interactive hang)
    // --no-display-prompt   = do not echo the prompt to stdout (leaves only generated response)
    let mut command = tokio::process::Command::new(&llama_cli);
    #[cfg(windows)]
    command.creation_flags(0x08000000);
    command
        .arg("--model").arg(&model_path_str)
        .arg("-sys").arg(&system_prompt)
        .arg("-p").arg(&user_prompt)
        .arg("--n-predict").arg("1024")
        .arg("--ctx-size").arg("4096")
        .arg("--temp").arg("0.2")
        .arg("--threads").arg(num_cpus_half().to_string())
        .arg("-st")
        .arg("--no-display-prompt")
        .stdin(std::process::Stdio::null())     // no stdin → can never go interactive
        .stderr(std::process::Stdio::null());   // discard verbose llama.cpp logs

    if !thinking {
        command.arg("--reasoning-budget").arg("0");
    }

    let output = tokio::time::timeout(
        std::time::Duration::from_secs(120),
        command.output(),
    )
    .await
    .map_err(|_| "llama-cli timed out after 120s".to_string())?
    .map_err(|e| format!("Failed to run llama-cli: {}", e))?;

    let stdout_raw = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr_raw = String::from_utf8_lossy(&output.stderr).to_string();

    if !output.status.success() {
        let diag = [stderr_raw.trim(), stdout_raw.trim()]
            .iter()
            .find(|s| !s.is_empty())
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("exit code {:?}", output.status.code()));
        return Err(format!("llama-cli failed: {}", diag));
    }

    eprintln!("[DEBUG] stdout_raw ({} bytes): {:?}", stdout_raw.len(), &stdout_raw[..stdout_raw.len().min(500)]);

    let mut result = stdout_raw.trim().to_string();

    // Strip <think>...</think> blocks if model emitted them
    if result.contains("<think>") {
        if let Some(s) = result.find("<think>") {
            if let Some(e) = result.find("</think>") {
                result = format!("{}{}", &result[..s], result[e + "</think>".len()..].trim());
            } else {
                result = result[..s].trim().to_string();
            }
        }
    }

    // Trim any trailing stop/special tokens and junk generated by LLM
    for token in &[
        "[end of text]",
        "<|endoftext|>",
        "<|im_end|>",
        "</s>",
        "[/INST]",
        "[INST]",
        "Raw transcript:",
    ] {
        if let Some(pos) = result.find(token) {
            result.truncate(pos);
        }
    }

    // Strip wrapping quotes (first pass)
    result = result.trim().to_string();
    if (result.starts_with('"') && result.ends_with('"')) || (result.starts_with('\'') && result.ends_with('\'')) {
        if result.len() >= 2 {
            result = result[1..result.len() - 1].trim().to_string();
        }
    }

    // Clean up common prefixes at the start of the output
    for prefix in &[
        "Cleaned text:",
        "Cleaned transcript:",
        "Refined text:",
        "Refined transcript:",
        "Here is the cleaned text:",
        "Here is the refined text:",
        "Here is the cleaned transcript:",
        "Here is the refined transcript:",
    ] {
        if result.to_lowercase().starts_with(&prefix.to_lowercase()) {
            result = result[prefix.len()..].trim().to_string();
        }
    }

    // Strip wrapping quotes (second pass, in case they were inside the prefix)
    result = result.trim().to_string();
    if (result.starts_with('"') && result.ends_with('"')) || (result.starts_with('\'') && result.ends_with('\'')) {
        if result.len() >= 2 {
            result = result[1..result.len() - 1].trim().to_string();
        }
    }

    // Strip trailing code-fence repetition (``` ``` ``` ...) Qwen3 sometimes emits
    let lines: Vec<&str> = result.lines().collect();
    let clean_lines: Vec<&str> = lines.iter()
        .rev()
        .skip_while(|l| l.trim() == "```" || l.trim().is_empty())
        .cloned()
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    result = clean_lines.join("\n");

    let final_result = result.trim().to_string();
    eprintln!("[DEBUG] Final result ({} chars): {:?}", final_result.len(), &final_result[..final_result.len().min(200)]);
    
    if final_result.is_empty() {
        eprintln!("[WARNING] Local LLM refinement returned empty result. Falling back to raw transcript.");
        Ok(raw_text.to_string())
    } else {
        Ok(final_result)
    }
}

fn num_cpus_half() -> usize {
    let cpus = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);
    (cpus / 2).max(1)
}

/// Downloads llama-cli + companion DLLs from official llama.cpp GitHub releases on first use.
/// Extracts everything into <config_dir>/llama_bin/ so the DLLs are alongside the binary.
async fn ensure_llama_cli(
    config_dir: &std::path::Path,
    app_handle: &AppHandle,
) -> Result<std::path::PathBuf, String> {
    #[cfg(windows)]
    let binary_name = "llama-cli.exe";
    #[cfg(not(windows))]
    let binary_name = "llama-cli";

    // Store everything in a dedicated subdirectory so DLLs stay next to the binary
    let bin_dir = config_dir.join("llama_bin");
    let bin_path = bin_dir.join(binary_name);

    // Remove any old single-file extraction that would be missing DLLs
    if config_dir.join(binary_name).exists() && !bin_path.exists() {
        let _ = std::fs::remove_file(config_dir.join(binary_name));
    }

    if bin_path.exists() {
        return Ok(bin_path);
    }

    emit_status(app_handle, "Downloading LLM engine...");

    #[cfg(windows)]
    let download_url = "https://github.com/ggml-org/llama.cpp/releases/download/b5618/llama-b5618-bin-win-cpu-x64.zip";
    #[cfg(not(windows))]
    let download_url = "https://github.com/ggml-org/llama.cpp/releases/download/b5618/llama-b5618-bin-ubuntu-x64.zip";

    let zip_path = config_dir.join("llama_cli_tmp.zip");

    // Download
    let response = reqwest::get(download_url)
        .await
        .map_err(|e| format!("Failed to download llama engine: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Failed to download llama engine: HTTP {}", response.status()));
    }

    let bytes = response.bytes()
        .await
        .map_err(|e| format!("Failed to read llama engine download: {}", e))?;

    std::fs::write(&zip_path, &bytes)
        .map_err(|e| format!("Failed to save llama engine zip: {}", e))?;

    // Extract ALL files from the zip into bin_dir (DLLs must be alongside the binary)
    let _ = std::fs::create_dir_all(&bin_dir);
    let zip_file = std::fs::File::open(&zip_path)
        .map_err(|e| format!("Failed to open zip: {}", e))?;
    let mut archive = zip::ZipArchive::new(zip_file)
        .map_err(|e| format!("Failed to read zip archive: {}", e))?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)
            .map_err(|e| format!("Zip read error: {}", e))?;

        if file.is_dir() {
            continue;
        }

        // Flatten the path — just use the filename, no subdirectory structure
        let file_name = std::path::Path::new(file.name())
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        if file_name.is_empty() {
            continue;
        }

        // Only extract executables and DLLs / shared libraries
        let ext = std::path::Path::new(&file_name)
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default();

        let is_relevant = matches!(ext.as_str(), "exe" | "dll" | "so" | "dylib" | "")
            || file_name == "llama-cli";

        if !is_relevant {
            continue;
        }

        let dest = bin_dir.join(&file_name);
        let mut out = std::fs::File::create(&dest)
            .map_err(|e| format!("Failed to create {}: {}", file_name, e))?;
        std::io::copy(&mut file, &mut out)
            .map_err(|e| format!("Failed to extract {}: {}", file_name, e))?;
    }

    // Cleanup zip
    let _ = std::fs::remove_file(&zip_path);

    if !bin_path.exists() {
        return Err(format!(
            "llama-cli binary not found after extraction. Expected: {}",
            bin_path.display()
        ));
    }

    // Make executable on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&bin_path)
            .map_err(|e| e.to_string())?
            .permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&bin_path, perms)
            .map_err(|e| e.to_string())?;
    }

    emit_status(app_handle, "LLM engine ready");
    Ok(bin_path)
}


