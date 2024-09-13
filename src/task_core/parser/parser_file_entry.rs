use std::{iter::Peekable, path::Path};

use color_eyre::{eyre::bail, Result};
use tracing::error;
use winnow::{
    ascii::{space0, space1},
    combinator::{alt, preceded, repeat},
    token::take_till,
    PResult, Parser,
};

use crate::task_core::{config::Config, file_entry::FileEntry, task::Task};

use super::task::parse_task;

enum FileToken {
    Header((String, usize)),    // Header, Depth
    Description(String, usize), // Content, indent length
    Task(Task, usize),          // Content, indent length
}

#[allow(clippy::module_name_repetitions)]
pub struct ParserFileEntry<'a> {
    pub config: &'a Config,
}

impl<'i> ParserFileEntry<'i> {
    fn parse_indent(input: &mut &str) -> PResult<usize> {
        let indent_length: String = repeat(1.., " ").parse_next(input)?;
        Ok(indent_length.len())
    }
    fn parse_task(&self, input: &mut &str) -> PResult<FileToken> {
        let indent_length = Self::parse_indent(input).unwrap_or(0);

        let mut task_parser = |input: &mut &str| parse_task(input, self.config);
        let task_res = task_parser.parse_next(input)?;
        Ok(FileToken::Task(task_res, indent_length))
    }
    fn parse_header(input: &mut &str) -> PResult<FileToken> {
        let header_depth: String = repeat(1.., "#").parse_next(input)?;
        let header_content = preceded(space0, take_till(1.., |c| c == '\n')).parse_next(input)?;

        Ok(FileToken::Header((
            header_content.to_string(),
            header_depth.len(),
        )))
    }
    fn parse_description(input: &mut &str) -> PResult<FileToken> {
        let indent_length = space1.map(|s: &str| s.len()).parse_next(input)?;
        let desc_content = take_till(1.., |c| c == '\n').parse_next(input)?;
        Ok(FileToken::Description(
            desc_content.to_string(),
            indent_length,
        ))
    }
    /// Inserts a `FileEntry` at the specific header `depth` in `file_entry`.
    fn insert_at(
        file_entry: &mut FileEntry,
        object: FileEntry,
        target_header_depth: usize,
        target_task_depth: usize,
    ) {
        fn insert_at_aux(
            file_entry: &mut FileEntry,
            object: FileEntry,
            current_header_depth: usize,
            target_header_depth: usize,
            current_task_depth: usize,
            target_task_depth: usize,
        ) {
            match file_entry {
                FileEntry::Header(_, header_children) => {
                    match (current_header_depth).cmp(&target_header_depth) {
                        std::cmp::Ordering::Greater => error!(
                            "bad call to `insert_at`, file_entry:{file_entry}\nobject:{object}"
                        ), // shouldn't happen
                        std::cmp::Ordering::Equal => {
                            // Found correct header level
                            if current_task_depth == target_task_depth {
                                header_children.push(object);
                            } else {
                                for child in header_children.iter_mut().rev() {
                                    if let FileEntry::Task(_, _) = child {
                                        return insert_at_aux(
                                            child,
                                            object,
                                            current_header_depth,
                                            target_header_depth,
                                            current_task_depth + 1,
                                            target_task_depth,
                                        );
                                    }
                                }
                            }
                        }
                        std::cmp::Ordering::Less => {
                            // Still haven't found correct header level, going deeper
                            for child in header_children.iter_mut().rev() {
                                if let FileEntry::Header(_, _) = child {
                                    insert_at_aux(
                                        child,
                                        object,
                                        current_header_depth + 1,
                                        target_header_depth,
                                        current_task_depth,
                                        target_task_depth,
                                    );
                                    return;
                                }
                            }
                            header_children.push(object);
                        }
                    }
                }
                FileEntry::Task(_task, subtasks) => {
                    if current_task_depth < target_task_depth {
                        for subtask in subtasks.iter_mut().rev() {
                            if let FileEntry::Task(_, _) = subtask {
                                return insert_at_aux(
                                    subtask,
                                    object,
                                    current_header_depth,
                                    target_header_depth,
                                    current_task_depth + 1,
                                    target_task_depth,
                                );
                            }
                        }
                    } else {
                        subtasks.push(object);
                    }
                }
            }
        }
        insert_at_aux(
            file_entry,
            object,
            0,
            target_header_depth,
            0,
            target_task_depth,
        );
    }

