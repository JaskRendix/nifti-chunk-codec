### System & Benchmark Results
- **CPU:** Intel Core i5-4210U @ 1.70GHz
- **RAM:** 7.7 GiB
- **Dataset:** Synthetic 3D volume ($128 \times 128 \times 128$, 12-bit quantization)
- **Profile:** Release (Optimized via Criterion)

| Operation | Throughput | Mean Time |
| :--- | :--- | :--- |
| **Encode (12-bit)** | 8.11 MiB/s | 985.7 ms |
| **Decode (12-bit)** | 66.43 MiB/s | 120.4 ms |