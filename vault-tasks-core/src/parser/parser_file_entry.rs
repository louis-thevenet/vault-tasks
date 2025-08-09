use std::iter::Peekable;

use color_eyre::eyre::bail;
use tracing::{debug, error, info};
use winnow::{
    Parser, Result,
    ascii::{space0, space1},
    combinator::{alt, delimited, preceded, repeat},
    token::{any, take_till, take_until, take_while},
};

use crate::{
    TasksConfig,
    parser::tracker::{parse_entries, parse_header, parse_separator},
    task::Task,
    tracker::{NewTracker, Tracker},
    vault_data::VaultData,
};

use super::{task::parse_task, tracker::parse_tracker_definition};

enum FileToken {
    /// Name, Heading level
    Header((String, usize)),
    /// Content, Indent length
    Description(String, usize),
    /// Task, Indent length
    Task(Task, usize),
    /// A tag found outside a task in the file
    FileTag(String),
    // Full comment
    FullComment,
    /// New comment
    StartOfComment,
    /// A comment was closed
    EndOfComment,
    /// New code block
    StartOfCodeBlock,
    /// A code block was closed
    EndOfCodeBlock,
    /// Tracker Definition
    TrackerDefinition(NewTracker),
}

#[allow(clippy::module_name_repetitions)]
pub struct ParserFileEntry<'a> {
    pub config: &'a TasksConfig,
    pub filename: String,
}

