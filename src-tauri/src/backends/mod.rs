// Backend trait and implementations

pub mod claude;
pub mod codex;
pub mod custom;

use axum::http::HeaderMap;
use crate::requestresponsemetadata::{RequestMetadata, ResponseMetadata};

/// Trait for API backend implementations
/// Each backend (Claude, OpenAI, Gemini, etc.) implements this trait
pub trait Backend: Send + Sync {
    /// Returns the backend name (e.g., "claude", "openai", "gemini")
    fn name(&self) -> &str;

    /// Returns the base URL for this backend's API
    fn base_url(&self) -> &str;

    /// Parse request body to extract metadata
    fn parse_request_metadata(&self, body: &str) -> RequestMetadata;

    /// Parse response body to extract metadata
    fn parse_response_metadata(&self, body: &str, is_streaming: bool) -> ResponseMetadata;

    /// Determine if this request should be logged
    /// (e.g., only log Messages API calls, not token counting)
    fn should_log(&self, body: &str) -> bool;

    /// Extract backend-specific metadata as JSON string
    /// This is stored in the extra_metadata column for flexible, backend-specific data
    /// Default implementation returns None (no extra metadata)
    fn extract_extra_metadata(
        &self,
        _request_body: &str,
        _response_body: &str,
        _headers: &HeaderMap,
    ) -> Option<String> {
        None
    }
}

// Re-export backends for convenience
pub use claude::ClaudeBackend;
pub use codex::CodexBackend;
pub use custom::CustomBackend;
