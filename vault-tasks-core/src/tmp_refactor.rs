/// Temporary module for ``VaultData`` refactoring
use std::path::{Path, PathBuf};

use crate::vault_data::{NewFileEntry, NewNode, NewVaultData, VaultData};

impl VaultData {
    /// Converts legacy `VaultData` to the new `NewFileEntry` type
    #[must_use]
    pub fn to_new_file_entry(&self) -> Option<NewFileEntry> {
        match self {
            VaultData::Header(level, name, content) => {
                let new_content = content
                    .iter()
                    .filter_map(VaultData::to_new_file_entry)
                    .collect();
                Some(NewFileEntry::Header {
                    name: name.clone(),
                    heading_level: *level,
                    content: new_content,
                })
            }
            VaultData::Task(task) => Some(NewFileEntry::Task(task.clone())),
            VaultData::Tracker(tracker) => Some(NewFileEntry::Tracker(tracker.clone())),
            VaultData::Directory(_, _) => None, // Directories don't belong in file entries
        }
    }

    /// Converts legacy `VaultData` to the new `NewNode` type
    ///
    /// # Arguments
    /// * `base_path` - The base path to use for generating paths (since legacy type doesn't store them)
    ///
    /// # Note
    /// In the legacy system, files are represented as `Header(0, filename, content)` where
    /// the heading level is 0 and the name is the filename.
    ///
    /// If non-file headers or tasks/trackers are encountered at the directory level
    /// (which can happen in test code or malformed vault structures), they are automatically
    /// wrapped in a file node.
    #[must_use]
    pub fn to_new_node(&self, base_path: &Path) -> NewNode {
        match self {
            VaultData::Directory(name, content) => {
                let dir_path = base_path.join(name);
                let new_content = content
                    .iter()
                    .map(|item| item.to_new_node(&dir_path))
                    .collect();
                NewNode::Directory {
                    name: name.clone(),
                    path: dir_path,
                    content: new_content,
                }
            }
            VaultData::Header(0, filename, content) => {
                // Level 0 headers represent files in the legacy system
                let file_path = base_path.join(filename);
                let file_content = content
                    .iter()
                    .filter_map(VaultData::to_new_file_entry)
                    .collect();
                NewNode::File {
                    name: filename.clone(),
                    path: file_path,
                    content: file_content,
                }
            }
            VaultData::Header(level, name, content) => {
                // Non-file headers at directory level -> wrap in a file
                // This can happen in test code or malformed vault structures
                let file_name = format!("{}.md", name.to_lowercase().replace(' ', "_"));
                let file_path = base_path.join(&file_name);
                let header_entry = NewFileEntry::Header {
                    name: name.clone(),
                    heading_level: *level,
                    content: content
                        .iter()
                        .filter_map(VaultData::to_new_file_entry)
                        .collect(),
                };
                NewNode::File {
                    name: file_name,
                    path: file_path,
                    content: vec![header_entry],
                }
            }
            VaultData::Task(_) | VaultData::Tracker(_) => {
                // Tasks/Trackers at directory level -> wrap in a file
                // This can happen in test code or malformed vault structures
                let file_name = "untitled.md".to_string();
                let file_path = base_path.join(&file_name);
                let content = if let Some(entry) = self.to_new_file_entry() {
                    vec![entry]
                } else {
                    vec![]
                };
                NewNode::File {
                    name: file_name,
                    path: file_path,
                    content,
                }
            }
        }
    }
}

/// Helper function to convert a Vec<VaultData> to `NewVaultData`
/// This assumes each top-level `VaultData` represents a vault or directory
#[must_use]
pub fn convert_legacy_to_new(legacy_data: Vec<VaultData>) -> NewVaultData {
    let root = legacy_data
        .into_iter()
        .map(|item| item.to_new_node(&PathBuf::new()))
        .collect::<Vec<NewNode>>();
    // convert all first directory to vaults
    let root = root
        .iter()
        .map(|node| {
            let NewNode::Directory {
                name,
                path,
                content,
            } = node
            else {
                return node.clone();
            };
            NewNode::Vault {
                name: name.clone(),
                path: path.clone(),
                content: content.clone(),
            }
        })
        .collect();

    NewVaultData::new(root)
}

