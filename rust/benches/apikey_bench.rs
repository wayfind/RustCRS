// API Key Service Performance Benchmarks
//
// 测试 API Key 验证和操作的性能

use claude_relay::models::api_key::{ApiKeyCreateOptions, ApiKeyPermissions, ExpirationMode};
use claude_relay::services::ApiKeyService;
use claude_relay::{RedisPool, Settings};
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::sync::Arc;

/// 创建测试用的 ApiKeyService
async fn create_test_service() -> ApiKeyService {
    // 使用内存配置（不连接实际 Redis）
    let settings = Settings::default();
    let pool = RedisPool::default();

    ApiKeyService::new(Arc::new(pool), settings.security.encryption_key.clone())
}

/// Benchmark API Key creation
fn bench_key_creation(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let service = rt.block_on(create_test_service());

    c.bench_function("create_api_key", |b| {
        b.to_async(&rt).iter(|| async {
            let options = ApiKeyCreateOptions {
                name: "benchmark-key".to_string(),
                rate_limit: Some(1000),
                permissions: Some(ApiKeyPermissions::all()),
                expiration: ExpirationMode::NoExpiration,
                cost_limit: None,
                model_blacklist: None,
                client_restrictions: None,
            };

            service
                .create_key(black_box(options), "benchmark")
                .await
                .expect("Failed to create key")
        });
    });
}

/// Benchmark API Key validation (hash lookup)
fn bench_key_validation(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let service = rt.block_on(create_test_service());

    // 预先创建一个 key
    let api_key = rt.block_on(async {
        let options = ApiKeyCreateOptions {
            name: "validation-bench-key".to_string(),
            rate_limit: Some(1000),
            permissions: Some(ApiKeyPermissions::all()),
            expiration: ExpirationMode::NoExpiration,
            cost_limit: None,
            model_blacklist: None,
            client_restrictions: None,
        };

        service
            .create_key(options, "benchmark")
            .await
            .expect("Failed to create key")
    });

    c.bench_function("validate_api_key", |b| {
        b.to_async(&rt).iter(|| async {
            service
                .validate_key(black_box(&api_key))
                .await
                .expect("Validation failed")
        });
    });
}

/// Benchmark hash computation
fn bench_hash_computation(c: &mut Criterion) {
    use sha2::{Digest, Sha256};

    c.bench_function("sha256_hash", |b| {
        let data = "cr_1234567890abcdef1234567890abcdef";

        b.iter(|| {
            let mut hasher = Sha256::new();
            hasher.update(black_box(data.as_bytes()));
            hex::encode(hasher.finalize())
        });
    });
}

criterion_group!(
    apikey_benches,
    bench_key_creation,
    bench_key_validation,
    bench_hash_computation
);
criterion_main!(apikey_benches);
