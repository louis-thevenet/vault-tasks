use color_eyre::Result;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};
use tokio::sync::mpsc::UnboundedSender;
use tracing::{debug, error};
use tui_widget_list::{ListBuilder, ListState, ListView};

use super::Component;

use crate::task_core::TaskManager;
use crate::widgets::file_data::FileData;
use crate::{action::Action, config::Config};

#[derive(Default)]
pub struct Home {
    command_tx: Option<UnboundedSender<Action>>,
    config: Config,
    task_mgr: TaskManager,
    current_path: Vec<String>,
    state_left_view: ListState,
    entries_left_view: (Vec<String>, Vec<String>),
    state_center_view: ListState,
    entries_center_view: (Vec<String>, Vec<String>),
    entries_right_view: FileData,
}

impl Home {
    pub fn new() -> Self {
        Self::default()
    }

    fn enter_selected_entry(&mut self) -> Result<()> {
        // Update path with selected entry
        self.current_path.push(
            self.entries_center_view.1[self.state_center_view.selected.unwrap_or_default()].clone(),
        );

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
    fn leave_selected_entry(&mut self) -> Result<()> {
        if self.current_path.is_empty() {
            return Ok(());
        }

        self.current_path.pop().unwrap_or_default();
        // Update index of selected entry to previous selected entry
        self.state_center_view.select(self.state_left_view.selected);

        self.update_entries()?;

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
        Ok(())
    }

    fn update_entries(&mut self) -> Result<()> {
        debug!("Updating entries");
        if self.current_path.is_empty() {
            // Vault root
            self.entries_left_view = (vec![], vec![]);
        } else {
            self.entries_left_view = self
                .task_mgr
                .get_navigator_entries(&self.current_path[0..self.current_path.len() - 1])?;
        }
        self.entries_center_view =
            if let Ok(res) = self.task_mgr.get_navigator_entries(&self.current_path) {
                res
            } else {
                self.leave_selected_entry()?;
                return Ok(());
            };
        self.update_preview();
        Ok(())
    }
    fn update_preview(&mut self) {
        debug!("Updating preview");
        let mut path_to_preview = self.current_path.clone();
        if self.entries_center_view.1.is_empty() {
            return;
        }
        path_to_preview.push(
            self.entries_center_view.1[self.state_center_view.selected.unwrap_or_default()].clone(),
        );

        if let Ok(data) = self.task_mgr.get_vault_data_from_path(&path_to_preview) {
            self.entries_right_view = FileData::new(&self.config, &data);
        }
    }
}

impl Component for Home {
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<()> {
        self.command_tx = Some(tx);
        Ok(())
    }

    fn register_config_handler(&mut self, config: Config) -> Result<()> {
        self.task_mgr = TaskManager::load_from_config(&config)?;
        self.config = config;
        self.update_entries()?;
        self.state_center_view.selected = Some(0);
        Ok(())
    }

    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        match action {
            Action::Up => {
                self.state_center_view.previous();
                self.update_preview();
            }
            Action::Down => {
                self.state_center_view.next();
                self.update_preview();
            }
            Action::Right | Action::Enter => self.enter_selected_entry()?,
            Action::Left | Action::Cancel => self.leave_selected_entry()?,
            Action::Help => todo!(),
            _ => (),
        }
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, _area: Rect) -> Result<()> {
        if self.entries_center_view.0.is_empty() {
            error!("Center view is empty"); // is it always an error ?
            self.update_entries()?;
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
        self.entries_right_view
            .clone()
            .render(layout[2], frame.buffer_mut());
        Ok(())
    }
}
