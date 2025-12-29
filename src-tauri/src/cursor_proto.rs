// Schema-less Protobuf Decoder
// Extracts all strings from protobuf messages without requiring schema definitions
// Handles Connect protocol frames and gzip compression

use flate2::read::GzDecoder;
use std::io::Read;

/// Protobuf wire types
#[derive(Debug, Clone, Copy, PartialEq)]
enum WireType {
    Varint = 0,
    Fixed64 = 1,
    LengthDelimited = 2,
    Fixed32 = 5,
}

impl WireType {
    fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(WireType::Varint),
            1 => Some(WireType::Fixed64),
            2 => Some(WireType::LengthDelimited),
            5 => Some(WireType::Fixed32),
            _ => None,
        }
    }
}

/// Decompress gzip data
fn decompress_gzip(data: &[u8]) -> Option<Vec<u8>> {
    let mut decoder = GzDecoder::new(data);
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed).ok()?;
    Some(decompressed)
}

/// Read a varint from the buffer, returning (value, bytes_consumed)
fn read_varint(data: &[u8]) -> Option<(u64, usize)> {
    let mut result: u64 = 0;
    let mut shift = 0;

    for (i, &byte) in data.iter().enumerate() {
        if i >= 10 {
            // Varint too long
            return None;
        }

        result |= ((byte & 0x7F) as u64) << shift;

        if byte & 0x80 == 0 {
            return Some((result, i + 1));
        }

        shift += 7;
    }

    None
}

/// Check if bytes look like valid UTF-8 text (not binary garbage)
fn is_likely_text(data: &[u8]) -> bool {
    if data.is_empty() {
        return false;
    }

    // Must be valid UTF-8
    let text = match std::str::from_utf8(data) {
        Ok(t) => t,
        Err(_) => return false,
    };

    // Empty or very short strings are suspicious
    if text.len() < 2 {
        return false;
    }

    // Count printable vs non-printable characters
    let printable_count = text.chars().filter(|c| {
        c.is_alphanumeric() || c.is_whitespace() || c.is_ascii_punctuation() || *c > '\u{007F}'
    }).count();

    let total = text.chars().count();

    // At least 80% should be printable
    printable_count * 100 / total >= 80
}

/// Check if data looks like a valid protobuf message
fn looks_like_protobuf(data: &[u8]) -> bool {
    if data.is_empty() {
        return false;
    }

    let mut offset = 0;
    let mut field_count = 0;

    while offset < data.len() && field_count < 100 {
        // Read tag (varint)
        let (tag, tag_len) = match read_varint(&data[offset..]) {
            Some(v) => v,
            None => return false,
        };
        offset += tag_len;

        let wire_type = match WireType::from_u8((tag & 0x07) as u8) {
            Some(wt) => wt,
            None => return false,
        };

        let field_number = tag >> 3;

        // Sanity check field number
        if field_number == 0 || field_number > 536870911 {
            return false;
        }

        match wire_type {
            WireType::Varint => {
                let (_, len) = match read_varint(&data[offset..]) {
                    Some(v) => v,
                    None => return false,
                };
                offset += len;
            }
            WireType::Fixed64 => {
                if offset + 8 > data.len() {
                    return false;
                }
                offset += 8;
            }
            WireType::Fixed32 => {
                if offset + 4 > data.len() {
                    return false;
                }
                offset += 4;
            }
            WireType::LengthDelimited => {
                let (length, len) = match read_varint(&data[offset..]) {
                    Some(v) => v,
                    None => return false,
                };
                offset += len;

                let length = length as usize;
                if offset + length > data.len() {
                    return false;
                }
                offset += length;
            }
        }

        field_count += 1;
    }

    // Should consume entire buffer (or at least most of it)
    field_count > 0 && offset == data.len()
}

