// Cursor Hooks API Handlers
//
// Implements endpoints for Cursor IDE hooks to enable DLP blocking.
// Hooks: beforeSubmitPrompt, beforeReadFile, beforeTabFileRead,
//        afterAgentResponse, afterAgentThought

use crate::database::Database;
use crate::dlp::{check_dlp_patterns, DlpDetection};
use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::post,
    Json, Router,
};
use serde::{Deserialize, Serialize};

// ============================================================================
// Common Input Fields (present in all hooks)
// Fields are required for JSON deserialization but not all are actively used
// ============================================================================

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct CommonHookFields {
    pub conversation_id: String,
    pub generation_id: String,
    pub model: String,
    pub hook_event_name: String,
    pub cursor_version: String,
    pub workspace_roots: Vec<String>,
    pub user_email: Option<String>,
}

// ============================================================================
// Hook-specific Input Structures
// Some fields are only used for JSON deserialization, not actively read
// ============================================================================

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct BeforeSubmitPromptInput {
    // Common fields
    pub conversation_id: String,
    pub generation_id: String,
    pub model: String,
    pub hook_event_name: String,
    pub cursor_version: String,
    pub workspace_roots: Vec<String>,
    pub user_email: Option<String>,
    // Hook-specific
    pub prompt: String,
    pub attachments: Vec<Attachment>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct Attachment {
    #[serde(rename = "type")]
    pub attachment_type: Option<String>,
    #[serde(rename = "filePath")]
    pub file_path: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct BeforeReadFileInput {
    // Common fields
    pub conversation_id: String,
    pub generation_id: String,
    pub model: String,
    pub hook_event_name: String,
    pub cursor_version: String,
    pub workspace_roots: Vec<String>,
    pub user_email: Option<String>,
    // Hook-specific
    pub file_path: String,
    pub content: Option<String>,
    pub attachments: Option<Vec<Attachment>>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct BeforeTabFileReadInput {
    // Common fields
    pub conversation_id: String,
    pub generation_id: String,
    pub model: String,
    pub hook_event_name: String,
    pub cursor_version: String,
    pub workspace_roots: Vec<String>,
    pub user_email: Option<String>,
    // Hook-specific
    pub file_path: String,
    pub content: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct AfterAgentResponseInput {
    // Common fields
    pub conversation_id: String,
    pub generation_id: String,
    pub model: String,
    pub hook_event_name: String,
    pub cursor_version: String,
    pub workspace_roots: Vec<String>,
    pub user_email: Option<String>,
    // Hook-specific
    pub text: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct AfterTabFileEditInput {
    // Common fields
    pub conversation_id: String,
    pub generation_id: String,
    pub model: String,
    pub hook_event_name: String,
    pub cursor_version: String,
    pub workspace_roots: Vec<String>,
    pub user_email: Option<String>,
    // Hook-specific
    pub file_path: String,
    pub edits: Vec<TabEdit>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TabEdit {
    pub old_string: String,
    pub new_string: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub range: Option<TabEditRange>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub old_line: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_line: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TabEditRange {
    pub start_line_number: i32,
    pub start_column: i32,
    pub end_line_number: i32,
    pub end_column: i32,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct AfterAgentThoughtInput {
    // Common fields
    pub conversation_id: String,
    pub generation_id: String,
    pub model: String,
    pub hook_event_name: String,
    pub cursor_version: String,
    pub workspace_roots: Vec<String>,
    pub user_email: Option<String>,
    // Hook-specific
    pub text: String,
    pub duration_ms: Option<i64>,
}

// ============================================================================
// Response Structures
// ============================================================================

#[derive(Debug, Serialize)]
pub struct BeforeSubmitPromptResponse {
    #[serde(rename = "continue")]
    pub should_continue: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_message: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct BeforeReadFileResponse {
    pub permission: String, // "allow" or "deny"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_message: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct BeforeTabFileReadResponse {
    pub permission: String, // "allow" or "deny"
}

#[derive(Debug, Serialize)]
pub struct GenericResponse {
    pub status: String,
}

// ============================================================================
// Extra Metadata for DB Storage
// ============================================================================

#[derive(Debug, Serialize)]
struct CursorHookMetadata {
    conversation_id: String,
    generation_id: String,
    hook_event_name: String,
    user_email: Option<String>,
    cursor_version: String,
    workspace_roots: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    file_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    thinking_word_count: Option<i32>,
}

// ============================================================================
// State
// ============================================================================

#[derive(Clone)]
pub struct CursorHooksState {
    pub db: Database,
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Count words in a string (simple whitespace split)
fn count_words(text: &str) -> i32 {
    text.split_whitespace().count() as i32
}

/// Format detected entities for user message
fn format_detection_message(detections: &[DlpDetection]) -> String {
    let mut message = String::from("Blocked: Sensitive data detected:\n");
    for detection in detections {
        message.push_str(&format!(
            "- {} ({})\n",
            detection.pattern_name, detection.pattern_type
        ));
    }
    message
}

// ============================================================================
// Handlers
// ============================================================================

/// POST /cursor_hook/before_submit_prompt
/// Checks prompt and attached files for sensitive data, blocks if found
async fn before_submit_prompt_handler(
    State(state): State<CursorHooksState>,
    Json(input): Json<BeforeSubmitPromptInput>,
) -> impl IntoResponse {
    println!(
        "[CURSOR_HOOK] before_submit_prompt - generation_id: {}, attachments: {}",
        input.generation_id,
        input.attachments.len()
    );

    // Check DLP patterns on prompt text
    let mut all_detections = check_dlp_patterns(&input.prompt);
    let mut total_word_count = count_words(&input.prompt);

    // Also check attached files
    for attachment in &input.attachments {
        if let (Some(file_path), Some(att_type)) = (&attachment.file_path, &attachment.attachment_type) {
            if att_type == "file" {
                // Read and check the file content
                if let Ok(content) = std::fs::read_to_string(file_path) {
                    let file_detections = check_dlp_patterns(&content);
                    if !file_detections.is_empty() {
                        println!(
                            "[CURSOR_HOOK] DLP detected in attached file: {}",
                            file_path
                        );
                        all_detections.extend(file_detections);
                    }
                    total_word_count += count_words(&content);
                }
            }
        }
    }

    let is_blocked = !all_detections.is_empty();
    let word_count = total_word_count;

    // Build extra metadata
    let metadata = CursorHookMetadata {
        conversation_id: input.conversation_id,
        generation_id: input.generation_id.clone(),
        hook_event_name: input.hook_event_name,
        user_email: input.user_email,
        cursor_version: input.cursor_version,
        workspace_roots: input.workspace_roots,
        file_path: None,
        thinking_word_count: None,
    };
    let metadata_json = serde_json::to_string(&metadata).ok();

    // Create or update request entry
    let response_status = if is_blocked { 403 } else { 200 };
    let user_message = if is_blocked {
        Some(format_detection_message(&all_detections))
    } else {
        None
    };

    // Log to database
    if let Ok(request_id) = state.db.log_cursor_hook_request(
        &input.generation_id,
        "CursorChat",
        &input.model,
        word_count,
        0, // output_tokens will be updated later
        &input.prompt,
        if is_blocked { "BLOCKED" } else { "" },
        response_status,
        metadata_json.as_deref(),
    ) {
        // Log DLP detections if any
        if !all_detections.is_empty() {
            let _ = state.db.log_dlp_detections(request_id, &all_detections);
        }
    }

    let response = BeforeSubmitPromptResponse {
        should_continue: !is_blocked,
        user_message,
    };

    (StatusCode::OK, Json(response))
}

/// POST /cursor_hook/before_read_file
/// Checks file content for sensitive data, blocks if found
async fn before_read_file_handler(
    State(state): State<CursorHooksState>,
    Json(input): Json<BeforeReadFileInput>,
) -> impl IntoResponse {
    println!(
        "[CURSOR_HOOK] before_read_file - generation_id: {}, file: {}",
        input.generation_id, input.file_path
    );

    // Get content: prefer provided content, fallback to reading file
    let content = match input.content {
        Some(c) => c,
        None => {
            // Read file from disk
            match std::fs::read_to_string(&input.file_path) {
                Ok(c) => c,
                Err(e) => {
                    println!(
                        "[CURSOR_HOOK] Failed to read file {}: {}",
                        input.file_path, e
                    );
                    // Allow if we can't read (file might not exist or be binary)
                    return (
                        StatusCode::OK,
                        Json(BeforeReadFileResponse {
                            permission: "allow".to_string(),
                            user_message: None,
                            agent_message: None,
                        }),
                    );
                }
            }
        }
    };

    // Check DLP patterns
    let detections = check_dlp_patterns(&content);
    let is_blocked = !detections.is_empty();

    let (permission, user_message, agent_message) = if is_blocked {
        let msg = format_detection_message(&detections);
        (
            "deny".to_string(),
            Some(msg.clone()),
            Some(format!(
                "Access to file {} was blocked due to sensitive data detection.",
                input.file_path
            )),
        )
    } else {
        ("allow".to_string(), None, None)
    };

    // Build extra metadata
    let metadata = CursorHookMetadata {
        conversation_id: input.conversation_id,
        generation_id: input.generation_id.clone(),
        hook_event_name: input.hook_event_name,
        user_email: input.user_email,
        cursor_version: input.cursor_version,
        workspace_roots: input.workspace_roots,
        file_path: Some(input.file_path.clone()),
        thinking_word_count: None,
    };
    let metadata_json = serde_json::to_string(&metadata).ok();

    // Log blocked file reads to database
    if is_blocked {
        let word_count = count_words(&content);
        let response_status = 403;

        if let Ok(request_id) = state.db.log_cursor_hook_request(
            &input.generation_id,
            "CursorChat",
            "",
            word_count,
            0,
            &format!("File read: {}", input.file_path),
            "BLOCKED - file read denied",
            response_status,
            metadata_json.as_deref(),
        ) {
            let _ = state.db.log_dlp_detections(request_id, &detections);
        }
    }

    let response = BeforeReadFileResponse {
        permission,
        user_message,
        agent_message,
    };

    (StatusCode::OK, Json(response))
}

/// POST /cursor_hook/before_tab_file_read
/// Checks file content for Tab completions, blocks if sensitive data found
async fn before_tab_file_read_handler(
    State(state): State<CursorHooksState>,
    Json(input): Json<BeforeTabFileReadInput>,
) -> impl IntoResponse {
    println!(
        "[CURSOR_HOOK] before_tab_file_read - generation_id: {}, file: {}",
        input.generation_id, input.file_path
    );

    // Get content: prefer provided content, fallback to reading file
    let content = match input.content {
        Some(c) => c,
        None => {
            match std::fs::read_to_string(&input.file_path) {
                Ok(c) => c,
                Err(e) => {
                    println!(
                        "[CURSOR_HOOK] Failed to read file {}: {}",
                        input.file_path, e
                    );
                    // Allow if we can't read
                    return (
                        StatusCode::OK,
                        Json(BeforeTabFileReadResponse {
                            permission: "allow".to_string(),
                        }),
                    );
                }
            }
        }
    };

    // Check DLP patterns
    let detections = check_dlp_patterns(&content);
    let is_blocked = !detections.is_empty();

    // Build extra metadata
    let metadata = CursorHookMetadata {
        conversation_id: input.conversation_id,
        generation_id: input.generation_id.clone(),
        hook_event_name: input.hook_event_name,
        user_email: input.user_email,
        cursor_version: input.cursor_version,
        workspace_roots: input.workspace_roots,
        file_path: Some(input.file_path.clone()),
        thinking_word_count: None,
    };
    let metadata_json = serde_json::to_string(&metadata).ok();

    // Log to database
    let word_count = count_words(&content);
    let response_status = if is_blocked { 403 } else { 200 };

    if let Ok(request_id) = state.db.log_cursor_hook_request(
        &input.generation_id,
        "CursorTab",
        &input.model,
        word_count,
        0,
        &format!("Tab file read: {}", input.file_path),
        if is_blocked { "BLOCKED" } else { "allowed" },
        response_status,
        metadata_json.as_deref(),
    ) {
        if !detections.is_empty() {
            let _ = state.db.log_dlp_detections(request_id, &detections);
        }
    }

    let response = BeforeTabFileReadResponse {
        permission: if is_blocked { "deny" } else { "allow" }.to_string(),
    };

    (StatusCode::OK, Json(response))
}

/// POST /cursor_hook/after_agent_response
/// Logs agent response for monitoring (word count as output_tokens)
async fn after_agent_response_handler(
    State(state): State<CursorHooksState>,
    Json(input): Json<AfterAgentResponseInput>,
) -> impl IntoResponse {
    println!(
        "[CURSOR_HOOK] after_agent_response - generation_id: {}",
        input.generation_id
    );

    let word_count = count_words(&input.text);

    // Update existing request entry with output tokens, or create new one
    let _ = state.db.update_cursor_hook_output(
        &input.generation_id,
        word_count,
        Some(&input.text),
    );

    (StatusCode::OK, Json(GenericResponse { status: "ok".to_string() }))
}

/// POST /cursor_hook/after_agent_thought
/// Logs agent thinking for monitoring (word count added to output_tokens)
async fn after_agent_thought_handler(
    State(state): State<CursorHooksState>,
    Json(input): Json<AfterAgentThoughtInput>,
) -> impl IntoResponse {
    println!(
        "[CURSOR_HOOK] after_agent_thought - generation_id: {}, duration_ms: {:?}",
        input.generation_id, input.duration_ms
    );

    let word_count = count_words(&input.text);

    // Add thinking word count to output tokens
    let _ = state.db.add_cursor_hook_thinking_tokens(
        &input.generation_id,
        word_count,
    );

    (StatusCode::OK, Json(GenericResponse { status: "ok".to_string() }))
}

/// POST /cursor_hook/after_tab_file_edit
/// Logs Tab edits for monitoring - updates existing entry from beforeTabFileRead
async fn after_tab_file_edit_handler(
    State(state): State<CursorHooksState>,
    Json(input): Json<AfterTabFileEditInput>,
) -> impl IntoResponse {
    println!(
        "[CURSOR_HOOK] after_tab_file_edit - generation_id: {}, file: {}, edits: {}",
        input.generation_id, input.file_path, input.edits.len()
    );

    // Calculate word count from new_string in all edits (represents output/generated code)
    let output_word_count: i32 = input
        .edits
        .iter()
        .map(|edit| count_words(&edit.new_string))
        .sum();

    // Serialize edits for response body
    let edits_json = serde_json::to_string(&input.edits).unwrap_or_default();
    let response_body = format!("Tab edit: {}\nEdits: {}", input.file_path, edits_json);

    // Update existing entry from beforeTabFileRead with output tokens
    let _ = state.db.update_cursor_hook_output(
        &input.generation_id,
        output_word_count,
        Some(&response_body),
    );

    (StatusCode::OK, Json(GenericResponse { status: "ok".to_string() }))
}

// ============================================================================
// Router
// ============================================================================

pub fn create_cursor_hooks_router(db: Database) -> Router {
    let state = CursorHooksState { db };

    Router::new()
        .route("/before_submit_prompt", post(before_submit_prompt_handler))
        .route("/before_read_file", post(before_read_file_handler))
        .route("/before_tab_file_read", post(before_tab_file_read_handler))
        .route("/after_agent_response", post(after_agent_response_handler))
        .route("/after_agent_thought", post(after_agent_thought_handler))
        .route("/after_tab_file_edit", post(after_tab_file_edit_handler))
        .with_state(state)
}
