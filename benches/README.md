### System & Benchmark Results
- **CPU:** Intel Core i5-4210U @ 1.70GHz
- **RAM:** 7.7 GiB
- **Dataset:** Synthetic 3D volume (128 × 128 × 128, 12-bit quantization)
- **Profile:** Release (Criterion benchmark)

| Operation | Throughput | Mean Time |
| :--- | :--- | :--- |
| **Encode (12-bit)** | ~14.2 MiB/s | ~563 ms |
| **Decode (12-bit)** | ~45.5 MiB/s | ~176 ms |