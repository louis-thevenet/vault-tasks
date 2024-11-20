use color_eyre::Result;
use ratatui::prelude::*;
use tokio::sync::mpsc::UnboundedSender;
use tracing::debug;

use super::Component;

use crate::app::Mode;
use crate::tui::Tui;
use crate::widgets::help_menu::HelpMenu;
use crate::{action::Action, config::Config};

#[derive(Default)]
pub struct TimeManagementTab<'a> {
    command_tx: Option<UnboundedSender<Action>>,
    config: Config,
    is_focused: bool,
    /// Whether the help panel is open or not
    show_help: bool,
    help_menu_wigdet: HelpMenu<'a>,
}
impl<'a> TimeManagementTab<'a> {
    pub fn new() -> Self {
        Self::default()
    }
}
impl<'a> Component for TimeManagementTab<'a> {
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<()> {
        let _ = tx; // to appease clippy
        Ok(())
    }

    fn register_config_handler(&mut self, config: Config) -> Result<()> {
        self.config = config;
        self.help_menu_wigdet = HelpMenu::new(Mode::TimeManagement, &self.config);
        Ok(())
    }

    fn update(&mut self, tui: Option<&mut Tui>, action: Action) -> Result<Option<Action>> {
        if !self.is_focused {
            match action {
                Action::Focus(Mode::TimeManagement) => self.is_focused = true,
                Action::Focus(mode) if !(mode == Mode::TimeManagement) => self.is_focused = false,
                _ => (),
            }
        } else if self.show_help {
            match action {
                Action::ViewUp | Action::Up => self.help_menu_wigdet.scroll_up(),
                Action::ViewDown | Action::Down => self.help_menu_wigdet.scroll_down(),
                Action::Help | Action::Escape | Action::Enter => {
                    self.show_help = !self.show_help;
                }
                _ => (),
            }
        } else {
            match action {
                Action::Focus(mode) if mode != Mode::TimeManagement => self.is_focused = false,
                Action::Focus(Mode::TimeManagement) => self.is_focused = true,

                Action::Help => self.show_help = !self.show_help,
                _ => (),
            }
        }
        Ok(None)
    }

    fn escape_blocking_mode(&self) -> Vec<Action> {
        std::vec![]
    }

    fn blocking_mode(&self) -> bool {
        false
    }
    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        if !self.is_focused {
            return Ok(());
        }
        let _ = frame;
        let _ = area;

        if self.show_help {
            debug!("showing help");
            self.help_menu_wigdet.clone().render(
                area,
                frame.buffer_mut(),
                &mut self.help_menu_wigdet.state,
            );
        }
        Ok(())
    }
}
