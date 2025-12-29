// DLP Settings Tauri Commands

use crate::dlp_pattern_config::DB_PATH;
use rusqlite::Connection;
use serde::Serialize;

#[derive(Serialize)]
pub struct DlpPattern {
    id: i64,
    name: String,
    pattern_type: String,
    patterns: Vec<String>,
    enabled: bool,
}

#[derive(Serialize)]
pub struct DlpSettings {
    api_keys_enabled: bool,
    custom_patterns: Vec<DlpPattern>,
}

#[tauri::command]
pub fn get_dlp_settings() -> Result<DlpSettings, String> {
    let conn = Connection::open(DB_PATH).map_err(|e| e.to_string())?;

    // Ensure tables exist
    let _ = conn.execute(
        "CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT NOT NULL)",
        [],
    );
    let _ = conn.execute(
        "CREATE TABLE IF NOT EXISTS dlp_patterns (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            pattern_type TEXT NOT NULL,
            patterns TEXT NOT NULL,
            enabled INTEGER DEFAULT 1,
            created_at TEXT NOT NULL
        )",
        [],
    );

    // Get built-in API keys setting
    let api_keys_enabled: bool = conn
        .query_row(
            "SELECT value FROM settings WHERE key = 'dlp_api_keys_enabled'",
            [],
            |row| row.get::<_, String>(0),
        )
        .ok()
        .map(|v| v == "1")
        .unwrap_or(false);

    // Get custom patterns
    let mut stmt = conn
        .prepare("SELECT id, name, pattern_type, patterns, enabled FROM dlp_patterns ORDER BY id")
        .map_err(|e| e.to_string())?;

    let patterns: Vec<DlpPattern> = stmt
        .query_map([], |row| {
            let patterns_json: String = row.get(3)?;
            let patterns: Vec<String> = serde_json::from_str(&patterns_json).unwrap_or_default();
            Ok(DlpPattern {
                id: row.get(0)?,
                name: row.get(1)?,
                pattern_type: row.get(2)?,
                patterns,
                enabled: row.get::<_, i32>(4)? == 1,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(DlpSettings {
        api_keys_enabled,
        custom_patterns: patterns,
    })
}

#[tauri::command]
pub fn set_dlp_builtin(key: String, enabled: bool) -> Result<(), String> {
    let conn = Connection::open(DB_PATH).map_err(|e| e.to_string())?;

    let setting_key = format!("dlp_{}_enabled", key);
    let value = if enabled { "1" } else { "0" };

    conn.execute(
        "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
        rusqlite::params![setting_key, value],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn add_dlp_pattern(
    name: String,
    pattern_type: String,
    patterns: Vec<String>,
) -> Result<i64, String> {
    if name.trim().is_empty() {
        return Err("Name is required".to_string());
    }
    if patterns.is_empty() {
        return Err("At least one pattern is required".to_string());
    }

    let conn = Connection::open(DB_PATH).map_err(|e| e.to_string())?;
    let patterns_json = serde_json::to_string(&patterns).map_err(|e| e.to_string())?;
    let created_at = chrono::Utc::now().to_rfc3339();

    conn.execute(
        "INSERT INTO dlp_patterns (name, pattern_type, patterns, enabled, created_at) VALUES (?1, ?2, ?3, 1, ?4)",
        rusqlite::params![name.trim(), pattern_type, patterns_json, created_at],
    )
    .map_err(|e| e.to_string())?;

    Ok(conn.last_insert_rowid())
}

#[tauri::command]
pub fn toggle_dlp_pattern(id: i64, enabled: bool) -> Result<(), String> {
    let conn = Connection::open(DB_PATH).map_err(|e| e.to_string())?;

    conn.execute(
        "UPDATE dlp_patterns SET enabled = ?1 WHERE id = ?2",
        rusqlite::params![enabled as i32, id],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn delete_dlp_pattern(id: i64) -> Result<(), String> {
    let conn = Connection::open(DB_PATH).map_err(|e| e.to_string())?;

    conn.execute(
        "DELETE FROM dlp_patterns WHERE id = ?1",
        rusqlite::params![id],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

#[derive(Serialize)]
pub struct DlpDetectionRecord {
    id: i64,
    request_id: i64,
    timestamp: String,
    pattern_name: String,
    pattern_type: String,
    original_value: String,
    placeholder: String,
    message_index: Option<i32>,
}

#[derive(Serialize)]
pub struct DlpStats {
    total_detections: i64,
    detections_by_pattern: Vec<PatternCount>,
    recent_detections: Vec<DlpDetectionRecord>,
}

#[derive(Serialize)]
pub struct PatternCount {
    pattern_name: String,
    count: i64,
}

#[tauri::command]
pub fn get_dlp_detection_stats(time_range: String) -> Result<DlpStats, String> {
    let conn = Connection::open(DB_PATH).map_err(|e| e.to_string())?;

    // Ensure table exists
    let _ = conn.execute(
        "CREATE TABLE IF NOT EXISTS dlp_detections (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            request_id INTEGER,
            timestamp TEXT NOT NULL,
            pattern_name TEXT NOT NULL,
            pattern_type TEXT NOT NULL,
            original_value TEXT NOT NULL,
            placeholder TEXT NOT NULL,
            message_index INTEGER,
            FOREIGN KEY (request_id) REFERENCES requests(id)
        )",
        [],
    );

    let hours = match time_range.as_str() {
        "1h" => 1,
        "6h" => 6,
        "1d" => 24,
        "7d" => 24 * 7,
        _ => 24,
    };
    let cutoff = chrono::Utc::now() - chrono::Duration::hours(hours);
    let cutoff_ts = cutoff.to_rfc3339();

    // Get total detections count
    let total_detections: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM dlp_detections WHERE timestamp >= ?1",
            [&cutoff_ts],
            |row| row.get(0),
        )
        .unwrap_or(0);

    // Get detections by pattern
    let mut stmt = conn
        .prepare(
            "SELECT pattern_name, COUNT(*) as count FROM dlp_detections
             WHERE timestamp >= ?1 GROUP BY pattern_name ORDER BY count DESC",
        )
        .map_err(|e| e.to_string())?;

    let detections_by_pattern: Vec<PatternCount> = stmt
        .query_map([&cutoff_ts], |row| {
            Ok(PatternCount {
                pattern_name: row.get(0)?,
                count: row.get(1)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    // Get recent detections
    let mut stmt = conn
        .prepare(
            "SELECT id, request_id, timestamp, pattern_name, pattern_type, original_value, placeholder, message_index
             FROM dlp_detections WHERE timestamp >= ?1 ORDER BY id DESC LIMIT 50",
        )
        .map_err(|e| e.to_string())?;

    let recent_detections: Vec<DlpDetectionRecord> = stmt
        .query_map([&cutoff_ts], |row| {
            Ok(DlpDetectionRecord {
                id: row.get(0)?,
                request_id: row.get(1)?,
                timestamp: row.get(2)?,
                pattern_name: row.get(3)?,
                pattern_type: row.get(4)?,
                original_value: row.get(5)?,
                placeholder: row.get(6)?,
                message_index: row.get(7)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(DlpStats {
        total_detections,
        detections_by_pattern,
        recent_detections,
    })
}
