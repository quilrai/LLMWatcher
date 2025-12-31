// DLP (Data Loss Prevention) Redaction Logic

use crate::dlp_pattern_config::{BUILTIN_API_KEY_PATTERNS, DB_PATH};
use regex::Regex;
use rusqlite::Connection;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct DlpDetection {
    pub pattern_name: String,
    pub pattern_type: String, // "builtin" or "keyword" or "regex"
    pub original_value: String,
    pub placeholder: String,
    pub message_index: Option<i32>,
}

#[derive(Clone)]
pub struct DlpRedactionResult {
    pub redacted_body: String,
    pub replacements: HashMap<String, String>, // placeholder -> original
    pub detections: Vec<DlpDetection>,
}

/// Get all enabled DLP patterns from database
/// Returns: Vec of (name, pattern_type, regexes)
pub fn get_enabled_dlp_patterns() -> Vec<(String, String, Vec<Regex>)> {
    let mut patterns: Vec<(String, String, Vec<Regex>)> = Vec::new();

    // Check if API keys detection is enabled
    let api_keys_enabled = {
        let conn = match Connection::open(DB_PATH) {
            Ok(c) => c,
            Err(_) => return patterns,
        };
        conn.query_row(
            "SELECT value FROM settings WHERE key = 'dlp_api_keys_enabled'",
            [],
            |row| row.get::<_, String>(0),
        )
        .ok()
        .map(|v| v == "1")
        .unwrap_or(false)
    };

    if api_keys_enabled {
        let mut regexes = Vec::new();
        for pattern in BUILTIN_API_KEY_PATTERNS {
            if let Ok(re) = Regex::new(pattern) {
                regexes.push(re);
            }
        }
        if !regexes.is_empty() {
            patterns.push(("API Keys".to_string(), "builtin".to_string(), regexes));
        }
    }

    // Get custom patterns from database
    let conn = match Connection::open(DB_PATH) {
        Ok(c) => c,
        Err(_) => return patterns,
    };

    let mut stmt = match conn.prepare(
        "SELECT name, pattern_type, patterns FROM dlp_patterns WHERE enabled = 1",
    ) {
        Ok(s) => s,
        Err(_) => return patterns,
    };

    let custom_patterns: Vec<(String, String, String)> = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .ok()
        .map(|iter| iter.filter_map(|r| r.ok()).collect())
        .unwrap_or_default();

    for (name, pattern_type, patterns_json) in custom_patterns {
        let pattern_list: Vec<String> =
            serde_json::from_str(&patterns_json).unwrap_or_default();

        let mut regexes = Vec::new();
        for p in pattern_list {
            let regex_pattern = if pattern_type == "keyword" {
                // Escape special regex chars and match as literal, case-insensitive
                format!(r"(?i){}", regex::escape(&p))
            } else {
                p
            };

            if let Ok(re) = Regex::new(&regex_pattern) {
                regexes.push(re);
            }
        }

        if !regexes.is_empty() {
            patterns.push((name, pattern_type, regexes));
        }
    }

    patterns
}

