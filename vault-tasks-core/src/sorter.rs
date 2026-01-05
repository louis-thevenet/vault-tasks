use std::cmp::Ordering;

use chrono::NaiveTime;
use lexical_sort::lexical_cmp;
use strum::EnumIter;
use strum_macros::FromRepr;

use super::{date::Date, task::Task};

#[derive(Default, Clone, Copy, FromRepr, EnumIter, strum_macros::Display)]
pub enum SortingMode {
    #[default]
    #[strum(to_string = "Due Date")]
    ByDueDate,
    #[strum(to_string = "Title")]
    ByName,
}

impl SortingMode {
    #[must_use]
    pub fn next(self) -> Self {
        match self {
            Self::ByDueDate => Self::ByName,
            Self::ByName => Self::ByDueDate,
        }
    }
    pub fn sort(tasks: &mut [Task], sorter: Self) {
        tasks.sort_by(|t1, t2| Self::cmp(t1, t2, sorter));
    }

    /// Compare two tasks by due date
    pub fn cmp_due_date(t1: &Task, t2: &Task) -> Ordering {
        match (&t1.due_date, &t2.due_date) {
            (Some(Date::Day(d1)), Some(Date::Day(d2))) => d1.cmp(d2),
            (Some(Date::DayTime(d1)), Some(Date::DayTime(d2))) => d1.cmp(d2),
            (Some(Date::Day(d1)), Some(Date::DayTime(d2))) => {
                d1.and_time(NaiveTime::default()).cmp(d2)
            }
            (Some(Date::DayTime(d1)), Some(Date::Day(d2))) => {
                d1.cmp(&d2.and_time(NaiveTime::default()))
            }
            (None, Some(Date::Day(_) | Date::DayTime(_))) => Ordering::Greater,
            (Some(Date::Day(_) | Date::DayTime(_)), None) => Ordering::Less,
            _ => Ordering::Equal,
        }
    }
    /// Compares two tasks with the specified sorting mode
    /// Sorting mode is used first
    /// If equal, other attributes will be used:
    /// - State: `ToDo` < `Done` (in Ord impl of `State`)
    /// - The other sorting mode
    /// - Priority: usual number ordering
    /// - Tags: not used
    fn cmp(t1: &Task, t2: &Task, sorter: Self) -> Ordering {
        let res_initial_sort = match sorter {
            Self::ByDueDate => Self::cmp_due_date(t1, t2),
            Self::ByName => lexical_cmp(&t1.name, &t2.name),
        };

        if !matches!(res_initial_sort, Ordering::Equal) {
            return res_initial_sort;
        }

        // Compare states
        let res = t1.state.cmp(&t2.state);
        if !matches!(res, Ordering::Equal) {
            return res;
        }

        // We do the other sorting methods
        let res = match sorter {
            Self::ByDueDate => lexical_cmp(&t1.name, &t2.name),
            Self::ByName => Self::cmp_due_date(t1, t2),
        };
        if !matches!(res, Ordering::Equal) {
            return res;
        }

        t1.priority.cmp(&t2.priority)
    }
}
#[cfg(test)]
mod tests {

    use std::path::PathBuf;

    use insta::{assert_debug_snapshot, with_settings};

    use super::SortingMode;
    use crate::{TasksConfig, parser::task::parse_task, task::Task};
    #[test]
    fn task_sort_by_name() {
        let mut source = [
            "- [ ] test 25-10-11",
            "- [ ] test 25-10-9",
            "- [ ] test 25-10-10 p5",
            "- [ ] test 25-10-10 10:00",
            "- [x] zèbre",
            "- [x] zzz",
            "- [ ] zzz",
            "- [ ] test 25-10-10 p2",
            "- [x] test",
            "- [ ] test2",
            "- [ ] test 25-10-10 5:00",
            "- [ ] abc",
        ];
        let config = TasksConfig {
            ..Default::default()
        };
        let mut tasks: Vec<Task> = source
            .iter_mut()
            .map(|input| parse_task(input, &PathBuf::new(), &config).unwrap())
            .collect();

        let sorting_mode = SortingMode::ByName;
        SortingMode::sort(&mut tasks, sorting_mode);

        let tasks = tasks
            .iter()
            .map(|task| task.get_fixed_attributes(&config, 2))
            .collect::<Vec<String>>();

        with_settings!({
            info=>&source,
            description => "", // the template source code
        }, {
                assert_debug_snapshot!(tasks);
        });
    }
    #[test]
    fn task_sort_by_due_date() {
        let mut source = [
            "- [ ] test 2025/10/11",
            "- [ ] test 2025/10/9",
            "- [ ] test 2025/10/10 p5",
            "- [ ] test 2025/10/10 10:00",
            "- [x] zèbre",
            "- [x] zzz",
            "- [ ] zzz",
            "- [ ] test 2025/10/10 p2",
            "- [x] test",
            "- [ ] test2",
            "- [ ] test 2025/10/10 5:00",
            "- [ ] abc",
        ];
        let config = TasksConfig {
            ..Default::default()
        };
        let mut tasks: Vec<Task> = source
            .iter_mut()
            .map(|input| parse_task(input, &PathBuf::new(), &config).unwrap())
            .collect();

        let sorting_mode = SortingMode::ByDueDate;
        SortingMode::sort(&mut tasks, sorting_mode);

        let tasks = tasks
            .iter()
            .map(|task| task.get_fixed_attributes(&config, 2))
            .collect::<Vec<String>>();

        with_settings!({
            info=>&source,
            description => "", // the template source code
        }, {
                assert_debug_snapshot!(tasks);
        });
    }
    #[test]
    fn task_sort_states() {
        let mut source = ["- [ ] test", "- [x] test", "- [/] test", "- [-] test"];
        let config = TasksConfig {
            ..Default::default()
        };
        let mut tasks: Vec<Task> = source
            .iter_mut()
            .map(|input| parse_task(input, &PathBuf::new(), &config).unwrap())
            .collect();

        let sorting_mode = SortingMode::ByDueDate;
        SortingMode::sort(&mut tasks, sorting_mode);

        let tasks = tasks
            .iter()
            .map(|task| task.get_fixed_attributes(&config, 2))
            .collect::<Vec<String>>();

        with_settings!({
            info=>&source,
            description => "", // the template source code
        }, {
                assert_debug_snapshot!(tasks);
        });
    }
}
