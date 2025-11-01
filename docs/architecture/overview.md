# Claude Relay Service - Architecture Design

> **Version**: 2.0 (Rust Rewrite)
> **Status**: In Development
> **Last Updated**: 2025-10-30

## Overview

Claude Relay Service is a high-performance, multi-platform AI API relay service built with Rust. It provides authentication, rate limiting, cost tracking, and unified scheduling across multiple AI providers.

## Architecture Principles

### Core Design Goals
- **Performance**: 3-5x faster than Node.js version
- **Memory Efficiency**: 50-70% reduction in memory usage
- **Type Safety**: Rust's type system eliminates runtime errors
- **Network Stability**: Robust error handling and retry mechanisms
- **Scalability**: Horizontal scaling support

### Technology Stack
- **Runtime**: Tokio async runtime
- **Web Framework**: Axum (modern, type-safe)
- **Database**: Redis (with connection pooling)
- **HTTP Client**: Reqwest (with proxy support)
- **Logging**: Tracing ecosystem
- **Configuration**: Config-rs + dotenvy

## System Components

### 1. Service Layer

#### Account Management Services
```rust
pub trait AccountService {
    async fn get_account(&self, id: &str) -> Result<Account>;
    async fn refresh_token(&self, account: &mut Account) -> Result<()>;
    async fn update_status(&self, id: &str, status: AccountStatus) -> Result<()>;
}
```

- `ClaudeAccountService` - Claude OAuth management
- `GeminiAccountService` - Gemini Google OAuth
- `OpenAIAccountService` - OpenAI Responses (Codex)
- `BedrockAccountService` - AWS Bedrock credentials

#### Relay Services
```rust
pub trait RelayService {
    async fn forward_request(&self, req: Request) -> Result<Response>;
    async fn forward_stream(&self, req: Request) -> Result<Stream<Event>>;
}
```

- Stream processing with zero-copy where possible
- Automatic usage statistics capture
- Error handling and retry logic
- Proxy support per account

### 2. Routing Layer

#### Unified Router
```rust
async fn route_request(req: Request) -> Result<Response> {
    match req.path() {
        "/api/v1/messages" => claude_handler(req).await,
        "/gemini/v1/*" => gemini_handler(req).await,
        "/openai/v1/*" => openai_handler(req).await,
        _ => not_found().await,
    }
}
```

### 3. Middleware Stack

```
Request → CORS → Auth → RateLimit → Logging → Handler → Response
```

#### Authentication Middleware
- SHA-256 API key hashing with O(1) lookup
- Permission validation (claude/gemini/openai)
- Client restrictions (User-Agent based)
- Model blacklist enforcement

#### Rate Limiting
- Token bucket algorithm (governor crate)
- Redis-backed distributed limiting
- Per-key and per-account limits
- Concurrency control with sorted sets

### 4. Data Layer

#### Redis Data Models
```rust
#[derive(Serialize, Deserialize)]
pub struct ApiKey {
    pub id: String,
    pub hash: String,
    pub user_id: String,
    pub permissions: Vec<Permission>,
    pub rate_limit: RateLimit,
}

#[derive(Serialize, Deserialize)]
pub struct ClaudeAccount {
    pub id: String,
    pub name: String,
    pub account_type: AccountType,
    pub credentials: EncryptedCredentials,
    pub proxy: Option<ProxyConfig>,
}
```

#### Key Patterns
- `api_key:{id}` - API key details
- `api_key_hash:{hash}` - Fast lookup mapping
- `claude_account:{id}` - Account with encrypted credentials
- `sticky_session:{hash}` - Session-account binding
- `concurrency:{account_id}` - Sorted set for concurrency tracking

### 5. Unified Scheduler

```rust
pub struct UnifiedScheduler {
    accounts: Vec<Account>,
    sticky_sessions: LruCache<String, String>,
}

impl UnifiedScheduler {
    pub async fn select_account(&self, req: &Request) -> Result<Account> {
        // 1. Check sticky session
        if let Some(account) = self.check_sticky_session(req).await? {
            return Ok(account);
        }

        // 2. Filter available accounts
        let available = self.filter_available_accounts().await?;

        // 3. Select based on load balancing
        self.select_optimal_account(&available).await
    }
}
```

