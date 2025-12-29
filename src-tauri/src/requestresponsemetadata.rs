// Common Request and Response Metadata Structures
// These are the normalized formats that all backends produce

/// Metadata extracted from API requests
#[derive(Default, Clone)]
pub struct RequestMetadata {
    pub model: Option<String>,
    pub has_system_prompt: bool,
    pub has_tools: bool,
    pub user_message_count: i32,
    pub assistant_message_count: i32,
}

/// Metadata extracted from API responses
#[derive(Default, Clone)]
pub struct ResponseMetadata {
    pub input_tokens: i32,
    pub output_tokens: i32,
    pub cache_read_tokens: i32,
    pub cache_creation_tokens: i32,
    pub stop_reason: Option<String>,
    pub has_thinking: bool,
}
