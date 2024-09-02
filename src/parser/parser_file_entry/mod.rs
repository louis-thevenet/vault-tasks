use std::iter::Peekable;

use winnow::{
    ascii::space0,
    combinator::{alt, preceded, repeat},
    token::take_till,
    PResult, Parser,
};

use crate::{config::Config, core::FileEntry, task::Task};

use super::parser_task::parse_task;

enum FileToken {
    Task(Task),
    Header((String, usize)), // Header / Depth
}
pub struct ParserFileEntry<'a> {
    pub config: &'a Config,
}

impl<'i> ParserFileEntry<'i> {
    fn parse_task(&self, input: &mut &str) -> PResult<FileToken> {
        let mut task_parser = |input: &mut &str| parse_task(input, self.config);
        let task_res = task_parser.parse_next(input)?;
        Ok(FileToken::Task(task_res))
    }
    fn parse_header(&self, input: &mut &str) -> PResult<FileToken> {
        let header_depth: String = repeat(1.., "#").parse_next(input)?;
        let header_content = preceded(space0, take_till(1.., |c| c == '\n')).parse_next(input)?;

        Ok(FileToken::Header((
            header_content.to_string(),
            header_depth.len(),
        )))
    }
    /// Inserts a `FileEntry` at the specific `depth` in `file_entry`.
    fn insert_at(
        file_entry: &mut FileEntry,
        object: FileEntry,
        current_depth: usize,
        target_depth: usize,
    ) {
        match file_entry {
            FileEntry::Tasks(_) => todo!(),
            FileEntry::Header(_, header_children) => match current_depth.cmp(&target_depth) {
                std::cmp::Ordering::Greater => todo!(),
                std::cmp::Ordering::Equal => header_children.push(object),
                std::cmp::Ordering::Less => {
                    for child in header_children.iter_mut().rev() {
                        if let FileEntry::Header(_, _) = child {
                            Self::insert_at(child, object, current_depth + 1, target_depth);
                            return;
                        }
                    }
                    header_children.push(object);
                }
            },
        }
    }

    /// Recursively parses the input file passed as a string.
    fn parse_file_aux<'a, I>(
        &self,
        mut input: Peekable<I>,
        file_entry: &mut FileEntry,
        depth: usize,
    ) where
        I: Iterator<Item = &'a str>,
    {
        let mut parser = alt((
            |input: &mut &str| self.parse_header(input),
            |input: &mut &str| self.parse_task(input),
        ));

        let line_opt = input.next();
        if line_opt.is_none() {
            return;
        }

        let mut line = line_opt.unwrap();

        match parser.parse_next(&mut line) {
            Ok(FileToken::Task(mut task)) => {
                // we look for a description
                let mut description = String::new();
                while let Some(desc_line) = input.peek() {
                    if desc_line.starts_with(' ') {
                        description.push('\n');
                        description.push_str(&desc_line[self.config.indent_length.unwrap_or(2)..]);
                        input.next();
                    } else {
                        break;
                    }
                }
                task.description = Some(description);
                Self::insert_at(file_entry, FileEntry::Tasks(task), 1, depth);
                self.parse_file_aux(input, file_entry, depth)
            }
            Ok(FileToken::Header((header, target_depth))) => {
                Self::insert_at(
                    file_entry,
                    FileEntry::Header(header, vec![]),
                    1,
                    target_depth,
                );
                self.parse_file_aux(input, file_entry, target_depth)
            }
            Err(_) => self.parse_file_aux(input, file_entry, depth),
        }
    }
    pub fn parse_file(&self, filename: String, input: &mut &str) -> Option<FileEntry> {
        let lines = input.split('\n');
        let mut res = FileEntry::Header(filename, vec![]);
        self.parse_file_aux(lines.peekable(), &mut res, 1);
        println!("Header found: {:#?}", res);
        Some(res)
    }
}
