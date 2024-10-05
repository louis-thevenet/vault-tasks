use ratatui::{
    prelude::*,
    widgets::{Block, Borders},
};

use tui_widget_list::{ListBuilder, ListState, ListView};

use crate::{config::Config, task_core::vault_data::VaultData};

use super::task_list_item::TaskListItem;

#[derive(Default, Clone)]
pub struct TaskList {
    file_content: Vec<VaultData>,
    not_american_format: bool,
    state: ListState,
    display_filename: bool,
}

impl TaskList {
    pub fn new(config: &Config, file_content: &[VaultData], display_filename: bool) -> Self {
        Self {
            state: ListState::default(),
            not_american_format: !config.tasks_config.use_american_format,
            file_content: file_content.to_vec(),
            display_filename,
        }
    }
}
impl Widget for TaskList {
    fn render(mut self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        let surrounding_block = Block::default().borders(Borders::NONE);
        let count = self.file_content.len();

        let builder = ListBuilder::new(move |context| {
            let item = TaskListItem::new(
                self.file_content[context.index].clone(),
                self.not_american_format,
                self.display_filename,
            );
            let height = item.height;
            (item, height.try_into().unwrap())
        });

        let lateral_entries_list = ListView::new(builder, count).block(surrounding_block);
        let state = &mut self.state;
        lateral_entries_list.render(area, buf, state);
    }
}
