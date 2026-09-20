pub struct QuantParams {
    pub bits: u8,
    pub q_min: f32,
    pub q_max: f32,
}

pub fn quantize_volume(volume: &[f32], params: &QuantParams) -> Vec<u16> {
    let levels = (1u32 << params.bits) - 1;
    volume
        .iter()
        .map(|&v| {
            let scaled = ((v - params.q_min) / (params.q_max - params.q_min)).clamp(0.0, 1.0);
            let q = (scaled * levels as f32 + 0.5) as u32;
            q as u16
        })
        .collect()
}
