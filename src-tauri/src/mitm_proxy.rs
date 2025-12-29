// MITM HTTP Proxy Handler
// Intercepts HTTPS traffic for DLP inspection using hudsucker

use crate::ca::get_or_generate_ca;
use crate::cursor_proto;
use crate::database::Database;
use crate::dlp_pattern_config::DB_PATH;
use crate::{MITM_PROXY_PORT, MITM_RESTART_SENDER};

use http_body_util::{BodyExt, Full};
use hudsucker::{
    certificate_authority::RcgenAuthority,
    hyper::{Request, Response},
    rcgen::{Issuer, KeyPair},
    rustls::crypto::aws_lc_rs,
    Body, HttpContext, HttpHandler, Proxy, RequestOrResponse,
};
use std::net::SocketAddr;
use tokio::sync::watch;

/// Domains to intercept TLS for
const INTERCEPT_DOMAINS: &[&str] = &[
    "api.anthropic.com",
    "api.openai.com",
    "api.cursor.sh",
    "api2.cursor.sh",
    "api3.cursor.sh",
];

/// Endpoints to log/monitor (AI-related endpoints)
const MONITORED_ENDPOINTS: &[&str] = &[
    // AI Service endpoints (where chat content appears)
    "/aiserver.v1.AiService/",
    // Chat Service endpoints
    "/aiserver.v1.ChatService/",
    // CmdK endpoint
    "/aiserver.v1.CmdKService/",
];

/// Endpoints to skip (noisy, no user content)
const SKIP_ENDPOINTS: &[&str] = &[
    "/AnalyticsService/",
    "/DashboardService/",
    "/tev1/",
    "/auth/",
    "/updates/",
    "/extensions-control",
    "CheckNumberConfig",
    "CheckFeaturesStatus",
    "AvailableModels",
    "AvailableDocs",
    "ServerTime",
    "GetDefaultModel",
    "KnowledgeBaseList",
    "BootstrapStatsig",
    "ServerConfig",
    "CppEditHistoryStatus",
    "CheckQueuePosition",
    "GetDefaultModelNudgeData",
];

/// Check if a host should have TLS intercepted
fn should_intercept(host: &str) -> bool {
    INTERCEPT_DOMAINS.iter().any(|d| host.contains(d))
}

/// Check if a URI should be logged/monitored
fn should_log_endpoint(uri: &str) -> bool {
    // Skip noisy endpoints
    if SKIP_ENDPOINTS.iter().any(|e| uri.contains(e)) {
        return false;
    }
    // Log AI-related endpoints
    MONITORED_ENDPOINTS.iter().any(|e| uri.contains(e))
}

/// Format request/response body - handles JSON, protobuf, and binary
fn format_body_bytes(body: &[u8]) -> String {
    if body.is_empty() {
        return "(empty)".to_string();
    }

    // Try to parse as UTF-8 JSON first
    if let Ok(text) = std::str::from_utf8(body) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(text) {
            return serde_json::to_string_pretty(&json).unwrap_or_else(|_| text.to_string());
        }
    }

    // Try protobuf decoding (before falling back to text)
    // This is important because protobuf with string fields looks like valid UTF-8
    let proto_result = cursor_proto::decode_and_format(body);

    // If protobuf decoded to something meaningful (not Binary), use it
    if !proto_result.starts_with("[Binary:") {
        return proto_result;
    }

    // Fall back to text display if it's mostly printable UTF-8
    if let Ok(text) = std::str::from_utf8(body) {
        let printable = text.chars().filter(|c| !c.is_control() || *c == '\n').count();
        if printable > text.len() * 9 / 10 {
            if text.len() > 1000 {
                return format!("[Raw text] {}... (truncated, {} bytes)", &text[..1000], text.len());
            }
            return format!("[Raw text] {}", text);
        }
    }

    // Return the Binary result
    proto_result
}

#[derive(Clone)]
pub struct DlpHttpHandler {
    #[allow(dead_code)]
    db: Database,
}

impl DlpHttpHandler {
    pub fn new(db: Database) -> Self {
        Self { db }
    }
}

impl HttpHandler for DlpHttpHandler {
    async fn handle_request(
        &mut self,
        _ctx: &HttpContext,
        req: Request<Body>,
    ) -> RequestOrResponse {
        let method = req.method().to_string();
        let uri = req.uri().to_string();
        let host = req
            .headers()
            .get("host")
            .and_then(|h| h.to_str().ok())
            .unwrap_or("unknown")
            .to_string();

        // Check if this is a CONNECT request (just tunnel setup, no body)
        if method == "CONNECT" {
            // Silent pass-through for CONNECT requests
            return req.into();
        }

        // Only process monitored endpoints on intercepted domains
        if should_intercept(&host) && should_log_endpoint(&uri) {
            let (parts, body) = req.into_parts();

            // Collect the body
            let body_bytes = match body.collect().await {
                Ok(collected) => collected.to_bytes(),
                Err(e) => {
                    println!("[MITM] Failed to read request body: {}", e);
                    return RequestOrResponse::Request(Request::from_parts(parts, Body::empty()));
                }
            };

            println!("\n[MITM] ┌─────────────────── REQUEST ───────────────────┐");
            println!("[MITM] │ {} {}", method, uri);
            println!("[MITM] │ Host: {}", host);
            println!("[MITM] ├─────────────────── BODY ──────────────────────┤");
            for line in format_body_bytes(&body_bytes).lines() {
                println!("[MITM] │ {}", line);
            }
            println!("[MITM] └────────────────────────────────────────────────┘\n");

            // Recreate the body and return
            let new_body = Body::from(Full::new(body_bytes));
            return RequestOrResponse::Request(Request::from_parts(parts, new_body));
        }

        // All other requests pass through silently
        req.into()
    }

