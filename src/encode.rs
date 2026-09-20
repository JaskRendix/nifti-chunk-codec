#![allow(clippy::too_many_arguments)]
#![allow(clippy::type_complexity)]
#![allow(clippy::seek_from_current)]

use crate::chunk::ceil_div;
use crate::compress::compress_chunk;
use crate::format::{ChunkEntry, MAGIC, MriHeader, VERSION};
use crate::nifti_io::load_nifti_volume;
use crate::quantize::{QuantParams, quantize_volume};

use anyhow::Result;
use byteorder::{LittleEndian, WriteBytesExt};
use serde_json;
use std::collections::HashSet;
use std::fs::File;
use std::io::{Seek, SeekFrom, Write};

/// Encode a NIfTI file into MRI format.
pub fn encode_mri(
    input_path: &str,
    output_path: &str,
    bits: u8,
    chunk_size_mri: [usize; 3],
    chunk_size_mask: [usize; 3],
    roi_threshold: u16,
    roi_level: i32,
    bg_level: i32,
) -> Result<()> {
    // Load NIfTI
    let (volume_f32, dims, voxel_size) = load_nifti_volume(input_path)?;
    let (_x, y, z) = (dims[0], dims[1], dims[2]);

    // Detect mask vs intensity
    let unique_vals: HashSet<u32> = volume_f32.iter().map(|v| *v as u32).collect();
    let is_mask =
        unique_vals.len() <= 16 && unique_vals.iter().all(|v| *v == (*v as f32).round() as u32);

    // Unified type: always store chunks as u16
    let (dtype_str, quantized, q_bits, q_min, q_max, chunk_size, raw_u16): (
        String,
        bool,
        Option<u8>,
        Option<f32>,
        Option<f32>,
        [usize; 3],
        Vec<u16>,
    ) = if is_mask {
        let vol_u16: Vec<u16> = volume_f32.iter().map(|v| *v as u16).collect();
        (
            "uint8".to_string(),
            false,
            None,
            None,
            None,
            chunk_size_mask,
            vol_u16,
        )
    } else {
        let q_bits = bits;
        let q_min = percentile(&volume_f32, 0.5);
        let q_max = percentile(&volume_f32, 99.5).max(q_min + 1.0);

        let params = QuantParams {
            bits: q_bits,
            q_min,
            q_max,
        };
        let vol_q = quantize_volume(&volume_f32, &params);

        (
            "uint16".to_string(),
            true,
            Some(q_bits),
            Some(q_min),
            Some(q_max),
            chunk_size_mri,
            vol_q,
        )
    };

    let (cx, cy, cz) = (chunk_size[0], chunk_size[1], chunk_size[2]);
    let nx = ceil_div(dims[0], cx);
    let ny = ceil_div(y, cy);
    let nz = ceil_div(z, cz);

    // Build header
    let header = MriHeader {
        dimensions: dims,
        voxel_size,
        dtype: dtype_str.clone(),
        endianness: "little".to_string(),
        modalities: vec!["T1".to_string()],
        chunk_size,
        compression: "zstd".to_string(),
        is_mask,
        quantized,
        q_bits,
        q_min,
        q_max,
    };

    let header_bytes = serde_json::to_vec(&header)?;
    let header_len = header_bytes.len() as u32;

    // Prepare output file
    let mut f = File::create(output_path)?;
    f.write_all(MAGIC)?;
    f.write_u16::<LittleEndian>(VERSION)?;
    f.write_u32::<LittleEndian>(header_len)?;
    f.write_all(&header_bytes)?;

    // Reserve chunk table space
    let num_chunks = nx * ny * nz;
    let table_offset = f.stream_position()?;
    let entry_size = std::mem::size_of::<ChunkEntry>() as u64;

    let reserve_bytes: i64 = ((num_chunks as u64) * entry_size)
        .try_into()
        .expect("chunk table size overflow");
    f.seek(SeekFrom::Current(reserve_bytes))?;

    // Chunk encoding
    let mut chunk_table: Vec<ChunkEntry> = Vec::with_capacity(num_chunks);
    let mut offset = f.stream_position()?;

    for ix in 0..nx {
        for iy in 0..ny {
            for iz in 0..nz {
                let x0 = ix * cx;
                let y0 = iy * cy;
                let z0 = iz * cz;

                let x1 = (x0 + cx).min(dims[0]);
                let y1 = (y0 + cy).min(y);
                let z1 = (z0 + cz).min(z);

                let chunk = extract_chunk(&raw_u16, dims, [x0, y0, z0], [x1, y1, z1]);

                let is_zero = chunk.iter().all(|v| *v == 0);

                let roi_flag = if is_zero {
                    0
                } else if chunk.iter().any(|v| *v > roi_threshold) {
                    1
                } else {
                    0
                };

                let compressed = if is_zero {
                    Vec::new()
                } else {
                    let level = if roi_flag == 1 { roi_level } else { bg_level };
                    compress_chunk(&chunk_as_bytes(&chunk), level)?
                };

                let size = compressed.len() as u32;
                let flags = if is_zero { 1 } else { 0 };

                if size > 0 {
                    f.write_all(&compressed)?;
                }

                chunk_table.push(ChunkEntry {
                    offset: if size > 0 { offset } else { 0 },
                    size,
                    mid: 0,
                    roi: roi_flag,
                    flags,
                });

                offset += size as u64;
            }
        }
    }

    // Write chunk table
    f.seek(SeekFrom::Start(table_offset))?;
    for entry in &chunk_table {
        f.write_u64::<LittleEndian>(entry.offset)?;
        f.write_u32::<LittleEndian>(entry.size)?;
        f.write_u16::<LittleEndian>(entry.mid)?;
        f.write_u8(entry.roi)?;
        f.write_u8(entry.flags)?;
    }

    Ok(())
}

/// Convert Vec<u16> chunk to &[u8] for compression
fn chunk_as_bytes(chunk: &[u16]) -> Vec<u8> {
    let mut out = Vec::with_capacity(chunk.len() * 2);
    for v in chunk {
        out.extend_from_slice(&v.to_le_bytes());
    }
    out
}

/// Extract a chunk from a flat 3D volume buffer.
fn extract_chunk(raw: &[u16], dims: [usize; 3], start: [usize; 3], end: [usize; 3]) -> Vec<u16> {
    let (_x, y, z) = (dims[0], dims[1], dims[2]);
    let mut out = Vec::new();

    for xi in start[0]..end[0] {
        for yi in start[1]..end[1] {
            for zi in start[2]..end[2] {
                let idx = xi * y * z + yi * z + zi;
                out.push(raw[idx]);
            }
        }
    }

    out
}

/// Compute percentile (simple implementation).
fn percentile(data: &[f32], pct: f32) -> f32 {
    let mut v = data.to_vec();
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let idx = ((pct / 100.0) * (v.len() as f32)) as usize;
    v[idx.min(v.len() - 1)]
}
