use std::time::Duration;

use chrono::TimeDelta;
use color_eyre::Result;
use ratatui::prelude::*;
use tokio::sync::mpsc::UnboundedSender;
use tracing::debug;
use vault_tasks_time_management::time_management_technique::TimeManagementTechnique;
use vault_tasks_time_management::TimeManagementEngine;

use super::Component;

use crate::app::Mode;
use crate::tui::Tui;
use crate::widgets::help_menu::HelpMenu;
use crate::widgets::timer::{TimerState, TimerWidget};
use crate::{action::Action, config::Config};

/// Struct that helps with drawing the component
struct TimeManagementTabArea {
    content: Rect,
    footer: Rect,
}
#[derive(Default)]
pub struct TimeManagementTab<'a> {
    command_tx: Option<UnboundedSender<Action>>,
    config: Config,
    is_focused: bool,
    timer_state: TimerState,
    /// Whether the help panel is open or not
    show_help: bool,
    help_menu_wigdet: HelpMenu<'a>,
}
impl<'a> TimeManagementTab<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    fn split_frame(area: Rect) -> TimeManagementTabArea {
        let vertical = Layout::vertical([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
            Constraint::Length(1),
        ]);
        let [_header, content, footer, _tab_footera] = vertical.areas(area);

        TimeManagementTabArea { content, footer }
    }

    // fn render_sorting_modes(&self, area: Rect, buf: &mut Buffer) {
    //     let titles = TimeManagementTechnique::iter()
    //         .map(|arg0: TimeManagementTechnique| TimeManagementTechnique::to_string(&arg0));

    //     let highlight_style = *self
    //         .config
    //         .styles
    //         .get(&crate::app::Mode::Home)
    //         .unwrap()
    //         .get("highlighted_tab")
    //         .unwrap();

    //     let selected_tab_index = self.sorting_mode as usize;
    //     Tabs::new(titles)
    //         .select(selected_tab_index)
    //         .highlight_style(highlight_style)
    //         .padding("", "")
    //         .divider(" ")
    //         .block(Block::bordered().title("Sort By"))
    //         .render(area, buf);
    // }
    fn render_footer(&self, area: Rect, frame: &mut Frame) {
        Line::raw("Footer Place Holder")
            .centered()
            .render(area, frame.buffer_mut());
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
        // We always perform this action
        if matches!(action, Action::Tick) {
            debug!("{}", self.timer_state.tick());
        }

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
                Action::Enter => {
                    self.timer_state = TimerState::ClockDown {
                        stop_at: chrono::Local::now()
                            .checked_add_signed(
                                TimeDelta::from_std(Duration::from_secs(15)).unwrap(),
                            )
                            .unwrap()
                            .time(),
                        started_at: chrono::Local::now().time(),
                    }
                }

                Action::Focus(mode) if mode != Mode::TimeManagement => self.is_focused = false,
                Action::Focus(Mode::TimeManagement) => self.is_focused = true,

                Action::Help => self.show_help = !self.show_help,
                _ => (),
            }
        }
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        if !self.is_focused {
            return Ok(());
        }

        let areas = Self::split_frame(area);
        self.render_footer(areas.footer, frame);
        TimerWidget {}.render(areas.content, frame.buffer_mut(), &mut self.timer_state);
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
