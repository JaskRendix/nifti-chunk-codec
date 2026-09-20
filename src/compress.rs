use zstd::stream::encode_all;

pub fn compress_chunk(input: &[u8], level: i32) -> anyhow::Result<Vec<u8>> {
    let compressed = encode_all(input, level)?;
    Ok(compressed)
}
