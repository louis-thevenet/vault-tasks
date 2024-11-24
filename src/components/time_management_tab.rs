use std::time::Duration;

use color_eyre::eyre::bail;
use color_eyre::Result;
use crossterm::event::Event;
use layout::Flex;
use notify_rust::Notification;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Cell, List, ListItem, ListState, Row, Table, TableState};
use strum::IntoEnumIterator;
use tokio::sync::mpsc::UnboundedSender;
use tracing::{debug, error};
use tui_input::backend::crossterm::EventHandler;
use tui_input::Input;
use vault_tasks_time_management::flow_time::FlowTime;
use vault_tasks_time_management::pomodoro::Pomodoro;
use vault_tasks_time_management::time_management_technique::TimeManagementTechnique;
use vault_tasks_time_management::{State, TimeManagementEngine};

use super::Component;

use crate::app::Mode;
use crate::config::{MethodSettingsValue, MethodsAvailable};
use crate::tui::Tui;
use crate::widgets::help_menu::HelpMenu;
use crate::widgets::input_bar::InputBar;
use crate::widgets::timer::{TimerState, TimerWidget};
use crate::{action::Action, config::Config};

/// Struct that helps with drawing the component
struct TimeManagementTabArea {
    timer: Rect,
    methods_list: Rect,
    method_settings: Rect,
    footer: Rect,
}

#[derive(Default)]
pub struct TimeManagementTab<'a> {
    // command_tx: Option<UnboundedSender<Action>>,
    config: Config,
    is_focused: bool,
    // Timer
    timer_state: TimerState,
    tm_engine: TimeManagementEngine,
    methods_list_state: ListState,
    method_settings_list_state: TableState,
    edit_setting_bar: InputBar<'a>,
    // Whether the help panel is open or not
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
        let [_header, clock, methods_area, footer, _tab_footera] = vertical.areas(area);

        let [methods_list, methods_settings] = Layout::horizontal([
            Constraint::Length(
                u16::try_from(
                    MethodsAvailable::iter()
                        .map(|t| 3 + 1 + t.to_string().len())
                        .max()
                        .unwrap_or_default(),
                )
                .unwrap_or_default(),
            ),
            Constraint::Min(0),
        ])
        .areas(methods_area);

        TimeManagementTabArea {
            timer: clock,
            methods_list,
            method_settings: methods_settings,
            footer,
        }
    }

    /// Skips to the next segment using the `TimeManagementEngine`.
    fn time_management_method_switch(&mut self, notify: bool) -> Result<()> {
        let time_spent = match self.timer_state.get_time_spent() {
            Ok(d) => d,
            Err(e) => bail!("{e}"),
        };
        let (to_spend, notification_body) = match self.tm_engine.switch(time_spent) {
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
        Line::raw(
            "Next Segment: <space> | Pause: p | Edit setting: e (navigate: j|k | ▼|▲) | Cycle methods: tab|Shift-tab",
        )
        .centered()
        .render(area, frame.buffer_mut());
    }
    /// Retrieve a settings value from its key.
    fn find_settings_value(&self, method: MethodsAvailable, key: &str) -> MethodSettingsValue {
        self.config
            .time_management_methods_settings
            .get(&method)
            .unwrap()
            .iter()
            .find(|e| e.name == key)
            .unwrap()
            .value
            .clone()
    }
    fn find_settings_int(&self, method: MethodsAvailable, key: &str) -> u32 {
        match self.find_settings_value(method, key) {
            MethodSettingsValue::Duration(_duration) => 0,
            MethodSettingsValue::Int(n) => n,
        }
    }
    fn find_settings_duration(&self, method: MethodsAvailable, key: &str) -> Duration {
        match self.find_settings_value(method, key) {
            MethodSettingsValue::Duration(duration) => duration,
            MethodSettingsValue::Int(_n) => Duration::ZERO,
        }
    }
    /// Updates `TImeManagementEngine` to the new selected method.
    fn update_time_management_engine(&mut self) {
        let method: Box<dyn TimeManagementTechnique> = if let Some(i) =
            self.methods_list_state.selected()
        {
            match MethodsAvailable::from_repr(i) {
                Some(MethodsAvailable::Pomodoro) => Box::new(Pomodoro::new(
                    self.find_settings_duration(MethodsAvailable::Pomodoro, "Focus Time"),
                    self.find_settings_int(MethodsAvailable::Pomodoro, "Long Break Interval")
                        as usize,
                    self.find_settings_duration(MethodsAvailable::Pomodoro, "Short Break Time"),
                    self.find_settings_duration(MethodsAvailable::Pomodoro, "Long Break Time"),
                )),
                Some(MethodsAvailable::FlowTime) => Box::new(
                    FlowTime::new(
                        self.find_settings_int(MethodsAvailable::FlowTime, "Break Factor"),
                    )
                    .unwrap(),
                ),
                None => {
                    error!("No corresponding time management method found, yet an update was triggered");
                    return;
                }
            }
        } else {
            error!("No time management method selected, yet an update was triggered");
            return;
        };
        self.tm_engine = TimeManagementEngine::new(method);
        self.timer_state = TimerState::default();
    }
}
impl<'a> Component for TimeManagementTab<'a> {
    fn blocking_mode(&self) -> bool {
        self.is_focused && (self.show_help || self.edit_setting_bar.is_focused)
    }
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<()> {
        let _ = tx; // to appease clippy
        Ok(())
    }

