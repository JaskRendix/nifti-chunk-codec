use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use nifti_chunk_codec_rs::{decode_mri, encode_mri};
use std::hint::black_box;
use std::path::Path;
use std::time::Duration;

fn bench_pipeline_throughput(c: &mut Criterion) {
    let input_path = "synthetic_realistic_hardcore.nii";

    if !Path::new(input_path).exists() {
        println!(
            "\n[Notice]: Pipeline benchmark skipped because '{}' was not found.",
            input_path
        );
        return;
    }

    let metadata = std::fs::metadata(input_path).unwrap();
    let file_bytes = metadata.len();

    let mri_path = "bench_throughput.mri";
    let decoded_path = "bench_throughput.nii";

    let mut group = c.benchmark_group("MRI Pipeline Throughput");

    // Configure warmup and sample size to prevent timing warnings
    group.warm_up_time(Duration::from_secs(1));
    group.sample_size(20);

    // Enable automated MB/s calculation via Criterion's Throughput metric
    group.throughput(Throughput::Bytes(file_bytes));

    group.bench_function("encode_12bit", |b| {
        b.iter(|| {
            encode_mri(
                black_box(input_path),
                black_box(mri_path),
                black_box(12),
                black_box([32, 32, 32]),
                black_box([64, 64, 64]),
                black_box(200),
                black_box(3),
                black_box(1),
            )
            .unwrap();
        });
    });

    // Pre-encode once to isolate decoding benchmark performance
    encode_mri(
        input_path,
        mri_path,
        12,
        [32, 32, 32],
        [64, 64, 64],
        200,
        3,
        1,
    )
    .expect("Failed to pre-encode MRI file for decoding benchmark");

    group.bench_function("decode_12bit", |b| {
        b.iter(|| {
            decode_mri(black_box(mri_path), black_box(decoded_path)).unwrap();
        });
    });

    group.finish();

    // Clean up temporary files
    let _ = std::fs::remove_file(mri_path);
    let _ = std::fs::remove_file(decoded_path);
}

fn bench_real_datasets(c: &mut Criterion) {
    let potential_datasets = ["synthetic_realistic_hardcore.nii"];
    let mut found_any = false;

    for dataset in &potential_datasets {
        if Path::new(dataset).exists() {
            found_any = true;
            let bench_name = format!("encode_real_{}", dataset.replace(['/', '.'], "_"));
            let out_path = format!("bench_{}.mri", dataset.replace(['/', '.'], "_"));

            let mut group = c.benchmark_group("Real Dataset Encodings");
            group.sample_size(20);

            group.bench_function(&bench_name, |b| {
                b.iter(|| {
                    encode_mri(
                        black_box(dataset),
                        black_box(&out_path),
                        black_box(12),
                        black_box([32, 32, 32]),
                        black_box([64, 64, 64]),
                        black_box(200),
                        black_box(3),
                        black_box(1),
                    )
                    .unwrap();
                });
            });

            group.finish();
            let _ = std::fs::remove_file(out_path);
        }
    }

    if !found_any {
        println!("\n[Notice]: No datasets found. Skipping dataset benchmarks.\n");
    }
}

criterion_group!(benches, bench_pipeline_throughput, bench_real_datasets);
criterion_main!(benches);
