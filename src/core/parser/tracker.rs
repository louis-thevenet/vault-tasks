use winnow::{
    Parser, Result,
    combinator::{alt, fail, preceded, repeat},
    token::any,
};
mod parse_frequency;

use crate::core::tracker::{IncompleteTracker, NewTracker, frequency::Frequency};
fn parse_tracker_definition(input: &mut &str) -> Result<NewTracker> {
    preceded("Tracker: ", |input: &mut &str| {
        let res = repeat(0.., any)
            .fold(String::new, |mut string, c| {
                string.push(c);
                string
            })
            .parse_next(input)?;
        Ok(NewTracker::new(res))
    })
    .parse_next(input)
}
// // fn parse_tracker_first_row(new_tracker: NewTracker, input: &mut &str) -> Result<IncompleteTracker> {

// //     // | Frequency | categories | ... | Notes !
// }
