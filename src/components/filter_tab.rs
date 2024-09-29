use std::time::Duration;

use color_eyre::eyre::bail;
use color_eyre::owo_colors::OwoColorize;
use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::widgets::Paragraph;
use ratatui::{prelude::*, widgets::Block};
use tokio::sync::mpsc::UnboundedSender;
use tracing::{debug, error};

use super::Component;

use crate::task_core::filter::filter;
use crate::task_core::parser::task::parse_task;
use crate::task_core::task::Task;
use crate::task_core::vault_data::VaultData;
use crate::task_core::TaskManager;
use crate::widgets::task_list::TaskList;
use crate::{action::Action, config::Config};
use tui_input::{backend::crossterm::EventHandler, Input};

#[derive(Default)]
pub struct FilterTab {
    task_mgr: TaskManager,
    matching_entries: Vec<Task>,
    command_tx: Option<UnboundedSender<Action>>,
    config: Config,
    focused: bool,
    input: Input,
    input_mode: InputMode,
}

#[derive(Default)]
enum InputMode {
    Normal,
    #[default]
    Editing,
}
impl InputMode {
    const fn invert(&self) -> Self {
        match self {
            Self::Normal => Self::Editing,
            Self::Editing => Self::Normal,
        }
    }
}
impl FilterTab {
    pub fn new() -> Self {
        Self::default()
    }
    fn update_matching_entries(&mut self) -> Result<()> {
        let input_with_state = &format!("- [ ] {}", self.input.value());
        let mut input_str = input_with_state.as_str();
        let search = match parse_task(&mut input_str, &self.config) {
            Ok(t) => t,
            Err(e) => {
                bail!("Error: {e}");
            }
        };
        self.matching_entries = filter(&self.task_mgr.tasks, &search);
        Ok(())
    }
}
impl Component for FilterTab {
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<()> {
        self.command_tx = Some(tx);
        Ok(())
    }

    fn register_config_handler(&mut self, config: Config) -> Result<()> {
        self.task_mgr = TaskManager::load_from_config(&config)?;
        self.config = config;
        Ok(())
    }

    fn editing_mode(&self) -> bool {
        self.focused
            && match self.input_mode {
                InputMode::Normal => false,
                InputMode::Editing => true,
            }
    }
    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        if self.focused {
            match action {
                Action::FocusExplorer => self.focused = false,
                Action::FocusFilter => self.focused = true,
                Action::Enter => self.input_mode = self.input_mode.invert(),
                Action::Key(key) if matches!(self.input_mode, InputMode::Editing) => match key.code
                {
                    KeyCode::Enter => self.input_mode = self.input_mode.invert(),
                    _ => {
                        self.input.handle_event(&Event::Key(key));
                        self.update_matching_entries()?
                    }
                },
                _ => (),
            }
        } else {
            match action {
                Action::FocusExplorer => self.focused = false,
                Action::FocusFilter => self.focused = true,
                _ => (),
            }
        }
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, _area: Rect) -> Result<()> {
        if !self.focused {
            return Ok(());
        }
        let vertical = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(1),
        ]);
        let [_header_area, search_area, content_area, _footer_areaa] = vertical.areas(frame.area());

        let width = search_area.width.max(3) - 3; // 2 for borders, 1 for cursor
        let scroll = self.input.visual_scroll(width as usize);
        match self.input_mode {
            InputMode::Normal =>
                // Hide the cursor. `Frame` does this by default, so we don't need to do anything here
                {}

            InputMode::Editing => {
                // Make the cursor visible and ask tui-rs to put it at the specified coordinates after rendering
                frame.set_cursor_position((
                    // Put cursor past the end of the input text
                    search_area
                        .x
                        .saturating_add(((self.input.visual_cursor()).max(scroll) - scroll) as u16)
                        + 1,
                    // Move one line down, from the border to the input line
                    search_area.y + 1,
                ));
            }
        }

        let input =
            Paragraph::new(self.input.value())
                .style(Style::reset())
                .block(Block::bordered().title("Input").style(Style::new().fg(
                    match self.input_mode {
                        InputMode::Editing => Color::Rgb(255, 153, 0),
                        InputMode::Normal => Color::default(),
                    },
                )))
                .scroll((0, scroll as u16));
        frame.render_widget(input, search_area);

        let entries_list = TaskList::new(
            &self.config,
            &self
                .matching_entries
                .clone()
                .iter()
                .map(|t| VaultData::Task(t.clone()))
                .collect::<Vec<VaultData>>(),
        );
        entries_list.render(content_area, frame.buffer_mut());
        Ok(())
    }
}
