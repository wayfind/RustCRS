// Crypto Service Performance Benchmarks
//
// 测试加密解密操作的性能

use claude_relay::utils::crypto::CryptoService;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};

/// Benchmark encryption performance
fn bench_encryption(c: &mut Criterion) {
    let crypto = CryptoService::new("benchmark-encryption-key-32chars!!".to_string());

    let mut group = c.benchmark_group("encryption");

    // 测试不同数据大小的加密性能
    for size in [10, 100, 1_000, 10_000].iter() {
        let data = "x".repeat(*size);

        group.throughput(Throughput::Bytes(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &data, |b, data| {
            b.iter(|| crypto.encrypt(black_box(data)).expect("Encryption failed"));
        });
    }

    group.finish();
}

/// Benchmark decryption performance
fn bench_decryption(c: &mut Criterion) {
    let crypto = CryptoService::new("benchmark-encryption-key-32chars!!".to_string());

    let mut group = c.benchmark_group("decryption");

    // 预先加密数据
    for size in [10, 100, 1_000, 10_000].iter() {
        let data = "x".repeat(*size);
        let encrypted = crypto.encrypt(&data).expect("Pre-encryption failed");

        group.throughput(Throughput::Bytes(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &encrypted, |b, enc| {
            b.iter(|| crypto.decrypt(black_box(enc)).expect("Decryption failed"));
        });
    }

    group.finish();
}

/// Benchmark encryption with cache (decrypt operations)
fn bench_decryption_with_cache(c: &mut Criterion) {
    let crypto = CryptoService::new("benchmark-encryption-key-32chars!!".to_string());

    // 清除缓存
    let _ = CryptoService::clear_cache();

    let mut group = c.benchmark_group("decryption_with_cache");

    let data = "x".repeat(1_000);
    let encrypted = crypto.encrypt(&data).expect("Pre-encryption failed");

    // 第一次解密（缓存未命中）
    group.bench_function("first_decrypt_cache_miss", |b| {
        b.iter(|| {
            let _ = CryptoService::clear_cache(); // 确保每次都是缓存未命中
            crypto
                .decrypt(black_box(&encrypted))
                .expect("Decryption failed")
        });
    });

    // 第二次解密（缓存命中）
    group.bench_function("second_decrypt_cache_hit", |b| {
        // 预热缓存
        let _ = crypto.decrypt(&encrypted);

        b.iter(|| {
            crypto
                .decrypt(black_box(&encrypted))
                .expect("Decryption failed")
        });
    });

    group.finish();
}

/// Benchmark key derivation (Scrypt)
fn bench_key_derivation(c: &mut Criterion) {
    let mut group = c.benchmark_group("key_derivation");

    // Scrypt 密钥派生（CPU 密集型）
    group.bench_function("scrypt_derive_key", |b| {
        b.iter(|| {
            let crypto = CryptoService::new(black_box("benchmark-key-32chars!!".to_string()));
            // derive_key 是私有方法，通过加密操作间接测试
            let _ = crypto.encrypt("test");
        });
    });

    group.finish();
}

criterion_group!(
    crypto_benches,
    bench_encryption,
    bench_decryption,
    bench_decryption_with_cache,
    bench_key_derivation
);
criterion_main!(crypto_benches);
