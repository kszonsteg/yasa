//! Benchmark comparing tract-onnx vs candle implementations
//!
//! Run with: cargo bench --bench value_policy_benchmark

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::time::Duration;

use yasa_core::mcts::evaluation::{
    candle_impl::CandleValuePolicy, tract_impl::TractValuePolicy, NN_BOARD_HEIGHT, NN_BOARD_WIDTH,
    NUM_SPATIAL_LAYERS,
};

/// Create dummy spatial input data for benchmarking
fn create_dummy_spatial_input() -> Vec<f32> {
    let size = NUM_SPATIAL_LAYERS * NN_BOARD_WIDTH * NN_BOARD_HEIGHT;
    let mut data = vec![0.0f32; size];

    // Add some non-zero values to simulate a real game state
    for i in (0..size).step_by(7) {
        data[i] = 1.0;
    }

    data
}

/// Create dummy non-spatial input data for benchmarking
fn create_dummy_non_spatial_input() -> Vec<f32> {
    vec![
        1.0, // half
        5.0, // round
        3.0, // home rerolls
        1.0, // home score
        2.0, // away rerolls
        0.0, // away score
        1.0, // blitz available
        1.0, // pass available
        1.0, // handoff available
        0.0, // foul available
        1.0, // weather: nice
        0.0, // weather: very sunny
        0.0, // weather: pouring rain
        0.0, // weather: blizzard
        0.0, // weather: sweltering heat
    ]
}

fn benchmark_tract_inference(c: &mut Criterion) {
    // Try to load the tract model
    let tract_policy = match TractValuePolicy::new() {
        Ok(policy) => policy,
        Err(e) => {
            eprintln!("Warning: Could not load tract ONNX model: {}", e);
            eprintln!("Skipping tract benchmark. Make sure the model exists at:");
            eprintln!("  {}", TractValuePolicy::DEFAULT_MODEL_PATH);
            return;
        }
    };

    let spatial_input = create_dummy_spatial_input();
    let non_spatial_input = create_dummy_non_spatial_input();

    let mut group = c.benchmark_group("tract_onnx");
    group.throughput(Throughput::Elements(1));
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(100);

    group.bench_function("inference", |b| {
        b.iter(|| {
            tract_policy
                .infer(black_box(&spatial_input), black_box(&non_spatial_input))
                .unwrap()
        })
    });

    group.finish();
}

fn benchmark_candle_inference(c: &mut Criterion) {
    // Try to load the candle model, or use random weights for benchmarking
    let candle_policy = match CandleValuePolicy::new() {
        Ok(policy) => {
            eprintln!("Loaded candle model from SafeTensors");
            policy
        }
        Err(e) => {
            eprintln!("Warning: Could not load candle SafeTensors model: {}", e);
            eprintln!("Using random weights for candle benchmark.");
            eprintln!("To use trained weights, export model to SafeTensors format:");
            eprintln!(
                "  python -m nn.value_network.export --checkpoint <path> --format safetensors"
            );

            match CandleValuePolicy::with_random_weights(&candle_core::Device::Cpu) {
                Ok(policy) => policy,
                Err(e) => {
                    eprintln!(
                        "Error: Could not create candle model with random weights: {}",
                        e
                    );
                    return;
                }
            }
        }
    };

    let spatial_input = create_dummy_spatial_input();
    let non_spatial_input = create_dummy_non_spatial_input();

    let mut group = c.benchmark_group("candle");
    group.throughput(Throughput::Elements(1));
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(100);

    group.bench_function("inference", |b| {
        b.iter(|| {
            candle_policy
                .infer(black_box(&spatial_input), black_box(&non_spatial_input))
                .unwrap()
        })
    });

    group.finish();
}

fn benchmark_comparison(c: &mut Criterion) {
    let spatial_input = create_dummy_spatial_input();
    let non_spatial_input = create_dummy_non_spatial_input();

    // Try to load both models
    let tract_policy = TractValuePolicy::new().ok();
    let candle_policy = CandleValuePolicy::new()
        .or_else(|_| CandleValuePolicy::with_random_weights(&candle_core::Device::Cpu))
        .ok();

    if tract_policy.is_none() && candle_policy.is_none() {
        eprintln!("Warning: No models available for comparison benchmark");
        return;
    }

    let mut group = c.benchmark_group("comparison");
    group.throughput(Throughput::Elements(1));
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(100);

    if let Some(ref policy) = tract_policy {
        group.bench_with_input(BenchmarkId::new("tract-onnx", "single"), &(), |b, _| {
            b.iter(|| {
                policy
                    .infer(black_box(&spatial_input), black_box(&non_spatial_input))
                    .unwrap()
            })
        });
    }

    if let Some(ref policy) = candle_policy {
        group.bench_with_input(BenchmarkId::new("candle", "single"), &(), |b, _| {
            b.iter(|| {
                policy
                    .infer(black_box(&spatial_input), black_box(&non_spatial_input))
                    .unwrap()
            })
        });
    }

    group.finish();
}

fn benchmark_input_creation(c: &mut Criterion) {
    // This benchmarks the overhead of creating input tensors
    // to understand if input preparation is a bottleneck

    let mut group = c.benchmark_group("input_creation");
    group.throughput(Throughput::Elements(1));

    group.bench_function("spatial_input", |b| b.iter(|| create_dummy_spatial_input()));

    group.bench_function("non_spatial_input", |b| {
        b.iter(|| create_dummy_non_spatial_input())
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_tract_inference,
    benchmark_candle_inference,
    benchmark_comparison,
    benchmark_input_creation,
);

criterion_main!(benches);