    /// Appends `desc` to the description of an existing `Task` in the `FileEntry`.
    fn append_description(
        file_entry: &mut FileEntry,
        desc: String,
        target_header_depth: usize,
        target_task_depth: usize,
    ) -> Result<()> {
        fn append_description_aux(
            file_entry: &mut FileEntry,
            desc: String,
            current_header_depth: usize,
            target_header_depth: usize,
            current_task_depth: usize,
            target_task_depth: usize,
        ) -> Result<()> {
            match file_entry {
                FileEntry::Header(_, header_children) => {
                    match current_header_depth.cmp(&target_header_depth) {
                        std::cmp::Ordering::Greater => panic!("bad call for desc"), // shouldn't happen
                        std::cmp::Ordering::Equal => {
                            // Found correct header level
                            for child in header_children.iter_mut().rev() {
                                if let FileEntry::Task(mut task, subtasks) = child.clone() {
                                    if current_task_depth == target_task_depth {
                                        match &mut task.description {
                                            Some(d) => {
                                                d.push('\n');
                                                d.push_str(&desc.clone());
                                            }
                                            None => task.description = Some(desc.clone()),
                                        }
                                        *child = FileEntry::Task(task, subtasks);
                                    } else {
                                        return append_description_aux(
                                            child,
                                            desc,
                                            current_header_depth,
                                            target_header_depth,
                                            current_task_depth + 1,
                                            target_task_depth,
                                        );
                                    }
                                }
                            }
                            Ok(())
                        }
                        std::cmp::Ordering::Less => {
                            // Going deeper in header levels
                            for child in header_children.iter_mut().rev() {
                                if let FileEntry::Header(_, _) = child {
                                    return append_description_aux(
                                        child,
                                        desc,
                                        current_header_depth + 1,
                                        target_header_depth,
                                        current_task_depth,
                                        target_task_depth,
                                    );
                                }
                            }
                            bail!("Failed to insert description: previous task not found");
                        }
                    }
                }
                FileEntry::Task(task, subtasks) => {
                    if current_task_depth == target_task_depth {
                        match &mut task.description {
                            Some(d) => {
                                d.push('\n');
                                d.push_str(&desc);
                                Ok(())
                            }
                            None => {
                                task.description = Some(desc.clone());
                                Ok(())
                            }
                        }
                    } else if let Some(last_task) = subtasks.last_mut() {
                        append_description_aux(
                            last_task,
                            desc,
                            current_header_depth,
                            target_header_depth,
                            current_task_depth + 1,
                            target_task_depth,
                        )
                    } else {
                        error!("Could not find description's parent task, indenting may be wrong. Closest task line number: {}",task.line_number);
                        bail!("Failed to insert description")
                    }
                }
            }
        }
        append_description_aux(
            file_entry,
            desc,
            0,
            target_header_depth,
            0,
            target_task_depth,
        )
    }

    /// Recursively parses the input file passed as a string.
    fn parse_file_aux<'a, I>(
        &self,
        mut input: Peekable<I>,
        file_entry: &mut FileEntry,
        header_depth: usize,
    ) where
        I: Iterator<Item = (usize, &'a str)>,
    {
        let mut parser = alt((
            Self::parse_header,
            |input: &mut &str| self.parse_task(input),
            Self::parse_description,
        ));

        let line_opt = input.next();
        if line_opt.is_none() {
            return;
        }

        let (line_number, mut line) = line_opt.unwrap();

        match parser.parse_next(&mut line) {
            Ok(FileToken::Task(mut task, indent_length)) => {
                task.line_number = line_number + 1; // line 1 was element 0 of iterator
                Self::insert_at(
                    file_entry,
                    FileEntry::Task(task, vec![]),
                    header_depth,
                    indent_length / self.config.indent_length,
                );
                self.parse_file_aux(input, file_entry, header_depth);
            }
            Ok(FileToken::Header((header, new_depth))) => {
                Self::insert_at(
                    file_entry,
                    FileEntry::Header(header, vec![]),
                    new_depth - 1,
                    0,
                );
                self.parse_file_aux(input, file_entry, new_depth);
            }
            Ok(FileToken::Description(description, indent_length)) => {
                if Self::append_description(
                    file_entry,
                    description.clone(),
                    header_depth,
                    indent_length / self.config.indent_length,
                )
                .is_err()
                {
                    error!("Failed to insert description {description}]");
                }
                self.parse_file_aux(input, file_entry, header_depth);
            }

            Err(_) => self.parse_file_aux(input, file_entry, header_depth),
        }
    }

    /// Removes any empty headers from a `FileEntry`
    fn clean_file_entry(file_entry: &mut FileEntry) -> Option<&FileEntry> {
        match file_entry {
            FileEntry::Header(_, children) => {
                let mut actual_children = vec![];
                for child in children.iter_mut() {
                    let mut child_clone = child.clone();
                    if Self::clean_file_entry(&mut child_clone).is_some() {
                        actual_children.push(child_clone);
                    }
                }
                *children = actual_children;
                if children.is_empty() {
                    None
                } else {
                    Some(file_entry)
                }
            }
            FileEntry::Task(_, _) => Some(file_entry),
        }
    }

    pub fn parse_file(&self, filename: &Path, input: &&str) -> Option<FileEntry> {
        let lines = input.split('\n');

        let filename = filename
            .strip_prefix(self.config.vault_path.clone())
            .unwrap();

        let mut res = FileEntry::Header(filename.to_str().unwrap().to_owned(), vec![]);
        self.parse_file_aux(lines.enumerate().peekable(), &mut res, 0);
        Self::clean_file_entry(&mut res).cloned()
    }
}
#[cfg(test)]
mod tests {

