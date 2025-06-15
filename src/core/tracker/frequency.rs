use chrono::Datelike;
use chrono::{NaiveDate, NaiveDateTime, TimeDelta};
use std::fmt::Display;
use strum::{EnumIter, FromRepr};

use crate::core::date::Date;

#[derive(Clone, Copy, FromRepr, EnumIter, Debug, PartialEq, Eq)]
pub enum Frequency {
    #[strum(to_string = "minute")]
    Minutes(u64),
    #[strum(to_string = "hour")]
    Hours(u64),
    #[strum(to_string = "day")]
    Days(u64),
    #[strum(to_string = "week")]
    Weeks(u64),
    #[strum(to_string = "month")]
    Months(u64),
    #[strum(to_string = "yeah")]
    Years(u64),
}
impl Display for Frequency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let word = match self {
            Frequency::Minutes(_) => "minute",
            Frequency::Hours(_) => "hour",
            Frequency::Days(_) => "day",
            Frequency::Weeks(_) => "week",
            Frequency::Months(_) => "month",
            Frequency::Years(_) => "year",
        };
        match self {
            Frequency::Minutes(n)
            | Frequency::Hours(n)
            | Frequency::Days(n)
            | Frequency::Weeks(n)
            | Frequency::Months(n)
            | Frequency::Years(n) => {
                if *n == 1 {
                    write!(f, "Every {word}")
                } else {
                    write!(f, "Every {n} {word}s")
                }
            }
        }
    }
}