impl NewFileEntry {
    /// Converts new `NewFileEntry` back to legacy `VaultData` type
    #[must_use]
    pub fn to_legacy(&self) -> VaultData {
        match self {
            NewFileEntry::Header {
                name,
                heading_level,
                content,
            } => {
                let legacy_content = content.iter().map(NewFileEntry::to_legacy).collect();
                VaultData::Header(*heading_level, name.clone(), legacy_content)
            }
            NewFileEntry::Task(task) => VaultData::Task(task.clone()),
            NewFileEntry::Tracker(tracker) => VaultData::Tracker(tracker.clone()),
        }
    }
}

impl NewNode {
    /// Converts new `NewNode` back to legacy `VaultData` type
    #[must_use]
    pub fn to_legacy(&self) -> VaultData {
        match self {
            NewNode::Vault { name, content, .. } | NewNode::Directory { name, content, .. } => {
                let legacy_content = content.iter().map(NewNode::to_legacy).collect();
                VaultData::Directory(name.clone(), legacy_content)
            }
            NewNode::File { name, content, .. } => {
                // Files are represented as Header(0, filename, content) in legacy
                let legacy_content = content.iter().map(NewFileEntry::to_legacy).collect();
                VaultData::Header(0, name.clone(), legacy_content)
            }
        }
    }
}

impl NewVaultData {
    /// Converts new `NewVaultData` back to legacy `Vec<VaultData>` type
    #[must_use]
    pub fn to_legacy(&self) -> Vec<VaultData> {
        self.root.iter().map(NewNode::to_legacy).collect()
    }
}

