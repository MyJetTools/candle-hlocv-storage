use rust_extensions::date_time::{IntervalKey, MinuteKey};

use crate::calendar;
use crate::candle_model::CANDLE_SIZE;

/// Year block is always sized for a leap year: 366 * 1440.
pub const SLOTS_PER_YEAR: u32 = 527_040;
pub const BLOCK_SIZE: u64 = SLOTS_PER_YEAR as u64 * CANDLE_SIZE as u64;

/// Key format is YYYYMMDDHHMM.
pub fn year_of(key: IntervalKey<MinuteKey>) -> u16 {
    (key.to_i64() / 100_000_000) as u16
}

/// Sequential minute number since Jan 1 00:00 of the key's year:
/// 01.Jan 00:00 -> 0, 01.Jan 00:01 -> 1, ...
pub fn slot_of(key: IntervalKey<MinuteKey>) -> u32 {
    let value = key.to_i64();
    let minute = (value % 100) as u32;
    let hour = ((value / 100) % 100) as u32;
    let day = ((value / 10_000) % 100) as u8;
    let month = ((value / 1_000_000) % 100) as u8;
    let year = (value / 100_000_000) as u16;

    (calendar::day_of_year(year, month, day) as u32 - 1) * 1440 + hour * 60 + minute
}

pub fn key_from_slot(year: u16, slot: u32) -> IntervalKey<MinuteKey> {
    let day_of_year = (slot / 1440 + 1) as u16;
    let minute_of_day = slot % 1440;
    let (month, day) = calendar::month_day_from_day_of_year(year, day_of_year);

    let value = year as i64 * 100_000_000
        + month as i64 * 1_000_000
        + day as i64 * 10_000
        + (minute_of_day / 60) as i64 * 100
        + (minute_of_day % 60) as i64;

    IntervalKey::from_i64(value)
}

/// Last used slot of the year: 525_599 for a regular year, 527_039 for a leap one.
pub fn last_slot_of_year(year: u16) -> u32 {
    calendar::days_in_year(year) as u32 * 1440 - 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_minutes_of_year() {
        assert_eq!(slot_of(IntervalKey::from_i64(202601010000)), 0);
        assert_eq!(slot_of(IntervalKey::from_i64(202601010001)), 1);
        assert_eq!(slot_of(IntervalKey::from_i64(202601010100)), 60);
    }

    #[test]
    fn year_of_key() {
        assert_eq!(year_of(IntervalKey::from_i64(202601010000)), 2026);
        assert_eq!(year_of(IntervalKey::from_i64(202412312359)), 2024);
    }

    #[test]
    fn leap_year_shift() {
        // 1 March: leap year has one extra day before it
        assert_eq!(
            slot_of(IntervalKey::from_i64(202403010000)),
            (31 + 29) * 1440
        );
        assert_eq!(
            slot_of(IntervalKey::from_i64(202603010000)),
            (31 + 28) * 1440
        );
    }

    #[test]
    fn last_minute_of_year() {
        assert_eq!(slot_of(IntervalKey::from_i64(202612312359)), 525_599);
        assert_eq!(slot_of(IntervalKey::from_i64(202412312359)), 527_039);
        assert_eq!(last_slot_of_year(2026), 525_599);
        assert_eq!(last_slot_of_year(2024), 527_039);
        assert_eq!(last_slot_of_year(2024), SLOTS_PER_YEAR - 1);
    }

    #[test]
    fn slot_roundtrip() {
        for year in [2024u16, 2026] {
            for slot in (0..=last_slot_of_year(year)).step_by(719) {
                let key = key_from_slot(year, slot);
                assert_eq!(slot_of(key), slot, "year {} slot {}", year, slot);
                assert_eq!(year_of(key), year);
            }
            // exact boundaries
            for slot in [0, 1439, 1440, last_slot_of_year(year)] {
                assert_eq!(slot_of(key_from_slot(year, slot)), slot);
            }
        }
    }
}
