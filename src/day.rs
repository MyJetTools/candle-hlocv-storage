use rust_extensions::date_time::{DayKey, IntervalKey};

use crate::calendar;
use crate::candle_model::CANDLE_SIZE;

/// Year block is always sized for a leap year: 366 days.
pub const SLOTS_PER_YEAR: u32 = 366;
pub const BLOCK_SIZE: u64 = SLOTS_PER_YEAR as u64 * CANDLE_SIZE as u64;

/// Key format is YYYYMMDD.
pub fn year_of(key: IntervalKey<DayKey>) -> u16 {
    (key.to_i64() / 10_000) as u16
}

/// Sequential day number since Jan 1 of the key's year: 01.Jan -> 0, 02.Jan -> 1, ...
pub fn slot_of(key: IntervalKey<DayKey>) -> u32 {
    let value = key.to_i64();
    let day = (value % 100) as u8;
    let month = ((value / 100) % 100) as u8;
    let year = (value / 10_000) as u16;

    calendar::day_of_year(year, month, day) as u32 - 1
}

pub fn key_from_slot(year: u16, slot: u32) -> IntervalKey<DayKey> {
    let (month, day) = calendar::month_day_from_day_of_year(year, (slot + 1) as u16);
    IntervalKey::from_i64(year as i64 * 10_000 + month as i64 * 100 + day as i64)
}

/// Last used slot of the year: 364 for a regular year, 365 for a leap one.
pub fn last_slot_of_year(year: u16) -> u32 {
    calendar::days_in_year(year) as u32 - 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_days_of_year() {
        assert_eq!(slot_of(IntervalKey::from_i64(20260101)), 0);
        assert_eq!(slot_of(IntervalKey::from_i64(20260102)), 1);
        assert_eq!(slot_of(IntervalKey::from_i64(20260201)), 31);
    }

    #[test]
    fn year_of_key() {
        assert_eq!(year_of(IntervalKey::from_i64(20260101)), 2026);
        assert_eq!(year_of(IntervalKey::from_i64(20241231)), 2024);
    }

    #[test]
    fn leap_year_shift() {
        assert_eq!(slot_of(IntervalKey::from_i64(20240301)), 31 + 29);
        assert_eq!(slot_of(IntervalKey::from_i64(20260301)), 31 + 28);
    }

    #[test]
    fn last_day_of_year() {
        assert_eq!(slot_of(IntervalKey::from_i64(20261231)), 364);
        assert_eq!(slot_of(IntervalKey::from_i64(20241231)), 365);
        assert_eq!(last_slot_of_year(2024), SLOTS_PER_YEAR - 1);
    }

    #[test]
    fn slot_roundtrip() {
        for year in [2024u16, 2026] {
            for slot in 0..=last_slot_of_year(year) {
                let key = key_from_slot(year, slot);
                assert_eq!(slot_of(key), slot, "year {} slot {}", year, slot);
                assert_eq!(year_of(key), year);
            }
        }
    }
}
