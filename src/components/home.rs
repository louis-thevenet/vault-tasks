use color_eyre::Result;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, Paragraph};
use tokio::sync::mpsc::UnboundedSender;

use super::Component;
use crate::task_core::TaskManager;
use crate::{action::Action, config::Config};

#[derive(Default)]
pub struct Home {
    command_tx: Option<UnboundedSender<Action>>,
    task_mgr: TaskManager,
    current_path: Vec<String>,
    selected_entry_index_left_view: usize,
    selected_entry_index_center_view: usize,
    entries_left_view: (Vec<String>, Vec<String>),
    entries_center_view: (Vec<String>, Vec<String>),
}

impl Home {
    pub fn new() -> Self {
        Self::default()
    }
    fn select_previous_entry(&mut self) {
        self.selected_entry_index_center_view =
            (self.selected_entry_index_center_view + self.entries_center_view.0.len() - 1)
                % self.entries_center_view.0.len();
    }
    fn select_next_entry(&mut self) {
        self.selected_entry_index_center_view =
            (self.selected_entry_index_center_view + 1) % self.entries_center_view.0.len();
    }

    fn enter_selected_entry(&mut self) {
        self.current_path
            .push(self.entries_center_view.1[self.selected_entry_index_center_view].clone());
        self.selected_entry_index_left_view = self.selected_entry_index_center_view;
        self.selected_entry_index_center_view = 0;
        self.update_entries();
    }
    fn leave_selected_entry(&mut self) {
        if self.current_path.is_empty() {
            return;
        }

        self.current_path.pop().unwrap_or_default();

        self.update_entries();

        // Update index of selected entry to previous selected entry

        self.selected_entry_index_center_view = self.selected_entry_index_left_view;

        if let Some(new_previous_entry) = self.current_path.last() {
            self.selected_entry_index_left_view = self
                .entries_left_view
                .clone()
                .1
                .into_iter()
                .enumerate()
                .find(|(_, name)| name == new_previous_entry)
                .unwrap_or_default()
                .0;
        }
    }

    fn update_entries(&mut self) {
        if self.current_path.is_empty() {
            // Vault root
            self.entries_left_view = (vec![], vec![]);
        } else {
            self.entries_left_view = match self
                .task_mgr
                .get_entries(&self.current_path[0..self.current_path.len() - 1])
            {
                Ok(items) => items,
                Err(e) => (vec![String::new()], vec![e.to_string()]),
            };
        }
        self.entries_center_view = match self.task_mgr.get_entries(&self.current_path) {
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
            Action::Right | Action::Enter => self.enter_selected_entry(),
            Action::Left | Action::Cancel => self.leave_selected_entry(),
            Action::Help => todo!(),
            _ => (),
        }
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, _area: Rect) -> Result<()> {
        if self.entries_center_view.0.is_empty() {
            self.update_entries();
        }

        let outer_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Length(2), Constraint::Percentage(100)])
            .split(frame.area());

        // Current path
        frame.render_widget(
            Paragraph::new(format!("./{}", self.current_path.join("/"))),
            outer_layout[0],
        );

        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![
                Constraint::Percentage(10),
                Constraint::Percentage(30),
                Constraint::Percentage(60),
            ])
            .split(outer_layout[1]);

        // Left Block
        let mut entries_to_display: Vec<String> = self
            .entries_left_view
            .1
            .iter()
            .enumerate()
            .map(|(i, item)| format!("{} {}", self.entries_left_view.0[i], item))
            .collect();

        if !entries_to_display.is_empty() {
            entries_to_display[self.selected_entry_index_left_view] = format!(
                "> {}",
                entries_to_display[self.selected_entry_index_left_view]
            );
        }
        let surrounding_left_block = Block::default().borders(Borders::RIGHT);
        let left_entries_list = List::new(entries_to_display).block(surrounding_left_block);
        frame.render_widget(left_entries_list, layout[0]);

        // Center Block
        let mut entries_to_display: Vec<String> = self
            .entries_center_view
            .1
            .iter()
            .enumerate()
            .map(|(i, item)| format!("{} {}", self.entries_center_view.0[i], item))
            .collect();

        entries_to_display[self.selected_entry_index_center_view] = format!(
            "> {}",
            entries_to_display[self.selected_entry_index_center_view]
        );
        let surrounding_lateral_block = Block::default().borders(Borders::RIGHT);
        let lateral_entries_list = List::new(entries_to_display).block(surrounding_lateral_block);
        frame.render_widget(lateral_entries_list, layout[1]);

        // Right Block
        let surrounding_center_block = Block::default().borders(Borders::NONE);
        let mut path_to_preview = self.current_path.clone();
        path_to_preview
            .push(self.entries_center_view.1[self.selected_entry_index_center_view].clone());
        let (preview_items_prefixes, preview_items_list) =
            match self.task_mgr.get_entries(&path_to_preview) {
                Ok(items) => items,
                Err(e) => (vec![String::new()], vec![e.to_string()]),
            };

        let displayed_preview_entries: Vec<String> = preview_items_list
            .iter()
            .enumerate()
            .map(|(i, item)| format!("{} {}", preview_items_prefixes[i], item))
            .collect();

        let preview_entries = List::new(displayed_preview_entries).block(surrounding_center_block);
        frame.render_widget(preview_entries, layout[2]);

        Ok(())
    }
}
