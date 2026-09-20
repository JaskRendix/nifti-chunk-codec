# NIFTI-CHUNK-CODEC  
A Rust port of the KMRI format — structure‑aware compression for volumetric MRI data.

This repository is a Rust re‑implementation of the original KMRI project by Kiamehr:  
[https://github.com/Kiamehr5/KMRI](https://github.com/Kiamehr5/KMRI)

The original version explored a simple question:  
what happens if MRI compression stops treating the data as a flat byte stream,  
and instead starts paying attention to the structure of the volume?

NIFTI-CHUNK-CODEC continues that idea, but from a systems‑programming perspective.  
The goal isn’t to chase performance numbers for their own sake.  
It’s to understand how far you can push a format when you control every layer:  
chunking, metadata, quantization, compression, and decoding.

---

## Why this exists

Most MRI pipelines still rely on `.nii.gz`.  
It’s convenient, but it’s also a blunt instrument:  
gzip compresses the entire volume as if it were a text file.

NIFTI-CHUNK-CODEC takes a different approach.  
It splits the volume into spatial chunks,  
detects masks vs intensities,  
applies quantization only where appropriate,  
and compresses each chunk independently.

This makes the format more predictable,  
more tunable,  
and often faster to decode.

The Rust port exists because the original idea deserved a lower‑level implementation.  
Rust makes it easier to reason about memory, binary layout, and throughput.  
It also makes it easier to build a clean CLI, a proper library, and a benchmark suite.

---

## What NIFTI-CHUNK-CODEC does

- Reads and writes `.mri` files  
- Encodes NIfTI volumes into chunked MRI containers  
- Decodes MRI back into NIfTI  
- Supports optional N‑bit quantization (8–16 bits)  
- Preserves segmentation masks losslessly  
- Detects sparse chunks and skips them  
- Applies ROI‑aware compression strategies  
- Provides tools for inspecting, verifying, and extracting chunks  
- Includes a synthetic MRI generator for testing  
- Includes Criterion benchmarks for throughput and latency

The format is simple:  
magic bytes, version, JSON header, chunk table, chunk payloads.  
Everything is explicit.  
Nothing is hidden behind opaque libraries.

---

## What makes this interesting

NIFTI-CHUNK-CODEC isn’t trying to replace `.nii.gz`.  
It’s trying to explore a different design space.

Chunking changes how you think about compression.  
Quantization changes how you think about fidelity.  
ROI detection changes how you think about importance.  
Sparse skipping changes how you think about empty space.  
Throughput benchmarks change how you think about real‑world performance.

The project is a way to connect all these dots.  
It’s a compression format, but also a study in how structure affects data.

---

## How it works

### Encoding

1. Load NIfTI  
2. Detect modality (mask vs intensity)  
3. Split into 3D chunks  
4. Quantize intensities (optional)  
5. Compress each chunk  
6. Write header + chunk table + payloads

### Decoding

1. Read header  
2. Read chunk table  
3. Decompress chunks  
4. Reconstruct volume  
5. Dequantize if needed  
6. Save NIfTI

Everything is explicit and implemented in Rust.

---

## Benchmarks

NIFTI-CHUNK-CODEC includes Criterion benchmarks for:

- full‑pipeline encode/decode  
- throughput (MiB/s)  
- real datasets (if available)  
- component‑level placeholders (quantization, compression)

The synthetic generator produces a stable test volume,  
so benchmarks are reproducible.

---

## Project structure

```
src/
  bin/
    mri.rs          CLI
    generate.rs      synthetic MRI generator
  encode.rs          encoder
  decode.rs          decoder
  decompress.rs      chunk decompression
  compress.rs        chunk compression
  quantize.rs        quantization
  chunk.rs           chunk utilities
  format.rs          header + metadata
  stats.rs           compression + quality metrics
  nifti_io.rs        NIfTI reader/writer
  lib.rs             library entry point
benchmarks/
Cargo.toml
README.md
```

---

## License

BSD‑3 (same as the original KMRI project).
