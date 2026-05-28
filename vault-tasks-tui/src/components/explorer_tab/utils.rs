use crate::action::Action;
use crate::tui::Tui;
use open_editor::EditorCallBuilder;
use vault_tasks_core::{
    Found,
    task::Task,
    vault_data::{FileEntryNode, VaultNode},
};

use super::{DIRECTORY_EMOJI, ExplorerTab, FILE_EMOJI};
use color_eyre::Result;
use color_eyre::eyre::bail;
use std::cmp::Ordering;
use tracing::{debug, error, info};

impl ExplorerTab<'_> {
    pub(super) fn apply_prefixes(entries: &[(String, String)]) -> Vec<String> {
        entries
            .iter()
            .map(|item| format!("{} {}", item.0, item.1))
            .collect()
    }

    fn vault_data_to_prefix_name(vd: &Found) -> (String, String) {
        match vd {
            Found::Root(_new_vault_data) => {
                unreachable!()
            }
            Found::Node(
                VaultNode::Vault {
                    content: _content,
                    name,
                    path: _path,
                }
                | VaultNode::Directory {
                    content: _content,
                    name,
                    path: _path,
                },
            ) => (DIRECTORY_EMOJI.to_owned(), name.clone()),
            Found::Node(VaultNode::File {
                content: _content,
                name,
                path: _path,
            }) => (FILE_EMOJI.to_owned(), name.clone()),
            Found::FileEntry(FileEntryNode::Header {
                content: _content,
                name,
                path: _,
                heading_level,
                line_number: _,
            }) => ("#".repeat(*heading_level).clone(), name.clone()),
            Found::FileEntry(FileEntryNode::Task(task)) => {
                (task.state.to_string(), task.name.clone())
            }
        }
    }
    pub(super) fn sort_entries(vd: &[Found]) -> Vec<Found> {
        let mut vd = vd.to_vec();
        vd.sort_by(|a, b| {
            let a_is_folder = matches!(
                a,
                Found::Node(VaultNode::Vault { .. } | VaultNode::Directory { .. })
            );
            let b_is_folder = matches!(
                b,
                Found::Node(VaultNode::Vault { .. } | VaultNode::Directory { .. })
            );

            let a_name = Self::vault_data_to_prefix_name(a).1;
            let b_name = Self::vault_data_to_prefix_name(b).1;

            if a_is_folder {
                if b_is_folder {
                    a_name.cmp(&b_name)
                } else {
                    Ordering::Less
                }
            } else if b_is_folder {
                Ordering::Greater
            } else {
                a_name.cmp(&b_name)
            }
        });
        vd
    }
    pub(super) fn vault_data_to_entry_list(vd: &[Found]) -> Vec<(String, String)> {
        vd.iter()
            .map(Self::vault_data_to_prefix_name)
            .collect::<Vec<(String, String)>>()
    }

    pub(super) fn get_preview_path(&self) -> Result<Vec<String>> {
        let mut path_to_preview = self.current_path.clone();
        if self.entries_center_view.is_empty() {
            bail!("Center view is empty for {:?}", self.current_path)
        }
        debug!(
            "preview path {:#?}",
            self.entries_center_view
                .get(self.state_center_view.selected.unwrap_or_default())
                .unwrap()
                .get_name()
        );
        match self
            .entries_center_view
            .get(self.state_center_view.selected.unwrap_or_default())
        {
            Some(res) => path_to_preview.push(res.get_name()),
            None => bail!(
                "Index ({:?}) of selected entry out of range {:?}",
                self.state_center_view.selected,
                self.entries_center_view
            ),
        }
        Ok(path_to_preview)
    }
    pub(super) fn open_current_file(&self, tui_opt: Option<&mut Tui>) -> Result<()> {
        let Some(tui) = tui_opt else {
            bail!("Could not open current entry, Tui was None")
        };
        let preview_path = self
            .get_preview_path()
            .unwrap_or_else(|_| self.current_path.clone());

        let entry = self.task_mgr.resolve_path(&preview_path)?;
        let line = entry.get_position_in_file().unwrap_or_default();

        let path = entry.get_path();
        if let Some(tx) = &self.command_tx {
            tui.exit()?;
            EditorCallBuilder::new()
                .at_line(line)
                .wait_for_editor(true)
                .open_file(&path)?;
            tui.enter()?;
            tx.send(Action::ClearScreen)?;
        } else {
            bail!("Failed to open current path")
        }
        if let Some(tx) = self.command_tx.clone() {
            tx.send(Action::ReloadVault)?;
        }
        Ok(())
    }

    pub(super) fn get_selected_task(&self) -> Option<Task> {
        let path = match self.get_preview_path() {
            Ok(path) => path,
            Err(e) => {
                error!("Error while getting path for selected task: {}", e);
                return None;
            }
        };
        debug!("Getting selected task from current path: {:?}", path);

        let Ok(entry) = self.task_mgr.resolve_path(&path) else {
            error!("Error while collecting tasks from path");
            return None;
        };

        if let Found::FileEntry(FileEntryNode::Task(task)) = entry {
            Some(task.clone())
        } else {
            info!("Selected object is not a Task");
            None
        }
    }
}
