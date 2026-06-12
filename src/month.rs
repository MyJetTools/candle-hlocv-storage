//! Month candles are NOT stored — they are aggregated on the fly from the
//! day candles of the month. Only pure helpers live here; the file I/O is in
//! [`crate::CandleHLOCWriter::read_month_candle`].

use rust_extensions::date_time::{IntervalKey, MonthKey};

use crate::calendar;
use crate::candle_model::CandleModel;

/// Key format is YYYYMM.
pub fn year_of(key: IntervalKey<MonthKey>) -> u16 {
    (key.to_i64() / 100) as u16
}

pub fn month_of(key: IntervalKey<MonthKey>) -> u8 {
    (key.to_i64() % 100) as u8
}

pub fn key_from(year: u16, month: u8) -> IntervalKey<MonthKey> {
    IntervalKey::from_i64(year as i64 * 100 + month as i64)
}

pub fn next_month(year: u16, month: u8) -> (u16, u8) {
    if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    }
}

/// Day-slot range `[from..=to]` the month occupies inside its year block.
pub fn day_slot_range(year: u16, month: u8) -> (u32, u32) {
    let from = calendar::day_of_year(year, month, 1) as u32 - 1;
    (from, from + calendar::days_in_month(year, month) as u32 - 1)
}

/// Folds day candles (in chronological order) into one month candle.
pub fn aggregate(day_candles: &[CandleModel]) -> Option<CandleModel> {
    let mut result = *day_candles.first()?;
    for candle in &day_candles[1..] {
        result.high = result.high.max(candle.high);
        result.low = result.low.min(candle.low);
        result.close = candle.close;
        result.volume += candle.volume;
    }
    Some(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_parts() {
        let key = IntervalKey::from_i64(202606);
        assert_eq!(year_of(key), 2026);
        assert_eq!(month_of(key), 6);
        assert_eq!(key_from(2026, 6).to_i64(), 202606);
    }

    #[test]
    fn next_month_rolls_over_year() {
        assert_eq!(next_month(2026, 6), (2026, 7));
        assert_eq!(next_month(2026, 12), (2027, 1));
    }

    #[test]
    fn day_slot_ranges() {
        assert_eq!(day_slot_range(2026, 1), (0, 30));
        assert_eq!(day_slot_range(2026, 2), (31, 58)); // 28 days
        assert_eq!(day_slot_range(2024, 2), (31, 59)); // 29 days
        assert_eq!(day_slot_range(2026, 3), (59, 89));
        assert_eq!(day_slot_range(2026, 12), (334, 364));
        assert_eq!(day_slot_range(2024, 12), (335, 365));
    }

    #[test]
    fn aggregate_folds_days() {
        let days = [
            CandleModel { open: 10.0, high: 12.0, low: 9.0, close: 11.0, volume: 100 },
            CandleModel { open: 11.0, high: 15.0, low: 10.5, close: 14.0, volume: 200 },
            CandleModel { open: 14.0, high: 14.5, low: 8.0, close: 9.5, volume: 300 },
        ];
        assert_eq!(
            aggregate(&days),
            Some(CandleModel { open: 10.0, high: 15.0, low: 8.0, close: 9.5, volume: 600 })
        );
        assert_eq!(aggregate(&days[..1]), Some(days[0]));
        assert_eq!(aggregate(&[]), None);
    }
}
