use crate::format::{MAGIC, MriHeader, VERSION};
use crate::nifti_io::load_nifti_volume;

use anyhow::{Result, anyhow};
use byteorder::{LittleEndian, ReadBytesExt};
use serde_json;
use std::fs::File;
use std::io::Read;

/// Compute MSE and PSNR between two volumes.
fn mse_psnr(a: &[f32], b: &[f32], peak: f64) -> (f64, f64) {
    let mut mse = 0.0;
    for i in 0..a.len() {
        let d = a[i] - b[i];
        mse += (d * d) as f64;
    }
    mse /= a.len() as f64;

    let psnr = if mse == 0.0 {
        f64::INFINITY
    } else {
        20.0 * (peak / mse.sqrt()).log10()
    };

    (mse, psnr)
}

/// Print compression statistics and optional quality metrics.
pub fn stats_mri(input_path: &str, original_nifti: Option<&str>) -> Result<()> {
    let mut f = File::open(input_path)?;
    let file_len = f.metadata()?.len();

    // --- MAGIC ---
    let mut magic = [0u8; 4];
    f.read_exact(&mut magic)?;
    if magic[..] != MAGIC[..] {
        return Err(anyhow!("Not a MRI file"));
    }

    // --- VERSION ---
    let version = f.read_u16::<LittleEndian>()?;
    if version != VERSION {
        return Err(anyhow!("Unsupported MRI version {}", version));
    }

    // --- HEADER ---
    let header_len = f.read_u32::<LittleEndian>()?;
    let mut header_bytes = vec![0u8; header_len as usize];
    f.read_exact(&mut header_bytes)?;
    let header: MriHeader = serde_json::from_slice(&header_bytes)?;

    let dims = header.dimensions;
    let cs = header.chunk_size;

    let nx = dims[0].div_ceil(cs[0]);
    let ny = dims[1].div_ceil(cs[1]);
    let nz = dims[2].div_ceil(cs[2]);
    let num_chunks = nx * ny * nz;

    println!("=== MRI Statistics ===");
    println!("File:                {}", input_path);
    println!(
        "File Size:          {} bytes ({:.2} MB)",
        file_len,
        file_len as f64 / 1_048_576.0
    );
    println!("Dimensions:         {:?}", dims);
    println!("Chunk Size:         {:?}", cs);
    println!("Chunk Grid:         {} × {} × {}", nx, ny, nz);
    println!("Total Chunks:       {}", num_chunks);

    // --- Chunk table statistics (handling edge chunks properly) ---
    let mut total_comp_bytes = 0u64;
    let mut total_raw_bytes = 0u64;
    let mut zero_chunks = 0usize;

    for ix in 0..nx {
        for iy in 0..ny {
            for iz in 0..nz {
                let _offset = f.read_u64::<LittleEndian>()?;
                let size = f.read_u32::<LittleEndian>()?;
                let _mid = f.read_u16::<LittleEndian>()?;
                let _roi = f.read_u8()?;
                let flags = f.read_u8()?;

                let is_zero = flags & 1 == 1;

                // Compute exact raw chunk size considering boundary edge chunks
                let x0 = ix * cs[0];
                let y0 = iy * cs[1];
                let z0 = iz * cs[2];
                let x1 = (x0 + cs[0]).min(dims[0]);
                let y1 = (y0 + cs[1]).min(dims[1]);
                let z1 = (z0 + cs[2]).min(dims[2]);

                let raw_size = (x1 - x0) * (y1 - y0) * (z1 - z0);
                total_raw_bytes += raw_size as u64;

                if is_zero {
                    zero_chunks += 1;
                } else {
                    total_comp_bytes += size as u64;
                }
            }
        }
    }

    println!("Zero Chunks:        {}", zero_chunks);
    println!("Compressed Bytes:   {}", total_comp_bytes);
    println!("Raw Bytes (est.):   {}", total_raw_bytes);

    let ratio = total_raw_bytes as f64 / total_comp_bytes.max(1) as f64;
    println!("Compression Ratio:  {:.2}×", ratio);

    // --- Optional PSNR/MSE ---
    if let Some(orig_path) = original_nifti {
        println!("\n=== Quality Metrics ===");
        println!("Original NIfTI:     {}", orig_path);

        // Decode MRI fully into memory
        let decoded = crate::decode::decode_mri_to_vec(input_path)?;

        // Load original NIfTI
        let (original_vol, _orig_dims, _orig_spacing) = load_nifti_volume(orig_path)?;

        if original_vol.len() != decoded.len() {
            return Err(anyhow!(
                "Original NIfTI and MRI volume sizes differ: {} vs {}",
                original_vol.len(),
                decoded.len()
            ));
        }

        // Determine correct peak signal range based on header quantization or volume bounds
        let peak = if header.quantized && !header.is_mask {
            let q_min = header.q_min.unwrap() as f64;
            let q_max = header.q_max.unwrap() as f64;
            q_max - q_min
        } else {
            // Fallback to max - min of the original volume
            let min_val = original_vol.iter().cloned().fold(f32::INFINITY, f32::min) as f64;
            let max_val = original_vol
                .iter()
                .cloned()
                .fold(f32::NEG_INFINITY, f32::max) as f64;
            (max_val - min_val).max(1.0)
        };

        let (mse, psnr) = mse_psnr(&decoded, &original_vol, peak);
        println!("MSE:                {:.6}", mse);
        println!("PSNR:               {:.2} dB", psnr);
    }

    Ok(())
}
