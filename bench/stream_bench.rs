mod common;

use common::{
    TestMessage, TestResponse, process_streaming_response, stream_to_vec, streaming_request,
    test_utils,
};
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use tokio::runtime::Runtime;

// Benchmark for creating streaming requests
fn bench_streaming_request(c: &mut Criterion) {
    let mut group = c.benchmark_group("streaming_request");

    // Benchmark with different message counts
    for count in [10, 100, 1000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(count), count, |b, &count| {
            b.iter(|| {
                let messages = test_utils::create_test_messages(count);
                streaming_request(messages)
            });
        });
    }

    group.finish();
}

// Benchmark for processing streaming responses
fn bench_process_streaming_response(c: &mut Criterion) {
    let mut group = c.benchmark_group("process_streaming_response");

    let rt = Runtime::new().unwrap();

    // Benchmark with different response counts
    for count in [10, 100, 1000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(count), count, |b, &count| {
            b.iter(|| {
                let responses = (0..count)
                    .map(|i| TestResponse::new(i, format!("Response {}", i)))
                    .collect();

                let stream_response = test_utils::create_stream_response(responses);

                rt.block_on(async {
                    process_streaming_response(stream_response, |_, _| {
                        // No-op callback
                    })
                    .await;
                });
            });
        });
    }

    group.finish();
}

// Benchmark for converting streaming responses to vectors
fn bench_stream_to_vec(c: &mut Criterion) {
    let mut group = c.benchmark_group("stream_to_vec");

    let rt = Runtime::new().unwrap();

    // Benchmark with different response counts
    for count in [10, 100, 1000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(count), count, |b, &count| {
            b.iter(|| {
                let responses = (0..count)
                    .map(|i| TestResponse::new(i, format!("Response {}", i)))
                    .collect();

                let stream_response = test_utils::create_stream_response(responses);

                rt.block_on(async { stream_to_vec(stream_response).await });
            });
        });
    }

    group.finish();
}

// Benchmark for different message sizes
fn bench_message_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("message_sizes");

    // Benchmark with different data sizes
    for &size in &[10, 100, 1000, 10000] {
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            b.iter(|| {
                // Create a message with specified data size
                let data = "x".repeat(size);
                let message = TestMessage::new("1", data);
                let messages = vec![message.clone(); 10];

                streaming_request(messages)
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_streaming_request,
    bench_process_streaming_response,
    bench_stream_to_vec,
    bench_message_sizes
);
criterion_main!(benches);