    async fn handle_response(&mut self, _ctx: &HttpContext, res: Response<Body>) -> Response<Body> {
        let status = res.status();

        // Check content-type to decide if we should log
        let content_type = res
            .headers()
            .get("content-type")
            .and_then(|h| h.to_str().ok())
            .unwrap_or("")
            .to_string();

        // Log responses with proto/grpc/connect/SSE content types (AI responses)
        let is_ai_response = content_type.contains("proto")
            || content_type.contains("grpc")
            || content_type.contains("connect")
            || content_type.contains("event-stream"); // SSE for StreamUnifiedChatWithToolsSSE

        if is_ai_response {
            let (parts, body) = res.into_parts();

            // Collect the body
            let body_bytes = match body.collect().await {
                Ok(collected) => collected.to_bytes(),
                Err(e) => {
                    println!("[MITM] Failed to read response body: {}", e);
                    return Response::from_parts(parts, Body::empty());
                }
            };

            println!("\n[MITM] ┌─────────────────── RESPONSE ──────────────────┐");
            println!("[MITM] │ Status: {} | Content-Type: {}", status, content_type);
            println!("[MITM] ├─────────────────── BODY ──────────────────────┤");
            for line in format_body_bytes(&body_bytes).lines() {
                println!("[MITM] │ {}", line);
            }
            println!("[MITM] └────────────────────────────────────────────────┘\n");

            // Recreate the body and return
            let new_body = Body::from(Full::new(body_bytes));
            return Response::from_parts(parts, new_body);
        }

        res
    }
}

/// Start the MITM proxy server
pub async fn start_mitm_proxy() {
    loop {
        let port = *MITM_PROXY_PORT.lock().unwrap();

        println!("[MITM] Starting MITM proxy on port {}...", port);

        // Get or generate CA certificate
        let (ca_cert_pem, ca_key_pem) = match get_or_generate_ca() {
            Ok((cert, key)) => (cert, key),
            Err(e) => {
                eprintln!("[MITM] Failed to get CA certificate: {}", e);
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                continue;
            }
        };

        // Parse CA key pair
        let key_pair = match KeyPair::from_pem(&ca_key_pem) {
            Ok(kp) => kp,
            Err(e) => {
                eprintln!("[MITM] Failed to parse CA key: {}", e);
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                continue;
            }
        };

        // Create issuer from CA cert
        let issuer = match Issuer::from_ca_cert_pem(&ca_cert_pem, key_pair) {
            Ok(issuer) => issuer,
            Err(e) => {
                eprintln!("[MITM] Failed to create issuer: {}", e);
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                continue;
            }
        };

        // Create certificate authority
        let ca = RcgenAuthority::new(issuer, 1000, aws_lc_rs::default_provider());

        // Initialize database
        let db = match Database::new(DB_PATH) {
            Ok(db) => db,
            Err(e) => {
                eprintln!("[MITM] Failed to initialize database: {}", e);
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                continue;
            }
        };

        // Create handler
        let handler = DlpHttpHandler::new(db);

        // Create shutdown channel
        let (tx, mut rx) = watch::channel(false);
        {
            let mut sender = MITM_RESTART_SENDER.lock().unwrap();
            *sender = Some(tx);
        }

        // Build proxy
        let proxy = match Proxy::builder()
            .with_addr(SocketAddr::from(([0, 0, 0, 0], port)))
            .with_ca(ca)
            .with_rustls_connector(aws_lc_rs::default_provider())
            .with_http_handler(handler)
            .with_graceful_shutdown(async move {
                loop {
                    rx.changed().await.ok();
                    if *rx.borrow() {
                        println!("[MITM] Received shutdown signal");
                        break;
                    }
                }
            })
            .build()
        {
            Ok(proxy) => proxy,
            Err(e) => {
                eprintln!("[MITM] Failed to build proxy: {}", e);
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                continue;
            }
        };

        println!("[MITM] MITM Proxy running on http://0.0.0.0:{}", port);
        println!(
            "[MITM] Configure Cursor with HTTP_PROXY=http://127.0.0.1:{}",
            port
        );

        // Run proxy
        if let Err(e) = proxy.start().await {
            eprintln!("[MITM] Proxy error: {}", e);
        }

        println!("[MITM] Proxy stopped, restarting...");
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    }
}
