use std::{iter::Peekable, path::PathBuf};

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
    vault_data::NewFileEntry,
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
    pub path: PathBuf,
}

impl ParserFileEntry<'_> {
    fn parse_indent(input: &mut &str) -> Result<usize> {
        let indent_length: String = repeat(1.., " ").parse_next(input)?;
        Ok(indent_length.len())
    }
    fn parse_task(&self, input: &mut &str) -> Result<FileToken> {
        let indent_length = Self::parse_indent(input).unwrap_or(0);

        let mut task_parser = |input: &mut &str| parse_task(input, &self.path, self.config);
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
        file_entry: &mut NewFileEntry,
        task: Task,
        header_depth: usize,
        indent_length: usize,
    ) -> color_eyre::Result<()> {
        fn append_task_aux(
            file_entry: &mut NewFileEntry,
            task_to_insert: Task,
            current_header_depth: usize,
            target_header_depth: usize,
            current_task_depth: usize,
            target_task_depth: usize,
        ) -> color_eyre::Result<()> {
            match file_entry {
                NewFileEntry::Header {
                    content: header_children,
                    ..
                } => {
                    match current_header_depth.cmp(&target_header_depth) {
                        std::cmp::Ordering::Greater => panic!(
                            "Target header level was greater than current level which is impossible"
                        ), // shouldn't happen
                        std::cmp::Ordering::Equal => {
                            // Found correct header level
                            if current_task_depth == target_task_depth {
                                header_children.push(NewFileEntry::Task(task_to_insert));
                                Ok(())
                            } else {
                                for child in header_children.iter_mut().rev() {
                                    if let NewFileEntry::Task(_task) = child {
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
                                if let NewFileEntry::Header { .. } = child {
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
                NewFileEntry::Task(task) => {
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
                NewFileEntry::Tracker(_tracker) => {
                    bail!("Failed to insert task: tried to insert into a tracker")
                }
            }
        }
        append_task_aux(file_entry, task, 0, header_depth, 0, indent_length)
    }
    fn insert_tracker_at(
        file_entry: &mut NewFileEntry,
        tracker: Tracker,
        header_depth: usize,
    ) -> color_eyre::Result<()> {
        fn append_tracker_aux(
            file_entry: &mut NewFileEntry,
            tracker_to_insert: Tracker,
            current_header_depth: usize,
            target_header_depth: usize,
        ) -> color_eyre::Result<()> {
            match file_entry {
                NewFileEntry::Header {
                    content: header_children,
                    ..
                } => {
                    match current_header_depth.cmp(&target_header_depth) {
                        std::cmp::Ordering::Greater => {
                            bail!(
                                "Target header level was greater than current level which is impossible"
                            )
                        }
                        std::cmp::Ordering::Equal => {
                            // Found correct header level
                            header_children.push(NewFileEntry::Tracker(tracker_to_insert));
                            Ok(())
                        }
                        std::cmp::Ordering::Less => {
                            // Going deeper in header levels
                            for child in header_children.iter_mut().rev() {
                                if let NewFileEntry::Header { .. } = child {
                                    return append_tracker_aux(
                                        child,
                                        tracker_to_insert,
                                        current_header_depth + 1,
                                        target_header_depth,
                                    );
                                }
                            }
                            bail!(
                                "Couldn't find correct parent header to insert tracker {}",
                                tracker_to_insert.name
                            )
                        }
                    }
                }
                NewFileEntry::Task(_task) => {
                    bail!("Tried to insert a Tracker in a task")
                }
                NewFileEntry::Tracker(_tracker) => {
                    bail!("Tried to insert a Tracker in a tracker")
                }
            }
        }
        append_tracker_aux(file_entry, tracker, 0, header_depth)
    }
    /// Inserts a header at the specific depth in `file_entry`.
    fn insert_header_at(
        file_entry: &mut NewFileEntry,
        name: String,
        heading_level: usize,
        target_header_depth: usize,
        target_task_depth: usize,
    ) -> color_eyre::Result<()> {
        fn insert_at_aux(
            file_entry: &mut NewFileEntry,
            name: String,
            heading_level: usize,
            current_header_depth: usize,
            target_header_depth: usize,
            current_task_depth: usize,
            target_task_depth: usize,
        ) -> color_eyre::Result<()> {
            match file_entry {
                NewFileEntry::Header {
                    content: header_children,
                    ..
                } => {
                    match current_header_depth.cmp(&target_header_depth) {
                        std::cmp::Ordering::Greater => {
                            bail!(
                                "Target header level was greater than current level which is impossible"
                            )
                        }
                        std::cmp::Ordering::Equal => {
                            // Found correct header level
                            if current_task_depth == target_task_depth {
                                header_children.push(NewFileEntry::Header {
                                    name,
                                    heading_level,
                                    content: vec![],
                                });
                                Ok(())
                            } else {
                                for child in header_children.iter_mut().rev() {
                                    if let NewFileEntry::Task(_) = child {
                                        return insert_at_aux(
                                            child,
                                            name,
                                            heading_level,
                                            current_header_depth,
                                            target_header_depth,
                                            current_task_depth + 1,
                                            target_task_depth,
                                        );
                                    }
                                }
                                bail!("Couldn't find correct parent task to insert header")
                            }
                        }
                        std::cmp::Ordering::Less => {
                            // Still haven't found correct header level, going deeper
                            for child in header_children.iter_mut().rev() {
                                if let NewFileEntry::Header { .. } = child {
                                    return insert_at_aux(
                                        child,
                                        name,
                                        heading_level,
                                        current_header_depth + 1,
                                        target_header_depth,
                                        current_task_depth,
                                        target_task_depth,
                                    );
                                }
                            }
                            // No child header found, append to current level
                            header_children.push(NewFileEntry::Header {
                                name,
                                heading_level,
                                content: vec![],
                            });
                            Ok(())
                        }
                    }
                }
                NewFileEntry::Task(task) => {
                    let mut current_task_depth = current_task_depth;
                    let mut last_task = task;
                    while current_task_depth < target_task_depth {
                        if last_task.subtasks.is_empty() {
                            bail!(
                                "Could not find parent task to insert header, indenting may be wrong"
                            )
                        }
                        last_task = last_task.subtasks.last_mut().unwrap();
                        current_task_depth += 1;
                    }
                    bail!("Error: tried to insert a header into a task")
                }
                NewFileEntry::Tracker(_tracker) => {
                    bail!("Error: tried to insert a header into a tracker")
                }
            }
        }
        insert_at_aux(
            file_entry,
            name,
            heading_level,
            0,
            target_header_depth,
            0,
            target_task_depth,
        )
    }

    /// Appends `desc` to the description of an existing `Task` in the `FileEntry`.
    fn append_description(
        file_entry: &mut NewFileEntry,
        desc: String,
        target_header_depth: usize,
        target_task_depth: usize,
    ) -> color_eyre::Result<()> {
        fn append_description_aux(
            file_entry: &mut NewFileEntry,
            desc: String,
            current_header_depth: usize,
            target_header_depth: usize,
            current_task_depth: usize,
            target_task_depth: usize,
        ) -> color_eyre::Result<()> {
            match file_entry {
                NewFileEntry::Header {
                    content: header_children,
                    ..
                } => {
                    match current_header_depth.cmp(&target_header_depth) {
                        std::cmp::Ordering::Greater => {
                            bail!(
                                "Target header level was greater than current level which is impossible"
                            )
                        }
                        std::cmp::Ordering::Equal => {
                            // Found correct header level
                            for child in header_children.iter_mut().rev() {
                                if let NewFileEntry::Task(task) = child {
                                    if current_task_depth == target_task_depth {
                                        match &mut task.description {
                                            Some(d) => {
                                                d.push('\n');
                                                d.push_str(&desc);
                                            }
                                            None => task.description = Some(desc.clone()),
                                        }
                                        return Ok(());
                                    }
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
                            Ok(())
                        }
                        std::cmp::Ordering::Less => {
                            // Going deeper in header levels
                            for child in header_children.iter_mut().rev() {
                                if let NewFileEntry::Header { .. } = child {
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
                NewFileEntry::Task(task) => {
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
                        } else if let Some(subtask) = task.subtasks.last_mut() {
                            insert_desc_task(description, subtask, current_level + 1, target_level)
                        } else {
                            debug!(
                                "Description was too indented, adding to closest task: {description}"
                            );
                            // Add to current task if no subtask exists
                            if let Some(d) = &mut task.description {
                                d.push('\n');
                                d.push_str(&description);
                                Ok(())
                            } else {
                                task.description = Some(description);
                                Ok(())
                            }
                        }
                    }
                    insert_desc_task(desc, task, current_task_depth, target_task_depth)
                }
                NewFileEntry::Tracker(_tracker) => {
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
        file_entry: &mut Option<NewFileEntry>,
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
                    if file_entry.is_none() {
                        *file_entry = Some(NewFileEntry::Header {
                            name: self
                                .path
                                .file_name()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .to_string(),
                            heading_level: 0,
                            content: vec![],
                        });
                    }
                    if Self::insert_task_at(
                        file_entry.as_mut().unwrap(), // we ensured it's Some above
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
                    if file_entry.is_none() {
                        *file_entry = Some(NewFileEntry::Header {
                            name: self
                                .path
                                .file_name()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .to_string(),
                            heading_level: 0,
                            content: vec![],
                        });
                    }
                    if Self::insert_header_at(
                        file_entry.as_mut().unwrap(), // we ensured it's Some above
                        header.clone(),
                        new_depth,
                        new_depth - 1,
                        0,
                    )
                    .is_err()
                    {
                        error!("Failed to insert header {}", header);
                    }
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
                    if let Some(entry) = file_entry.as_mut()
                        && Self::append_description(
                            entry,
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
                        if let Ok(mut tracker) =
                            parse_header(&tracker_def, &self.path, line_number, &mut next_line)
                        {
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
                                if let Some(entry) = file_entry.as_mut() {
                                    if Self::insert_tracker_at(entry, fixed_tracker, header_depth)
                                        .is_ok()
                                    {
                                        info!("Successfully inserted Tracker");
                                    } else {
                                        error!("Failed to insert tracker");
                                    }
                                } else {
                                    error!("Failed to insert tracker: no file entry");
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
    fn clean_file_entry(&self, file_entry: &mut NewFileEntry) -> Option<NewFileEntry> {
        match file_entry {
            NewFileEntry::Header {
                name,
                heading_level: _,
                content,
            } => {
                let mut actual_content = vec![];
                for child in content.iter_mut() {
                    let mut child_clone = child.clone();
                    if self.clean_file_entry(&mut child_clone).is_some() {
                        actual_content.push(child_clone);
                    }
                }
                *content = actual_content;
                // If the `config.tasks_drop_file` happens to be empty, don't drop it
                if content.is_empty() && name != &self.config.core.tasks_drop_file {
                    return None;
                }
            }
            NewFileEntry::Task(_task) => (),
            NewFileEntry::Tracker(_tracker) => (),
        }
        Some(file_entry.to_owned())
    }

    pub fn parse_file(&mut self, input: &&str) -> Option<NewFileEntry> {
        let replaced = input.replace('\r', "");
        let lines = replaced.split('\n');

        let mut res = None;
        let mut file_tags = vec![];
        self.parse_file_aux(
            lines.enumerate().peekable(),
            &mut res,
            &mut file_tags,
            0,
            0,
            false,
        );

        if let Some(mut entry) = res {
            if self.config.core.file_tags_propagation {
                for t in &file_tags {
                    add_global_tag(&mut entry, t);
                }
            }
            self.clean_file_entry(&mut entry)
        } else {
            None
        }
    }
}

fn add_global_tag(file_entry: &mut NewFileEntry, tag: &String) {
    fn add_tag_aux(file_entry: &mut NewFileEntry, tag: &String) {
        match file_entry {
            NewFileEntry::Header {
                content: children, ..
            } => {
                for child in children.iter_mut().rev() {
                    add_tag_aux(child, tag);
                }
            }
            NewFileEntry::Task(task) => {
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
            NewFileEntry::Tracker(tracker) => {
                error!("Tried to add a tag to a tracker: {}", tracker.name);
            }
        }
    }
    add_tag_aux(file_entry, tag);
}
#[cfg(test)]
mod tests {

    use std::path::PathBuf;

    use insta::assert_snapshot;

    use super::ParserFileEntry;

    use crate::{TasksConfig, task::Task, vault_data::NewFileEntry};
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
        let mut res = Some(NewFileEntry::Header {
            name: "Test".to_string(),
            heading_level: 0,
            content: vec![],
        });
        let parser = ParserFileEntry {
            config: &config,
            path: PathBuf::new(),
        };
        let expected = Some(NewFileEntry::Header {
            name: "Test".to_string(),
            heading_level: 0,
            content: vec![
                NewFileEntry::Header {
                    name: "1 useless".to_string(),
                    heading_level: 1,
                    content: vec![NewFileEntry::Header {
                        name: "2 useless".to_string(),
                        heading_level: 2,
                        content: vec![NewFileEntry::Header {
                            name: "3 useless".to_string(),
                            heading_level: 3,
                            content: vec![],
                        }],
                    }],
                },
                NewFileEntry::Header {
                    name: "2 useful".to_string(),
                    heading_level: 1,
                    content: vec![
                        NewFileEntry::Header {
                            name: "3 useless".to_string(),
                            heading_level: 3,
                            content: vec![],
                        },
                        NewFileEntry::Header {
                            name: "4 useful".to_string(),
                            heading_level: 2,
                            content: vec![NewFileEntry::Task(Task {
                                name: "test".to_string(),
                                line_number: Some(8),
                                description: Some("test\ndesc".to_string()),
                                ..Default::default()
                            })],
                        },
                    ],
                },
            ],
        });
        parser.parse_file_aux(input, &mut res, &mut vec![], 0, 0, false);
        assert_eq!(res, expected);

        let expected_after_cleaning = Some(NewFileEntry::Header {
            name: "Test".to_string(),
            heading_level: 0,
            content: vec![NewFileEntry::Header {
                name: "2 useful".to_string(),
                heading_level: 1,
                content: vec![NewFileEntry::Header {
                    name: "4 useful".to_string(),
                    heading_level: 2,
                    content: vec![NewFileEntry::Task(Task {
                        name: "test".to_string(),
                        line_number: Some(8),
                        description: Some("test\ndesc".to_string()),
                        ..Default::default()
                    })],
                }],
            }],
        });
        if let Some(mut entry) = res {
            parser.clean_file_entry(&mut entry);
            assert_eq!(Some(entry), expected_after_cleaning);
        } else {
            panic!("Expected Some(NewFileEntry), got None");
        }
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
        let mut res = Some(NewFileEntry::Header {
            name: "Test".to_string(),
            heading_level: 0,
            content: vec![],
        });
        let parser = ParserFileEntry {
            config: &config,
            path: PathBuf::new(),
        };
        let expected = Some(NewFileEntry::Header {
            name: "Test".to_string(),
            heading_level: 0,
            content: vec![NewFileEntry::Header {
                name: "1 Header".to_string(),
                heading_level: 1,
                content: vec![
                    NewFileEntry::Task(Task {
                        name: "Task".to_string(),
                        line_number: Some(2),
                        ..Default::default()
                    }),
                    NewFileEntry::Header {
                        name: "2 Header".to_string(),
                        heading_level: 2,
                        content: vec![NewFileEntry::Header {
                            name: "3 Header".to_string(),
                            heading_level: 3,
                            content: vec![
                                NewFileEntry::Task(Task {
                                    name: "Task".to_string(),
                                    line_number: Some(6),
                                    ..Default::default()
                                }),
                                NewFileEntry::Task(Task {
                                    name: "Task 2".to_string(),
                                    line_number: Some(7),
                                    ..Default::default()
                                }),
                            ],
                        }],
                    },
                    NewFileEntry::Header {
                        name: "2 Header 2".to_string(),
                        heading_level: 2,
                        content: vec![NewFileEntry::Task(Task {
                            name: "Task".to_string(),
                            line_number: Some(9),
                            description: Some("Description".to_string()),
                            ..Default::default()
                        })],
                    },
                ],
            }],
        });
        parser.parse_file_aux(input, &mut res, &mut vec![], 0, 0, false);
        assert_eq!(res, expected);
    }
    // TODO: Refactor this test after add_global_tag is refactored to use NewFileEntry
    // #[test]
    // fn test_insert_global_tag() {
    //     let input = r"# 1 Header
    // - [ ] Task

    // ## 2 Header
    // ### 3 Header
    // - [ ] Task
    // - [ ] Task 2
    // ## 2 Header 2
    // - [ ] Task
    //   Description

    // "
    //     .split('\n')
    //     .enumerate()
    //     .peekable();

    //     let config = TasksConfig {
    //         ..Default::default()
    //     };
    //     let mut res = VaultData::Header(0, "Test".to_string(), vec![]);
    //     let parser = ParserFileEntry {
    //         config: &config,
    //         path: PathBuf::new(),
    //     };
    //     parser.parse_file_aux(input, &mut res, &mut vec![], 0, 0, false);
    //     add_global_tag(&mut res, &String::from("test"));
    //     assert_snapshot!(res);
    // }
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
        let mut res = Some(NewFileEntry::Header {
            name: "Test".to_string(),
            heading_level: 0,
            content: vec![],
        });
        let parser = ParserFileEntry {
            config: &config,
            path: PathBuf::new(),
        };
        let expected = Some(NewFileEntry::Header {
            name: "Test".to_string(),
            heading_level: 0,
            content: vec![NewFileEntry::Header {
                name: "1 Header".to_string(),
                heading_level: 1,
                content: vec![
                    NewFileEntry::Task(Task {
                        name: "Task".to_string(),
                        line_number: Some(3),
                        ..Default::default()
                    }),
                    NewFileEntry::Header {
                        name: "2 Header".to_string(),
                        heading_level: 2,
                        content: vec![],
                    },
                ],
            }],
        });
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
        let mut res = Some(NewFileEntry::Header {
            name: "Test".to_string(),
            heading_level: 0,
            content: vec![],
        });
        let parser = ParserFileEntry {
            config: &config,
            path: PathBuf::new(),
        };
        let expected = Some(NewFileEntry::Header {
            name: "Test".to_string(),
            heading_level: 0,
            content: vec![NewFileEntry::Header {
                name: "1 Header".to_string(),
                heading_level: 1,
                content: vec![NewFileEntry::Header {
                    name: "Test".to_string(),
                    heading_level: 2,
                    content: vec![NewFileEntry::Task(Task {
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
                }],
            }],
        });
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
        let mut res = Some(NewFileEntry::Header {
            name: "Test".to_string(),
            heading_level: 0,
            content: vec![],
        });
        let parser = ParserFileEntry {
            config: &config,
            path: PathBuf::new(),
        };
        parser.parse_file_aux(input, &mut res, &mut vec![], 0, 0, false);
        assert_snapshot!(res.unwrap());
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
        let mut res = Some(NewFileEntry::Header {
            name: "Test".to_string(),
            heading_level: 0,
            content: vec![],
        });
        let parser = ParserFileEntry {
            config: &config,
            path: PathBuf::new(),
        };
        parser.parse_file_aux(input, &mut res, &mut vec![], 0, 0, false);
        assert_snapshot!(res.unwrap());
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
        let mut res = Some(NewFileEntry::Header {
            name: "Test".to_string(),
            heading_level: 0,
            content: vec![],
        });
        let parser = ParserFileEntry {
            config: &config,
            path: PathBuf::new(),
        };
        parser.parse_file_aux(input, &mut res, &mut vec![], 0, 0, false);
        assert_snapshot!(res.unwrap());
    }
}
