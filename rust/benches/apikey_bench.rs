// API Key Service Performance Benchmarks
//
// 测试 API Key 验证和操作的性能

use claude_relay::models::api_key::{ApiKeyCreateOptions, ApiKeyPermissions, ExpirationMode};
use claude_relay::services::ApiKeyService;
use claude_relay::{RedisPool, Settings};
use criterion::{black_box, criterion_group, criterion_main, Criterion};

/// 创建测试用的 ApiKeyService
async fn create_test_service() -> ApiKeyService {
    // 加载配置
    let settings = Settings::new().expect("Failed to load settings");
    let pool = RedisPool::new(&settings).expect("Failed to create Redis pool");

    ApiKeyService::new(pool, settings)
}

/// Benchmark API Key creation
fn bench_key_creation(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let service = rt.block_on(create_test_service());

    c.bench_function("create_api_key", |b| {
        b.iter(|| {
            rt.block_on(async {
                let options = ApiKeyCreateOptions {
                    name: "benchmark-key".to_string(),
                    description: None,
                    icon: None,
                    permissions: ApiKeyPermissions::All,
                    is_active: true,
                    token_limit: 0,
                    concurrency_limit: 0,
                    rate_limit_window: None,
                    rate_limit_requests: Some(1000),
                    rate_limit_cost: None,
                    daily_cost_limit: 0.0,
                    total_cost_limit: 0.0,
                    weekly_opus_cost_limit: 0.0,
                    enable_model_restriction: false,
                    restricted_models: vec![],
                    enable_client_restriction: false,
                    allowed_clients: vec![],
                    tags: vec![],
                    expiration_mode: ExpirationMode::Fixed,
                    activation_days: 0,
                    activation_unit: Default::default(),
                    expires_at: None,
                    claude_account_id: None,
                    claude_console_account_id: None,
                    gemini_account_id: None,
                    openai_account_id: None,
                    azure_openai_account_id: None,
                    bedrock_account_id: None,
                    droid_account_id: None,
                    user_id: None,
                    created_by: None,
                    created_by_type: None,
                };

                service
                    .generate_key(black_box(options))
                    .await
                    .expect("Failed to create key")
            })
        });
    });
}

/// Benchmark API Key validation (hash lookup)
fn bench_key_validation(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let service = rt.block_on(create_test_service());

    // 预先创建一个 key
    let (api_key, _) = rt.block_on(async {
        let options = ApiKeyCreateOptions {
            name: "validation-bench-key".to_string(),
            description: None,
            icon: None,
            permissions: ApiKeyPermissions::All,
            is_active: true,
            token_limit: 0,
            concurrency_limit: 0,
            rate_limit_window: None,
            rate_limit_requests: Some(1000),
            rate_limit_cost: None,
            daily_cost_limit: 0.0,
            total_cost_limit: 0.0,
            weekly_opus_cost_limit: 0.0,
            enable_model_restriction: false,
            restricted_models: vec![],
            enable_client_restriction: false,
            allowed_clients: vec![],
            tags: vec![],
            expiration_mode: ExpirationMode::Fixed,
            activation_days: 0,
            activation_unit: Default::default(),
            expires_at: None,
            claude_account_id: None,
            claude_console_account_id: None,
            gemini_account_id: None,
            openai_account_id: None,
            azure_openai_account_id: None,
            bedrock_account_id: None,
            droid_account_id: None,
            user_id: None,
            created_by: None,
            created_by_type: None,
        };

        service
            .generate_key(options)
            .await
            .expect("Failed to create key")
    });

    c.bench_function("validate_api_key", |b| {
        b.iter(|| {
            rt.block_on(async {
                service
                    .validate_key(black_box(&api_key))
                    .await
                    .expect("Validation failed")
            })
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
