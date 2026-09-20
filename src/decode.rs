use crate::decompress::decompress_chunk;
use crate::format::{ChunkEntry, MAGIC, MriHeader, VERSION};
use crate::nifti_io::save_nifti_volume;

use anyhow::{Result, anyhow};
use byteorder::{LittleEndian, ReadBytesExt};
use serde_json;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};

/// Dump a specific chunk from a MRI file by its 3D grid index.
pub fn dump_chunk(input_path: &str, index: [usize; 3], output_path: &str) -> Result<()> {
    let mut f = File::open(input_path)?;

    // --- MAGIC ---
    let mut magic = [0u8; 4];
    f.read_exact(&mut magic)?;
    if magic[..] != MAGIC[..] {
        return Err(anyhow!("Not a MRI file"));
    }

    // --- VERSION ---
    let version = f.read_u16::<LittleEndian>()?;
    if version != VERSION {
        return Err(anyhow!("Unsupported MRI version: {}", version));
    }

    // --- HEADER ---
    let header_len = f.read_u32::<LittleEndian>()?;
    let mut header_bytes = vec![0u8; header_len as usize];
    f.read_exact(&mut header_bytes)?;
    let header: MriHeader = serde_json::from_slice(&header_bytes)?;

    let dims = header.dimensions;
    let chunk_size = header.chunk_size;

    let (cx, cy, cz) = (chunk_size[0], chunk_size[1], chunk_size[2]);
    let (nx, ny, nz) = (
        ceil_div(dims[0], cx),
        ceil_div(dims[1], cy),
        ceil_div(dims[2], cz),
    );

    // --- Validate index ---
    let [ix, iy, iz] = index;
    if ix >= nx || iy >= ny || iz >= nz {
        return Err(anyhow!(
            "Chunk index {:?} out of bounds for grid [{}, {}, {}]",
            index,
            nx,
            ny,
            nz
        ));
    }

    // --- Compute flat index ---
    let table_idx = ix * ny * nz + iy * nz + iz;

    // --- Chunk table offset ---
    let table_start = 4 + 2 + 4 + header_len as u64; // MAGIC + VERSION + HEADER_LEN + HEADER_JSON
    let entry_size = 8 + 4 + 2 + 1 + 1; // offset + size + mid + roi + flags = 16 bytes
    let entry_offset = table_start + (table_idx as u64 * entry_size);

    f.seek(SeekFrom::Start(entry_offset))?;

    let offset = f.read_u64::<LittleEndian>()?;
    let size = f.read_u32::<LittleEndian>()?;
    let _mid = f.read_u16::<LittleEndian>()?;
    let _roi = f.read_u8()?;
    let flags = f.read_u8()?;

    // --- Compute chunk shape ---
    let x0 = ix * cx;
    let y0 = iy * cy;
    let z0 = iz * cz;

    let x1 = (x0 + cx).min(dims[0]);
    let y1 = (y0 + cy).min(dims[1]);
    let z1 = (z0 + cz).min(dims[2]);

    let shape = [x1 - x0, y1 - y0, z1 - z0];
    let bytes_per_voxel = if header.quantized && !header.is_mask {
        2
    } else {
        1
    };
    let expected_size = shape[0] * shape[1] * shape[2] * bytes_per_voxel;

    let is_zero = flags & 1 == 1;

    // --- Read or synthesize chunk ---
    let chunk_data: Vec<u8> = if is_zero || size == 0 {
        vec![0u8; expected_size]
    } else {
        f.seek(SeekFrom::Start(offset))?;
        let mut comp = vec![0u8; size as usize];
        f.read_exact(&mut comp)?;
        decompress_chunk(&comp, expected_size)?
    };

    // --- Write raw chunk bytes ---
    std::fs::write(output_path, &chunk_data)?;

    println!(
        "Dumped chunk {:?} (shape {:?}, {} bytes) → {}",
        index,
        shape,
        chunk_data.len(),
        output_path
    );

    Ok(())
}

