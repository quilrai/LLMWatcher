# DLP Demo App - Tauri Backend Overview

This is an **Agent Gateway with DLP (Data Loss Prevention) capabilities** that acts as an HTTP reverse proxy for LLM API requests.

## Directory Structure

```
src-tauri/
├── src/
│   ├── lib.rs                      # Entry point - app initialization
│   ├── main.rs                     # Thin wrapper calling lib.rs
│   ├── proxy.rs                    # HTTP reverse proxy server
│   ├── database.rs                 # SQLite operations
│   ├── dlp.rs                      # DLP redaction/unredaction engine
│   ├── dlp_pattern_config.rs       # Built-in patterns & constants
│   ├── requestresponsemetadata.rs  # Metadata type definitions
│   ├── backends/
│   │   ├── mod.rs                  # Backend trait definition
│   │   ├── claude.rs               # Anthropic Claude implementation
│   │   └── codex.rs                # OpenAI Codex implementation
│   └── commands/
│       ├── mod.rs                  # Command exports
│       ├── stats.rs                # Dashboard & monitoring commands
│       └── dlp.rs                  # DLP settings commands
├── Cargo.toml                      # Dependencies
└── proxy_requests.db               # SQLite database
```

## Module Responsibilities

| Module | Purpose |
|--------|---------|
| **lib.rs** | App initialization, Tauri command registration, spawns proxy server |
| **proxy.rs** | Axum-based HTTP proxy - intercepts requests, applies DLP, forwards to backends, logs everything |
| **dlp.rs** | Pattern matching engine - redacts sensitive data before sending, unredacts responses |
| **dlp_pattern_config.rs** | 16 built-in regex patterns for API keys (OpenAI, Anthropic, AWS, GitHub, Slack, Stripe, Google) |
| **database.rs** | SQLite wrapper with tables for requests, settings, dlp_patterns, dlp_detections |
| **backends/*.rs** | Backend trait + implementations for Claude & Codex APIs (metadata extraction, URL routing) |
| **commands/*.rs** | Tauri commands exposed to frontend (dashboard stats, message logs, DLP settings, port management) |

## Data Flow

```
Client Request
     ↓
  Proxy (proxy.rs)
     ├→ DLP Redaction (replace sensitive data with placeholders)
     ├→ Forward to Backend API (Claude/Codex)
     ├→ Receive Response
     ├→ DLP Unredaction (restore original values)
     ├→ Log to SQLite
     └→ Return to client

Frontend → Tauri Commands → Database Queries → Analytics
```

## Key Features

- **Routes**: `/claude` → Anthropic API, `/codex` → OpenAI API
- **Streaming support**: Handles both streaming and non-streaming responses
- **Token tracking**: Input, output, cache read/creation tokens
- **Data retention**: Auto-cleanup of records older than 7 days
- **Default port**: 8008 (configurable)