/// Apply DLP redaction to request body (only user messages, not system)
/// Supports both Claude (messages array) and Codex (input array) formats
pub fn apply_dlp_redaction(body: &str) -> DlpRedactionResult {
    println!("[DLP] Starting redaction...");
    let patterns = get_enabled_dlp_patterns();
    println!("[DLP] Got {} pattern groups", patterns.len());

    if patterns.is_empty() {
        println!("[DLP] No patterns enabled, skipping redaction");
        return DlpRedactionResult {
            redacted_body: body.to_string(),
            replacements: HashMap::new(),
            detections: Vec::new(),
        };
    }

    let mut json: serde_json::Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(_) => {
            return DlpRedactionResult {
                redacted_body: body.to_string(),
                replacements: HashMap::new(),
                detections: Vec::new(),
            }
        }
    };

    let mut replacements: HashMap<String, String> = HashMap::new();
    let mut detections: Vec<DlpDetection> = Vec::new();
    let mut counter = 1;

    // Process Claude format: messages array
    if let Some(messages) = json.get_mut("messages").and_then(|m| m.as_array_mut()) {
        println!("[DLP] Processing {} Claude messages", messages.len());
        for (msg_idx, message) in messages.iter_mut().enumerate() {
            // Only process user messages (skip assistant, system handled separately)
            let role = message.get("role").and_then(|r| r.as_str()).unwrap_or("");
            if role != "user" {
                println!("[DLP] Skipping message {} with role: {}", msg_idx, role);
                continue;
            }

            println!("[DLP] Processing user message {}", msg_idx);
            // Recursively process entire content structure
            if let Some(content) = message.get_mut("content") {
                redact_value_recursive(
                    content,
                    &patterns,
                    &mut replacements,
                    &mut detections,
                    &mut counter,
                    Some(msg_idx as i32),
                );
            }
            println!("[DLP] Done processing user message {}", msg_idx);
        }
    }

    // Process Codex format: input array
    if let Some(input) = json.get_mut("input").and_then(|m| m.as_array_mut()) {
        println!("[DLP] Processing {} Codex input items", input.len());
        for (item_idx, item) in input.iter_mut().enumerate() {
            let item_type = item.get("type").and_then(|t| t.as_str()).unwrap_or("");

            match item_type {
                "message" => {
                    // Only process user messages
                    let role = item.get("role").and_then(|r| r.as_str()).unwrap_or("");
                    if role != "user" {
                        println!("[DLP] Skipping Codex message {} with role: {}", item_idx, role);
                        continue;
                    }

                    println!("[DLP] Processing Codex user message {}", item_idx);
                    // Process content array (contains {type: "input_text", text: "..."} items)
                    if let Some(content) = item.get_mut("content") {
                        redact_value_recursive(
                            content,
                            &patterns,
                            &mut replacements,
                            &mut detections,
                            &mut counter,
                            Some(item_idx as i32),
                        );
                    }
                }
                "function_call_output" => {
                    // Function call outputs may contain sensitive data echoed back
                    println!("[DLP] Processing Codex function_call_output {}", item_idx);
                    if let Some(output) = item.get_mut("output") {
                        redact_value_recursive(
                            output,
                            &patterns,
                            &mut replacements,
                            &mut detections,
                            &mut counter,
                            Some(item_idx as i32),
                        );
                    }
                }
                _ => {
                    // Skip reasoning, function_call, etc.
                    println!("[DLP] Skipping Codex item {} with type: {}", item_idx, item_type);
                }
            }
        }
    }

    println!(
        "[DLP] Redaction complete. {} detections, {} replacements",
        detections.len(),
        replacements.len()
    );
    DlpRedactionResult {
        redacted_body: serde_json::to_string(&json).unwrap_or_else(|_| body.to_string()),
        replacements,
        detections,
    }
}

/// Recursively redact all string values in a JSON structure
fn redact_value_recursive(
    value: &mut serde_json::Value,
    patterns: &[(String, String, Vec<Regex>)],
    replacements: &mut HashMap<String, String>,
    detections: &mut Vec<DlpDetection>,
    counter: &mut u32,
    message_index: Option<i32>,
) {
    match value {
        serde_json::Value::String(s) => {
            let len = s.len();
            if len > 100 {
                println!("[DLP-R] Processing string of length {}", len);
            }
            let redacted = redact_text(s, patterns, replacements, detections, counter, message_index);
            *s = redacted;
        }
        serde_json::Value::Array(arr) => {
            println!("[DLP-R] Processing array of {} items", arr.len());
            for item in arr.iter_mut() {
                redact_value_recursive(item, patterns, replacements, detections, counter, message_index);
            }
        }
        serde_json::Value::Object(obj) => {
            println!("[DLP-R] Processing object with {} keys", obj.len());
            for (key, v) in obj.iter_mut() {
                println!("[DLP-R] Processing key: {}", key);
                redact_value_recursive(v, patterns, replacements, detections, counter, message_index);
            }
        }
        _ => {} // Numbers, bools, null - no redaction needed
    }
}

