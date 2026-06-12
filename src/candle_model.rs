/// On-disk record size: O, H, L, C as f64 + volume as u64, little-endian.
pub const CANDLE_SIZE: usize = 40;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CandleModel {
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: u64,
}

impl CandleModel {
    pub fn to_bytes(&self) -> [u8; CANDLE_SIZE] {
        let mut result = [0u8; CANDLE_SIZE];
        result[0..8].copy_from_slice(&self.open.to_le_bytes());
        result[8..16].copy_from_slice(&self.high.to_le_bytes());
        result[16..24].copy_from_slice(&self.low.to_le_bytes());
        result[24..32].copy_from_slice(&self.close.to_le_bytes());
        result[32..40].copy_from_slice(&self.volume.to_le_bytes());
        result
    }

    pub fn from_bytes(src: &[u8; CANDLE_SIZE]) -> Self {
        Self {
            open: f64::from_le_bytes(src[0..8].try_into().unwrap()),
            high: f64::from_le_bytes(src[8..16].try_into().unwrap()),
            low: f64::from_le_bytes(src[16..24].try_into().unwrap()),
            close: f64::from_le_bytes(src[24..32].try_into().unwrap()),
            volume: u64::from_le_bytes(src[32..40].try_into().unwrap()),
        }
    }

    /// All-zero slot means "no candle written" — a real candle always has open > 0.
    pub fn is_empty_slot(src: &[u8; CANDLE_SIZE]) -> bool {
        src.iter().all(|itm| *itm == 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bytes_roundtrip() {
        let candle = CandleModel {
            open: 104325.57,
            high: 104400.01,
            low: 104300.0,
            close: 104399.99,
            volume: 1_234_567_890_123,
        };
        let bytes = candle.to_bytes();
        assert!(!CandleModel::is_empty_slot(&bytes));
        assert_eq!(CandleModel::from_bytes(&bytes), candle);
    }

    #[test]
    fn zero_slot_is_empty() {
        assert!(CandleModel::is_empty_slot(&[0u8; CANDLE_SIZE]));
    }
}
