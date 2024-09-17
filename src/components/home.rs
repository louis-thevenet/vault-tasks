use color_eyre::Result;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, Paragraph};
use tokio::sync::mpsc::UnboundedSender;
use tui_widget_list::{ListBuilder, ListState, ListView};

use super::Component;
use crate::task_core::TaskManager;
use crate::{action::Action, config::Config};

#[derive(Default)]
pub struct Home {
    command_tx: Option<UnboundedSender<Action>>,
    task_mgr: TaskManager,
    current_path: Vec<String>,
    state_left_view: ListState,
    entries_left_view: (Vec<String>, Vec<String>),
    state_center_view: ListState,
    entries_center_view: (Vec<String>, Vec<String>),
}

impl Home {
    pub fn new() -> Self {
        Self::default()
    }

    fn enter_selected_entry(&mut self) {
        self.current_path.push(
            self.entries_center_view.1[self.state_center_view.selected.unwrap_or_default()].clone(),
        );
        self.state_left_view
            .select(Some(self.state_center_view.selected.unwrap_or_default()));
        self.state_center_view.select(Some(0));
        self.update_entries();
    }
    fn leave_selected_entry(&mut self) {
        if self.current_path.is_empty() {
            return;
        }

        self.current_path.pop().unwrap_or_default();
        self.update_entries();

        // Update index of selected entry to previous selected entry
        self.state_center_view.select(self.state_left_view.selected);

        // Find previously selected entry
        if let Some(new_previous_entry) = self.current_path.last() {
            self.state_left_view.select(Some(
                self.entries_left_view
                    .clone()
                    .1
                    .into_iter()
                    .enumerate()
                    .find(|(_, name)| name == new_previous_entry)
                    .unwrap_or_default()
                    .0,
            ));
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
            Action::Up => self.state_center_view.previous(),
            Action::Down => self.state_center_view.next(),
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
            self.state_center_view.selected = Some(0);
        }

        // Outer Layout : path on top, main layout on bottom
        let outer_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Length(2), Constraint::Percentage(100)])
            .split(frame.area());

        // Current path
        frame.render_widget(
            Paragraph::new(format!("./{}", self.current_path.join("/"))),
            outer_layout[0],
        );

        // Main Layout
        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![
                Constraint::Percentage(10),
                Constraint::Percentage(30),
                Constraint::Percentage(60),
            ])
            .split(outer_layout[1]);

        // Left Block
        let surrounding_left_block = Block::default().borders(Borders::RIGHT);
        let entries_to_display: Vec<String> = self
            .entries_left_view
            .1
            .iter()
            .enumerate()
            .map(|(i, item)| format!("{} {}", self.entries_left_view.0[i], item))
            .collect();

        let builder = ListBuilder::new(move |context| {
            let mut item = Paragraph::new(entries_to_display[context.index].clone());
            if context.is_selected {
                item = item.style(
                    Style::default()
                        .bg(Color::Rgb(255, 153, 0))
                        .fg(Color::Rgb(28, 28, 32)),
                );
            };
            let main_axis_size = 1;
            (item, main_axis_size)
        });

        let item_count = self.entries_left_view.0.len();
        let left_entries_list = ListView::new(builder, item_count).block(surrounding_left_block);
        let state = &mut self.state_left_view;
        left_entries_list.render(layout[0], frame.buffer_mut(), state);

        // Center Block
        let surrounding_center_block = Block::default().borders(Borders::RIGHT);
        let entries_to_display: Vec<String> = self
            .entries_center_view
            .1
            .iter()
            .enumerate()
            .map(|(i, item)| format!("{} {}", self.entries_center_view.0[i], item))
            .collect();

        let builder = ListBuilder::new(move |context| {
            let mut item = Paragraph::new(entries_to_display[context.index].clone());
            if context.is_selected {
                item = item.style(
                    Style::default()
                        .bg(Color::Rgb(255, 153, 0))
                        .fg(Color::Rgb(28, 28, 32)),
                );
            };
            let main_axis_size = 1;
            (item, main_axis_size)
        });

        let item_count = self.entries_center_view.0.len();
        let lateral_entries_list =
            ListView::new(builder, item_count).block(surrounding_center_block);
        let state = &mut self.state_center_view;
        lateral_entries_list.render(layout[1], frame.buffer_mut(), state);

        // Right Block
        let surrounding_center_block = Block::default().borders(Borders::NONE);
        let mut path_to_preview = self.current_path.clone();
        path_to_preview.push(
            self.entries_center_view.1[self.state_center_view.selected.unwrap_or_default()].clone(),
        );
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
