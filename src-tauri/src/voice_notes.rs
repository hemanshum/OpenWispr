use rusqlite::{Connection, Result as SqliteResult};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct VoiceNote {
    pub id: i64,
    pub title: String,
    pub transcription: String,
    pub audio_path: String,
    pub duration_seconds: Option<i32>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Deserialize)]
pub struct CreateVoiceNoteRequest {
    pub title: String,
    pub transcription: String,
    pub audio_source_path: String,
}

/// Initialize the voice_notes table in the database
pub fn init_voice_notes_table(conn: &Connection) -> SqliteResult<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS voice_notes (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            title TEXT NOT NULL,
            transcription TEXT NOT NULL,
            audio_path TEXT NOT NULL,
            duration_seconds INTEGER,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        )",
        [],
    )?;
    Ok(())
}

/// Get the directory where voice note audio files are stored
fn get_voice_notes_dir(app_handle: &AppHandle) -> PathBuf {
    let mut path = app_handle.path().app_data_dir().unwrap_or_else(|_| PathBuf::from("."));
    path.push("voice_notes");
    let _ = fs::create_dir_all(&path);
    path
}

/// Get the path for a specific voice note's audio file
fn get_audio_file_path(app_handle: &AppHandle, note_id: i64) -> PathBuf {
    let mut path = get_voice_notes_dir(app_handle);
    path.push(format!("vn_{}.wav", note_id));
    path
}

/// Create a new voice note from a recording
#[tauri::command]
pub fn create_voice_note(
    app_handle: AppHandle,
    title: String,
    transcription: String,
    audio_source_path: String,
) -> Result<VoiceNote, String> {
    // Get database connection
    let db_path = {
        let mut path = app_handle
            .path()
            .app_data_dir()
            .map_err(|e| format!("Failed to get app data dir: {}", e))?;
        path.push("history.db");
        path
    };

    let conn = Connection::open(&db_path).map_err(|e| format!("Failed to open database: {}", e))?;

    // Insert into database first to get the ID
    let now = chrono::Local::now().to_rfc3339();
    conn.execute(
        "INSERT INTO voice_notes (title, transcription, audio_path, duration_seconds, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        (&title, &transcription, "", None::<i32>, &now, &now),
    )
    .map_err(|e| format!("Failed to insert voice note: {}", e))?;

    let note_id = conn.last_insert_rowid();

    // Now copy the audio file to the voice_notes directory
    let dest_path = get_audio_file_path(&app_handle, note_id);
    let dest_path_str = dest_path.to_string_lossy().to_string();

    fs::copy(&audio_source_path, &dest_path).map_err(|e| {
        // Clean up database entry if file copy fails
        let _ = conn.execute("DELETE FROM voice_notes WHERE id = ?1", [note_id]);
        format!("Failed to copy audio file: {}", e)
    })?;

    // Update the audio_path in database
    conn.execute(
        "UPDATE voice_notes SET audio_path = ?1 WHERE id = ?2",
        (&dest_path_str, note_id),
    )
    .map_err(|e| format!("Failed to update audio path: {}", e))?;

    Ok(VoiceNote {
        id: note_id,
        title,
        transcription,
        audio_path: dest_path_str,
        duration_seconds: None,
        created_at: now.clone(),
        updated_at: now,
    })
}