/// Recursively extract all text strings from protobuf data
fn extract_strings_recursive(data: &[u8], strings: &mut Vec<String>, depth: usize) {
    if depth > 20 || data.is_empty() {
        return;
    }

    let mut offset = 0;

    while offset < data.len() {
        // Read tag (varint)
        let (tag, tag_len) = match read_varint(&data[offset..]) {
            Some(v) => v,
            None => break,
        };
        offset += tag_len;

        let wire_type = match WireType::from_u8((tag & 0x07) as u8) {
            Some(wt) => wt,
            None => break,
        };

        let field_number = tag >> 3;

        // Sanity check
        if field_number == 0 || field_number > 536870911 {
            break;
        }

        match wire_type {
            WireType::Varint => {
                let (_, len) = match read_varint(&data[offset..]) {
                    Some(v) => v,
                    None => break,
                };
                offset += len;
            }
            WireType::Fixed64 => {
                if offset + 8 > data.len() {
                    break;
                }
                offset += 8;
            }
            WireType::Fixed32 => {
                if offset + 4 > data.len() {
                    break;
                }
                offset += 4;
            }
            WireType::LengthDelimited => {
                let (length, len) = match read_varint(&data[offset..]) {
                    Some(v) => v,
                    None => break,
                };
                offset += len;

                let length = length as usize;
                if offset + length > data.len() {
                    break;
                }

                let field_data = &data[offset..offset + length];
                offset += length;

                // Try to decode as nested protobuf first
                if looks_like_protobuf(field_data) {
                    extract_strings_recursive(field_data, strings, depth + 1);
                } else if is_likely_text(field_data) {
                    // It's a text string
                    if let Ok(text) = std::str::from_utf8(field_data) {
                        // Filter out very short strings and things that look like IDs/hashes
                        if text.len() >= 3 && !looks_like_id(text) {
                            strings.push(text.to_string());
                        }
                    }
                }
            }
        }
    }
}

/// Check if string looks like an ID, hash, or other non-human-readable content
fn looks_like_id(s: &str) -> bool {
    // Skip very long hex strings
    if s.len() > 20 && s.chars().all(|c| c.is_ascii_hexdigit()) {
        return true;
    }

    // Skip UUIDs
    if s.len() == 36 && s.chars().filter(|c| *c == '-').count() == 4 {
        let parts: Vec<&str> = s.split('-').collect();
        if parts.len() == 5 && parts.iter().all(|p| p.chars().all(|c| c.is_ascii_hexdigit())) {
            return true;
        }
    }

    // Skip base64-looking strings that are mostly alphanumeric with no spaces
    if s.len() > 30 && !s.contains(' ') && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '=') {
        return true;
    }

    false
}

/// Parse Connect protocol frames from binary data
fn parse_connect_frames(data: &[u8]) -> Vec<Vec<u8>> {
    let mut frames = Vec::new();
    let mut offset = 0;

    while offset < data.len() {
        // Need at least 5 bytes (1 type + 4 length)
        if offset + 5 > data.len() {
            break;
        }

        let frame_type = data[offset];
        if frame_type > 3 {
            break;
        }

        let msg_len = u32::from_be_bytes(
            data[offset + 1..offset + 5]
                .try_into()
                .unwrap_or([0, 0, 0, 0]),
        ) as usize;

        offset += 5;

        if offset + msg_len > data.len() {
            break;
        }

        let frame_data = data[offset..offset + msg_len].to_vec();
        offset += msg_len;

        // Decompress if gzip (frame types 1 and 3)
        let final_data = if frame_type == 1 || frame_type == 3 {
            decompress_gzip(&frame_data).unwrap_or(frame_data)
        } else {
            frame_data
        };

        frames.push(final_data);
    }

    frames
}

