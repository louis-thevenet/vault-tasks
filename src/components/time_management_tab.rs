use color_eyre::eyre::bail;
use color_eyre::Result;
use notify_rust::Notification;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, ListState};
use strum::{EnumIter, FromRepr, IntoEnumIterator};
use tokio::sync::mpsc::UnboundedSender;
use tracing::{debug, error};
use vault_tasks_time_management::flow_time::FlowTime;
use vault_tasks_time_management::pomodoro::Pomodoro;
use vault_tasks_time_management::time_management_technique::TimeManagementTechnique;
use vault_tasks_time_management::{State, TimeManagementEngine};

use super::Component;

use crate::app::Mode;
use crate::tui::Tui;
use crate::widgets::help_menu::HelpMenu;
use crate::widgets::timer::{TimerState, TimerWidget};
use crate::{action::Action, config::Config};

/// Struct that helps with drawing the component
struct TimeManagementTabArea {
    clock: Rect,
    technique_list: Rect,
    technique_settings: Rect,
    footer: Rect,
}

#[derive(Default, Clone, Copy, FromRepr, EnumIter, strum_macros::Display)]
enum TimerTechniquesAvailable {
    #[default]
    #[strum(to_string = "Pomodoro")]
    Pomodoro,
    #[strum(to_string = "Flowtime")]
    FlowTime,
}

#[derive(Default)]
pub struct TimeManagementTab<'a> {
    command_tx: Option<UnboundedSender<Action>>,
    config: Config,
    is_focused: bool,
    /// Timer
    time_techniques_list_state: ListState,
    timer_state: TimerState,
    timer_engine: TimeManagementEngine,
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
            Constraint::Max(10), // Label + Block
            Constraint::Min(0),
            Constraint::Length(1),
            Constraint::Length(1),
        ]);
        let [_header, clock, techniques_area, footer, _tab_footera] = vertical.areas(area);

        let [technique_list, technique_settings] = Layout::horizontal([
            Constraint::Length(
                u16::try_from(
                    TimerTechniquesAvailable::iter()
                        .map(|t| 3 + 1 + t.to_string().len())
                        .max()
                        .unwrap_or_default(),
                )
                .unwrap_or_default(),
            ),
            Constraint::Min(0),
        ])
        .areas(techniques_area);

        TimeManagementTabArea {
            clock,
            technique_list,
            technique_settings,
            footer,
        }
    }
    fn time_technique_switch(&mut self, notify: bool) -> Result<()> {
        let time_spent = match self.timer_state.get_time_spent() {
            Ok(d) => d,
            Err(e) => bail!("{e}"),
        };
        let (to_spend, notification_body) = match self.timer_engine.switch(time_spent) {
            State::Focus(d) => (d, "Time to focus!"),
            State::Break(d) => (d, "Time for a break!"),
        };
        self.timer_state = TimerState::new(to_spend);
        if notify
            && Notification::new()
                .summary("VaultTasks")
                .body(notification_body)
                .show()
                .is_err()
        {
            error!("Failed to send notification"); // Don't crash for this
        }
        Ok(())
    }
    fn render_footer(area: Rect, frame: &mut Frame) {
        Line::raw("Press hjkl|◄▼▲▶ to change settings | Tab|Shift+Tab to cycle through techniques")
            .centered()
            .render(area, frame.buffer_mut());
    }

    fn update_time_manegement_engine(&mut self) {
        let technique: Box<dyn TimeManagementTechnique> = if let Some(i) =
            self.time_techniques_list_state.selected()
        {
            match TimerTechniquesAvailable::from_repr(i) {
                Some(TimerTechniquesAvailable::Pomodoro) => Box::new(Pomodoro::classic_pomodoro()),
                Some(TimerTechniquesAvailable::FlowTime) => Box::new(FlowTime::new(5).unwrap()),
                None => {
                    error!("No corresponding technique found, yet an update was triggered");
                    return;
                }
            }
        } else {
            error!("No technique selected, yet an update was triggered");
            return;
        };
        self.timer_engine = TimeManagementEngine::new(technique);
    }
}
impl<'a> Component for TimeManagementTab<'a> {
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<()> {
        let _ = tx; // to appease clippy
        Ok(())
    }

    fn register_config_handler(&mut self, config: Config) -> Result<()> {
        self.config = config;
        self.time_techniques_list_state.select(Some(0));
        self.help_menu_wigdet = HelpMenu::new(Mode::TimeManagement, &self.config);
        Ok(())
    }

    fn update(&mut self, tui: Option<&mut Tui>, action: Action) -> Result<Option<Action>> {
        let _ = tui;
        // We always perform this action
        if matches!(action, Action::Tick) && self.timer_state.tick() {
            self.time_technique_switch(true);
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
                Action::PreviousTechnique => {
                    self.time_techniques_list_state.select_previous();
                    self.update_time_manegement_engine();
                }
                Action::NextTechnique => {
                    self.time_techniques_list_state.select_next();

                    self.update_time_manegement_engine();
                }

                Action::NextSegment => self.time_technique_switch(false)?,
                Action::Pause => self.timer_state = self.timer_state.clone().pause(),

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

        // Timer
        TimerWidget {}.render(areas.clock, frame.buffer_mut(), &mut self.timer_state);

        // Techniques List
        self.render_technique_list(areas.technique_list, frame.buffer_mut());

        // Technique Settings
        self.render_technique_settings(areas.technique_settings, frame.buffer_mut());

        // Footer
        Self::render_footer(areas.footer, frame);
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
impl<'a> TimeManagementTab<'a> {
    fn render_technique_list(&mut self, area: Rect, buffer: &mut Buffer) {
        let block = Block::new()
            .title(Line::raw("Techniques").centered())
            .borders(Borders::ALL);

        let highlight_style = *self
            .config
            .styles
            .get(&crate::app::Mode::Home)
            .unwrap()
            .get("highlighted_tab")
            .unwrap();

        let items: Vec<ListItem> = TimerTechniquesAvailable::iter()
            .map(|item| ListItem::from(item.to_string()))
            .collect();

        let list = List::new(items)
            .block(block)
            .highlight_style(highlight_style);

        StatefulWidget::render(list, area, buffer, &mut self.time_techniques_list_state);
    }
    fn render_technique_settings(&mut self, area: Rect, buffer: &mut Buffer) {
        Block::bordered().title("Settings").render(area, buffer);
    }
}
