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

#[cfg(test)]
mod tests {
    use super::*;
    use zstd::stream::encode_all;

    #[test]
    fn decompress_chunk_success() {
        let original = vec![1u8, 2, 3, 4, 5];

        let compressed = encode_all(&original[..], 1).unwrap();

        let decompressed = decompress_chunk(&compressed, original.len()).unwrap();

        assert_eq!(decompressed, original);
    }

    #[test]
    fn decompress_chunk_size_mismatch() {
        let original = [1u8, 2, 3, 4, 5];

        let compressed = encode_all(&original[..], 1).unwrap();

        let err = decompress_chunk(&compressed, original.len() + 1).unwrap_err();

        assert!(err.to_string().contains("Decompressed size mismatch"));
    }

    #[test]
    fn decompress_chunk_invalid_input() {
        let invalid = b"this is not zstd data";

        assert!(decompress_chunk(invalid, 0).is_err());
    }

    #[test]
    fn decompress_chunk_empty_buffer() {
        let original: Vec<u8> = Vec::new();

        let compressed = encode_all(&original[..], 1).unwrap();

        let decompressed = decompress_chunk(&compressed, 0).unwrap();

        assert!(decompressed.is_empty());
    }
}
