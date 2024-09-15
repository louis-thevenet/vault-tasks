use color_eyre::Result;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List};
use tokio::sync::mpsc::UnboundedSender;

use super::Component;
use crate::task_core::TaskManager;
use crate::{action::Action, config::Config};

#[derive(Default)]
pub struct Home {
    command_tx: Option<UnboundedSender<Action>>,
    // config: Config,
    task_mgr: TaskManager,
    selected_header_path: Vec<String>,
    selected_entry_index: usize,
    current_prefixes_entries: (Vec<String>, Vec<String>),
}

impl Home {
    pub fn new() -> Self {
        Self::default()
    }
    fn select_previous_entry(&mut self) {
        self.selected_entry_index =
            (self.selected_entry_index + self.current_prefixes_entries.0.len() - 1)
                % self.current_prefixes_entries.0.len();
    }
    fn select_next_entry(&mut self) {
        self.selected_entry_index =
            (self.selected_entry_index + 1) % self.current_prefixes_entries.0.len();
    }

    fn get_into_selected_entry(&mut self) {
        self.selected_header_path
            .push(self.current_prefixes_entries.1[self.selected_entry_index].clone());
        self.selected_entry_index = 0;
        self.update_entries();
    }
    fn get_out_of_selected_entry(&mut self) {
        if self.selected_header_path.is_empty() {
            return;
        }
        self.selected_header_path.pop();
        self.selected_entry_index = 0;
        self.update_entries();
    }
    fn update_entries(&mut self) {
        self.current_prefixes_entries =
            match self.task_mgr.get_entries(self.selected_header_path.clone()) {
                Ok(items) => items,
                Err(e) => (vec![String::new()], vec![e.to_string()]),
            };
    }
}

impl Component for Home {
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<()> {
        self.command_tx = Some(tx);
        Ok(())
    }

    fn register_config_handler(&mut self, config: Config) -> Result<()> {
        // self.config = config;
        self.task_mgr = TaskManager::load_from_config(&config);
        Ok(())
    }

    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        match action {
            Action::Up => self.select_previous_entry(),
            Action::Down => self.select_next_entry(),
            Action::Right | Action::Enter => self.get_into_selected_entry(),
            Action::Left | Action::Cancel => self.get_out_of_selected_entry(),
            Action::Help => todo!(),
            _ => (),
        }
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, _area: Rect) -> Result<()> {
        if self.current_prefixes_entries.0.is_empty() {
            self.update_entries();
        }
        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![Constraint::Percentage(20), Constraint::Percentage(80)])
            .split(frame.area());

        // Lateral Menu
        let mut entries_to_display: Vec<String> = self
            .current_prefixes_entries
            .1
            .iter()
            .enumerate()
            .map(|(i, item)| format!("{} {}", self.current_prefixes_entries.0[i], item))
            .collect();

        entries_to_display[self.selected_entry_index] =
            format!("> {}", entries_to_display[self.selected_entry_index]);
        let surrounding_lateral_block =
            Block::default().borders(Borders::ALL).title("Lateral Menu");
        let lateral_entries_list = List::new(entries_to_display).block(surrounding_lateral_block);
        frame.render_widget(lateral_entries_list, layout[0]);

        // Center View
        let surrounding_center_block = Block::default().borders(Borders::ALL).title("Center View");
        let mut path_to_preview = self.selected_header_path.clone();
        path_to_preview.push(self.current_prefixes_entries.1[self.selected_entry_index].clone());
        let (preview_items_prefixes, preview_items_list) =
            match self.task_mgr.get_entries(path_to_preview) {
                Ok(items) => items,
                Err(e) => (vec![String::new()], vec![e.to_string()]),
            };

        let displayed_preview_entries: Vec<String> = preview_items_list
            .iter()
            .enumerate()
            .map(|(i, item)| format!("{} {}", preview_items_prefixes[i], item))
            .collect();

        let preview_entries = List::new(displayed_preview_entries).block(surrounding_center_block);
        frame.render_widget(preview_entries, layout[1]);

        Ok(())
    }
}
