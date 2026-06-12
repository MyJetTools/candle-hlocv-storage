const DAYS_IN_MONTH: [u16; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

pub fn is_leap_year(year: u16) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

pub fn days_in_year(year: u16) -> u16 {
    if is_leap_year(year) {
        366
    } else {
        365
    }
}

pub fn days_in_month(year: u16, month: u8) -> u16 {
    if month == 2 && is_leap_year(year) {
        29
    } else {
        DAYS_IN_MONTH[month as usize - 1]
    }
}

/// 1-based: (year, 1, 1) -> 1.
pub fn day_of_year(year: u16, month: u8, day: u8) -> u16 {
    let mut result = day as u16;
    for month_index in 0..(month as usize - 1) {
        result += DAYS_IN_MONTH[month_index];
    }
    if month > 2 && is_leap_year(year) {
        result += 1;
    }
    result
}

/// Inverse of [`day_of_year`]: 1-based day-of-year -> (month, day).
pub fn month_day_from_day_of_year(year: u16, day_of_year: u16) -> (u8, u8) {
    let mut rest = day_of_year;
    for (month_index, days) in DAYS_IN_MONTH.iter().enumerate() {
        let days = if month_index == 1 && is_leap_year(year) {
            29
        } else {
            *days
        };
        if rest <= days {
            return (month_index as u8 + 1, rest as u8);
        }
        rest -= days;
    }
    panic!("day_of_year {} is out of range for year {}", day_of_year, year);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leap_years() {
        assert!(is_leap_year(2024));
        assert!(!is_leap_year(2026));
        assert!(!is_leap_year(2100));
        assert!(is_leap_year(2000));
    }

    #[test]
    fn day_of_year_basics() {
        assert_eq!(day_of_year(2026, 1, 1), 1);
        assert_eq!(day_of_year(2026, 3, 1), 31 + 28 + 1);
        assert_eq!(day_of_year(2024, 3, 1), 31 + 29 + 1);
        assert_eq!(day_of_year(2026, 12, 31), 365);
        assert_eq!(day_of_year(2024, 12, 31), 366);
    }

    #[test]
    fn month_day_roundtrip_full_years() {
        for year in [2024u16, 2026] {
            for doy in 1..=days_in_year(year) {
                let (month, day) = month_day_from_day_of_year(year, doy);
                assert_eq!(day_of_year(year, month, day), doy, "year {} doy {}", year, doy);
            }
        }
    }
}
