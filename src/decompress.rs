use zstd::stream::decode_all;

/// Decompress Zstd bytes into a raw buffer.
///
/// Caller is responsible for reshaping into (x, y, z).
pub fn decompress_chunk(input: &[u8], expected_size: usize) -> anyhow::Result<Vec<u8>> {
    let decompressed = decode_all(input)?;
    if decompressed.len() != expected_size {
        anyhow::bail!(
            "Decompressed size mismatch: expected {}, got {}",
            expected_size,
            decompressed.len()
        );
    }
    Ok(decompressed)
}