    fn escape_blocking_mode(&self) -> Vec<Action> {
        vec![Action::Enter, Action::Escape]
    }

    fn register_config_handler(&mut self, config: Config) -> Result<()> {
        self.config = config;
        self.methods_list_state.select(Some(0));
        self.help_menu_wigdet = HelpMenu::new(Mode::TimeManagement, &self.config);
        if self.config.time_management_methods_settings.is_empty() {
            error!("Time management settings are empty");
        } else {
            self.method_settings_list_state.select_column(Some(1)); // Select value column
            self.method_settings_list_state.select(Some(0)); // Select first line
        }
        Ok(())
    }

    #[allow(clippy::too_many_lines)]
    fn update(&mut self, tui: Option<&mut Tui>, action: Action) -> Result<Option<Action>> {
        let _ = tui;
        // We always perform this action
        if matches!(action, Action::Tick) && self.timer_state.tick() {
            self.time_management_method_switch(true)?;
        }

        if !self.is_focused {
            match action {
                Action::Focus(Mode::TimeManagement) => self.is_focused = true,
                Action::Focus(mode) if !(mode == Mode::TimeManagement) => self.is_focused = false,
                _ => (),
            }
        } else if self.edit_setting_bar.is_focused {
            match action {
                Action::Enter => {
                    let input = self.edit_setting_bar.input.value();
                    let selected_method = MethodsAvailable::from_repr(
                        self.methods_list_state.selected().unwrap_or_default(),
                    );
                    let Some(settings) = self
                        .config
                        .time_management_methods_settings
                        .get(&selected_method.unwrap())
                    else {
                        bail!("Tried to edit a time management method that doesn't exist")
                    };

                    let Some(old_value) =
                        settings.get(self.method_settings_list_state.selected().unwrap())
                    else {
                        bail!("Tried to edit settings from a time management method that doesn't exist")
                    };
                    debug!("Editing field {}", old_value.name);
                    // Don't accept invalid inputs
                    if let Ok(value) = old_value.update(input) {
                        self.config
                            .time_management_methods_settings
                            .get_mut(&selected_method.unwrap())
                            .unwrap()[self.method_settings_list_state.selected().unwrap()] = value;
                        self.edit_setting_bar.is_focused = false;

                        // Update engine
                        self.update_time_management_engine();
                    }
                }

                Action::Escape => {
                    // Cancel editing
                    self.edit_setting_bar.input.reset();
                    self.edit_setting_bar.is_focused = !self.edit_setting_bar.is_focused;
                }
                Action::Key(key_event) => {
                    self.edit_setting_bar
                        .input
                        .handle_event(&Event::Key(key_event));
                }
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
                Action::Edit => {
                    let selected_method = MethodsAvailable::from_repr(
                        self.methods_list_state.selected().unwrap_or_default(),
                    );
                    let Some(settings) = self
                        .config
                        .time_management_methods_settings
                        .get(&selected_method.unwrap())
                    else {
                        bail!("Tried to edit a time management method that doesn't exist")
                    };

                    let Some(old_value) =
                        settings.get(self.method_settings_list_state.selected().unwrap())
                    else {
                        bail!("Tried to edit settings from a time management method that doesn't exist")
                    };
                    self.edit_setting_bar.input = Input::new(old_value.value.to_string());
                    self.edit_setting_bar.is_focused = true;
                }
                Action::PreviousMethod => {
                    self.methods_list_state.select_previous();
                    self.update_time_management_engine();
                }
                Action::NextMethod => {
                    self.methods_list_state.select_next();
                    self.update_time_management_engine();
                }
                Action::Up => self.method_settings_list_state.select_previous(),
                Action::Down => self.method_settings_list_state.select_next(),
                // Block selection of other columns (should remove from config)
                // Action::Left => self.time_management_settings_state.select_previous_column(),
                // Action::Right => self.time_management_settings_state.select_next_column(),
                Action::NextSegment => self.time_management_method_switch(false)?,
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
        TimerWidget {}.render(areas.timer, frame.buffer_mut(), &mut self.timer_state);

        // Methods List
        self.render_methods_list(areas.methods_list, frame.buffer_mut());

        // Method Settings
        self.render_methods_settings(areas.method_settings, frame.buffer_mut());

        if self.edit_setting_bar.is_focused {
            self.render_edit_bar(frame, area);
        }
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
    fn render_methods_list(&mut self, area: Rect, buffer: &mut Buffer) {
        let block = Block::new()
            .title(Line::raw("Methods").centered())
            .borders(Borders::ALL);

        let highlight_style = *self
            .config
            .styles
            .get(&crate::app::Mode::Home)
            .unwrap()
            .get("highlighted_style")
            .unwrap();

        let items: Vec<ListItem> = MethodsAvailable::iter()
            .map(|item| ListItem::from(item.to_string()))
            .collect();

        let list = List::new(items)
            .block(block)
            .highlight_style(highlight_style);

        StatefulWidget::render(list, area, buffer, &mut self.methods_list_state);
    }
    fn render_edit_bar(&mut self, frame: &mut Frame, area: Rect) {
        let vertical = Layout::vertical([Constraint::Length(3)]).flex(Flex::Center);
        let horizontal = Layout::horizontal([Constraint::Percentage(75)]).flex(Flex::Center);
        let [area] = vertical.areas(area);
        let [area] = horizontal.areas(area);

        let width = area.width.max(3) - 3; // 2 for borders, 1 for cursor
        let scroll = self.edit_setting_bar.input.visual_scroll(width as usize);

        // Make the cursor visible and ask tui-rs to put it at the specified coordinates after rendering
        frame.set_cursor_position((
            // Put cursor past the end of the input text
            area.x.saturating_add(
                ((self.edit_setting_bar.input.visual_cursor())
                    .max(scroll)
                    .saturating_sub(scroll)) as u16,
            ) + 1,
            // Move one line down, from the border to the input line
            area.y + 1,
        ));

        self.edit_setting_bar.block = Some(
            Block::bordered().title("Edit").style(
                *self
                    .config
                    .styles
                    .get(&crate::app::Mode::Home)
                    .unwrap()
                    .get("highlighted_bar_style")
                    .unwrap(),
            ),
        );
        self.edit_setting_bar
            .clone()
            .render(area, frame.buffer_mut());
    }
    fn render_methods_settings(&mut self, area: Rect, buffer: &mut Buffer) {
        let header = ["Name", "Value", "Hint"]
            .into_iter()
            .map(|s| Cell::new(Span::from(s).into_centered_line()))
            .collect::<Row>()
            .height(2);

        let widths = [
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(50),
        ];

        let selected_method =
            MethodsAvailable::from_repr(self.methods_list_state.selected().unwrap_or_default());
        let rows = self
            .config
            .time_management_methods_settings
            .get(&selected_method.unwrap_or_default())
            .unwrap()
            .iter()
            .map(|stg| {
                Row::new([
                    Span::from(stg.name.clone()).into_centered_line(),
                    Span::from(stg.value.to_string()).into_centered_line(),
                    Span::from(stg.hint.clone()).into_centered_line(),
                ])
            });

        let highlight_style = *self
            .config
            .styles
            .get(&crate::app::Mode::Home)
            .unwrap()
            .get("highlighted_style")
            .unwrap();

        StatefulWidget::render(
            Table::new(rows, widths)
                .cell_highlight_style(highlight_style)
                .header(header)
                .block(Block::bordered().title("Settings")),
            area,
            buffer,
            &mut self.method_settings_list_state,
        );
    }
}
