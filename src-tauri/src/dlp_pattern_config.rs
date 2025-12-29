// DLP Pattern Configuration and Constants

pub const DB_PATH: &str = "proxy_requests.db";
pub const DEFAULT_PORT: u16 = 8008;
pub const DEFAULT_MITM_PORT: u16 = 8888;

// Built-in API key patterns
pub const BUILTIN_API_KEY_PATTERNS: &[&str] = &[
    r"sk-[a-zA-Z0-9]{20,}",                           // OpenAI API keys
    r"sk-ant-[a-zA-Z0-9\-_]{20,}",                    // Anthropic API keys
    r"sk-proj-[a-zA-Z0-9\-_]{20,}",                   // OpenAI project keys
    r"AKIA[0-9A-Z]{16}",                              // AWS Access Key ID
    r"ghp_[a-zA-Z0-9]{36}",                           // GitHub personal access token
    r"gho_[a-zA-Z0-9]{36}",                           // GitHub OAuth token
    r"ghu_[a-zA-Z0-9]{36}",                           // GitHub user-to-server token
    r"ghs_[a-zA-Z0-9]{36}",                           // GitHub server-to-server token
    r"ghr_[a-zA-Z0-9]{36}",                           // GitHub refresh token
    r"xox[baprs]-[a-zA-Z0-9\\-]{10,}",                // Slack tokens
    r"sk_live_[a-zA-Z0-9]{24,}",                      // Stripe live secret key
    r"sk_test_[a-zA-Z0-9]{24,}",                      // Stripe test secret key
    r"pk_live_[a-zA-Z0-9]{24,}",                      // Stripe live publishable key
    r"pk_test_[a-zA-Z0-9]{24,}",                      // Stripe test publishable key
    r"AIza[0-9A-Za-z\-_]{35}",                        // Google API key
    r"ya29\.[0-9A-Za-z\-_]+",                         // Google OAuth token
    r"-----BEGIN\s+(RSA\s+)?PRIVATE\s+KEY-----",     // Private keys
    r"-----BEGIN\s+OPENSSH\s+PRIVATE\s+KEY-----",    // OpenSSH private keys
];
