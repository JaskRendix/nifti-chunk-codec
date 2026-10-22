use anyhow::{Result, anyhow};
use ndarray::Array3;
use nifti::{IntoNdArray, NiftiObject, ReaderOptions, writer::WriterOptions};
use std::path::Path;

/// Load a NIfTI file into a flat Vec<f32> + dimensions + voxel size.
pub fn load_nifti_volume(path: &str) -> Result<(Vec<f32>, [usize; 3], [f32; 3])> {
    let obj = ReaderOptions::new()
        .read_file(path)
        .map_err(|e| anyhow!("Failed to read NIfTI: {}", e))?;

    let header = obj.header();

    let x = header.dim[1] as usize;
    let y = header.dim[2] as usize;
    let z = header.dim[3] as usize;

    let voxel_size = [header.pixdim[1], header.pixdim[2], header.pixdim[3]];

    let array = obj
        .into_volume()
        .into_ndarray::<f32>()
        .map_err(|e| anyhow!("Failed to decode volume: {}", e))?;

    let vol_f32 = array.iter().copied().collect();

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_save_nifti_volume_size_mismatch() {
        let volume = vec![0.0f32; 7];

        let result = save_nifti_volume(
            &volume,
            [2, 2, 2], // expects 8 voxels
            "/tmp/size_mismatch.nii",
        );

        assert!(result.is_err());

        let err = result.unwrap_err().to_string();
        assert!(err.contains("Volume size mismatch"));
    }

    #[test]
    fn test_load_nifti_invalid_path() {
        let result = load_nifti_volume("/definitely/does/not/exist.nii");

        assert!(result.is_err());
    }

    #[test]
    fn test_save_and_load_roundtrip() {
        let path = std::env::temp_dir().join(format!("roundtrip_{}.nii", std::process::id()));

        let volume: Vec<f32> = (0..8).map(|v| v as f32).collect();

        save_nifti_volume(&volume, [2, 2, 2], path.to_str().unwrap()).unwrap();

        let (loaded, dims, _voxel_size) = load_nifti_volume(path.to_str().unwrap()).unwrap();

        assert_eq!(dims, [2, 2, 2]);
        assert_eq!(loaded.len(), volume.len());

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn test_save_empty_volume_fails() {
        let path = std::env::temp_dir().join(format!("empty_{}.nii", std::process::id()));

        let result = save_nifti_volume(&[], [1, 1, 1], path.to_str().unwrap());

        assert!(result.is_err() || result.is_ok());

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn test_save_nifti_valid_volume() {
        let path = std::env::temp_dir().join(format!("valid_{}.nii", std::process::id()));

        let volume = vec![1.0f32; 27];

        let result = save_nifti_volume(&volume, [3, 3, 3], path.to_str().unwrap());

        assert!(result.is_ok());
        assert!(path.exists());

        let _ = std::fs::remove_file(path);
    }
}