/// Create a same-length fake key that looks realistic
fn create_placeholder(id: u32, original: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    // Create a seeded "random" generator based on id
    let mut hasher = DefaultHasher::new();
    id.hash(&mut hasher);
    let mut seed = hasher.finish();

    // Helper to get next pseudo-random value
    let mut next_rand = || -> u64 {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        seed
    };

    let chars: Vec<char> = original
        .chars()
        .map(|c| {
            if c.is_ascii_lowercase() {
                // Replace with random lowercase letter
                let idx = (next_rand() % 26) as u8;
                (b'a' + idx) as char
            } else if c.is_ascii_uppercase() {
                // Replace with random uppercase letter
                let idx = (next_rand() % 26) as u8;
                (b'A' + idx) as char
            } else if c.is_ascii_digit() {
                // Replace with random digit
                let idx = (next_rand() % 10) as u8;
                (b'0' + idx) as char
            } else {
                // Keep special chars like -, _, etc.
                c
            }
        })
        .collect();

    chars.into_iter().collect()
}

/// Redact text and track replacements
fn redact_text(
    text: &str,
    patterns: &[(String, String, Vec<Regex>)],
    replacements: &mut HashMap<String, String>,
    detections: &mut Vec<DlpDetection>,
    counter: &mut u32,
    message_index: Option<i32>,
) -> String {
    let mut result = text.to_string();
    let text_len = text.len();

    for (name, pattern_type, regexes) in patterns {
        println!(
            "[DLP-T] Checking pattern '{}' ({} regexes) against text of len {}",
            name,
            regexes.len(),
            text_len
        );
        for (regex_idx, regex) in regexes.iter().enumerate() {
            if text_len > 1000 {
                println!("[DLP-T] Running regex {} of {}", regex_idx + 1, regexes.len());
            }
            // Find all matches and replace them
            let matches: Vec<String> = regex
                .find_iter(&result)
                .map(|m| m.as_str().to_string())
                .collect();

            for matched in matches {
                // Check if we already have a placeholder for this exact value
                let (placeholder, is_new) = replacements
                    .iter()
                    .find(|(_, v)| *v == &matched)
                    .map(|(k, _)| (k.clone(), false))
                    .unwrap_or_else(|| {
                        // Create same-length fake key that looks realistic
                        let p = create_placeholder(*counter, &matched);
                        replacements.insert(p.clone(), matched.clone());
                        *counter += 1;
                        (p, true)
                    });

                // Track detection (only for new placeholders to avoid duplicates)
                if is_new {
                    detections.push(DlpDetection {
                        pattern_name: name.clone(),
                        pattern_type: pattern_type.clone(),
                        original_value: matched.clone(),
                        placeholder: placeholder.clone(),
                        message_index,
                    });
                }

                result = result.replace(&matched, &placeholder);
            }
        }
    }

    result
}

/// Apply DLP unredaction to response body
pub fn apply_dlp_unredaction(body: &str, replacements: &HashMap<String, String>) -> String {
    if replacements.is_empty() {
        return body.to_string();
    }

    let mut result = body.to_string();

    // Replace all placeholders back with original values
    for (placeholder, original) in replacements {
        result = result.replace(placeholder, original);
    }

    result
}

/// Check text for DLP patterns without redaction (detection only)
/// Used by Cursor hooks to detect and block sensitive data
pub fn check_dlp_patterns(text: &str) -> Vec<DlpDetection> {
    let patterns = get_enabled_dlp_patterns();

    if patterns.is_empty() {
        return Vec::new();
    }

    let mut detections: Vec<DlpDetection> = Vec::new();
    let mut seen_values: std::collections::HashSet<String> = std::collections::HashSet::new();

    for (name, pattern_type, regexes) in patterns {
        for regex in regexes {
            let matches: Vec<String> = regex
                .find_iter(text)
                .map(|m| m.as_str().to_string())
                .collect();

            for matched in matches {
                // Skip duplicates
                if seen_values.contains(&matched) {
                    continue;
                }
                seen_values.insert(matched.clone());

                detections.push(DlpDetection {
                    pattern_name: name.clone(),
                    pattern_type: pattern_type.clone(),
                    original_value: matched,
                    placeholder: String::new(), // Not used for detection-only
                    message_index: None,
                });
            }
        }
    }

    detections
}