    use crate::task_core::{config::Config, file_entry::FileEntry, task::Task};

    use super::ParserFileEntry;

    #[test]
    fn test_with_useless_headers() {
        let input = r"# 1 useless
## 2 useless
### 3 useless

# 2 useful
### 3 useless
## 4 useful
- [ ] test
  test
  desc
"
        .split('\n')
        .enumerate()
        .peekable();

        let config = &Config::default();
        let mut res = FileEntry::Header("Test".to_string(), vec![]);
        let parser = ParserFileEntry { config };
        let expected = FileEntry::Header(
            "Test".to_string(),
            vec![
                FileEntry::Header(
                    "1 useless".to_string(),
                    vec![FileEntry::Header(
                        "2 useless".to_string(),
                        vec![FileEntry::Header("3 useless".to_string(), vec![])],
                    )],
                ),
                FileEntry::Header(
                    "2 useful".to_string(),
                    vec![
                        FileEntry::Header("3 useless".to_string(), vec![]),
                        FileEntry::Header(
                            "4 useful".to_string(),
                            vec![FileEntry::Task(
                                Task {
                                    name: "test".to_string(),
                                    line_number: 8,
                                    description: Some("test\ndesc".to_string()),
                                    ..Default::default()
                                },
                                vec![],
                            )],
                        ),
                    ],
                ),
            ],
        );
        parser.parse_file_aux(input, &mut res, 0);
        assert_eq!(res, expected);

        let expected_after_cleaning = FileEntry::Header(
            "Test".to_string(),
            vec![FileEntry::Header(
                "2 useful".to_string(),
                vec![FileEntry::Header(
                    "4 useful".to_string(),
                    vec![FileEntry::Task(
                        Task {
                            name: "test".to_string(),
                            line_number: 8,
                            description: Some("test\ndesc".to_string()),
                            ..Default::default()
                        },
                        vec![],
                    )],
                )],
            )],
        );
        ParserFileEntry::clean_file_entry(&mut res);
        assert_eq!(res, expected_after_cleaning);
    }
    #[test]
    fn test_simple_input() {
        let input = r"# 1 Header
- [ ] Task

## 2 Header
### 3 Header
- [ ] Task
- [ ] Task 2
## 2 Header 2
- [ ] Task
  Description

"
        .split('\n')
        .enumerate()
        .peekable();

        let config = &Config::default();
        let mut res = FileEntry::Header("Test".to_string(), vec![]);
        let parser = ParserFileEntry { config };
        let expected = FileEntry::Header(
            "Test".to_string(),
            vec![FileEntry::Header(
                "1 Header".to_string(),
                vec![
                    FileEntry::Task(
                        Task {
                            name: "Task".to_string(),
                            line_number: 2,
                            ..Default::default()
                        },
                        vec![],
                    ),
                    FileEntry::Header(
                        "2 Header".to_string(),
                        vec![FileEntry::Header(
                            "3 Header".to_string(),
                            vec![
                                FileEntry::Task(
                                    Task {
                                        name: "Task".to_string(),
                                        line_number: 6,
                                        ..Default::default()
                                    },
                                    vec![],
                                ),
                                FileEntry::Task(
                                    Task {
                                        name: "Task 2".to_string(),
                                        line_number: 7,
                                        ..Default::default()
                                    },
                                    vec![],
                                ),
                            ],
                        )],
                    ),
                    FileEntry::Header(
                        "2 Header 2".to_string(),
                        vec![FileEntry::Task(
                            Task {
                                name: "Task".to_string(),
                                line_number: 9,
                                description: Some("Description".to_string()),
                                ..Default::default()
                            },
                            vec![],
                        )],
                    ),
                ],
            )],
        );
        parser.parse_file_aux(input, &mut res, 0);
        assert_eq!(res, expected);
    }
    #[test]
    fn test_fake_description() {
        let input = r"# 1 Header
  test
- [ ] Task

## 2 Header
  test
"
        .split('\n')
        .enumerate()
        .peekable();

        let config = &Config::default();
        let mut res = FileEntry::Header("Test".to_string(), vec![]);
        let parser = ParserFileEntry { config };
        let expected = FileEntry::Header(
            "Test".to_string(),
            vec![FileEntry::Header(
                "1 Header".to_string(),
                vec![
                    FileEntry::Task(
                        Task {
                            name: "Task".to_string(),
                            line_number: 3,
                            ..Default::default()
                        },
                        vec![],
                    ),
                    FileEntry::Header("2 Header".to_string(), vec![]),
                ],
            )],
        );
        parser.parse_file_aux(input, &mut res, 0);
        assert_eq!(res, expected);
    }
}
