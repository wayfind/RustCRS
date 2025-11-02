// Serialization Performance Benchmarks
//
// 测试 JSON 序列化和反序列化性能

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
struct SimpleMessage {
    id: String,
    role: String,
    content: String,
}

#[derive(Serialize, Deserialize, Clone)]
struct ComplexMessage {
    id: String,
    r#type: String,
    role: String,
    content: Vec<Content>,
    model: String,
    stop_reason: Option<String>,
    usage: Usage,
}

#[derive(Serialize, Deserialize, Clone)]
struct Content {
    r#type: String,
    text: String,
}

#[derive(Serialize, Deserialize, Clone)]
struct Usage {
    input_tokens: u64,
    output_tokens: u64,
    cache_creation_input_tokens: u64,
    cache_read_input_tokens: u64,
}

/// Benchmark simple JSON serialization
fn bench_simple_serialization(c: &mut Criterion) {
    let message = SimpleMessage {
        id: "msg_123".to_string(),
        role: "assistant".to_string(),
        content: "Hello! How can I help you today?".to_string(),
    };

    c.bench_function("simple_serialize", |b| {
        b.iter(|| serde_json::to_string(black_box(&message)).expect("Serialization failed"));
    });

    let json = serde_json::to_string(&message).unwrap();
    c.bench_function("simple_deserialize", |b| {
        b.iter(|| {
            serde_json::from_str::<SimpleMessage>(black_box(&json)).expect("Deserialization failed")
        });
    });
}

/// Benchmark complex JSON serialization
fn bench_complex_serialization(c: &mut Criterion) {
    let message = ComplexMessage {
        id: "msg_456".to_string(),
        r#type: "message".to_string(),
        role: "assistant".to_string(),
        content: vec![Content {
            r#type: "text".to_string(),
            text: "This is a longer response with multiple parts. ".repeat(10),
        }],
        model: "claude-3-5-sonnet-20241022".to_string(),
        stop_reason: Some("end_turn".to_string()),
        usage: Usage {
            input_tokens: 100,
            output_tokens: 500,
            cache_creation_input_tokens: 200,
            cache_read_input_tokens: 50,
        },
    };

    c.bench_function("complex_serialize", |b| {
        b.iter(|| serde_json::to_string(black_box(&message)).expect("Serialization failed"));
    });

    let json = serde_json::to_string(&message).unwrap();
    c.bench_function("complex_deserialize", |b| {
        b.iter(|| {
            serde_json::from_str::<ComplexMessage>(black_box(&json))
                .expect("Deserialization failed")
        });
    });
}

/// Benchmark streaming JSON parsing (SSE events)
fn bench_sse_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("sse_parsing");

    // 模拟 SSE 数据块
    let sse_events = [
        r#"event: message_start
data: {"type":"message_start","message":{"id":"msg_123","type":"message","role":"assistant","content":[],"model":"claude-3-5-sonnet-20241022","usage":{"input_tokens":10,"output_tokens":0}}}
"#,
        r#"event: content_block_delta
data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hello"}}
"#,
        r#"event: message_delta
data: {"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":50}}
"#,
    ];

    for (idx, event) in sse_events.iter().enumerate() {
        group.bench_with_input(BenchmarkId::from_parameter(idx), event, |b, event| {
            b.iter(|| {
                // 提取 data 行
                if let Some(data_line) = event.lines().find(|line| line.starts_with("data: ")) {
                    let json = &data_line[6..]; // 移除 "data: " 前缀
                    let _: serde_json::Value =
                        serde_json::from_str(black_box(json)).expect("SSE parsing failed");
                }
            });
        });
    }

    group.finish();
}

/// Benchmark large array serialization
fn bench_large_array(c: &mut Criterion) {
    let mut group = c.benchmark_group("large_array");

    for count in [10, 100, 1_000].iter() {
        let messages: Vec<SimpleMessage> = (0..*count)
            .map(|i| SimpleMessage {
                id: format!("msg_{}", i),
                role: "user".to_string(),
                content: format!("Message {}", i),
            })
            .collect();

        group.throughput(Throughput::Elements(*count as u64));
        group.bench_with_input(BenchmarkId::from_parameter(count), &messages, |b, msgs| {
            b.iter(|| serde_json::to_string(black_box(msgs)).expect("Serialization failed"));
        });
    }

    group.finish();
}

criterion_group!(
    serialization_benches,
    bench_simple_serialization,
    bench_complex_serialization,
    bench_sse_parsing,
    bench_large_array
);
criterion_main!(serialization_benches);
