use zstd::stream::encode_all;

pub fn compress_chunk(input: &[u8], level: i32) -> anyhow::Result<Vec<u8>> {
    let compressed = encode_all(input, level)?;
    Ok(compressed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decompress::decompress_chunk;

    #[test]
    fn compress_chunk_success() {
        let input = b"hello world";

        let compressed = compress_chunk(input, 1).unwrap();

        assert!(compressed.len() > 1);
    }

    #[test]
    fn compress_decompress_roundtrip() {
        let input = b"hello world hello world hello world";

        let compressed = compress_chunk(input, 1).unwrap();

        let decompressed = decompress_chunk(&compressed, input.len()).unwrap();

        assert_eq!(decompressed, input);
    }

    #[test]
    fn compress_empty_input() {
        let compressed = compress_chunk(&[], 1).unwrap();

        let decompressed = decompress_chunk(&compressed, 0).unwrap();

        assert!(decompressed.is_empty());
    }

    #[test]
    fn compress_multiple_levels() {
        let input = vec![42u8; 1024];

        for level in [1, 3, 6, 9] {
            let compressed = compress_chunk(&input, level).unwrap();

            let decompressed = decompress_chunk(&compressed, input.len()).unwrap();

            assert_eq!(decompressed, input);
        }
    }
}