## Network Flow

### Request Processing Pipeline

```
Client Request
    ↓
[Auth Middleware]
    ↓ (API Key validated)
[RateLimit Check]
    ↓ (Rate limits passed)
[Unified Scheduler]
    ↓ (Account selected)
[Token Refresh] (if needed)
    ↓
[Relay Service]
    ↓ (Forward to upstream)
[Upstream API]
    ↓ (Stream response)
[Usage Capture]
    ↓ (Extract tokens from SSE)
[Cost Calculation]
    ↓
Client Response
```

### Streaming Response (SSE)

```rust
async fn handle_stream(req: Request) -> Result<Stream<Event>> {
    let upstream_stream = relay.forward_stream(req).await?;

    let monitored_stream = upstream_stream
        .inspect(|event| {
            if let Event::Message(msg) = event {
                // Extract usage data
                capture_usage(msg);
            }
        })
        .map(|event| convert_to_client_format(event));

    Ok(monitored_stream)
}
```

## Performance Optimizations

### 1. Connection Pooling
- HTTP client with configurable pool size
- Redis connection pool (deadpool-redis)
- Persistent connections to upstream APIs

### 2. Caching Strategy
- LRU cache for decrypted credentials
- In-memory cache for API key lookups
- Redis-backed distributed cache

### 3. Zero-Copy Operations
- Stream processing without buffering
- Direct pipe from upstream to client
- Minimal allocations in hot paths

### 4. Async All The Way
- Tokio runtime for maximum concurrency
- Non-blocking I/O operations
- Parallel request processing

## Security Model

### Data Encryption
- AES-256-GCM for credentials at rest
- TLS for all network communication
- Argon2 for password hashing

### Authentication Flow
```
API Key (cr_xxx) → SHA-256 Hash → Redis Lookup → Permission Check → Allow/Deny
```

### Proxy Security
- Per-account proxy configuration
- SOCKS5 and HTTP proxy support
- Proxy credentials encrypted in Redis

## Error Handling

### Error Types
```rust
#[derive(Debug, thiserror::Error)]
pub enum RelayError {
    #[error("Authentication failed: {0}")]
    AuthError(String),

    #[error("Rate limit exceeded")]
    RateLimitError,

    #[error("Account overloaded (529)")]
    AccountOverload,

    #[error("Upstream error: {0}")]
    UpstreamError(#[from] reqwest::Error),
}
```

### Retry Strategy
- Exponential backoff for transient errors
- Circuit breaker for failing accounts
- Automatic failover to alternative accounts

## Deployment Architecture

### Standalone Mode
```
Client → Rust Service (Port 8080) → Upstream APIs
```

### Reverse Proxy Mode
```
Client → Caddy/Nginx → Rust Service → Upstream APIs
```

### Horizontal Scaling
```
Client → Load Balancer
           ↓
    [Service A] [Service B] [Service C]
           ↓
    Redis Cluster (shared state)
```

## Migration Strategy

### Phase 1: Parallel Running
- Node.js version on port 3000
- Rust version on port 8080
- Gradual traffic migration (5% → 50% → 100%)

### Phase 2: Feature Parity
- All endpoints implemented
- Data format compatibility verified
- Performance benchmarks validated

### Phase 3: Complete Replacement
- Node.js version archived
- Rust version becomes primary
- Documentation updated

## Future Enhancements

### Planned Features
- [ ] HTTP/3 support (QUIC)
- [ ] gRPC endpoints for internal APIs
- [ ] Built-in metrics dashboard
- [ ] Distributed tracing (OpenTelemetry)
- [ ] WebAssembly plugins for extensibility

### Performance Goals
- [ ] Sub-millisecond p50 latency
- [ ] 10K+ requests/second on single instance
- [ ] <100MB memory for typical workload
- [ ] Zero-downtime deployments

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development guidelines.

## References

- [Rust Async Book](https://rust-lang.github.io/async-book/)
- [Axum Documentation](https://docs.rs/axum/)
- [Tokio Tutorial](https://tokio.rs/tokio/tutorial)
- [Redis Rust Client](https://docs.rs/redis/)
