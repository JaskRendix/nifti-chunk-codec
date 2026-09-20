#[derive(Debug, Clone)]
pub struct Chunk3D {
    pub origin: [usize; 3],
    pub size: [usize; 3],
    pub data: Vec<u8>, // raw bytes after quantization
}

pub fn ceil_div(a: usize, b: usize) -> usize {
    a.div_ceil(b)
}