impl ParserFileEntry<'_> {
    fn parse_indent(input: &mut &str) -> Result<usize> {
        let indent_length: String = repeat(1.., " ").parse_next(input)?;
        Ok(indent_length.len())
    }
    fn parse_task(&self, input: &mut &str) -> Result<FileToken> {
        let indent_length = Self::parse_indent(input).unwrap_or(0);

        let mut task_parser =
            |input: &mut &str| parse_task(input, self.filename.clone(), self.config);
        let task_res = task_parser.parse_next(input)?;
        Ok(FileToken::Task(task_res, indent_length))
    }
    fn parse_tracker_def(&self, input: &mut &str) -> Result<FileToken> {
        Ok(FileToken::TrackerDefinition(parse_tracker_definition(
            input,
            self.config,
        )?))
    }
    fn parse_header(input: &mut &str) -> Result<FileToken> {
        let header_depth: String = repeat(1.., "#").parse_next(input)?;
        let header_content = preceded(space0, take_till(1.., |c| c == '\n')).parse_next(input)?;

        Ok(FileToken::Header((
            header_content.to_string(),
            header_depth.len(),
        )))
    }
    fn parse_description(input: &mut &str) -> Result<FileToken> {
        let indent_length = space1.map(|s: &str| s.len()).parse_next(input)?;
        let desc_content = take_till(1.., |c| c == '\n').parse_next(input)?;
        Ok(FileToken::Description(
            desc_content.to_string(),
            indent_length,
        ))
    }
    fn parse_file_tag(input: &mut &str) -> Result<FileToken> {
        let tag = preceded(
            '#',
            take_while(1.., ('_', '0'..='9', 'A'..='Z', 'a'..='z', '0'..='9')),
        )
        .parse_next(input)?;
        Ok(FileToken::FileTag(tag.to_owned()))
    }

    fn parse_start_of_comment(input: &mut &str) -> Result<FileToken> {
        preceded::<_, _, (), _, _, _>(alt(("<--", "<!--")), repeat(0.., any)).parse_next(input)?;
        Ok(FileToken::StartOfComment)
    }
    fn parse_end_of_comment(input: &mut &str) -> Result<FileToken> {
        take_until(0.., "-->").parse_next(input)?;
        Ok(FileToken::EndOfComment)
    }
    fn parse_full_comment(input: &mut &str) -> Result<FileToken> {
        delimited(alt(("<!--", "<--")), take_until(0.., "-->"), "-->").parse_next(input)?;
        Ok(FileToken::FullComment)
    }
    fn parse_start_of_code_block(input: &mut &str) -> Result<FileToken> {
        preceded::<_, _, (), _, _, _>("```", repeat(0.., any)).parse_next(input)?;
        Ok(FileToken::StartOfCodeBlock)
    }
    fn parse_end_of_code_block(input: &mut &str) -> Result<FileToken> {
        take_until(0.., "```").parse_next(input)?;
        Ok(FileToken::EndOfCodeBlock)
    }
    fn insert_task_at(
        file_entry: &mut VaultData,
        task: Task,
        header_depth: usize,
        indent_length: usize,
    ) -> color_eyre::Result<()> {
        fn append_task_aux(
            file_entry: &mut VaultData,
            task_to_insert: Task,
            current_header_depth: usize,
            target_header_depth: usize,
            current_task_depth: usize,
            target_task_depth: usize,
        ) -> color_eyre::Result<()> {
            match file_entry {
                VaultData::Header(_, _, header_children) => {
                    match current_header_depth.cmp(&target_header_depth) {
                        std::cmp::Ordering::Greater => panic!(
                            "Target header level was greater than current level which is impossible"
                        ), // shouldn't happen
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
                                bail!(
                                    "Couldn't find correct parent task to insert task {}",
                                    task_to_insert.name
                                )
                            }
                        }
                        std::cmp::Ordering::Less => {
                            // Going deeper in header levels
                            for child in header_children.iter_mut().rev() {
                                if let VaultData::Header(_, _, _) = child {
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
                            bail!(
                                "Couldn't find correct parent header to insert task {}",
                                task_to_insert.name
                            )
                        }
                    }
                }
                VaultData::Task(task) => {
                    let mut current_task_depth = current_task_depth;
                    let mut last_task = task;
                    while current_task_depth < target_task_depth {
                        if last_task.subtasks.is_empty() {
                            error!(
                                "Could not find parent task, indenting may be wrong. Closest task line number: {:?}",
                                last_task.line_number
                            );
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
                VaultData::Tracker(_tracker) => {
                    bail!("Failed to insert task: tried to insert into a tracker")
                }
            }
        }
        append_task_aux(file_entry, task, 0, header_depth, 0, indent_length)
    }
    fn insert_tracker_at(
        file_entry: &mut VaultData,
        tracker: Tracker,
        header_depth: usize,
    ) -> color_eyre::Result<()> {
        fn append_tracker_aux(
            file_entry: &mut VaultData,
            tracker_to_insert: Tracker,
            current_header_depth: usize,
            target_header_depth: usize,
        ) -> color_eyre::Result<()> {
            match file_entry {
                VaultData::Header(_, _, header_children) => {
                    match current_header_depth.cmp(&target_header_depth) {
                        std::cmp::Ordering::Greater => panic!(
                            "Target header level was greater than current level which is impossible"
                        ), // shouldn't happen
                        std::cmp::Ordering::Equal => {
                            // Found correct header level
                            header_children.push(VaultData::Tracker(tracker_to_insert));
                            Ok(())
                        }

                        std::cmp::Ordering::Less => {
                            // Going deeper in header levels
                            for child in header_children.iter_mut().rev() {
                                if let VaultData::Header(_, _, _) = child {
                                    return append_tracker_aux(
                                        child,
                                        tracker_to_insert,
                                        current_header_depth + 1,
                                        target_header_depth,
                                    );
                                }
                            }
                            bail!(
                                "Couldn't find correct parent header to insert task {}",
                                tracker_to_insert.name
                            )
                        }
                    }
                }
                VaultData::Task(_task) => {
                    bail!("Tried to insert a Tracker in a task")
                }
                VaultData::Directory(_, _) => {
                    bail!("Failed to insert task: tried to insert into a directory")
                }
                VaultData::Tracker(_tracker) => bail!("Tried to insert a Tracker in a tracker"),
            }
        }
        append_tracker_aux(file_entry, tracker, 0, header_depth)
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
                VaultData::Tracker(tracker) => error!(
                    "Error: tried to insert a header into a tracker: {}",
                    tracker.name
                ),
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
    ) -> color_eyre::Result<()> {
        fn append_description_aux(
            file_entry: &mut VaultData,
            desc: String,
            current_header_depth: usize,
            target_header_depth: usize,
            current_task_depth: usize,
            target_task_depth: usize,
        ) -> color_eyre::Result<()> {
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
                    ) -> color_eyre::Result<()> {
                        if current_level == target_level {
                            if let Some(d) = &mut task.description {
                                d.push('\n');
                                d.push_str(&description);
                                Ok(())
                            } else {
                                task.description = Some(description.clone());
                                Ok(())
                            }
                        } else if let Some(task) = task.subtasks.last_mut() {
                            insert_desc_task(description, task, current_level + 1, target_level)
                        } else {
                            debug!(
                                "Description was too indented, adding to closest task: {description}"
                            );
                            insert_desc_task(description, task, current_level + 1, target_level)
                        }
                    }
                    insert_desc_task(desc, task, current_task_depth, target_task_depth)
                }
                VaultData::Directory(_, _) => {
                    bail!("Failed to insert description: tried to insert into a directory")
                }
                VaultData::Tracker(_tracker) => {
                    bail!("Failed to insert description: tried to insert into a tracker")
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
    #[allow(clippy::too_many_lines)]
    fn parse_file_aux<'a, I>(
        &self,
        mut input: Peekable<I>,
        file_entry: &mut VaultData,
        file_tags: &mut Vec<String>,
        header_depth: usize,
        comment_depth: usize,
        code_block: bool,
    ) where
        I: Iterator<Item = (usize, &'a str)>,
    {
        let mut parser = alt((
            Self::parse_full_comment,
            Self::parse_start_of_comment,
            Self::parse_end_of_comment,
            Self::parse_start_of_code_block,
            Self::parse_end_of_code_block,
            Self::parse_file_tag,
            |input: &mut &str| self.parse_tracker_def(input),
            Self::parse_header,
            |input: &mut &str| self.parse_task(input),
            Self::parse_description,
        ));

        let line_opt = input.next();
        if line_opt.is_none() {
            return;
        }

        let (line_number, mut line) = line_opt.unwrap();

        if code_block {
            // We're in a code block
            match parser.parse_next(&mut line) {
                Ok(FileToken::EndOfCodeBlock | FileToken::StartOfCodeBlock) => self.parse_file_aux(
                    input,
                    file_entry,
                    file_tags,
                    header_depth,
                    comment_depth,
                    false,
                ),

                _ => {
                    self.parse_file_aux(
                        input,
                        file_entry,
                        file_tags,
                        header_depth,
                        comment_depth,
                        code_block,
                    );
                }
            }
        } else if comment_depth > 0 {
            // We're in a comment
            match parser.parse_next(&mut line) {
                Ok(FileToken::StartOfComment) => self.parse_file_aux(
                    input,
                    file_entry,
                    file_tags,
                    header_depth,
                    comment_depth + 1,
                    false,
                ),

                Ok(FileToken::EndOfComment) => self.parse_file_aux(
                    input,
                    file_entry,
                    file_tags,
                    header_depth,
                    comment_depth.saturating_sub(1),
                    false,
                ),

                _ => {
                    self.parse_file_aux(
                        input,
                        file_entry,
                        file_tags,
                        header_depth,
                        comment_depth,
                        false,
                    );
                }
            }
        } else {
            match parser.parse_next(&mut line) {
                Ok(FileToken::Task(mut task, indent_length)) => {
                    task.line_number = Some(line_number + 1); // line 1 was element 0 of iterator
                    if Self::insert_task_at(
                        file_entry,
                        task,
                        header_depth,
                        indent_length / self.config.core.indent_length,
                    )
                    .is_err()
                    {
                        error!("Failed to insert task");
                    }
                    self.parse_file_aux(
                        input,
                        file_entry,
                        file_tags,
                        header_depth,
                        comment_depth,
                        false,
                    );
                }
                Ok(FileToken::Header((header, new_depth))) => {
                    Self::insert_header_at(
                        file_entry,
                        VaultData::Header(new_depth, header, vec![]),
                        new_depth - 1,
                        0,
                    );
                    self.parse_file_aux(
                        input,
                        file_entry,
                        file_tags,
                        new_depth,
                        comment_depth,
                        false,
                    );
                }
                Ok(FileToken::Description(description, indent_length)) => {
                    if Self::append_description(
                        file_entry,
                        description.clone(),
                        header_depth,
                        indent_length / self.config.core.indent_length,
                    )
                    .is_err()
                    {
                        error!("Failed to insert description {description}");
                    }
                    self.parse_file_aux(
                        input,
                        file_entry,
                        file_tags,
                        header_depth,
                        comment_depth,
                        false,
                    );
                }
                Ok(FileToken::TrackerDefinition(tracker_def)) => {
                    debug!("Parsed a Tracker Definition");

                    // Skip empty lines
                    while input.peek().is_some_and(|(_, l)| l.is_empty()) {
                        input.next();
                    }

                    if let Some((_next_line_number, mut next_line)) = input.peek().copied() {
                        // Parse header line
                        if let Ok(mut tracker) = parse_header(
                            &tracker_def,
                            self.filename.clone(),
                            line_number,
                            &mut next_line,
                        ) {
                            input.next();
                            // Parse separator |---|---|
                            if input.peek().copied().is_some_and(|(_, mut next_line)| {
                                let _ = parse_separator(&mut next_line); // useful when we want vault-tasks to create our table
                                true
                            }) {
                                input.next();

                                while input.peek().is_some() {
                                    let (_, mut next_line) = input.peek().copied().unwrap();
                                    if let Ok((date, entries)) =
                                        parse_entries(&tracker, self.config, &mut next_line)
                                    {
                                        debug!("inserting {entries:?}");
                                        tracker.add_event(&date, &entries);
                                    } else {
                                        error!("Failed to parse tracker entry (could be finished)");
                                        break;
                                    }
                                    input.next();
                                }
                                let fixed_tracker = tracker.add_blanks(self.config);
                                if Self::insert_tracker_at(file_entry, fixed_tracker, header_depth)
                                    .is_ok()
                                {
                                    info!("Successfully inserted Tracker");
                                } else {
                                    error!("Failed to insert tracker");
                                }
                            } else {
                                error!("Failed to parse tracker separator");
                            }
                        } else {
                            error!("Failed to parse tracker header");
                        }
                    } else {
                        error!("No line after tracker definition");
                    }

                    self.parse_file_aux(
                        input,
                        file_entry,
                        file_tags,
                        header_depth,
                        comment_depth,
                        false,
                    );
                }
                Ok(FileToken::FileTag(tag)) => {
                    if !file_tags.contains(&tag) {
                        file_tags.push(tag);
                    }
                    self.parse_file_aux(
                        input,
                        file_entry,
                        file_tags,
                        header_depth,
                        comment_depth,
                        false,
                    );
                }
                Ok(FileToken::StartOfComment) => {
                    self.parse_file_aux(input, file_entry, file_tags, header_depth, 1, false);
                    // We started from 0 here
                }
                Ok(FileToken::EndOfComment) => {
                    debug!("A EndOfComment was parsed but no comment was open");
                    self.parse_file_aux(input, file_entry, file_tags, header_depth, 0, false);
                    // we're still at 0
                }
                Ok(FileToken::StartOfCodeBlock | FileToken::EndOfCodeBlock) => {
                    self.parse_file_aux(input, file_entry, file_tags, header_depth, 0, true);
                }
                Err(_) | Ok(FileToken::FullComment) => {
                    self.parse_file_aux(
                        input,
                        file_entry,
                        file_tags,
                        header_depth,
                        comment_depth,
                        false,
                    );
                }
            }
        }
    }

    /// Removes any empty headers from a `FileEntry`
    fn clean_file_entry(&self, file_entry: &mut VaultData) -> Option<VaultData> {
        match file_entry {
            VaultData::Header(_, name, children) | VaultData::Directory(name, children) => {
                let mut actual_children = vec![];
                for child in children.iter_mut() {
                    let mut child_clone = child.clone();
                    if self.clean_file_entry(&mut child_clone).is_some() {
                        actual_children.push(child_clone);
                    }
                }
                *children = actual_children;
                // If the `config.tasks_drop_file` happens to be empty, don't drop it
                if children.is_empty() && name != &self.config.core.tasks_drop_file {
                    return None;
                }
            }
            VaultData::Task(_) | VaultData::Tracker(_) => (),
        }
        Some(file_entry.to_owned())
    }

    pub fn parse_file(&mut self, filename: &str, input: &&str) -> Option<VaultData> {
        let lines = input.split('\n');

        let mut res = VaultData::Header(0, filename.to_owned(), vec![]);
        let mut file_tags = vec![];
        self.filename = filename.to_string();
        self.parse_file_aux(
            lines.enumerate().peekable(),
            &mut res,
            &mut file_tags,
            0,
            0,
            false,
        );

        if self.config.core.file_tags_propagation {
            file_tags.iter().for_each(|t| add_global_tag(&mut res, t));
        }

        self.clean_file_entry(&mut res)
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
            VaultData::Tracker(tracker) => {
                error!("Tried to add a tag to a tracker: {}", tracker.name);
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
        TasksConfig, parser::parser_file_entry::add_global_tag, task::Task, vault_data::VaultData,
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
                                line_number: Some(8),
                                description: Some("test\ndesc".to_string()),
                                ..Default::default()
                            })],
                        ),
                    ],
                ),
            ],
        );
        parser.parse_file_aux(input, &mut res, &mut vec![], 0, 0, false);
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
                        line_number: Some(8),
                        description: Some("test\ndesc".to_string()),
                        ..Default::default()
                    })],
                )],
            )],
        );
        parser.clean_file_entry(&mut res);
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
                        line_number: Some(2),
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
                                    line_number: Some(6),
                                    ..Default::default()
                                }),
                                VaultData::Task(Task {
                                    name: "Task 2".to_string(),
                                    line_number: Some(7),
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
                            line_number: Some(9),
                            description: Some("Description".to_string()),
                            ..Default::default()
                        })],
                    ),
                ],
            )],
        );
        parser.parse_file_aux(input, &mut res, &mut vec![], 0, 0, false);
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
            ..Default::default()
        };
        let mut res = VaultData::Header(0, "Test".to_string(), vec![]);
        let parser = ParserFileEntry {
            config: &config,
            filename: String::new(),
        };
        parser.parse_file_aux(input, &mut res, &mut vec![], 0, 0, false);
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
                        line_number: Some(3),
                        ..Default::default()
                    }),
                    VaultData::Header(2, "2 Header".to_string(), vec![]),
                ],
            )],
        );
        parser.parse_file_aux(input, &mut res, &mut vec![], 0, 0, false);
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
                        line_number: Some(3),
                        subtasks: vec![Task {
                            name: "Test b".to_string(),
                            line_number: Some(4),
                            subtasks: vec![Task {
                                name: "Test c".to_string(),
                                line_number: Some(5),
                                ..Default::default()
                            }],
                            ..Default::default()
                        }],
                        ..Default::default()
                    })],
                )],
            )],
        );
        parser.parse_file_aux(input, &mut res, &mut vec![], 0, 0, false);
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
            ..Default::default()
        };
        let mut res = VaultData::Header(0, "Test".to_string(), vec![]);
        let parser = ParserFileEntry {
            config: &config,
            filename: String::new(),
        };
        parser.parse_file_aux(input, &mut res, &mut vec![], 0, 0, false);
        assert_snapshot!(res);
    }
    #[test]
    fn test_commented_task() {
        let input = r"# 1 Header
<!-- one line comment to be sure -->
<!--
- [ ] This task is commented out
-->

- [ ] This one is not
"
        .split('\n')
        .enumerate()
        .peekable();

        let config = TasksConfig {
            ..Default::default()
        };
        let mut res = VaultData::Header(0, "Test".to_string(), vec![]);
        let parser = ParserFileEntry {
            config: &config,
            filename: String::new(),
        };
        parser.parse_file_aux(input, &mut res, &mut vec![], 0, 0, false);
        assert_snapshot!(res);
    }
    #[test]
    fn test_code_block_task() {
        let input = r"# 1 Header
```
- [ ] This task is in a code block
```
- [ ] This one is not
"
        .split('\n')
        .enumerate()
        .peekable();

        let config = TasksConfig {
            ..Default::default()
        };
        let mut res = VaultData::Header(0, "Test".to_string(), vec![]);
        let parser = ParserFileEntry {
            config: &config,
            filename: String::new(),
        };
        parser.parse_file_aux(input, &mut res, &mut vec![], 0, 0, false);
        assert_snapshot!(res);
    }
}
