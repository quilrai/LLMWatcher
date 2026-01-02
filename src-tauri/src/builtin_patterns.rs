// Hardcoded builtin DLP patterns
// This replaces the JSON file to avoid bundling external files

/// Builtin pattern definition
pub struct BuiltinPattern {
    pub name: &'static str,
    pub pattern_type: &'static str,
    pub patterns: &'static [&'static str],
    pub negative_pattern_type: Option<&'static str>,
    pub negative_patterns: Option<&'static [&'static str]>,
    pub min_occurrences: i32,
    pub min_unique_chars: i32,
}

/// Get all builtin DLP patterns
pub fn get_builtin_patterns() -> &'static [BuiltinPattern] {
    &[
        BuiltinPattern {
            name: "API Keys",
            pattern_type: "regex",
            patterns: &[
                r"sk-[a-zA-Z0-9]{20,}",
                r"sk-ant-[a-zA-Z0-9\-_]{20,}",
                r"sk-proj-[a-zA-Z0-9\-_]{20,}",
                r"AKIA[0-9A-Z]{16}",
                r"ghp_[a-zA-Z0-9]{36}",
                r"gho_[a-zA-Z0-9]{36}",
                r"ghu_[a-zA-Z0-9]{36}",
                r"ghs_[a-zA-Z0-9]{36}",
                r"ghr_[a-zA-Z0-9]{36}",
                r"xox[baprs]-[a-zA-Z0-9\-]{10,}",
                r"sk_live_[a-zA-Z0-9]{24,}",
                r"sk_test_[a-zA-Z0-9]{24,}",
                r"pk_live_[a-zA-Z0-9]{24,}",
                r"pk_test_[a-zA-Z0-9]{24,}",
                r"AIza[0-9A-Za-z\-_]{35}",
                r"ya29\.[0-9A-Za-z\-_]+",
                r"-----BEGIN\s+(RSA\s+)?PRIVATE\s+KEY-----",
                r"-----BEGIN\s+OPENSSH\s+PRIVATE\s+KEY-----",
            ],
            negative_pattern_type: None,
            negative_patterns: None,
            min_occurrences: 1,
            min_unique_chars: 10,
        },
    ]
}