impl Frequency {
    /// There are cases where it doesn't make sense to have a minute-based frequency and a Date without time for example.
    pub fn fix_date(&self, date: &Date) -> Date {
        let now = chrono::Local::now().naive_local();
        match date {
            Date::Day(naive_date) => match self {
                Frequency::Hours(_) | Frequency::Minutes(_) => {
                    Date::DayTime(NaiveDateTime::new(*naive_date, now.time()))
                }
                _ => date.clone(),
            },
            Date::DayTime(naive_date_time) => match self {
                Frequency::Months(_)
                | Frequency::Years(_)
                | Frequency::Weeks(_)
                | Frequency::Days(_) => Date::Day(naive_date_time.date()),
                _ => date.clone(),
            },
        }
    }
    pub fn next_date(&self, date: &Date) -> Date {
        match *date {
            Date::Day(naive_date) => {
                match *self {
                    Frequency::Days(days) => Date::Day(naive_date + TimeDelta::days(days as i64)),
                    Frequency::Weeks(weeks) => {
                        Date::Day(naive_date + TimeDelta::weeks(weeks as i64))
                    }
                    Frequency::Months(months) => {
                        // Handle month addition more carefully since months have different lengths
                        let mut year = naive_date.year();
                        let mut month = naive_date.month() as i32 + months as i32;

                        // Handle year overflow
                        while month > 12 {
                            year += 1;
                            month -= 12;
                        }

                        let day = naive_date.day();
                        // Handle day overflow for shorter months
                        let new_date = NaiveDate::from_ymd_opt(year, month as u32, day)
                            .unwrap_or_else(|| {
                                // If day doesn't exist in target month, use last day of that month
                                NaiveDate::from_ymd_opt(year, month as u32, 1)
                                    .unwrap()
                                    .with_day(match month {
                                        2 => {
                                            if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
                                            {
                                                29
                                            } else {
                                                28
                                            }
                                        }
                                        4 | 6 | 9 | 11 => 30,
                                        _ => 31,
                                    })
                                    .unwrap()
                            });
                        Date::Day(new_date)
                    }
                    Frequency::Years(years) => {
                        let new_year = naive_date.year() + years as i32;
                        let new_date =
                            NaiveDate::from_ymd_opt(new_year, naive_date.month(), naive_date.day())
                                .unwrap_or_else(|| {
                                    // Handle leap year edge case (Feb 29 -> Feb 28)
                                    NaiveDate::from_ymd_opt(new_year, naive_date.month(), 28)
                                        .unwrap()
                                });
                        Date::Day(new_date)
                    }
                    _ => date.clone(), // Should not happen due to type conversion above
                }
            }
            Date::DayTime(naive_date_time) => {
                match *self {
                    Frequency::Minutes(minutes) => {
                        Date::DayTime(naive_date_time + TimeDelta::minutes(minutes as i64))
                    }
                    Frequency::Hours(hours) => {
                        Date::DayTime(naive_date_time + TimeDelta::hours(hours as i64))
                    }
                    _ => date.clone(), // Should not happen due to type conversion above
                }
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::date::Date;
    use chrono::{NaiveDate, NaiveDateTime};

    // Helper function to create a NaiveDateTime from date and time components
    fn datetime(year: i32, month: u32, day: u32, hour: u32, min: u32, sec: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(year, month, day)
            .unwrap()
            .and_hms_opt(hour, min, sec)
            .unwrap()
    }

    // Helper function to create a NaiveDate
    fn date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }

    #[test]
    fn test_every_x_minutes() {
        let start_datetime = datetime(2024, 6, 15, 10, 30, 0);
        let start_date = Date::DayTime(start_datetime);

        // Test 1 minute
        let freq = Frequency::Minutes(1);
        let next = freq.next_date(&start_date);
        assert_eq!(next, Date::DayTime(datetime(2024, 6, 15, 10, 31, 0)));

        // Test 15 minutes
        let freq = Frequency::Minutes(15);
        let next = freq.next_date(&start_date);
        assert_eq!(next, Date::DayTime(datetime(2024, 6, 15, 10, 45, 0)));

        // Test 60 minutes (1 hour)
        let freq = Frequency::Minutes(60);
        let next = freq.next_date(&start_date);
        assert_eq!(next, Date::DayTime(datetime(2024, 6, 15, 11, 30, 0)));

        // Test crossing day boundary
        let late_start = Date::DayTime(datetime(2024, 6, 15, 23, 45, 0));
        let freq = Frequency::Minutes(30);
        let next = freq.next_date(&late_start);
        assert_eq!(next, Date::DayTime(datetime(2024, 6, 16, 0, 15, 0)));
    }

    #[test]
    fn test_every_x_hours() {
        let start_datetime = datetime(2024, 6, 15, 10, 30, 0);
        let start_date = Date::DayTime(start_datetime);

        // Test 1 hour
        let freq = Frequency::Hours(1);
        let next = freq.next_date(&start_date);
        assert_eq!(next, Date::DayTime(datetime(2024, 6, 15, 11, 30, 0)));

        // Test 12 hours
        let freq = Frequency::Hours(12);
        let next = freq.next_date(&start_date);
        assert_eq!(next, Date::DayTime(datetime(2024, 6, 15, 22, 30, 0)));

        // Test 24 hours (1 day)
        let freq = Frequency::Hours(24);
        let next = freq.next_date(&start_date);
        assert_eq!(next, Date::DayTime(datetime(2024, 6, 16, 10, 30, 0)));

        // Test crossing day boundary
        let late_start = Date::DayTime(datetime(2024, 6, 15, 20, 0, 0));
        let freq = Frequency::Hours(6);
        let next = freq.next_date(&late_start);
        assert_eq!(next, Date::DayTime(datetime(2024, 6, 16, 2, 0, 0)));
    }

    #[test]
    fn test_every_x_days() {
        let start_date = Date::Day(date(2024, 6, 15));

        // Test 1 day
        let freq = Frequency::Days(1);
        let next = freq.next_date(&start_date);
        assert_eq!(next, Date::Day(date(2024, 6, 16)));

        // Test 7 days
        let freq = Frequency::Days(7);
        let next = freq.next_date(&start_date);
        assert_eq!(next, Date::Day(date(2024, 6, 22)));

        // Test 30 days
        let freq = Frequency::Days(30);
        let next = freq.next_date(&start_date);
        assert_eq!(next, Date::Day(date(2024, 7, 15)));

        // Test crossing year boundary
        let year_end = Date::Day(date(2024, 12, 25));
        let freq = Frequency::Days(10);
        let next = freq.next_date(&year_end);
        assert_eq!(next, Date::Day(date(2025, 1, 4)));
    }

    #[test]
    fn test_every_x_weeks() {
        let start_date = Date::Day(date(2024, 6, 15)); // Saturday

        // Test 1 week
        let freq = Frequency::Weeks(1);
        let next = freq.next_date(&start_date);
        assert_eq!(next, Date::Day(date(2024, 6, 22)));

        // Test 2 weeks
        let freq = Frequency::Weeks(2);
        let next = freq.next_date(&start_date);
        assert_eq!(next, Date::Day(date(2024, 6, 29)));

        // Test 4 weeks (about 1 month)
        let freq = Frequency::Weeks(4);
        let next = freq.next_date(&start_date);
        assert_eq!(next, Date::Day(date(2024, 7, 13)));

        // Test crossing year boundary
        let year_end = Date::Day(date(2024, 12, 25));
        let freq = Frequency::Weeks(2);
        let next = freq.next_date(&year_end);
        assert_eq!(next, Date::Day(date(2025, 1, 8)));
    }

    #[test]
    fn test_every_x_months() {
        let start_date = Date::Day(date(2024, 6, 15));

        // Test 1 month
        let freq = Frequency::Months(1);
        let next = freq.next_date(&start_date);
        assert_eq!(next, Date::Day(date(2024, 7, 15)));

        // Test 3 months
        let freq = Frequency::Months(3);
        let next = freq.next_date(&start_date);
        assert_eq!(next, Date::Day(date(2024, 9, 15)));

        // Test 6 months
        let freq = Frequency::Months(6);
        let next = freq.next_date(&start_date);
        assert_eq!(next, Date::Day(date(2024, 12, 15)));

        // Test crossing year boundary
        let freq = Frequency::Months(12);
        let next = freq.next_date(&start_date);
        assert_eq!(next, Date::Day(date(2025, 6, 15)));

        // Test multiple year crossing
        let freq = Frequency::Months(18);
        let next = freq.next_date(&start_date);
        assert_eq!(next, Date::Day(date(2025, 12, 15)));
    }

    #[test]
    fn test_every_x_months_day_overflow() {
        // Test day overflow for shorter months

        // January 31st + 1 month = February 28th (or 29th in leap year)
        let jan_31 = Date::Day(date(2024, 1, 31));
        let freq = Frequency::Months(1);
        let next = freq.next_date(&jan_31);
        assert_eq!(next, Date::Day(date(2024, 2, 29))); // 2024 is a leap year

        // January 31st, 2023 + 1 month = February 28th (non-leap year)
        let jan_31_2023 = Date::Day(date(2023, 1, 31));
        let next = freq.next_date(&jan_31_2023);
        assert_eq!(next, Date::Day(date(2023, 2, 28))); // 2023 is not a leap year

        // March 31st + 1 month = April 30th
        let mar_31 = Date::Day(date(2024, 3, 31));
        let next = freq.next_date(&mar_31);
        assert_eq!(next, Date::Day(date(2024, 4, 30)));

        // May 31st + 1 month = June 30th
        let may_31 = Date::Day(date(2024, 5, 31));
        let next = freq.next_date(&may_31);
        assert_eq!(next, Date::Day(date(2024, 6, 30)));

        // Test with multiple months
        let jan_31 = Date::Day(date(2024, 1, 31));
        let freq = Frequency::Months(2);
        let next = freq.next_date(&jan_31);
        assert_eq!(next, Date::Day(date(2024, 3, 31))); // Jan + 2 months = March
    }

    #[test]
    fn test_every_x_years() {
        let start_date = Date::Day(date(2024, 6, 15));

        // Test 1 year
        let freq = Frequency::Years(1);
        let next = freq.next_date(&start_date);
        assert_eq!(next, Date::Day(date(2025, 6, 15)));

        // Test 5 years
        let freq = Frequency::Years(5);
        let next = freq.next_date(&start_date);
        assert_eq!(next, Date::Day(date(2029, 6, 15)));

        // Test 10 years
        let freq = Frequency::Years(10);
        let next = freq.next_date(&start_date);
        assert_eq!(next, Date::Day(date(2034, 6, 15)));
    }

    #[test]
    fn test_every_x_years_leap_year_edge_case() {
        // Test leap year edge case: Feb 29 -> Feb 28 in non-leap year
        let leap_day = Date::Day(date(2024, 2, 29)); // 2024 is a leap year
        let freq = Frequency::Years(1);
        let next = freq.next_date(&leap_day);
        assert_eq!(next, Date::Day(date(2025, 2, 28))); // 2025 is not a leap year

        // Test leap year to leap year
        let freq = Frequency::Years(4);
        let next = freq.next_date(&leap_day);
        assert_eq!(next, Date::Day(date(2028, 2, 29))); // 2028 is a leap year

        // Test non-leap to leap year (should keep Feb 28)
        let feb_28 = Date::Day(date(2023, 2, 28));
        let freq = Frequency::Years(1);
        let next = freq.next_date(&feb_28);
        assert_eq!(next, Date::Day(date(2024, 2, 28))); // Keeps Feb 28, doesn't become Feb 29
    }

    #[test]
    fn test_frequency_display() {
        // Test Display implementation
        assert_eq!(format!("{}", Frequency::Minutes(1)), "Every minute");
        assert_eq!(format!("{}", Frequency::Minutes(5)), "Every 5 minutes");

        assert_eq!(format!("{}", Frequency::Hours(1)), "Every hour");
        assert_eq!(format!("{}", Frequency::Hours(3)), "Every 3 hours");

        assert_eq!(format!("{}", Frequency::Days(1)), "Every day");
        assert_eq!(format!("{}", Frequency::Days(7)), "Every 7 days");

        assert_eq!(format!("{}", Frequency::Weeks(1)), "Every week");
        assert_eq!(format!("{}", Frequency::Weeks(2)), "Every 2 weeks");

        assert_eq!(format!("{}", Frequency::Months(1)), "Every month");
        assert_eq!(format!("{}", Frequency::Months(6)), "Every 6 months");

        assert_eq!(format!("{}", Frequency::Years(1)), "Every year");
        assert_eq!(format!("{}", Frequency::Years(5)), "Every 5 years");
    }

    #[test]
    fn test_edge_cases_month_boundaries() {
        // Test various month boundary conditions

        // December + 1 month = January next year
        let dec_15 = Date::Day(date(2024, 12, 15));
        let freq = Frequency::Months(1);
        let next = freq.next_date(&dec_15);
        assert_eq!(next, Date::Day(date(2025, 1, 15)));

        // December + 2 months = February next year
        let freq = Frequency::Months(2);
        let next = freq.next_date(&dec_15);
        assert_eq!(next, Date::Day(date(2025, 2, 15)));

        // Test month overflow with multiple years
        let freq = Frequency::Months(25); // 2 years + 1 month
        let next = freq.next_date(&dec_15);
        assert_eq!(next, Date::Day(date(2027, 1, 15)));
    }

    #[test]
    fn test_large_values() {
        // Test with larger frequency values
        let start_date = Date::Day(date(2024, 6, 15));

        // 100 days
        let freq = Frequency::Days(100);
        let next = freq.next_date(&start_date);
        assert_eq!(next, Date::Day(date(2024, 9, 23)));

        // 50 weeks (almost a year)
        let freq = Frequency::Weeks(50);
        let next = freq.next_date(&start_date);
        assert_eq!(next, Date::Day(date(2025, 5, 31)));

        // 24 months (2 years)
        let freq = Frequency::Months(24);
        let next = freq.next_date(&start_date);
        assert_eq!(next, Date::Day(date(2026, 6, 15)));
    }

    #[test]
    fn test_minimum_values() {
        // Test with minimum non-zero values
        let start_date = Date::Day(date(2024, 6, 15));
        let start_datetime = Date::DayTime(datetime(2024, 6, 15, 12, 0, 0));

        // 1 of each unit
        let freq = Frequency::Days(1);
        let next = freq.next_date(&start_date);
        assert_eq!(next, Date::Day(date(2024, 6, 16)));

        let freq = Frequency::Minutes(1);
        let next = freq.next_date(&start_datetime);
        assert_eq!(next, Date::DayTime(datetime(2024, 6, 15, 12, 1, 0)));

        let freq = Frequency::Hours(1);
        let next = freq.next_date(&start_datetime);
        assert_eq!(next, Date::DayTime(datetime(2024, 6, 15, 13, 0, 0)));
    }
}
