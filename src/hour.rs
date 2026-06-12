use rust_extensions::date_time::{HourKey, IntervalKey};

use crate::calendar;
use crate::candle_model::CANDLE_SIZE;

/// Year block is always sized for a leap year: 366 * 24.
pub const SLOTS_PER_YEAR: u32 = 8_784;
pub const BLOCK_SIZE: u64 = SLOTS_PER_YEAR as u64 * CANDLE_SIZE as u64;

/// Key format is YYYYMMDDHH.
pub fn year_of(key: IntervalKey<HourKey>) -> u16 {
    (key.to_i64() / 1_000_000) as u16
}

/// Sequential hour number since Jan 1 00h of the key's year:
/// 01.Jan 00h -> 0, 01.Jan 01h -> 1, ...
pub fn slot_of(key: IntervalKey<HourKey>) -> u32 {
    let value = key.to_i64();
    let hour = (value % 100) as u32;
    let day = ((value / 100) % 100) as u8;
    let month = ((value / 10_000) % 100) as u8;
    let year = (value / 1_000_000) as u16;

    (calendar::day_of_year(year, month, day) as u32 - 1) * 24 + hour
}

pub fn key_from_slot(year: u16, slot: u32) -> IntervalKey<HourKey> {
    let day_of_year = (slot / 24 + 1) as u16;
    let hour = slot % 24;
    let (month, day) = calendar::month_day_from_day_of_year(year, day_of_year);

    let value =
        year as i64 * 1_000_000 + month as i64 * 10_000 + day as i64 * 100 + hour as i64;

    IntervalKey::from_i64(value)
}

/// Last used slot of the year: 8_759 for a regular year, 8_783 for a leap one.
pub fn last_slot_of_year(year: u16) -> u32 {
    calendar::days_in_year(year) as u32 * 24 - 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_hours_of_year() {
        assert_eq!(slot_of(IntervalKey::from_i64(2026010100)), 0);
        assert_eq!(slot_of(IntervalKey::from_i64(2026010101)), 1);
        assert_eq!(slot_of(IntervalKey::from_i64(2026010200)), 24);
    }

    #[test]
    fn year_of_key() {
        assert_eq!(year_of(IntervalKey::from_i64(2026010100)), 2026);
        assert_eq!(year_of(IntervalKey::from_i64(2024123123)), 2024);
    }

    #[test]
    fn leap_year_shift() {
        assert_eq!(slot_of(IntervalKey::from_i64(2024030100)), (31 + 29) * 24);
        assert_eq!(slot_of(IntervalKey::from_i64(2026030100)), (31 + 28) * 24);
    }

    #[test]
    fn last_hour_of_year() {
        assert_eq!(slot_of(IntervalKey::from_i64(2026123123)), 8_759);
        assert_eq!(slot_of(IntervalKey::from_i64(2024123123)), 8_783);
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
