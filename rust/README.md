# Claude Relay Service - Rust Implementation

> **Status**: 🚧 In Development (Phase 2 of Rewrite)

[![Rust Tests](https://github.com/YOUR_USERNAME/claude-relay-service/workflows/Rust%20Tests/badge.svg)](https://github.com/YOUR_USERNAME/claude-relay-service/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](../LICENSE)

This is the Rust rewrite of Claude Relay Service, designed for superior performance, memory efficiency, and network stability.

## Quick Start

### Prerequisites

- Rust 1.75+ ([Install Rust](https://rustup.rs/))
- Redis 6+
- (Optional) Docker for containerized deployment

### Installation

```bash
# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Build the project
cd rust/
cargo build --release

# Run the service
./target/release/claude-relay
```

### Development

```bash
# Run in development mode (with auto-reload)
cargo watch -x run

# Run tests
cargo test

# Run benchmarks
cargo bench

# Format code
cargo fmt

# Lint code
cargo clippy
```

## Configuration

### Environment Variables

Copy the example environment file and configure it:

```bash
cp .env.example .env
# Edit .env with your settings
```

The Rust version uses environment variables with `CRS__` prefix (double underscore for nesting):

```bash
# Server
CRS_SERVER__HOST=0.0.0.0
CRS_SERVER__PORT=8080

# Redis
CRS_REDIS__HOST=localhost
CRS_REDIS__PORT=6379

# Security (REQUIRED)
CRS_SECURITY__JWT_SECRET=your_jwt_secret_minimum_32_chars
CRS_SECURITY__ENCRYPTION_KEY=exactly_32_characters_here!!!

# Logging
CRS_LOGGING__LEVEL=info  # trace, debug, info, warn, error
CRS_LOGGING__FORMAT=pretty  # pretty or json
```

Configuration can also be loaded from `config/config.toml` files (with `RUN_MODE` environment variable support for `config.development.toml`, `config.production.toml`, etc.)

## Architecture

See [docs/ARCHITECTURE.md](../docs/ARCHITECTURE.md) for detailed architecture design.

### Key Components

- **`src/main.rs`**: Entry point
- **`src/config/`**: Configuration management
- **`src/models/`**: Data models (API keys, accounts, etc.)
- **`src/services/`**: Business logic (account management, relay services)
- **`src/routes/`**: HTTP routing layer
- **`src/middleware/`**: Authentication, rate limiting, logging
- **`src/utils/`**: Helper functions
- **`src/redis/`**: Redis operations and connection pooling

## Performance Goals

| Metric | Node.js (Current) | Rust (Target) | Improvement |
|--------|-------------------|---------------|-------------|
| Request Latency (p50) | ~50ms | <20ms | 2.5x faster |
| Memory Usage | ~200MB | <70MB | 65% reduction |
| Concurrent Requests | ~500/s | >2000/s | 4x throughput |
| CPU Efficiency | Baseline | +50% | More efficient |

## Migration Progress

### Phase 1: Infrastructure (Week 2) ✅
- [x] Project initialization
- [x] Cargo.toml dependencies
- [x] Module structure
- [x] Configuration loading (config-rs + environment variables)
- [x] Error handling framework (AppError + IntoResponse)
- [x] Redis connection pool (deadpool-redis)
- [x] HTTP client (reqwest with proxy support)
- [x] Logging system (tracing + tracing-subscriber)
- [x] Basic routing (Axum router)
- [x] Health check endpoints (/health, /ping)

### Phase 2: Core Features (Week 3-4)
- [ ] API key models and storage
- [ ] API key authentication middleware
- [ ] SHA-256 hashing and lookup
- [ ] Rate limiting (governor)
- [ ] Concurrency control
- [ ] Request usage tracking
- [ ] Cost calculation

### Phase 3: Account Management (Week 5-6)
- [ ] Claude account models
- [ ] Gemini account models
- [ ] OpenAI account models
- [ ] OAuth token refresh
- [ ] Account selection logic
- [ ] Sticky session support

### Phase 4: API Relay (Week 7-8)
- [ ] Claude API relay (/api/v1/messages)
- [ ] Gemini API relay
- [ ] OpenAI API relay
- [ ] Unified scheduler
- [ ] Streaming support (SSE)
- [ ] Usage data capture

### Phase 5: Admin Interface (Week 9)
- [ ] Web admin API endpoints
- [ ] Dashboard statistics
- [ ] Account management API
- [ ] API key management API
- [ ] System monitoring
- [ ] Full feature parity with Node.js version

## Testing

### Prerequisites

The integration tests use [testcontainers-rs](https://docs.rs/testcontainers/) for automatic Docker container management. Ensure Docker is installed and running:

```bash
# Check Docker status
docker info
```

### Running Tests

```bash
# Run all tests (unit + integration)
cargo test

# Run only unit tests (no Docker required)
cargo test --lib

# Run only integration tests
cargo test --test '*'

# Run specific test
cargo test test_complete_key_lifecycle

# Run with detailed output
cargo test -- --nocapture

# Test with coverage
cargo tarpaulin --out Html
```

### Test Architecture

- **Unit Tests** (`src/**/*_test.rs`): Fast, no external dependencies
- **Integration Tests** (`tests/**/*.rs`): Use testcontainers for Redis
  - Each test gets a fresh Redis instance
  - Automatic container cleanup
  - Parallel execution safe
  - Zero manual setup required

### Testcontainers Integration

The test suite automatically manages Docker containers:

- **Automatic lifecycle**: Containers start/stop per test
- **Dynamic ports**: No port conflicts during parallel runs
- **Clean state**: Each test has isolated Redis instance
- **CI/CD ready**: Works in GitHub Actions out of the box

If you encounter Docker-related issues:

```bash
# Ensure Docker daemon is running
sudo systemctl start docker  # Linux
# or
open -a Docker  # macOS

# Pull Redis image manually (optional)
docker pull redis:latest
```

## Deployment

### Standalone

```bash
cargo build --release
./target/release/claude-relay
```

### Docker

```bash
# Build image
docker build -t claude-relay:rust -f Dockerfile.rust .

# Run container
docker run -p 8080:8080 --env-file .env claude-relay:rust
```

### Reverse Proxy

See [docs/DEPLOYMENT.md](../docs/DEPLOYMENT.md) for Nginx/Caddy configuration.

## Contributing

1. Follow Rust formatting guidelines (`cargo fmt`)
2. Ensure all tests pass (`cargo test`)
3. Add tests for new features
4. Update documentation

## License

MIT License - see [LICENSE](../LICENSE)

## Resources

- [Rust Book](https://doc.rust-lang.org/book/)
- [Axum Documentation](https://docs.rs/axum/)
- [Tokio Tutorial](https://tokio.rs/tokio/tutorial)
- [Project Architecture](../docs/ARCHITECTURE.md)
