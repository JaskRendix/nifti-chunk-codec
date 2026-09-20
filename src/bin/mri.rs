use clap::{Parser, Subcommand};
use nifti_chunk_codec_rs::{decode_mri, dump_chunk, encode_mri, inspect_mri, verify_mri};

/// NIFTI-CHUNK-CODEC-RS: Chunked MRI encoder/decoder
#[derive(Parser)]
#[command(
    name = "nifti-chunk-codec",
    author = "Giorgio",
    version = "0.1.0",
    about = "MRI Rust CLI: structure-aware MRI compression",
    arg_required_else_help = true
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Encode a NIfTI file into MRI format
    Encode {
        /// Input NIfTI file (.nii or .nii.gz)
        input: String,
        /// Output MRI file (.mri)
        output: String,

        /// Quantization bits (default: 12)
        #[arg(short, long, default_value_t = 12)]
        bits: u8,

        /// MRI chunk size (default: 32 32 32)
        #[arg(long, num_args = 3, value_parser, default_values_t = [32, 32, 32])]
        chunk_mri: Vec<usize>,

        /// Mask chunk size (default: 64 64 64)
        #[arg(long, num_args = 3, value_parser, default_values_t = [64, 64, 64])]
        chunk_mask: Vec<usize>,

        /// ROI threshold (default: 200)
        #[arg(long, default_value_t = 200)]
        roi_threshold: u16,

        /// Compression level for ROI chunks (default: 3)
        #[arg(long, default_value_t = 3)]
        roi_level: i32,

        /// Compression level for background chunks (default: 1)
        #[arg(long, default_value_t = 1)]
        bg_level: i32,
    },

    /// Decode a MRI file into a NIfTI volume
    Decode {
        /// Input MRI file
        input: String,
        /// Output NIfTI file
        output: String,
    },

    /// Inspect metadata and header information of a MRI file
    Inspect {
        /// Input MRI file
        input: String,
    },

    /// Show compression statistics and quality metrics
    Stats {
        /// Input MRI file
        input: String,
        /// Optional original NIfTI file for PSNR/MSE comparison
        #[arg(long)]
        original: Option<String>,
    },

    /// Dump a specific chunk from the MRI file into a separate file
    DumpChunk {
        /// Input MRI file
        input: String,
        /// 3D index of the chunk (e.g., 0 1 2)
        #[arg(long, num_args = 3, value_parser)]
        index: Vec<usize>,
        /// Output file for the chunk data
        output: String,
    },

    /// Verify the integrity and validity of a MRI file
    Verify {
        /// Input MRI file
        input: String,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Encode {
            input,
            output,
            bits,
            chunk_mri,
            chunk_mask,
            roi_threshold,
            roi_level,
            bg_level,
        } => {
            println!("Encoding {} → {}", input, output);

            let chunk_mri: [usize; 3] = chunk_mri
                .try_into()
                .map_err(|_| anyhow::anyhow!("--chunk-mri requires exactly 3 values"))?;

            let chunk_mask: [usize; 3] = chunk_mask
                .try_into()
                .map_err(|_| anyhow::anyhow!("--chunk-mask requires exactly 3 values"))?;

            encode_mri(
                &input,
                &output,
                bits,
                chunk_mri,
                chunk_mask,
                roi_threshold,
                roi_level,
                bg_level,
            )?;

            println!("Done.");
        }

        Commands::Decode { input, output } => {
            println!("Decoding {} → {}", input, output);
            decode_mri(&input, &output)?;
            println!("Done.");
        }

        Commands::Inspect { input } => {
            println!("Inspecting metadata for: {}", input);
            inspect_mri(&input)?;
        }

        Commands::Stats { input, original } => {
            nifti_chunk_codec_rs::stats_mri(&input, original.as_deref())?;
        }

        Commands::DumpChunk {
            input,
            index,
            output,
        } => {
            let idx: [usize; 3] = index
                .try_into()
                .map_err(|_| anyhow::anyhow!("--index requires exactly 3 values (x y z)"))?;

            dump_chunk(&input, idx, &output)?;
        }

        Commands::Verify { input } => {
            println!("Verifying integrity of: {}", input);
            verify_mri(&input)?;
        }
    }

    Ok(())
}