/// Decode a MRI file into a NIfTI volume.
pub fn decode_mri(input_path: &str, output_path: &str) -> Result<()> {
    let mut f = File::open(input_path)?;

    // MAGIC
    let mut magic = [0u8; 3];
    f.read_exact(&mut magic)?;
    if magic[..] != MAGIC[..] {
        return Err(anyhow!("Not a MRI file"));
    }

    // VERSION
    let version = f.read_u16::<LittleEndian>()?;
    if version != VERSION {
        return Err(anyhow!("Unsupported MRI version: {}", version));
    }

    // HEADER
    let header_len = f.read_u32::<LittleEndian>()?;
    let mut header_bytes = vec![0u8; header_len as usize];
    f.read_exact(&mut header_bytes)?;

    let header: MriHeader = serde_json::from_slice(&header_bytes)?;

    let dims = header.dimensions;
    let (_x, y, z) = (dims[0], dims[1], dims[2]);

    let chunk_size = header.chunk_size;
    let (cx, cy, cz) = (chunk_size[0], chunk_size[1], chunk_size[2]);

    let nx = ceil_div(dims[0], cx);
    let ny = ceil_div(y, cy);
    let nz = ceil_div(z, cz);
    let num_chunks = nx * ny * nz;

    // Read chunk table
    let mut table: Vec<ChunkEntry> = Vec::with_capacity(num_chunks);

    for _ in 0..num_chunks {
        let offset = f.read_u64::<LittleEndian>()?;
        let size = f.read_u32::<LittleEndian>()?;
        let mid = f.read_u16::<LittleEndian>()?;
        let roi = f.read_u8()?;
        let flags = f.read_u8()?;

        table.push(ChunkEntry {
            offset,
            size,
            mid,
            roi,
            flags,
        });
    }

    // Allocate output volume
    let mut vol = vec![0f32; dims[0] * y * z];
    let bytes_per_voxel = if header.quantized && !header.is_mask {
        2
    } else {
        1
    };

    // Chunk reconstruction
    let mut idx = 0;
    for ix in 0..nx {
        for iy in 0..ny {
            for iz in 0..nz {
                let entry = &table[idx];
                idx += 1;

                let x0 = ix * cx;
                let y0 = iy * cy;
                let z0 = iz * cz;

                let x1 = (x0 + cx).min(dims[0]);
                let y1 = (y0 + cy).min(y);
                let z1 = (z0 + cz).min(z);

                let shape = [x1 - x0, y1 - y0, z1 - z0];
                let expected_size = shape[0] * shape[1] * shape[2] * bytes_per_voxel;

                let is_zero = entry.flags & 1 == 1;

                let chunk_data: Vec<u8> = if is_zero || entry.size == 0 {
                    vec![0u8; expected_size]
                } else {
                    f.seek(SeekFrom::Start(entry.offset))?;
                    let mut comp = vec![0u8; entry.size as usize];
                    f.read_exact(&mut comp)?;
                    decompress_chunk(&comp, expected_size)?
                };

                write_chunk_into_volume(
                    &mut vol,
                    dims,
                    [x0, y0, z0],
                    shape,
                    &chunk_data,
                    bytes_per_voxel == 2,
                );
            }
        }
    }

    // Dequantization
    let out_vol = if header.quantized && !header.is_mask {
        let bits = header.q_bits.unwrap();
        let q_min = header.q_min.unwrap();
        let q_max = header.q_max.unwrap();
        let levels = (1u32 << bits) - 1;

        vol.iter()
            .map(|&v| {
                let q = v / (levels as f32);
                q * (q_max - q_min) + q_min
            })
            .collect::<Vec<f32>>()
    } else {
        vol
    };

    save_nifti_volume(&out_vol, dims, output_path)?;
    Ok(())
}

/// Write a chunk into the full 3D volume buffer.
fn write_chunk_into_volume(
    vol: &mut [f32],
    dims: [usize; 3],
    start: [usize; 3],
    shape: [usize; 3],
    chunk: &[u8],
    is_u16: bool,
) {
    let (_x, y, z) = (dims[0], dims[1], dims[2]);

    let mut voxel_idx = 0;
    for xi in 0..shape[0] {
        for yi in 0..shape[1] {
            for zi in 0..shape[2] {
                let gx = start[0] + xi;
                let gy = start[1] + yi;
                let gz = start[2] + zi;

                let global_idx = gx * y * z + gy * z + gz;

                if is_u16 {
                    let byte_idx = voxel_idx * 2;
                    let val = u16::from_le_bytes([chunk[byte_idx], chunk[byte_idx + 1]]);
                    vol[global_idx] = val as f32;
                } else {
                    vol[global_idx] = chunk[voxel_idx] as f32;
                }
                voxel_idx += 1;
            }
        }
    }
}

/// Integer ceil division using Rust's built-in div_ceil
fn ceil_div(a: usize, b: usize) -> usize {
    a.div_ceil(b)
}

/// Decode a MRI file directly into an in-memory Vec<f32> volume using a temporary NIfTI file.
pub fn decode_mri_to_vec(input_path: &str) -> Result<Vec<f32>> {
    let temp_path = std::env::temp_dir().join(format!("mri_temp_{}.nii", std::process::id()));
    let temp_path_str = temp_path
        .to_str()
        .ok_or_else(|| anyhow!("Invalid temp path"))?;

    decode_mri(input_path, temp_path_str)?;

    let load_result = crate::nifti_io::load_nifti_volume(temp_path_str);

    let _ = std::fs::remove_file(&temp_path);

    let (vol, _, _) = load_result?;
    Ok(vol)
}
