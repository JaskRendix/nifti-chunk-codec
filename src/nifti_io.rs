use anyhow::{Result, anyhow};
use ndarray::Array3;
use nifti::{NiftiObject, ReaderOptions, writer::WriterOptions};
use std::path::Path;

/// Load a NIfTI file into a flat Vec<f32> + dimensions + voxel size.
pub fn load_nifti_volume(path: &str) -> Result<(Vec<f32>, [usize; 3], [f32; 3])> {
    let obj = ReaderOptions::new()
        .read_file(path)
        .map_err(|e| anyhow!("Failed to read NIfTI: {}", e))?;

    let header = obj.header();
    let dims = header.dim;

    if dims.len() < 4 {
        return Err(anyhow!("NIfTI must have at least 3 dimensions"));
    }

    let x = dims[1] as usize;
    let y = dims[2] as usize;
    let z = dims[3] as usize;

    let voxel_size = [header.pixdim[1], header.pixdim[2], header.pixdim[3]];

    // nifti version installed: volume().raw_data() exists
    let raw = obj.volume().raw_data();

    let vol_f32: Vec<f32> = raw.iter().map(|v| *v as f32).collect();

    Ok((vol_f32, [x, y, z], voxel_size))
}

/// Save a flat Vec<f32> as a NIfTI file.
pub fn save_nifti_volume(volume: &[f32], dims: [usize; 3], output_path: &str) -> Result<()> {
    let (x, y, z) = (dims[0], dims[1], dims[2]);
    let expected = x * y * z;

    if volume.len() != expected {
        return Err(anyhow!(
            "Volume size mismatch: expected {}, got {}",
            expected,
            volume.len()
        ));
    }

    // Convert &[f32] → Array3<f32>
    let array = Array3::from_shape_vec((x, y, z), volume.to_vec())
        .map_err(|e| anyhow!("Failed to reshape volume: {}", e))?;

    // nifti version installed: WriterOptions::new(path).write_nifti(&array)
    WriterOptions::new(Path::new(output_path))
        .write_nifti(&array)
        .map_err(|e| anyhow!("Failed to save NIfTI: {}", e))?;

    Ok(())
}