/// Get all voice notes
#[tauri::command]
pub fn get_voice_notes(app_handle: AppHandle) -> Result<Vec<VoiceNote>, String> {
    let db_path = {
        let mut path = app_handle
            .path()
            .app_data_dir()
            .map_err(|e| format!("Failed to get app data dir: {}", e))?;
        path.push("history.db");
        path
    };

    let conn = Connection::open(&db_path).map_err(|e| format!("Failed to open database: {}", e))?;

    let mut stmt = conn
        .prepare(
            "SELECT id, title, transcription, audio_path, duration_seconds, created_at, updated_at
             FROM voice_notes
             ORDER BY created_at DESC",
        )
        .map_err(|e| format!("Failed to prepare statement: {}", e))?;

    let notes = stmt
        .query_map([], |row| {
            Ok(VoiceNote {
                id: row.get(0)?,
                title: row.get(1)?,
                transcription: row.get(2)?,
                audio_path: row.get(3)?,
                duration_seconds: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            })
        })
        .map_err(|e| format!("Failed to query voice notes: {}", e))?
        .collect::<SqliteResult<Vec<_>>>()
        .map_err(|e| format!("Failed to collect voice notes: {}", e))?;

    Ok(notes)
}

/// Get a single voice note by ID
#[tauri::command]
pub fn get_voice_note(app_handle: AppHandle, id: i64) -> Result<VoiceNote, String> {
    let db_path = {
        let mut path = app_handle
            .path()
            .app_data_dir()
            .map_err(|e| format!("Failed to get app data dir: {}", e))?;
        path.push("history.db");
        path
    };

    let conn = Connection::open(&db_path).map_err(|e| format!("Failed to open database: {}", e))?;

    conn.query_row(
        "SELECT id, title, transcription, audio_path, duration_seconds, created_at, updated_at
         FROM voice_notes WHERE id = ?1",
        [id],
        |row| {
            Ok(VoiceNote {
                id: row.get(0)?,
                title: row.get(1)?,
                transcription: row.get(2)?,
                audio_path: row.get(3)?,
                duration_seconds: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            })
        },
    )
    .map_err(|e| format!("Failed to get voice note: {}", e))
}

/// Update a voice note's title and transcription
#[tauri::command]
pub fn update_voice_note(
    app_handle: AppHandle,
    id: i64,
    title: String,
    transcription: String,
) -> Result<VoiceNote, String> {
    let db_path = {
        let mut path = app_handle
            .path()
            .app_data_dir()
            .map_err(|e| format!("Failed to get app data dir: {}", e))?;
        path.push("history.db");
        path
    };

    let conn = Connection::open(&db_path).map_err(|e| format!("Failed to open database: {}", e))?;

    let now = chrono::Local::now().to_rfc3339();

    conn.execute(
        "UPDATE voice_notes SET title = ?1, transcription = ?2, updated_at = ?3 WHERE id = ?4",
        (&title, &transcription, &now, id),
    )
    .map_err(|e| format!("Failed to update voice note: {}", e))?;

    // Return the updated note
    get_voice_note(app_handle, id)
}

/// Delete a voice note and its audio file
#[tauri::command]
pub fn delete_voice_note(app_handle: AppHandle, id: i64) -> Result<(), String> {
    let db_path = {
        let mut path = app_handle
            .path()
            .app_data_dir()
            .map_err(|e| format!("Failed to get app data dir: {}", e))?;
        path.push("history.db");
        path
    };

    let conn = Connection::open(&db_path).map_err(|e| format!("Failed to open database: {}", e))?;

    // Get the audio path before deleting
    let audio_path: String = conn
        .query_row(
            "SELECT audio_path FROM voice_notes WHERE id = ?1",
            [id],
            |row| row.get(0),
        )
        .map_err(|e| format!("Failed to get audio path: {}", e))?;

    // Delete from database
    conn.execute("DELETE FROM voice_notes WHERE id = ?1", [id])
        .map_err(|e| format!("Failed to delete voice note: {}", e))?;

    // Delete the audio file
    let _ = fs::remove_file(&audio_path);

    Ok(())
}

/// Get the file URL for a voice note's audio file
#[tauri::command]
pub fn get_audio_file_url(app_handle: AppHandle, id: i64) -> Result<String, String> {
    let path = get_audio_file_path(&app_handle, id);
    if !path.exists() {
        return Err("Audio file not found".to_string());
    }
    let url = format!("file://{}", path.to_string_lossy());
    Ok(url)
}
