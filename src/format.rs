use anyhow::{Result, anyhow};
use byteorder::{LittleEndian, ReadBytesExt};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Read;

pub const MAGIC: &[u8; 3] = b"MRI";
pub const VERSION: u16 = 1;

#[derive(Debug, Serialize, Deserialize)]
pub struct MriHeader {
    pub dimensions: [usize; 3],
    pub voxel_size: [f32; 3],
    pub dtype: String,
    pub endianness: String,
    pub modalities: Vec<String>,
    pub chunk_size: [usize; 3],
    pub compression: String,
    pub is_mask: bool,
    pub quantized: bool,
    pub q_bits: Option<u8>,
    pub q_min: Option<f32>,
    pub q_max: Option<f32>,
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct ChunkEntry {
    pub offset: u64,
    pub size: u32,
    pub mid: u16,
    pub roi: u8,
    pub flags: u8,
}

/// Inspect and print metadata/header information of a MRI file.
pub fn inspect_mri(input_path: &str) -> Result<()> {
    let mut f = File::open(input_path)?;
    let metadata = f.metadata()?;

    // Read and validate MAGIC
    let mut magic = [0u8; 4];
    f.read_exact(&mut magic)?;
    if magic[..] != MAGIC[..] {
        return Err(anyhow!("Not a valid MRI file (bad magic bytes)"));
    }

    // Read and validate VERSION
    let version = f.read_u16::<LittleEndian>()?;

    // Read HEADER
    let header_len = f.read_u32::<LittleEndian>()?;
    let mut header_bytes = vec![0u8; header_len as usize];
    f.read_exact(&mut header_bytes)?;
    let header: MriHeader = serde_json::from_slice(&header_bytes)?;

    println!("=== MRI File Inspection ===");
    println!("File Path:         {}", input_path);
    println!(
        "File Size:         {} bytes ({:.2} MB)",
        metadata.len(),
        metadata.len() as f64 / 1_048_576.0
    );
    println!("Format Version:    {}", version);
    println!("Volume Dimensions: {:?}", header.dimensions);
    println!("Voxel Size:        {:?}", header.voxel_size);
    println!("Data Type:         {}", header.dtype);
    println!("Endianness:        {}", header.endianness);
    println!("Modalities:        {:?}", header.modalities);
    println!("Chunk Size:        {:?}", header.chunk_size);
    println!("Compression:       {}", header.compression);
    println!("Is Mask:           {}", header.is_mask);
    println!("Quantized:         {}", header.quantized);
    if header.quantized {
        println!("  - Quant Bits:    {:?}", header.q_bits);
        println!("  - Quant Min:     {:?}", header.q_min);
        println!("  - Quant Max:     {:?}", header.q_max);
    }

    Ok(())
}

/// Verify the structural integrity and validity of a MRI file.
pub fn verify_mri(input_path: &str) -> Result<()> {
    let mut f = File::open(input_path)?;
    let file_len = f.metadata()?.len();

    // 1. Check Magic Bytes
    let mut magic = [0u8; 4];
    f.read_exact(&mut magic)?;
    if magic[..] != MAGIC[..] {
        return Err(anyhow!("Verification failed: Invalid magic bytes"));
    }

    // 2. Check Version
    let version = f.read_u16::<LittleEndian>()?;
    if version != VERSION {
        return Err(anyhow!(
            "Verification failed: Unsupported version {}",
            version
        ));
    }

    // 3. Read Header
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

    // 4. Verify Chunk Table Entries
    for i in 0..num_chunks {
        let offset = f.read_u64::<LittleEndian>()?;
        let size = f.read_u32::<LittleEndian>()?;
        let _mid = f.read_u16::<LittleEndian>()?;
        let _roi = f.read_u8()?;
        let flags = f.read_u8()?;

        let is_zero = flags & 1 == 1;

        if !is_zero && size > 0 {
            // Ensure chunk data range lies within the file bounds
            if offset + size as u64 > file_len {
                return Err(anyhow!(
                    "Verification failed: Chunk {} points to out-of-bounds file offset {} (file size {})",
                    i,
                    offset + size as u64,
                    file_len
                ));
            }
        }
    }

    println!(
        "Verification passed successfully: {} chunks validated.",
        num_chunks
    );
    Ok(())
}