/// Extract all text strings from protobuf/connect data
/// Returns a vector of extracted text strings
pub fn extract_all_strings(data: &[u8]) -> Vec<String> {
    if data.is_empty() {
        return vec![];
    }

    let mut all_strings = Vec::new();

    // Check for raw GZIP data (magic bytes: 1f 8b)
    let data_to_process = if data.len() >= 2 && data[0] == 0x1f && data[1] == 0x8b {
        decompress_gzip(data).unwrap_or_else(|| data.to_vec())
    } else {
        data.to_vec()
    };

    // Try to detect Connect protocol frames
    if data_to_process.len() >= 5 && data_to_process[0] <= 3 {
        let potential_len = u32::from_be_bytes(
            data_to_process[1..5].try_into().unwrap_or([0, 0, 0, 0])
        ) as usize;

        if potential_len > 0 && potential_len + 5 <= data_to_process.len() {
            let frames = parse_connect_frames(&data_to_process);
            if !frames.is_empty() {
                for frame in frames {
                    // Try JSON first
                    if let Ok(text) = std::str::from_utf8(&frame) {
                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(text) {
                            extract_strings_from_json(&json, &mut all_strings);
                            continue;
                        }
                    }
                    // Otherwise extract from protobuf
                    extract_strings_recursive(&frame, &mut all_strings, 0);
                }
                return all_strings;
            }
        }
    }

    // Try as JSON first
    if let Ok(text) = std::str::from_utf8(&data_to_process) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(text) {
            extract_strings_from_json(&json, &mut all_strings);
            return all_strings;
        }
    }

    // Try direct protobuf extraction
    extract_strings_recursive(&data_to_process, &mut all_strings, 0);

    all_strings
}

/// Extract strings from JSON recursively
fn extract_strings_from_json(value: &serde_json::Value, strings: &mut Vec<String>) {
    match value {
        serde_json::Value::String(s) => {
            if s.len() >= 3 && !looks_like_id(s) {
                strings.push(s.clone());
            }
        }
        serde_json::Value::Array(arr) => {
            for item in arr {
                extract_strings_from_json(item, strings);
            }
        }
        serde_json::Value::Object(obj) => {
            for (_, v) in obj {
                extract_strings_from_json(v, strings);
            }
        }
        _ => {}
    }
}

/// Main entry point - decode and format for display
/// Returns a simple formatted string of extracted text
pub fn decode_and_format(data: &[u8]) -> String {
    if data.is_empty() {
        return "(empty)".to_string();
    }

    let strings = extract_all_strings(data);

    if strings.is_empty() {
        // Show hex preview for truly unknown binary
        let preview_len = std::cmp::min(64, data.len());
        return format!(
            "[Binary: {} bytes] {}",
            data.len(),
            hex::encode(&data[..preview_len])
        );
    }

    // Format extracted strings
    let mut output = Vec::new();

    for s in &strings {
        // Truncate very long strings for display
        let display = if s.len() > 500 {
            format!("{}... ({} chars)", &s[..500], s.len())
        } else {
            s.clone()
        };
        output.push(display);
    }

    output.join("\n---\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_likely_text() {
        assert!(is_likely_text(b"Hello, world!"));
        assert!(is_likely_text(b"This is a test message."));
        assert!(!is_likely_text(b"\x00\x01\x02\x03"));
        assert!(!is_likely_text(b""));
    }

    #[test]
    fn test_looks_like_id() {
        assert!(looks_like_id("550e8400-e29b-41d4-a716-446655440000"));
        assert!(looks_like_id("abcdef1234567890abcdef1234567890"));
        assert!(!looks_like_id("Hello world"));
        assert!(!looks_like_id("This is a normal sentence."));
    }

    #[test]
    fn test_read_varint() {
        // Single byte varint
        assert_eq!(read_varint(&[0x01]), Some((1, 1)));
        assert_eq!(read_varint(&[0x7F]), Some((127, 1)));

        // Multi-byte varint
        assert_eq!(read_varint(&[0x80, 0x01]), Some((128, 2)));
        assert_eq!(read_varint(&[0xAC, 0x02]), Some((300, 2)));
    }
}
