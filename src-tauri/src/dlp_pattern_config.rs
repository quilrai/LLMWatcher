// DLP Pattern Configuration and Constants

use std::env;
use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;

pub const DEFAULT_PORT: u16 = 8008;

static DB_PATH: OnceLock<String> = OnceLock::new();

/// Returns the path to the database file at ~/.quilrdlpapp/proxy_requests.db
/// Creates the directory if it doesn't exist.
pub fn get_db_path() -> &'static str {
    DB_PATH.get_or_init(|| {
        let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let dir = PathBuf::from(&home).join(".quilrdlpapp");

        // Create directory if it doesn't exist
        if !dir.exists() {
            fs::create_dir_all(&dir).expect("Failed to create ~/.quilrdlpapp directory");
        }

        dir.join("proxy_requests.db").to_string_lossy().to_string()
    })
}
