use color_eyre::Result;
use tracing::debug;

use super::ExplorerTab;

impl ExplorerTab<'_> {
    pub(super) fn leave_selected_entry(&mut self) -> Result<()> {
        if self.current_path.is_empty() {
            return Ok(());
        }

        self.current_path.pop().unwrap_or_default();
        // Update index of selected entry to previous selected entry
        self.state_center_view.select(self.state_left_view.selected);

        self.update_entries()?;

        // Find previously selected entry
        self.select_previous_left_entry();
        Ok(())
    }
    pub(super) fn enter_selected_entry(&mut self) -> Result<()> {
        // Update path with selected entry
        let entry = match self
            .entries_center_view
            .get(self.state_center_view.selected.unwrap_or_default())
        {
            Some(i) => i,
            None => return Ok(()), // No selected entry (vault is empty)
        }
        .1
        .clone();
        self.current_path.push(entry);

        // Can we enter ?
        if !self.task_mgr.can_enter(&self.current_path) {
            self.current_path.pop();
            debug!("Coudln't enter: {:?}", self.current_path);
            return Ok(());
        }

        // Update selections
        self.state_left_view
            .select(Some(self.state_center_view.selected.unwrap_or_default()));
        self.state_center_view.select(Some(0));

        debug!("Entering: {:?}", self.current_path);

        // Update entries
        self.update_entries()
    }

    pub(super) fn select_previous_left_entry(&mut self) {
        if let Some(new_previous_entry) = self.current_path.last() {
            self.state_left_view.select(Some(
                self.entries_left_view
                    .clone()
                    .into_iter()
                    .enumerate()
                    .find(|(_, entry)| &entry.1 == new_previous_entry)
                    .unwrap_or_default()
                    .0,
            ));
        }
    }
}
