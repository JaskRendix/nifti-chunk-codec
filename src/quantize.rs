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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quantize_min_maps_to_zero() {
        let params = QuantParams {
            bits: 8,
            q_min: 0.0,
            q_max: 100.0,
        };

        let out = quantize_volume(&[0.0], &params);

        assert_eq!(out, vec![0]);
    }

    #[test]
    fn test_quantize_max_maps_to_max_level() {
        let params = QuantParams {
            bits: 8,
            q_min: 0.0,
            q_max: 100.0,
        };

        let out = quantize_volume(&[100.0], &params);

        assert_eq!(out, vec![255]);
    }

    #[test]
    fn test_quantize_midpoint() {
        let params = QuantParams {
            bits: 8,
            q_min: 0.0,
            q_max: 100.0,
        };

        let out = quantize_volume(&[50.0], &params);

        assert_eq!(out, vec![128]);
    }

    #[test]
    fn test_quantize_clamps_below_min() {
        let params = QuantParams {
            bits: 8,
            q_min: 0.0,
            q_max: 100.0,
        };

        let out = quantize_volume(&[-10.0], &params);

        assert_eq!(out, vec![0]);
    }

    #[test]
    fn test_quantize_clamps_above_max() {
        let params = QuantParams {
            bits: 8,
            q_min: 0.0,
            q_max: 100.0,
        };

        let out = quantize_volume(&[150.0], &params);

        assert_eq!(out, vec![255]);
    }

    #[test]
    fn test_quantize_multiple_values() {
        let params = QuantParams {
            bits: 8,
            q_min: 0.0,
            q_max: 100.0,
        };

        let out = quantize_volume(&[0.0, 25.0, 50.0, 75.0, 100.0], &params);

        assert_eq!(out, vec![0, 64, 128, 191, 255]);
    }

    #[test]
    fn test_quantize_two_bit_levels() {
        let params = QuantParams {
            bits: 2,
            q_min: 0.0,
            q_max: 3.0,
        };

        let out = quantize_volume(&[0.0, 1.0, 2.0, 3.0], &params);

        assert_eq!(out, vec![0, 1, 2, 3]);
    }

    #[test]
    fn test_quantize_empty_volume() {
        let params = QuantParams {
            bits: 8,
            q_min: 0.0,
            q_max: 100.0,
        };

        let out = quantize_volume(&[], &params);

        assert!(out.is_empty());
    }
}
