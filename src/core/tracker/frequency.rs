use std::fmt::Display;
use strum::{EnumIter, FromRepr};

#[derive(Clone, Copy, FromRepr, EnumIter, Debug, PartialEq, Eq)]
pub enum Frequency {
    #[strum(to_string = "minute")]
    EveryXMinutes(u64),
    #[strum(to_string = "hour")]
    EveryXHours(u64),
    #[strum(to_string = "day")]
    EveryXDays(u64),
    #[strum(to_string = "week")]
    EveryXWeeks(u64),
    #[strum(to_string = "month")]
    EveryXMonths(u64),
    #[strum(to_string = "yeah")]
    EveryXYears(u64),
}
impl Display for Frequency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Frequency::EveryXMinutes(n)
            | Frequency::EveryXHours(n)
            | Frequency::EveryXDays(n)
            | Frequency::EveryXWeeks(n)
            | Frequency::EveryXMonths(n)
            | Frequency::EveryXYears(n) => {
                if *n == 1 {
                    write!(f, "Every {}", self.to_string())
                } else {
                    write!(f, "Every {} {}s", n, self.to_string())
                }
            }
        }
    }
}
