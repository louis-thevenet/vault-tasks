use std::iter::Peekable;

use color_eyre::{eyre::bail, Result};
use tracing::{debug, error};
use winnow::{
    ascii::{space0, space1},
    combinator::{alt, preceded, repeat},
    token::{take_till, take_while},
    PResult, Parser,
};

use crate::{task::Task, vault_data::VaultData, TasksConfig};

use super::task::parse_task;

enum FileToken {
    /// Name, Heading level
    Header((String, usize)),
    /// Content, Indent length
    Description(String, usize),
    /// Task, Indent length
    Task(Task, usize),
    /// A tag found outside a task in the file
    FileTag(String),
}

#[allow(clippy::module_name_repetitions)]
pub struct ParserFileEntry<'a> {
    pub config: &'a TasksConfig,
    pub filename: String,
}

impl<'i> ParserFileEntry<'i> {
    fn parse_indent(input: &mut &str) -> PResult<usize> {
        let indent_length: String = repeat(1.., " ").parse_next(input)?;
        Ok(indent_length.len())
    }
    fn parse_task(&self, input: &mut &str) -> PResult<FileToken> {
        let indent_length = Self::parse_indent(input).unwrap_or(0);

        let mut task_parser =
            |input: &mut &str| parse_task(input, self.filename.clone(), self.config);
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
    fn parse_file_tag(input: &mut &str) -> PResult<FileToken> {
        let tag = preceded(
            '#',
            take_while(1.., ('_', '0'..='9', 'A'..='Z', 'a'..='z', '0'..='9')),
        )
        .parse_next(input)?;
        Ok(FileToken::FileTag(tag.to_owned()))
    }
    fn insert_task_at(
        file_entry: &mut VaultData,
        task: Task,
        header_depth: usize,
        indent_length: usize,
    ) -> Result<()> {
        fn append_task_aux(
            file_entry: &mut VaultData,
            task_to_insert: Task,
            current_header_depth: usize,
            target_header_depth: usize,
            current_task_depth: usize,
            target_task_depth: usize,
        ) -> Result<()> {
            match file_entry {
                VaultData::Header(_, _, header_children) => {
                    match current_header_depth.cmp(&target_header_depth) {
                        std::cmp::Ordering::Greater => panic!("Target header level was greater than current level which is impossible"), // shouldn't happen
                        std::cmp::Ordering::Equal => {
                            // Found correct header level
                            if current_task_depth == target_task_depth {
                                header_children.push(VaultData::Task(task_to_insert));
                                Ok(())
                            } else {
                                for child in header_children.iter_mut().rev() {
                                    if let VaultData::Task(_task) = child {
                                        return append_task_aux(
                                            child,
                                            task_to_insert,
                                            current_header_depth,
                                            target_header_depth,
                                            current_task_depth + 1,
                                            target_task_depth,
                                        );
                                    }
                                }
                                bail!("Cound't find correct parent task to insert task {}", task_to_insert.name)
                            }
                        }
                        std::cmp::Ordering::Less => {
                            // Going deeper in header levels
                            for child in header_children.iter_mut().rev() {
                                if let VaultData::Header(_,_, _) = child {
                                    return append_task_aux(
                                        child,
                                        task_to_insert,
                                        current_header_depth + 1,
                                        target_header_depth,
                                        current_task_depth,
                                        target_task_depth,
                                    );
                                }
                            }
                                bail!("Cound't find correct parent header to insert task {}", task_to_insert.name)
                        }
                    }
                }
                VaultData::Task(task) => {
                    let mut current_task_depth = current_task_depth;
                    let mut last_task = task;
                    while current_task_depth < target_task_depth {
                        if last_task.subtasks.is_empty() {
                            error!("Could not find parent task, indenting may be wrong. Closest task line number: {}",last_task.line_number);
                            bail!("Failed to insert task")
                        }
                        last_task = last_task.subtasks.last_mut().unwrap();
                        current_task_depth += 1;
                    }
                    last_task.subtasks.push(task_to_insert);
                    Ok(())
                }
                VaultData::Directory(_, _) => {
                    bail!("Failed to insert task: tried to insert into a directory")
                }
            }
        }
        append_task_aux(file_entry, task, 0, header_depth, 0, indent_length)
    }
    /// Inserts a `FileEntry` at the specific header `depth` in `file_entry`.
    fn insert_header_at(
        file_entry: &mut VaultData,
        object: VaultData,
        target_header_depth: usize,
        target_task_depth: usize,
    ) {
        fn insert_at_aux(
            file_entry: &mut VaultData,
            object: VaultData,
            current_header_depth: usize,
            target_header_depth: usize,
            current_task_depth: usize,
            target_task_depth: usize,
        ) {
            match file_entry {
                VaultData::Header(_, _, header_children) => {
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
                                    if let VaultData::Task(_) = child {
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
                                if let VaultData::Header(_, _, _) = child {
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
                VaultData::Task(_) => {
                    error!("Error: tried to insert a header into a task");
                }
                VaultData::Directory(name, _) => {
                    error!("Error: tried to insert a header into a directory : {name}");
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
        file_entry: &mut VaultData,
        desc: String,
        target_header_depth: usize,
        target_task_depth: usize,
    ) -> Result<()> {
        fn append_description_aux(
            file_entry: &mut VaultData,
            desc: String,
            current_header_depth: usize,
            target_header_depth: usize,
            current_task_depth: usize,
            target_task_depth: usize,
        ) -> Result<()> {
            match file_entry {
                VaultData::Header(_, _, header_children) => {
                    match current_header_depth.cmp(&target_header_depth) {
                        std::cmp::Ordering::Greater => panic!("bad call for desc"), // shouldn't happen
                        std::cmp::Ordering::Equal => {
                            // Found correct header level
                            for child in header_children.iter_mut().rev() {
                                if let VaultData::Task(mut task) = child.clone() {
                                    if current_task_depth == target_task_depth {
                                        match &mut task.description {
                                            Some(d) => {
                                                d.push('\n');
                                                d.push_str(&desc.clone());
                                            }
                                            None => task.description = Some(desc.clone()),
                                        }
                                        *child = VaultData::Task(task);
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
                                if let VaultData::Header(_, _, _) = child {
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
                VaultData::Task(task) => {
                    fn insert_desc_task(
                        description: String,
                        task: &mut Task,
                        current_level: usize,
                        target_level: usize,
                    ) -> Result<()> {
                        if current_level == target_level {
                            match &mut task.description {
                                Some(d) => {
                                    d.push('\n');
                                    d.push_str(&description);
                                    Ok(())
                                }
                                None => {
                                    task.description = Some(description.clone());
                                    Ok(())
                                }
                            }
                        } else if let Some(task) = task.subtasks.last_mut() {
                            insert_desc_task(description, task, current_level + 1, target_level)
                        } else {
                            debug!("Description was too indented, adding to closest task: {description}");
                            insert_desc_task(description, task, current_level + 1, target_level)
                        }
                    }
                    insert_desc_task(desc, task, current_task_depth, target_task_depth)
                }
                VaultData::Directory(_, _) => {
                    bail!("Failed to insert description: tried to insert into a directory")
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
        file_entry: &mut VaultData,
        file_tags: &mut Vec<String>,
        header_depth: usize,
    ) where
        I: Iterator<Item = (usize, &'a str)>,
    {
        let mut parser = alt((
            Self::parse_file_tag,
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
                if Self::insert_task_at(
                    file_entry,
                    task,
                    header_depth,
                    indent_length / self.config.indent_length,
                )
                .is_err()
                {
                    error!("Failed to insert task");
                }
                self.parse_file_aux(input, file_entry, file_tags, header_depth);
            }
            Ok(FileToken::Header((header, new_depth))) => {
                Self::insert_header_at(
                    file_entry,
                    VaultData::Header(new_depth, header, vec![]),
                    new_depth - 1,
                    0,
                );
                self.parse_file_aux(input, file_entry, file_tags, new_depth);
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
                    error!("Failed to insert description {description}");
                }
                self.parse_file_aux(input, file_entry, file_tags, header_depth);
            }
            Ok(FileToken::FileTag(tag)) => {
                if !file_tags.contains(&tag) {
                    file_tags.push(tag);
                }
                self.parse_file_aux(input, file_entry, file_tags, header_depth);
            }
            Err(_) => self.parse_file_aux(input, file_entry, file_tags, header_depth),
        }
    }

    /// Removes any empty headers from a `FileEntry`
    fn clean_file_entry(file_entry: &mut VaultData) -> Option<&VaultData> {
        match file_entry {
            VaultData::Header(_, _, children) | VaultData::Directory(_, children) => {
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
            VaultData::Task(_) => Some(file_entry),
        }
    }

    pub fn parse_file(&mut self, filename: &str, input: &&str) -> Option<VaultData> {
        let lines = input.split('\n');

        let mut res = VaultData::Header(0, filename.to_owned(), vec![]);
        let mut file_tags = vec![];
        self.filename = filename.to_string();
        self.parse_file_aux(lines.enumerate().peekable(), &mut res, &mut file_tags, 0);

        if self.config.file_tags_propagation {
            file_tags.iter().for_each(|t| add_global_tag(&mut res, t));
        }

        // Filename is changed from Header to Directory variant at the end
        if let Some(VaultData::Header(_, name, children)) = Self::clean_file_entry(&mut res) {
            Some(VaultData::Directory(name.clone(), children.clone()))
        } else {
            None
        }
    }
}

fn add_global_tag(file_entry: &mut VaultData, tag: &String) {
    fn add_tag_aux(file_entry: &mut VaultData, tag: &String) {
        match file_entry {
            VaultData::Header(_, _, children) | VaultData::Directory(_, children) => {
                for child in children.iter_mut().rev() {
                    add_tag_aux(child, tag);
                }
            }
            VaultData::Task(task) => {
                fn insert_tag_task(task: &mut Task, tag: &String) {
                    match task.tags.clone() {
                        Some(mut tags) if !tags.contains(tag) => {
                            tags.push(tag.to_string());
                            task.tags = Some(tags);
                        }
                        None => task.tags = Some(vec![tag.to_string()]),
                        _ => (),
                    }

                    for st in &mut task.subtasks {
                        insert_tag_task(st, tag);
                    }
                }
                insert_tag_task(task, tag);
            }
        }
    }
    add_tag_aux(file_entry, tag);
}
#[cfg(test)]
mod tests {

    use insta::assert_snapshot;

    use super::ParserFileEntry;

    use crate::{
        parser::parser_file_entry::add_global_tag, task::Task, vault_data::VaultData, TasksConfig,
    };
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

        let config = TasksConfig {
            indent_length: 2,
            ..Default::default()
        };
        let mut res = VaultData::Header(0, "Test".to_string(), vec![]);
        let parser = ParserFileEntry {
            config: &config,
            filename: String::new(),
        };
        let expected = VaultData::Header(
            0,
            "Test".to_string(),
            vec![
                VaultData::Header(
                    1,
                    "1 useless".to_string(),
                    vec![VaultData::Header(
                        2,
                        "2 useless".to_string(),
                        vec![VaultData::Header(3, "3 useless".to_string(), vec![])],
                    )],
                ),
                VaultData::Header(
                    1,
                    "2 useful".to_string(),
                    vec![
                        VaultData::Header(3, "3 useless".to_string(), vec![]),
                        VaultData::Header(
                            2,
                            "4 useful".to_string(),
                            vec![VaultData::Task(Task {
                                name: "test".to_string(),
                                line_number: 8,
                                description: Some("test\ndesc".to_string()),
                                ..Default::default()
                            })],
                        ),
                    ],
                ),
            ],
        );
        parser.parse_file_aux(input, &mut res, &mut vec![], 0);
        assert_eq!(res, expected);

        let expected_after_cleaning = VaultData::Header(
            0,
            "Test".to_string(),
            vec![VaultData::Header(
                1,
                "2 useful".to_string(),
                vec![VaultData::Header(
                    2,
                    "4 useful".to_string(),
                    vec![VaultData::Task(Task {
                        name: "test".to_string(),
                        line_number: 8,
                        description: Some("test\ndesc".to_string()),
                        ..Default::default()
                    })],
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

        let config = TasksConfig {
            indent_length: 2,
            ..Default::default()
        };
        let mut res = VaultData::Header(0, "Test".to_string(), vec![]);
        let parser = ParserFileEntry {
            config: &config,
            filename: String::new(),
        };
        let expected = VaultData::Header(
            0,
            "Test".to_string(),
            vec![VaultData::Header(
                1,
                "1 Header".to_string(),
                vec![
                    VaultData::Task(Task {
                        name: "Task".to_string(),
                        line_number: 2,
                        ..Default::default()
                    }),
                    VaultData::Header(
                        2,
                        "2 Header".to_string(),
                        vec![VaultData::Header(
                            3,
                            "3 Header".to_string(),
                            vec![
                                VaultData::Task(Task {
                                    name: "Task".to_string(),
                                    line_number: 6,
                                    ..Default::default()
                                }),
                                VaultData::Task(Task {
                                    name: "Task 2".to_string(),
                                    line_number: 7,
                                    ..Default::default()
                                }),
                            ],
                        )],
                    ),
                    VaultData::Header(
                        2,
                        "2 Header 2".to_string(),
                        vec![VaultData::Task(Task {
                            name: "Task".to_string(),
                            line_number: 9,
                            description: Some("Description".to_string()),
                            ..Default::default()
                        })],
                    ),
                ],
            )],
        );
        parser.parse_file_aux(input, &mut res, &mut vec![], 0);
        assert_eq!(res, expected);
    }
    #[test]
    fn test_insert_global_tag() {
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

        let config = TasksConfig {
            indent_length: 2,
            ..Default::default()
        };
        let mut res = VaultData::Header(0, "Test".to_string(), vec![]);
        let parser = ParserFileEntry {
            config: &config,
            filename: String::new(),
        };
        parser.parse_file_aux(input, &mut res, &mut vec![], 0);
        add_global_tag(&mut res, &String::from("test"));
        assert_snapshot!(res);
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

        let config = TasksConfig {
            indent_length: 2,
            ..Default::default()
        };
        let mut res = VaultData::Header(0, "Test".to_string(), vec![]);
        let parser = ParserFileEntry {
            config: &config,
            filename: String::new(),
        };
        let expected = VaultData::Header(
            0,
            "Test".to_string(),
            vec![VaultData::Header(
                1,
                "1 Header".to_string(),
                vec![
                    VaultData::Task(Task {
                        name: "Task".to_string(),
                        line_number: 3,
                        ..Default::default()
                    }),
                    VaultData::Header(2, "2 Header".to_string(), vec![]),
                ],
            )],
        );
        parser.parse_file_aux(input, &mut res, &mut vec![], 0);
        assert_eq!(res, expected);
    }
    #[test]
    fn test_nested_tasks() {
        let input = r"# 1 Header
## Test
- [ ] Test a
  - [ ] Test b
    - [ ] Test c
"
        .split('\n')
        .enumerate()
        .peekable();

        let config = TasksConfig {
            indent_length: 2,
            ..Default::default()
        };
        let mut res = VaultData::Header(0, "Test".to_string(), vec![]);
        let parser = ParserFileEntry {
            config: &config,
            filename: String::new(),
        };
        let expected = VaultData::Header(
            0,
            "Test".to_string(),
            vec![VaultData::Header(
                1,
                "1 Header".to_string(),
                vec![VaultData::Header(
                    2,
                    "Test".to_string(),
                    vec![VaultData::Task(Task {
                        name: "Test a".to_string(),
                        line_number: 3,
                        subtasks: vec![Task {
                            name: "Test b".to_string(),
                            line_number: 4,
                            subtasks: vec![Task {
                                name: "Test c".to_string(),
                                line_number: 5,
                                ..Default::default()
                            }],
                            ..Default::default()
                        }],
                        ..Default::default()
                    })],
                )],
            )],
        );
        parser.parse_file_aux(input, &mut res, &mut vec![], 0);
        println!("{res:#?}");
        assert_eq!(res, expected);
    }
    #[test]
    fn test_nested_tasks_desc() {
        let input = r"# 1 Header
- [ ] t1
  t1
  - [ ] t2
    t2
  t1
    t2
    - [ ] t3
    t2
  t1
      t3
  t1
      - [ ] t4
    t2
        t4
      t3
        t4

"
        .split('\n')
        .enumerate()
        .peekable();

        let config = TasksConfig {
            indent_length: 2,
            ..Default::default()
        };
        let mut res = VaultData::Header(0, "Test".to_string(), vec![]);
        let parser = ParserFileEntry {
            config: &config,
            filename: String::new(),
        };
        parser.parse_file_aux(input, &mut res, &mut vec![], 0);
        assert_snapshot!(res);
    }
}
