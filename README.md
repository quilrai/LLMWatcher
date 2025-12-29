# DLP Demo App

A Tauri-based proxy application for monitoring and applying DLP (Data Loss Prevention) to AI API requests.

## Features

- Proxy support for multiple AI backends (Claude, Codex)
- DLP pattern detection and redaction
- Request/response logging with metadata extraction
- Flexible metadata storage for backend-specific data

## Recommended IDE Setup

- [VS Code](https://code.visualstudio.com/) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)

## How to Use

### Starting the Proxy

The proxy runs on `http://localhost:8008` by default (configurable).

### Claude (Anthropic)

Configure your Claude client to use the proxy:

```bash
# Environment variable
ANTHROPIC_BASE_URL="http://localhost:8008/claude"

# Or in your code
client = Anthropic(base_url="http://localhost:8008/claude")
```

The proxy forwards requests to `https://api.anthropic.com`.

### Codex (OpenAI GPT-5)

Configure your Codex/OpenAI client to use the proxy:

```bash
# Environment variable
OPENAI_BASE_URL="http://localhost:8008/codex"

# Or in your code (for OpenAI-compatible clients)
client = OpenAI(base_url="http://localhost:8008/codex")
```

The proxy forwards requests to `https://chatgpt.com/backend-api/codex`.

## API Routes

| Route | Backend | Upstream URL |
|-------|---------|--------------|
| `/` | Health check | - |
| `/claude/*` | Claude | `https://api.anthropic.com` |
| `/codex/*` | Codex | `https://chatgpt.com/backend-api/codex` |

## Development

```bash
# Install dependencies
npm install

# Run in development mode
npm run tauri dev

# Build for production
npm run tauri build
```

## Adding New Backends

To add a new backend (e.g., Gemini):

1. Create `src-tauri/src/backends/gemini.rs` implementing the `Backend` trait
2. Add `pub mod gemini;` and `pub use gemini::GeminiBackend;` to `src-tauri/src/backends/mod.rs`
3. In `src-tauri/src/proxy.rs`:
   - Create backend instance: `let gemini_backend: Arc<dyn Backend> = Arc::new(GeminiBackend::new());`
   - Create state: `let gemini_state = ProxyState { db: db.clone(), backend: gemini_backend };`
   - Create router: `let gemini_router = Router::new().fallback(proxy_handler).with_state(gemini_state);`
   - Add route: `.nest("/gemini", gemini_router)`

No database changes needed - use `extra_metadata` column for any backend-specific fields.
