use crate::core::task::Task;
use crate::tui::Tui;
use crate::{action::Action, core::vault_data::VaultData};

use super::{ExplorerTab, DIRECTORY_EMOJI, FILE_EMOJI};
use color_eyre::eyre::bail;
use color_eyre::Result;
use std::cmp::Ordering;
use std::path::PathBuf;
use tracing::{error, info};

impl ExplorerTab<'_> {
    pub(super) fn apply_prefixes(entries: &[(String, String)]) -> Vec<String> {
        entries
            .iter()
            .map(|item| format!("{} {}", item.0, item.1))
            .collect()
    }

    fn vault_data_to_prefix_name(vd: &VaultData) -> (String, String) {
        match vd {
            VaultData::Directory(name, _) => (
                if name.contains(".md") {
                    FILE_EMOJI.to_owned()
                } else {
                    DIRECTORY_EMOJI.to_owned()
                },
                name.clone(),
            ),
            VaultData::Header(level, name, _) => ("#".repeat(*level).clone(), name.clone()),
            VaultData::Task(task) => (task.state.to_string(), task.name.clone()),
        }
    }

    pub(super) fn vault_data_to_entry_list(vd: &[VaultData]) -> Vec<(String, String)> {
        let mut res = vd
            .iter()
            .map(Self::vault_data_to_prefix_name)
            .collect::<Vec<(String, String)>>();

        if let Some(entry) = res.first() {
            if entry.0 == DIRECTORY_EMOJI || entry.0 == FILE_EMOJI {
                res.sort_by(|a, b| {
                    if a.0 == DIRECTORY_EMOJI {
                        if b.0 == DIRECTORY_EMOJI {
                            a.1.cmp(&b.1)
                        } else {
                            Ordering::Less
                        }
                    } else if b.0 == DIRECTORY_EMOJI {
                        Ordering::Greater
                    } else {
                        a.1.cmp(&b.1)
                    }
                });
            }
        }
        res
    }
    pub(super) fn get_preview_path(&self) -> Result<Vec<String>> {
        let mut path_to_preview = self.current_path.clone();
        if self.entries_center_view.is_empty() {
            bail!("Center view is empty for {:?}", self.current_path)
        }
        match self
            .entries_center_view
            .get(self.state_center_view.selected.unwrap_or_default())
        {
            Some(res) => path_to_preview.push(res.clone().1),
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
        let path = self.get_current_path_to_file();
        info!("Opening {:?} in default editor.", path);
        if let Some(tx) = &self.command_tx {
            tui.exit()?;
            edit::edit_file(path)?;
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
    pub(super) fn get_current_path_to_file(&self) -> PathBuf {
        let mut path = self.config.tasks_config.vault_path.clone();
        for e in &self
            .get_preview_path()
            .unwrap_or_else(|_| self.current_path.clone())
        {
            if path
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
            {
                break;
            }
            path.push(e);
        }
        path
    }
    pub(super) fn get_selected_task(&self) -> Option<Task> {
        let Ok(entries) = self
            .task_mgr
            .get_vault_data_from_path(&self.current_path, false)
        else {
            error!("Error while collecting tasks from path");
            return None;
        };
        if entries.len() <= self.state_center_view.selected.unwrap_or_default() {
            error!("No task selected: Index of selected entry > list of entries");
            return None;
        }
        let entry = entries[self.state_center_view.selected.unwrap_or_default()].clone();
        if let VaultData::Task(task) = entry {
            Some(task)
        } else {
            info!("Selected object is not a Task");
            None
        }
    }
}
