use anyhow::Result;
use ndarray::{Array3, Zip};
use nifti_chunk_codec_rs::nifti_io::save_nifti_volume;
use rand::{RngExt, rng};

/// Simple separable Gaussian blur (X/Y/Z passes)
fn gaussian_blur(volume: &Array3<f32>, sigma: f32) -> Array3<f32> {
    let radius = (sigma * 3.0).ceil() as isize;
    let kernel_size = radius * 2 + 1;

    // 1D Gaussian kernel
    let mut kernel = Vec::with_capacity(kernel_size as usize);
    for i in -radius..=radius {
        let x = i as f32;
        kernel.push((-x * x / (2.0 * sigma * sigma)).exp());
    }
    let sum: f32 = kernel.iter().sum();
    for v in kernel.iter_mut() {
        *v /= sum;
    }

    let (sx, sy, sz) = volume.dim();
    let mut tmp = volume.clone();
    let mut out = volume.clone();

    // X pass
    for x in 0..sx {
        for y in 0..sy {
            for z in 0..sz {
                let mut acc = 0.0;
                for (k, w) in kernel.iter().enumerate() {
                    let dx = x as isize + (k as isize - radius);
                    if dx >= 0 && dx < sx as isize {
                        acc += volume[(dx as usize, y, z)] * w;
                    }
                }
                tmp[(x, y, z)] = acc;
            }
        }
    }

    // Y pass
    for x in 0..sx {
        for y in 0..sy {
            for z in 0..sz {
                let mut acc = 0.0;
                for (k, w) in kernel.iter().enumerate() {
                    let dy = y as isize + (k as isize - radius);
                    if dy >= 0 && dy < sy as isize {
                        acc += tmp[(x, dy as usize, z)] * w;
                    }
                }
                out[(x, y, z)] = acc;
            }
        }
    }

    // Z pass
    let mut final_vol = out.clone();
    for x in 0..sx {
        for y in 0..sy {
            for z in 0..sz {
                let mut acc = 0.0;
                for (k, w) in kernel.iter().enumerate() {
                    let dz = z as isize + (k as isize - radius);
                    if dz >= 0 && dz < sz as isize {
                        acc += out[(x, y, dz as usize)] * w;
                    }
                }
                final_vol[(x, y, z)] = acc;
            }
        }
    }

    final_vol
}

fn main() -> Result<()> {
    let size = 128;
    let mut vol = Array3::<f32>::zeros((size, size, size));

    // Coordinate grid
    let grid = Array3::from_shape_fn((size, size, size), |(x, y, z)| {
        let nx = (x as f32 / (size as f32 - 1.0)) * 2.0 - 1.0;
        let ny = (y as f32 / (size as f32 - 1.0)) * 2.0 - 1.0;
        let nz = (z as f32 / (size as f32 - 1.0)) * 2.0 - 1.0;
        (nx * nx + ny * ny + nz * nz).sqrt()
    });

    // Tissue layers
    Zip::from(&mut vol).and(&grid).for_each(|v, &r| {
        *v = if r < 0.55 {
            900.0
        } else if r < 0.75 {
            600.0
        } else {
            200.0
        };
    });

    // Random lesions
    let mut rng = rng();
    for _ in 0..10 {
        let cx = rng.random_range(0..size);
        let cy = rng.random_range(0..size);
        let cz = rng.random_range(0..size);
        let radius = rng.random_range(8..20);

        for x in 0..size {
            for y in 0..size {
                for z in 0..size {
                    let dx = (x as i32 - cx as i32).pow(2);
                    let dy = (y as i32 - cy as i32).pow(2);
                    let dz = (z as i32 - cz as i32).pow(2);
                    if dx + dy + dz < radius * radius {
                        vol[(x, y, z)] += rng.random_range(300.0..800.0);
                    }
                }
            }
        }
    }

    // Bias field
    let noise = Array3::<f32>::from_shape_fn((size, size, size), |_| rng.random_range(0.0..1.0));
    let bias = gaussian_blur(&noise, 40.0);
    Zip::from(&mut vol).and(&bias).for_each(|v, &b| {
        *v *= 1.0 + 0.4 * b;
    });

    // Rician noise
    let noise1 =
        Array3::<f32>::from_shape_fn((size, size, size), |_| rng.random_range(-40.0..40.0));
    let noise2 =
        Array3::<f32>::from_shape_fn((size, size, size), |_| rng.random_range(-40.0..40.0));
    Zip::from(&mut vol)
        .and(&noise1)
        .and(&noise2)
        .for_each(|v, &n1, &n2| {
            *v = ((*v + n1).powi(2) + n2.powi(2)).sqrt();
        });

    // High-frequency noise
    let hf = gaussian_blur(
        &Array3::<f32>::from_shape_fn((size, size, size), |_| rng.random_range(0.0..1.0)),
        1.0,
    );
    Zip::from(&mut vol).and(&hf).for_each(|v, &h| {
        *v += h * 20.0;
    });

    // Partial volume blur
    vol = gaussian_blur(&vol, 1.2);

    // Clip + convert to f32
    let vol_f32: Vec<f32> = vol.iter().map(|v| v.clamp(0.0, 2000.0)).collect();

    save_nifti_volume(
        &vol_f32,
        [size, size, size],
        "synthetic_realistic_hardcore.nii",
    )?;

    println!("Saved synthetic_realistic_hardcore.nii");

    Ok(())
}
