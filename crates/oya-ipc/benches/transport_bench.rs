//! Performance benchmarks for IPC transport
//!
//! Targets (median latency):
//! - send() 1KB: <2µs
//! - recv() 1KB: <3µs
//! - round-trip 1KB: <5µs
//! - send() 100KB: <20µs

#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use oya_ipc::IpcTransport;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct TestMessage {
    data: String,
}

fn create_message_of_size(size: usize) -> TestMessage {
    TestMessage {
        data: "x".repeat(size),
    }
}

fn bench_send_1kb_message(c: &mut Criterion) {
    let (mut client, _server) = IpcTransport::transport_pair();
    let msg = create_message_of_size(1024);

    c.bench_function("send_1kb", |b| b.iter(|| client.send(black_box(&msg))));
}

fn bench_recv_1kb_message(c: &mut Criterion) {
    let (mut client, mut server) = IpcTransport::transport_pair();
    let msg = create_message_of_size(1024);

    // Pre-send message
    client.send(&msg).unwrap();

    c.bench_function("recv_1kb", |b| {
        b.iter(|| {
            let result = server.recv::<TestMessage>().unwrap();
            // Re-send for next iteration
            client.send(&result).unwrap();
            result
        })
    });
}

fn bench_roundtrip_1kb(c: &mut Criterion) {
    c.bench_function("roundtrip_1kb", |b| {
        b.iter(|| {
            let (mut client, mut server) = IpcTransport::transport_pair();
            let msg = create_message_of_size(1024);

            client.send(black_box(&msg)).unwrap();
            let received = server.recv::<TestMessage>().unwrap();

            received
        })
    });
}

fn bench_send_100kb_message(c: &mut Criterion) {
    let (mut client, _server) = IpcTransport::transport_pair();
    let msg = create_message_of_size(100_000);

    c.bench_function("send_100kb", |b| b.iter(|| client.send(black_box(&msg))));
}

fn bench_send_various_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("send_various_sizes");

    for size in [16, 64, 256, 1024, 4096, 16_384, 65_536, 262_144].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let (mut client, _server) = IpcTransport::transport_pair();
            let msg = create_message_of_size(size);

            b.iter(|| client.send(black_box(&msg)))
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_send_1kb_message,
    bench_recv_1kb_message,
    bench_roundtrip_1kb,
    bench_send_100kb_message,
    bench_send_various_sizes
);

criterion_main!(benches);
