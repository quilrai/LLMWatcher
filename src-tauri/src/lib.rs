// DLP Demo App - Main Library
//
// A Tauri app that proxies LLM API requests with DLP (Data Loss Prevention) capabilities.
// Currently supports Claude (Anthropic), with plans for OpenAI, Gemini, etc.

mod backends;
pub mod ca;
mod commands;
mod cursor_proto;
mod database;
mod dlp;
mod dlp_pattern_config;
mod mitm_proxy;
mod proxy;
mod requestresponsemetadata;

use database::{get_mitm_port_from_db, get_port_from_db};
use dlp_pattern_config::{DEFAULT_MITM_PORT, DEFAULT_PORT};
use std::sync::{Arc, Mutex};
use tokio::sync::watch;

// Global state for reverse proxy control
pub static PROXY_PORT: std::sync::LazyLock<Arc<Mutex<u16>>> =
    std::sync::LazyLock::new(|| Arc::new(Mutex::new(DEFAULT_PORT)));
pub static RESTART_SENDER: std::sync::LazyLock<Arc<Mutex<Option<watch::Sender<bool>>>>> =
    std::sync::LazyLock::new(|| Arc::new(Mutex::new(None)));

// Global state for MITM proxy control
pub static MITM_PROXY_PORT: std::sync::LazyLock<Arc<Mutex<u16>>> =
    std::sync::LazyLock::new(|| Arc::new(Mutex::new(DEFAULT_MITM_PORT)));
pub static MITM_RESTART_SENDER: std::sync::LazyLock<Arc<Mutex<Option<watch::Sender<bool>>>>> =
    std::sync::LazyLock::new(|| Arc::new(Mutex::new(None)));

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize reverse proxy port from database
    {
        let port = get_port_from_db();
        let mut current_port = PROXY_PORT.lock().unwrap();
        *current_port = port;
    }

    // Initialize MITM proxy port from database
    {
        let port = get_mitm_port_from_db();
        let mut current_port = MITM_PROXY_PORT.lock().unwrap();
        *current_port = port;
    }

    // Spawn reverse proxy server
    std::thread::spawn(|| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(proxy::start_proxy_server());
    });

    // Spawn MITM proxy server
    std::thread::spawn(|| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(mitm_proxy::start_mitm_proxy());
    });

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::greet,
            commands::get_dashboard_stats,
            commands::get_backends,
            commands::get_message_logs,
            commands::get_port_setting,
            commands::save_port_setting,
            commands::restart_proxy,
            commands::get_dlp_settings,
            commands::set_dlp_builtin,
            commands::add_dlp_pattern,
            commands::toggle_dlp_pattern,
            commands::delete_dlp_pattern,
            commands::get_dlp_detection_stats,
            commands::set_shell_env,
            commands::check_shell_env,
            commands::remove_shell_env,
            commands::get_mitm_port_setting,
            commands::save_mitm_port_setting,
            commands::restart_mitm_proxy,
            commands::get_ca_cert_path,
            commands::get_ca_cert_content,
            commands::export_ca_cert,
            commands::ca_exists,
            commands::open_ca_cert
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