/// Helper function to convert `NewVaultData` to legacy `Vec<VaultData>`
#[must_use]
pub fn convert_new_to_legacy(new_data: &NewVaultData) -> Vec<VaultData> {
    new_data.to_legacy()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::date::Date;
    use crate::task::{State, Task};
    use crate::tracker::Tracker;
    use crate::tracker::frequency::Frequency;

    fn create_test_task() -> Task {
        Task {
            state: State::ToDo,
            description: Some("Test task".to_string()),
            subtasks: vec![],
            tags: None,
            due_date: None,
            priority: 0,
            path: PathBuf::from("/vault/test.md"),
            line_number: Some(1),
            name: "Test".to_string(),
            completion: None,
            is_today: false,
        }
    }

    fn create_test_tracker() -> Tracker {
        use chrono::NaiveDate;

        Tracker {
            name: "Test Tracker".to_string(),
            frequency: Frequency::Days(1),
            start_date: Date::Day(NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()),
            categories: vec![],
            length: 0,
            path: PathBuf::from("test.md"),
            line_number: 1,
        }
    }

    #[test]
    fn test_task_conversion_to_file_entry() {
        let task = create_test_task();
        let legacy = VaultData::Task(task.clone());

        let result = legacy.to_new_file_entry();
        assert!(result.is_some());

        match result.unwrap() {
            NewFileEntry::Task(converted_task) => {
                assert_eq!(converted_task.name, task.name);
                assert_eq!(converted_task.state, task.state);
            }
            _ => panic!("Expected Task variant"),
        }
    }

    #[test]
    fn test_tracker_conversion_to_file_entry() {
        let tracker = create_test_tracker();
        let legacy = VaultData::Tracker(tracker.clone());

        let result = legacy.to_new_file_entry();
        assert!(result.is_some());

        match result.unwrap() {
            NewFileEntry::Tracker(converted_tracker) => {
                assert_eq!(converted_tracker.name, tracker.name);
            }
            _ => panic!("Expected Tracker variant"),
        }
    }

    #[test]
    fn test_header_conversion_to_file_entry() {
        let task = create_test_task();
        let legacy = VaultData::Header(1, "My Header".to_string(), vec![VaultData::Task(task)]);

        let result = legacy.to_new_file_entry();
        assert!(result.is_some());

        match result.unwrap() {
            NewFileEntry::Header {
                name,
                heading_level,
                content,
            } => {
                assert_eq!(name, "My Header");
                assert_eq!(heading_level, 1);
                assert_eq!(content.len(), 1);
                assert!(matches!(content[0], NewFileEntry::Task(_)));
            }
            _ => panic!("Expected Header variant"),
        }
    }

    #[test]
    fn test_directory_returns_none_for_file_entry() {
        let legacy = VaultData::Directory("dir".to_string(), vec![]);
        let result = legacy.to_new_file_entry();
        assert!(result.is_none());
    }

    #[test]
    fn test_directory_conversion_to_node() {
        let task = create_test_task();
        // Directories contain files (Header level 0), not tasks directly
        let file = VaultData::Header(0, "test.md".to_string(), vec![VaultData::Task(task)]);
        let legacy = VaultData::Directory("my_folder".to_string(), vec![file]);

        let base_path = PathBuf::from("/vault");
        let result = legacy.to_new_node(&base_path);

        match result {
            NewNode::Directory {
                name,
                path,
                content,
            } => {
                assert_eq!(name, "my_folder");
                assert_eq!(path, base_path.join("my_folder"));
                assert_eq!(content.len(), 1);
                // Check that the content is a file
                assert!(matches!(content[0], NewNode::File { .. }));
            }
            _ => panic!("Expected Directory variant"),
        }
    }

    #[test]
    fn test_nested_directory_conversion() {
        let task = create_test_task();
        // Inner directory contains a file
        let file = VaultData::Header(0, "test.md".to_string(), vec![VaultData::Task(task)]);
        let inner_dir = VaultData::Directory("inner".to_string(), vec![file]);
        let outer_dir = VaultData::Directory("outer".to_string(), vec![inner_dir]);

        let base_path = PathBuf::from("/vault");
        let result = outer_dir.to_new_node(&base_path);

        match result {
            NewNode::Directory {
                name,
                path,
                content,
            } => {
                assert_eq!(name, "outer");
                assert_eq!(path, base_path.join("outer"));
                assert_eq!(content.len(), 1);

                match &content[0] {
                    NewNode::Directory {
                        name,
                        path,
                        content,
                    } => {
                        assert_eq!(name, "inner");
                        assert_eq!(*path, base_path.join("outer").join("inner"));
                        assert_eq!(content.len(), 1);
                        // Check the inner directory contains a file
                        assert!(matches!(content[0], NewNode::File { .. }));
                    }
                    _ => panic!("Expected nested Directory"),
                }
            }
            _ => panic!("Expected Directory variant"),
        }
    }

    #[test]
    fn test_task_at_directory_level_wrapped() {
        // Tasks at directory level get wrapped in a file
        // This can happen in test code or malformed vault structures
        let task = create_test_task();
        let legacy = VaultData::Task(task.clone());

        let base_path = PathBuf::from("/vault");
        let result = legacy.to_new_node(&base_path);

        match result {
            NewNode::File {
                name,
                path,
                content,
            } => {
                assert_eq!(name, "untitled.md");
                assert_eq!(path, base_path.join("untitled.md"));
                assert_eq!(content.len(), 1);
                assert!(matches!(content[0], NewFileEntry::Task(_)));
            }
            _ => panic!("Expected File variant"),
        }
    }

    #[test]
    fn test_non_file_header_at_directory_level_wrapped() {
        // Non-file headers at directory level get wrapped in a file
        let task = create_test_task();
        let legacy = VaultData::Header(1, "Section Title".to_string(), vec![VaultData::Task(task)]);

        let base_path = PathBuf::from("/vault");
        let result = legacy.to_new_node(&base_path);

        match result {
            NewNode::File {
                name,
                path,
                content,
            } => {
                assert_eq!(name, "section_title.md");
                assert_eq!(path, base_path.join("section_title.md"));
                assert_eq!(content.len(), 1);
                match &content[0] {
                    NewFileEntry::Header {
                        name,
                        heading_level,
                        content,
                    } => {
                        assert_eq!(name, "Section Title");
                        assert_eq!(*heading_level, 1);
                        assert_eq!(content.len(), 1);
                    }
                    _ => panic!("Expected Header in file"),
                }
            }
            _ => panic!("Expected File variant"),
        }
    }

    #[test]
    fn test_convert_legacy_to_new() {
        let task1 = create_test_task();
        let task2 = create_test_task();

        // Directories contain files (Header level 0)
        let file1 = VaultData::Header(0, "file1.md".to_string(), vec![VaultData::Task(task1)]);
        let file2 = VaultData::Header(0, "file2.md".to_string(), vec![VaultData::Task(task2)]);

        let legacy_data = vec![
            VaultData::Directory("folder1".to_string(), vec![file1]),
            VaultData::Directory("folder2".to_string(), vec![file2]),
        ];

        let result = convert_legacy_to_new(legacy_data);

        assert_eq!(result.root.len(), 2);

        for node in &result.root {
            match node {
                NewNode::Vault { content, .. } => {
                    assert_eq!(content.len(), 1);
                    // Each directory should contain a file
                    assert!(matches!(content[0], NewNode::File { .. }));
                }
                _ => panic!("Expected Vault nodes"),
            }
        }
    }

    #[test]
    fn test_complex_nested_structure() {
        let task = create_test_task();
        let tracker = create_test_tracker();

        // Build a complex structure that matches real vault structure:
        // Directory "project"
        //   ├── Directory "tasks"
        //   │   └── File "sprint.md" (Header level 0)
        //   │       └── Header "Sprint 1" (level 1)
        //   │           └── Task
        //   └── Directory "trackers"
        //       └── File "habits.md" (Header level 0)
        //           └── Tracker

        let tasks_file = VaultData::Header(
            0,
            "sprint.md".to_string(),
            vec![VaultData::Header(
                1,
                "Sprint 1".to_string(),
                vec![VaultData::Task(task)],
            )],
        );
        let tasks_dir = VaultData::Directory("tasks".to_string(), vec![tasks_file]);

        let trackers_file = VaultData::Header(
            0,
            "habits.md".to_string(),
            vec![VaultData::Tracker(tracker)],
        );
        let trackers_dir = VaultData::Directory("trackers".to_string(), vec![trackers_file]);

        let project_dir =
            VaultData::Directory("project".to_string(), vec![tasks_dir, trackers_dir]);

        let base_path = PathBuf::from("/vault");
        let result = project_dir.to_new_node(&base_path);

        match result {
            NewNode::Directory { name, content, .. } => {
                assert_eq!(name, "project");
                assert_eq!(content.len(), 2);

                // Check tasks directory
                match &content[0] {
                    NewNode::Directory { name, content, .. } => {
                        assert_eq!(name, "tasks");
                        assert_eq!(content.len(), 1);

                        // Check the file containing the header
                        match &content[0] {
                            NewNode::File { name, content, .. } => {
                                assert_eq!(name, "sprint.md");
                                assert_eq!(content.len(), 1);
                                match &content[0] {
                                    NewFileEntry::Header { name, content, .. } => {
                                        assert_eq!(name, "Sprint 1");
                                        assert_eq!(content.len(), 1);
                                        assert!(matches!(content[0], NewFileEntry::Task(_)));
                                    }
                                    _ => panic!("Expected Header in file"),
                                }
                            }
                            _ => panic!("Expected File node"),
                        }
                    }
                    _ => panic!("Expected tasks Directory"),
                }
            }
            _ => panic!("Expected project Directory"),
        }
    }

    #[test]
    fn test_empty_directory_conversion() {
        let legacy = VaultData::Directory("empty".to_string(), vec![]);
        let base_path = PathBuf::from("/vault");
        let result = legacy.to_new_node(&base_path);

        match result {
            NewNode::Directory {
                name,
                path,
                content,
            } => {
                assert_eq!(name, "empty");
                assert_eq!(path, base_path.join("empty"));
                assert_eq!(content.len(), 0);
            }
            _ => panic!("Expected Directory variant"),
        }
    }

    #[test]
    fn test_header_with_nested_headers() {
        let task = create_test_task();
        let inner_header =
            VaultData::Header(2, "Subsection".to_string(), vec![VaultData::Task(task)]);
        let outer_header = VaultData::Header(1, "Section".to_string(), vec![inner_header]);

        let result = outer_header.to_new_file_entry();
        assert!(result.is_some());

        match result.unwrap() {
            NewFileEntry::Header {
                name,
                heading_level,
                content,
            } => {
                assert_eq!(name, "Section");
                assert_eq!(heading_level, 1);
                assert_eq!(content.len(), 1);

                match &content[0] {
                    NewFileEntry::Header {
                        name,
                        heading_level,
                        content,
                    } => {
                        assert_eq!(name, "Subsection");
                        assert_eq!(*heading_level, 2);
                        assert_eq!(content.len(), 1);
                        assert!(matches!(content[0], NewFileEntry::Task(_)));
                    }
                    _ => panic!("Expected nested Header"),
                }
            }
            _ => panic!("Expected Header variant"),
        }
    }

    #[test]
    fn test_string_output_file_entry_conversion() {
        let task = create_test_task();
        let tracker = create_test_tracker();

        // Test 1: Single task
        let legacy_task = VaultData::Task(task.clone());
        let new_task = legacy_task.to_new_file_entry().unwrap();
        assert_eq!(
            legacy_task.to_string(),
            new_task.to_string(),
            "Task string output should match"
        );

        // Test 2: Single tracker
        let legacy_tracker = VaultData::Tracker(tracker.clone());
        let new_tracker = legacy_tracker.to_new_file_entry().unwrap();
        assert_eq!(
            legacy_tracker.to_string(),
            new_tracker.to_string(),
            "Tracker string output should match"
        );

        // Test 3: Header with content
        let legacy_header = VaultData::Header(
            1,
            "Test Header".to_string(),
            vec![VaultData::Task(task.clone())],
        );
        let new_header = legacy_header.to_new_file_entry().unwrap();
        assert_eq!(
            legacy_header.to_string(),
            new_header.to_string(),
            "Header string output should match"
        );

        // Test 4: Nested headers
        let inner_header = VaultData::Header(
            2,
            "Subsection".to_string(),
            vec![
                VaultData::Task(task.clone()),
                VaultData::Tracker(tracker.clone()),
            ],
        );
        let outer_header = VaultData::Header(
            1,
            "Section".to_string(),
            vec![inner_header, VaultData::Task(task.clone())],
        );
        let new_outer = outer_header.to_new_file_entry().unwrap();
        assert_eq!(
            outer_header.to_string(),
            new_outer.to_string(),
            "Nested header string output should match"
        );
    }

    #[test]
    fn test_string_output_complex_vault_structure() {
        let task1 = create_test_task();
        let mut task2 = create_test_task();
        task2.description = Some("Second task".to_string());
        let tracker = create_test_tracker();

        // Create a complex nested structure that matches how the parser actually works
        // In the legacy system, files are Header(0, filename, content)
        let tasks_header = VaultData::Header(
            2,
            "Tasks Section".to_string(),
            vec![
                VaultData::Task(task1.clone()),
                VaultData::Task(task2.clone()),
            ],
        );

        let trackers_header = VaultData::Header(
            2,
            "Trackers Section".to_string(),
            vec![VaultData::Tracker(tracker.clone())],
        );

        let main_header = VaultData::Header(
            1,
            "Main Section".to_string(),
            vec![tasks_header, trackers_header],
        );

        // Wrap in a file (Header with level 0)
        let tasks_file = VaultData::Header(0, "tasks.md".to_string(), vec![main_header]);
        let docs_file = VaultData::Header(
            0,
            "docs.md".to_string(),
            vec![VaultData::Task(task1.clone())],
        );

        let tasks_dir = VaultData::Directory("tasks".to_string(), vec![tasks_file]);
        let docs_dir = VaultData::Directory("docs".to_string(), vec![docs_file]);

        let project_dir = VaultData::Directory("project".to_string(), vec![tasks_dir, docs_dir]);

        // Convert to new structure
        let base_path = PathBuf::from("/vault");
        let new_structure = project_dir.to_new_node(&base_path);

        let legacy_str = project_dir.to_string();
        let new_str = new_structure.to_string();

        println!("=== Legacy Output ===");
        println!("{legacy_str}");
        println!("\n=== New Output ===");
        println!("{new_str}");

        // Now they should match exactly!
        assert_eq!(legacy_str, new_str, "String outputs should match exactly");
    }

    #[test]
    fn test_string_output_vault_data_conversion() {
        let task = create_test_task();
        let tracker = create_test_tracker();

        // Create a structure that matches how the parser actually works
        // Files are Header(0, filename, content)
        let project_a_file = VaultData::Header(
            0,
            "project_a.md".to_string(),
            vec![VaultData::Header(
                1,
                "Project A".to_string(),
                vec![VaultData::Task(task.clone())],
            )],
        );

        let habits_file = VaultData::Header(
            0,
            "habits.md".to_string(),
            vec![VaultData::Header(
                1,
                "Habits".to_string(),
                vec![VaultData::Tracker(tracker.clone())],
            )],
        );

        let legacy_data = vec![
            VaultData::Directory("Work".to_string(), vec![project_a_file]),
            VaultData::Directory("Personal".to_string(), vec![habits_file]),
        ];

        let new_vault = convert_legacy_to_new(legacy_data.clone());

        // Compare outputs
        let mut legacy_combined = String::new();
        for item in &legacy_data {
            legacy_combined.push_str(&item.to_string());
            legacy_combined.push('\n');
        }

        let new_output = new_vault.to_string();

        println!("=== Legacy Combined Output ===");
        println!("{legacy_combined}");
        println!("\n=== New Vault Output ===");
        println!("{new_output}");

        // Now they should match exactly!
        assert_eq!(
            legacy_combined, new_output,
            "String outputs should match exactly"
        );
    }

    #[test]
    fn test_round_trip_file_entry() {
        let task = create_test_task();
        let tracker = create_test_tracker();

        // Test 1: Task round-trip
        let legacy_task = VaultData::Task(task.clone());
        let new_task = legacy_task.to_new_file_entry().unwrap();
        let back_to_legacy = new_task.to_legacy();
        assert_eq!(
            legacy_task, back_to_legacy,
            "Task should survive round-trip"
        );

        // Test 2: Tracker round-trip
        let legacy_tracker = VaultData::Tracker(tracker.clone());
        let new_tracker = legacy_tracker.to_new_file_entry().unwrap();
        let back_to_legacy = new_tracker.to_legacy();
        assert_eq!(
            legacy_tracker, back_to_legacy,
            "Tracker should survive round-trip"
        );

        // Test 3: Header with content round-trip
        let legacy_header = VaultData::Header(
            1,
            "Test Header".to_string(),
            vec![VaultData::Task(task.clone()), VaultData::Tracker(tracker)],
        );
        let new_header = legacy_header.to_new_file_entry().unwrap();
        let back_to_legacy = new_header.to_legacy();
        assert_eq!(
            legacy_header, back_to_legacy,
            "Header should survive round-trip"
        );
    }

    #[test]
    fn test_round_trip_node() {
        let task = create_test_task();
        let tracker = create_test_tracker();

        // Test: File (Header level 0) round-trip
        let legacy_file = VaultData::Header(
            0,
            "test.md".to_string(),
            vec![
                VaultData::Header(
                    1,
                    "Section".to_string(),
                    vec![VaultData::Task(task.clone())],
                ),
                VaultData::Tracker(tracker),
            ],
        );

        let base_path = PathBuf::from("/vault");
        let new_node = legacy_file.to_new_node(&base_path);
        let back_to_legacy = new_node.to_legacy();

        assert_eq!(
            legacy_file, back_to_legacy,
            "File should survive round-trip"
        );
    }

    #[test]
    fn test_round_trip_directory() {
        let task1 = create_test_task();
        let mut task2 = create_test_task();
        task2.name = "Task 2".to_string();

        // Create a directory with files
        let file1 = VaultData::Header(0, "file1.md".to_string(), vec![VaultData::Task(task1)]);
        let file2 = VaultData::Header(
            0,
            "file2.md".to_string(),
            vec![VaultData::Header(
                1,
                "Header".to_string(),
                vec![VaultData::Task(task2)],
            )],
        );

        let legacy_dir = VaultData::Directory("test_dir".to_string(), vec![file1, file2]);

        let base_path = PathBuf::from("/vault");
        let new_node = legacy_dir.to_new_node(&base_path);
        let back_to_legacy = new_node.to_legacy();

        assert_eq!(
            legacy_dir, back_to_legacy,
            "Directory should survive round-trip"
        );
    }

    #[test]
    fn test_round_trip_full_vault() {
        let task = create_test_task();
        let tracker = create_test_tracker();

        // Create a complex vault structure
        let project_file = VaultData::Header(
            0,
            "project.md".to_string(),
            vec![VaultData::Header(
                1,
                "Project".to_string(),
                vec![VaultData::Task(task.clone())],
            )],
        );

        let habits_file = VaultData::Header(
            0,
            "habits.md".to_string(),
            vec![VaultData::Tracker(tracker)],
        );

        let work_dir = VaultData::Directory("Work".to_string(), vec![project_file]);
        let personal_dir = VaultData::Directory("Personal".to_string(), vec![habits_file]);

        let legacy_data = vec![work_dir, personal_dir];

        let new_vault = convert_legacy_to_new(legacy_data.clone());
        let back_to_legacy = convert_new_to_legacy(&new_vault);

        assert_eq!(
            legacy_data, back_to_legacy,
            "Full vault should survive round-trip"
        );
    }

    #[test]
    fn test_round_trip_nested_directories() {
        let task = create_test_task();

        // Create nested directories
        let file = VaultData::Header(0, "test.md".to_string(), vec![VaultData::Task(task)]);
        let inner_dir = VaultData::Directory("inner".to_string(), vec![file]);
        let middle_dir = VaultData::Directory("middle".to_string(), vec![inner_dir]);
        let outer_dir = VaultData::Directory("outer".to_string(), vec![middle_dir]);

        let legacy_data = vec![outer_dir];

        let new_vault = convert_legacy_to_new(legacy_data.clone());
        let back_to_legacy = convert_new_to_legacy(&new_vault);

        assert_eq!(
            legacy_data, back_to_legacy,
            "Nested directories should survive round-trip"
        );
    }

    #[test]
    fn test_round_trip_malformed_structures() {
        let task = create_test_task();

        // Test 1: Task at directory level gets wrapped in file
        let legacy_task = VaultData::Task(task.clone());
        let base_path = PathBuf::from("/vault");
        let new_node = legacy_task.to_new_node(&base_path);
        let back_to_legacy = new_node.to_legacy();

        // After round-trip, task should be wrapped in Header(0, "untitled.md", ...)
        match back_to_legacy {
            VaultData::Header(0, name, content) => {
                assert_eq!(name, "untitled.md");
                assert_eq!(content.len(), 1);
                assert!(matches!(content[0], VaultData::Task(_)));
            }
            _ => panic!("Expected wrapped task in file"),
        }

        // Test 2: Non-file header at directory level
        let legacy_header = VaultData::Header(
            1,
            "Test Header".to_string(),
            vec![VaultData::Task(task.clone())],
        );
        let new_node = legacy_header.to_new_node(&base_path);
        let back_to_legacy = new_node.to_legacy();

        // After round-trip, header should be wrapped in Header(0, filename, ...)
        match back_to_legacy {
            VaultData::Header(0, name, content) => {
                assert_eq!(name, "test_header.md");
                assert_eq!(content.len(), 1);
                match &content[0] {
                    VaultData::Header(1, header_name, _) => {
                        assert_eq!(header_name, "Test Header");
                    }
                    _ => panic!("Expected wrapped header"),
                }
            }
            _ => panic!("Expected wrapped header in file"),
        }
    }
}
