// Database operations and schema management

use crate::dlp::DlpDetection;
use crate::dlp_pattern_config::{DB_PATH, DEFAULT_MITM_PORT, DEFAULT_PORT};
use crate::requestresponsemetadata::{RequestMetadata, ResponseMetadata};
use rusqlite::Connection;
use std::sync::{Arc, Mutex};

/// Thread-safe database wrapper
#[derive(Clone)]
pub struct Database {
    conn: Arc<Mutex<Connection>>,
}

impl Database {
    pub fn new(path: &str) -> Result<Self, rusqlite::Error> {
        let conn = Connection::open(path)?;

        // Create requests table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS requests (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp TEXT NOT NULL,
                backend TEXT NOT NULL DEFAULT 'claude',
                endpoint_name TEXT NOT NULL,
                method TEXT NOT NULL,
                path TEXT NOT NULL,
                model TEXT,
                input_tokens INTEGER DEFAULT 0,
                output_tokens INTEGER DEFAULT 0,
                cache_read_tokens INTEGER DEFAULT 0,
                cache_creation_tokens INTEGER DEFAULT 0,
                latency_ms INTEGER DEFAULT 0,
                has_system_prompt INTEGER DEFAULT 0,
                has_tools INTEGER DEFAULT 0,
                has_thinking INTEGER DEFAULT 0,
                stop_reason TEXT,
                user_message_count INTEGER DEFAULT 0,
                assistant_message_count INTEGER DEFAULT 0,
                response_status INTEGER,
                is_streaming INTEGER NOT NULL DEFAULT 0,
                request_body TEXT,
                response_body TEXT,
                extra_metadata TEXT
            )",
            [],
        )?;

        // Migration: Add extra_metadata column if it doesn't exist (for existing databases)
        let _ = conn.execute(
            "ALTER TABLE requests ADD COLUMN extra_metadata TEXT",
            [],
        );

        // Create settings table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            )",
            [],
        )?;

        // Create DLP patterns table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS dlp_patterns (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                pattern_type TEXT NOT NULL,
                patterns TEXT NOT NULL,
                enabled INTEGER DEFAULT 1,
                created_at TEXT NOT NULL
            )",
            [],
        )?;

        // Create DLP detections table
        conn.execute(
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
        )?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Clean up data older than 7 days
    pub fn cleanup_old_data(&self) -> Result<usize, rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        let cutoff = chrono::Utc::now() - chrono::Duration::days(7);
        let cutoff_ts = cutoff.to_rfc3339();

        conn.execute(
            "DELETE FROM requests WHERE timestamp < ?1",
            rusqlite::params![cutoff_ts],
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn log_request(
        &self,
        backend: &str,
        method: &str,
        path: &str,
        endpoint_name: &str,
        request_body: &str,
        response_body: &str,
        response_status: u16,
        is_streaming: bool,
        latency_ms: u64,
        req_meta: &RequestMetadata,
        resp_meta: &ResponseMetadata,
        extra_metadata: Option<&str>,
    ) -> Result<i64, rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        let timestamp = chrono::Utc::now().to_rfc3339();

        conn.execute(
            "INSERT INTO requests (
                timestamp, backend, endpoint_name, method, path, model,
                input_tokens, output_tokens, cache_read_tokens, cache_creation_tokens,
                latency_ms, has_system_prompt, has_tools, has_thinking, stop_reason,
                user_message_count, assistant_message_count,
                response_status, is_streaming, request_body, response_body, extra_metadata
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22)",
            rusqlite::params![
                timestamp,
                backend,
                endpoint_name,
                method,
                path,
                req_meta.model,
                resp_meta.input_tokens,
                resp_meta.output_tokens,
                resp_meta.cache_read_tokens,
                resp_meta.cache_creation_tokens,
                latency_ms as i64,
                req_meta.has_system_prompt as i32,
                req_meta.has_tools as i32,
                resp_meta.has_thinking as i32,
                resp_meta.stop_reason,
                req_meta.user_message_count,
                req_meta.assistant_message_count,
                response_status,
                is_streaming as i32,
                request_body,
                response_body,
                extra_metadata,
            ],
        )?;

        Ok(conn.last_insert_rowid())
    }

    pub fn log_dlp_detections(
        &self,
        request_id: i64,
        detections: &[DlpDetection],
    ) -> Result<(), rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        let timestamp = chrono::Utc::now().to_rfc3339();

        for detection in detections {
            conn.execute(
                "INSERT INTO dlp_detections (request_id, timestamp, pattern_name, pattern_type, original_value, placeholder, message_index)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                rusqlite::params![
                    request_id,
                    timestamp,
                    detection.pattern_name,
                    detection.pattern_type,
                    detection.original_value,
                    detection.placeholder,
                    detection.message_index,
                ],
            )?;
        }

        Ok(())
    }
}

// Port management helpers

pub fn get_port_from_db() -> u16 {
    let conn = match Connection::open(DB_PATH) {
        Ok(c) => c,
        Err(_) => return DEFAULT_PORT,
    };

    // Ensure settings table exists
    let _ = conn.execute(
        "CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT NOT NULL)",
        [],
    );

    conn.query_row(
        "SELECT value FROM settings WHERE key = 'proxy_port'",
        [],
        |row| row.get::<_, String>(0),
    )
    .ok()
    .and_then(|v| v.parse().ok())
    .unwrap_or(DEFAULT_PORT)
}

pub fn save_port_to_db(port: u16) -> Result<(), String> {
    let conn = Connection::open(DB_PATH).map_err(|e| e.to_string())?;

    conn.execute(
        "INSERT OR REPLACE INTO settings (key, value) VALUES ('proxy_port', ?1)",
        rusqlite::params![port.to_string()],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

// MITM Port management helpers

pub fn get_mitm_port_from_db() -> u16 {
    let conn = match Connection::open(DB_PATH) {
        Ok(c) => c,
        Err(_) => return DEFAULT_MITM_PORT,
    };

    // Ensure settings table exists
    let _ = conn.execute(
        "CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT NOT NULL)",
        [],
    );

    conn.query_row(
        "SELECT value FROM settings WHERE key = 'mitm_proxy_port'",
        [],
        |row| row.get::<_, String>(0),
    )
    .ok()
    .and_then(|v| v.parse().ok())
    .unwrap_or(DEFAULT_MITM_PORT)
}

pub fn save_mitm_port_to_db(port: u16) -> Result<(), String> {
    let conn = Connection::open(DB_PATH).map_err(|e| e.to_string())?;

    conn.execute(
        "INSERT OR REPLACE INTO settings (key, value) VALUES ('mitm_proxy_port', ?1)",
        rusqlite::params![port.to_string()],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}
