use color_eyre::eyre::eyre;
use color_eyre::Result;
use crossterm::event::Event;
use layout::Flex;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};
use tokio::sync::mpsc::UnboundedSender;
use tracing::{debug, error, info};

use crate::core::task::State;
use tui_input::backend::crossterm::EventHandler;
use tui_input::Input;
use tui_scrollview::ScrollViewState;
use tui_widget_list::{ListBuilder, ListState, ListView};

use super::Component;

use crate::app::Mode;
use crate::core::filter::parse_search_input;
use crate::core::parser::task::parse_task;
use crate::core::vault_data::VaultData;
use crate::core::TaskManager;
use crate::tui::Tui;
use crate::widgets::help_menu::HelpMenu;
use crate::widgets::input_bar::InputBar;
use crate::widgets::task_list::TaskList;
use crate::{action::Action, config::Config};

mod entry_list;
mod utils;

pub const FILE_EMOJI: &str = "📄";
pub const DIRECTORY_EMOJI: &str = "📁";
pub const WARNING_EMOJI: &str = "⚠️";

/// Struct that helps with drawing the component
struct ExplorerArea {
    path: Rect,
    search: Rect,
    previous: Rect,
    current: Rect,
    preview: Rect,
    footer: Rect,
}
#[derive(Default)]
pub struct ExplorerTab<'a> {
    command_tx: Option<UnboundedSender<Action>>,
    config: Config,
    is_focused: bool,
    task_mgr: TaskManager,
    current_path: Vec<String>,
    state_left_view: ListState,
    entries_left_view: Vec<(String, String)>,
    state_center_view: ListState,
    entries_center_view: Vec<(String, String)>,
    entries_right_view: Vec<VaultData>,
    search_bar_widget: InputBar<'a>,
    task_list_widget_state: ScrollViewState,
    show_help: bool,
    help_menu_widget: HelpMenu<'a>,
    edit_task_bar: InputBar<'a>,
}

impl ExplorerTab<'_> {
    pub fn new() -> Self {
        Self::default()
    }

    /// Updates left and center entries.
    pub(super) fn update_entries(&mut self) -> Result<()> {
        debug!("Updating entries");

        if self.current_path.is_empty() {
            // Vault root
            self.entries_left_view = vec![];
        } else {
            self.entries_left_view = match self.task_mgr.get_explorer_entries_without_children(
                &self.current_path[0..self.current_path.len() - 1],
            ) {
                Ok(res) => Self::vault_data_to_entry_list(&res),
                Err(e) => vec![(String::from(WARNING_EMOJI), (e.to_string()))],
            };
        }
        self.entries_center_view = match self
            .task_mgr
            .get_explorer_entries_without_children(&self.current_path)
        {
            Ok(res) => Self::vault_data_to_entry_list(&res),
            Err(_e) => {
                // If no entries are found, go to parent object
                while self
                    .task_mgr
                    .get_explorer_entries_without_children(&self.current_path)
                    .is_err()
                    && !self.current_path.is_empty()
                {
                    self.leave_selected_entry()?;
                }
                Self::vault_data_to_entry_list(
                    &self
                        .task_mgr
                        .get_explorer_entries_without_children(&self.current_path)
                        .unwrap_or_default(),
                )
            }
        };
        if self.state_left_view.selected.unwrap_or_default() >= self.entries_left_view.len() {
            self.state_left_view.select(None);
        } else {
            self.select_previous_left_entry();
        }
        if self.state_center_view.selected.unwrap_or_default() >= self.entries_center_view.len() {
            self.state_center_view.select(Some(0));
        }
        self.update_preview();
        Ok(())
    }

    pub(super) fn update_preview(&mut self) {
        debug!("Updating preview");
        let Ok(path_to_preview) = self.get_preview_path() else {
            self.entries_right_view = vec![];
            return;
        };

        self.entries_right_view = match self.task_mgr.get_vault_data_from_path(&path_to_preview, 1)
        {
            Ok(res) => res,
            Err(e) => vec![VaultData::Directory(e.to_string(), vec![])],
        };
        self.task_list_widget_state.scroll_up();
    }
    pub(super) fn build_list(
        entries_to_display: Vec<String>,
        surrounding_block: Block<'_>,
        highlighted_style: Style,
    ) -> ListView<'_, Paragraph<'_>> {
        let item_count = entries_to_display.len();

        let builder = ListBuilder::new(move |context| {
            let mut item = Paragraph::new(entries_to_display[context.index].clone());
            if context.is_selected {
                item = item.style(highlighted_style);
            };
            let main_axis_size = 1;
            (item, main_axis_size)
        });

        ListView::new(builder, item_count).block(surrounding_block)
    }
    fn path_to_paragraph(&self) -> Paragraph {
        Paragraph::new(
            self.current_path
                .iter()
                .map(|item| {
                    let span = Span::from(item.to_string());
                    if item.contains(".md") {
                        span.bold()
                    } else {
                        span
                    }
                })
                .fold(Line::from("."), |mut acc, x| {
                    acc.push_span(Span::from("/"));
                    acc.push_span(x);
                    acc
                }),
        )
    }

    fn split_frame(area: Rect) -> ExplorerArea {
        let vertical = Layout::vertical([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
            Constraint::Length(1),
        ]);
        let [_header, inner, footer, _tab_footer] = vertical.areas(area);

        let [search_path, explorer] =
            Layout::vertical(vec![Constraint::Length(3), Constraint::Percentage(100)]).areas(inner);

        let [path, search] =
            Layout::horizontal(vec![Constraint::Percentage(70), Constraint::Percentage(30)])
                .areas(search_path);

        // Main Layout
        let [previous, current, preview] = Layout::horizontal(vec![
            Constraint::Percentage(10),
            Constraint::Percentage(30),
            Constraint::Percentage(60),
        ])
        .areas(explorer);
        ExplorerArea {
            path,
            search,
            previous,
            current,
            preview,
            footer,
        }
    }

    pub fn render_footer(area: Rect, frame: &mut Frame) {
        Line::raw("Navigate: <hjkl|◄▼▲▶> | Open in editor: o | Quick edit: e | Filter: s")
            .centered()
            .render(area, frame.buffer_mut());
    }

    fn render_search_bar(&mut self, frame: &mut Frame, area: Rect) {
        // Search Bar
        if self.search_bar_widget.is_focused {
            let width = area.width.max(3) - 3; // 2 for borders, 1 for cursor
            let scroll = self.search_bar_widget.input.visual_scroll(width as usize);

            // Make the cursor visible and ask tui-rs to put it at the specified coordinates after rendering
            frame.set_cursor_position((
                // Put cursor past the end of the input text
                area.x.saturating_add(
                    ((self.search_bar_widget.input.visual_cursor()).max(scroll) - scroll) as u16,
                ) + 1,
                // Move one line down, from the border to the input line
                area.y + 1,
            ));
        }

        self.search_bar_widget.block = Some(Block::bordered().title("Search").style(
            if self.search_bar_widget.is_focused {
                *self
                    .config
                    .styles
                    .get(&crate::app::Mode::Home)
                    .unwrap()
                    .get("highlighted_bar_style")
                    .unwrap()
            } else {
                Style::new()
            },
        ));
        self.search_bar_widget
            .clone()
            .render(area, frame.buffer_mut());
    }
    fn render_preview(&mut self, frame: &mut Frame, area: Rect, highlighted_style: Style) {
        // If we have tasks, then render a TaskList widget
        match self.entries_right_view.first() {
            Some(VaultData::Task(_) | VaultData::Header(_, _, _)) => {
                TaskList::new(&self.config, &self.entries_right_view, area.width, false).render(
                    area,
                    frame.buffer_mut(),
                    &mut self.task_list_widget_state,
                );
            }
            // Else render a ListView widget
            Some(VaultData::Directory(_, _)) => Self::build_list(
                Self::apply_prefixes(&Self::vault_data_to_entry_list(
                    &self
                        .task_mgr
                        .get_explorer_entries_without_children(
                            &self
                                .get_preview_path()
                                .unwrap_or_else(|_| self.current_path.clone()),
                        )
                        .unwrap_or_default(),
                )),
                Block::new(),
                highlighted_style,
            )
            .render(area, frame.buffer_mut(), &mut ListState::default()),
            None => (),
        }
    }
    fn render_edit_bar(&mut self, frame: &mut Frame, area: Rect) {
        let vertical = Layout::vertical([Constraint::Length(3)]).flex(Flex::Center);
        let horizontal = Layout::horizontal([Constraint::Percentage(75)]).flex(Flex::Center);
        let [area] = vertical.areas(area);
        let [area] = horizontal.areas(area);

        let width = area.width.max(3) - 3; // 2 for borders, 1 for cursor
        let scroll = self.edit_task_bar.input.visual_scroll(width as usize);

        // Make the cursor visible and ask tui-rs to put it at the specified coordinates after rendering
        frame.set_cursor_position((
            // Put cursor past the end of the input text
            area.x.saturating_add(
                ((self.edit_task_bar.input.visual_cursor()).max(scroll) - scroll) as u16,
            ) + 1,
            // Move one line down, from the border to the input line
            area.y + 1,
        ));

        self.edit_task_bar.block = Some(
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
        self.edit_task_bar.clone().render(area, frame.buffer_mut());
    }

    fn edit_selected_task_state(&mut self, new_state: State) -> Result<()> {
        if let Some(mut task) = self.get_selected_task() {
            task.state = new_state;
            task.fix_task_attributes(&self.config.tasks_config, &self.get_current_path_to_file())?;
            return Ok(());
        }
        Err(eyre!("No selected task"))
    }

    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
    fn edit_selected_task_completion(&mut self, action: &Action) -> Result<()> {
        if let Some(mut task) = self.get_selected_task() {
            let length = self.config.tasks_config.completion_bar_length as isize;
            let diff = match action {
                Action::IncreaseCompletion => length,
                Action::DecreaseCompletion => -length,
                _ => return Err(eyre!("Invalid state")),
            };
            task.completion = match task.completion {
                Some(c) => {
                    let res = c as isize + diff;
                    if res == 0 {
                        None
                    } else {
                        Some(res.unsigned_abs())
                    }
                }
                None if diff > 0 => Some(diff.unsigned_abs()),
                None => None,
            };
            task.fix_task_attributes(&self.config.tasks_config, &self.get_current_path_to_file())?;
            return Ok(());
        }
        Err(eyre!("No selected task"))
    }
}

impl Component for ExplorerTab<'_> {
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<()> {
        self.command_tx = Some(tx);
        Ok(())
    }

    fn register_config_handler(&mut self, config: Config) -> Result<()> {
        self.task_mgr = TaskManager::load_from_config(&config.tasks_config)?;
        self.config = config;
        self.help_menu_widget = HelpMenu::new(Mode::Explorer, &self.config);
        self.search_bar_widget.input = self.search_bar_widget.input.clone().with_value(
            self.config
                .tasks_config
                .explorer_default_search_string
                .clone(),
        );
        self.task_mgr.current_filter = Some(parse_search_input(
            self.search_bar_widget.input.value(),
            &self.config.tasks_config,
        ));
        self.update_entries()?;
        self.state_center_view.selected = Some(0);

        Ok(())
    }

    fn escape_blocking_mode(&self) -> Vec<Action> {
        vec![Action::Enter, Action::Escape]
    }
    fn blocking_mode(&self) -> bool {
        self.is_focused
            && (self.search_bar_widget.is_focused
                || self.show_help
                || self.edit_task_bar.is_focused)
    }

    #[allow(clippy::too_many_lines)]
    fn update(&mut self, tui: Option<&mut Tui>, action: Action) -> Result<Option<Action>> {
        if !self.is_focused {
            match action {
                Action::Focus(Mode::Explorer) => {
                    self.is_focused = true;
                }
                Action::ReloadVault => {
                    self.task_mgr.reload(&self.config.tasks_config)?;
                    self.update_entries()?;
                }
                _ => (),
            }
            return Ok(None);
        }
        if self.edit_task_bar.is_focused {
            match action {
                Action::Enter => {
                    // We're already sure it exists since we entered the task editing mode
                    if let VaultData::Task(task) = self
                        .task_mgr
                        .get_vault_data_from_path(&self.current_path, 0)
                        .unwrap()[self.state_center_view.selected.unwrap_or_default()]
                    .clone()
                    {
                        // Get input
                        let mut input = self.edit_task_bar.input.value();
                        // Parse it
                        let Ok(mut parsed_task) = parse_task(
                            &mut input,
                            self.get_current_path_to_file()
                                .to_str()
                                .unwrap()
                                .to_string(),
                            &self.config.tasks_config,
                        ) else {
                            // Don't accept invalid input
                            return Ok(None);
                        };
                        // Write changes
                        parsed_task.line_number = task.line_number;
                        parsed_task.fix_task_attributes(
                            &self.config.tasks_config,
                            &self.get_current_path_to_file(),
                        )?;
                        // Quit editing mode
                        self.edit_task_bar.is_focused = !self.edit_task_bar.is_focused;
                        // Reload vault
                        return Ok(Some(Action::ReloadVault));
                    }
                }
                Action::Escape => {
                    // Cancel editing
                    self.edit_task_bar.input.reset();
                    self.edit_task_bar.is_focused = !self.edit_task_bar.is_focused;
                }
                Action::Key(key_event) => {
                    self.edit_task_bar
                        .input
                        .handle_event(&Event::Key(key_event));
                }
                _ => (),
            }
        } else if self.search_bar_widget.is_focused {
            match action {
                Action::Enter | Action::Escape => {
                    self.search_bar_widget.is_focused = !self.search_bar_widget.is_focused;
                }
                Action::Key(key_event) => {
                    self.search_bar_widget
                        .input
                        .handle_event(&Event::Key(key_event));

                    // Update search input in TaskManager
                    self.task_mgr.current_filter = Some(parse_search_input(
                        self.search_bar_widget.input.value(),
                        &self.config.tasks_config,
                    ));
                    self.update_entries()?;
                }
                _ => (),
            }
        } else if self.show_help {
            match action {
                Action::ViewUp | Action::Up => self.help_menu_widget.scroll_up(),
                Action::ViewDown | Action::Down => self.help_menu_widget.scroll_down(),
                Action::Help | Action::Escape | Action::Enter => {
                    self.show_help = !self.show_help;
                }
                _ => (),
            }
        } else {
            match action {
                // Change tab
                Action::Focus(mode) if mode != Mode::Explorer => self.is_focused = false,
                // Search bar
                Action::Search => {
                    self.search_bar_widget.is_focused = !self.search_bar_widget.is_focused;
                }
                Action::IncreaseCompletion | Action::DecreaseCompletion => {
                    if self.edit_selected_task_completion(&action).is_ok() {
                        return Ok(Some(Action::ReloadVault));
                    }
                }
                Action::MarkDone => {
                    if self.edit_selected_task_state(State::Done).is_ok() {
                        return Ok(Some(Action::ReloadVault));
                    }
                }
                Action::MarkCancel => {
                    if self.edit_selected_task_state(State::Canceled).is_ok() {
                        return Ok(Some(Action::ReloadVault));
                    }
                }
                Action::MarkToDo => {
                    if self.edit_selected_task_state(State::ToDo).is_ok() {
                        return Ok(Some(Action::ReloadVault));
                    }
                }
                Action::MarkIncomplete => {
                    if self.edit_selected_task_state(State::Incomplete).is_ok() {
                        return Ok(Some(Action::ReloadVault));
                    }
                }
                Action::Edit => {
                    if let Some(task) = self.get_selected_task() {
                        self.edit_task_bar.input =
                            Input::new(task.get_fixed_attributes(&self.config.tasks_config, 0));
                        self.edit_task_bar.is_focused = !self.edit_task_bar.is_focused;
                    } else {
                        info!("Only tasks can be edited");
                        return Ok(None);
                    }
                }

                // Navigation
                Action::Up => {
                    self.state_center_view.previous();
                    self.update_preview();
                }
                Action::Down => {
                    self.state_center_view.next();
                    self.update_preview();
                }
                Action::Right | Action::Enter => self.enter_selected_entry()?,
                Action::Cancel | Action::Left | Action::Escape => self.leave_selected_entry()?,
                // Preview
                Action::ViewUp => self.task_list_widget_state.scroll_up(),
                Action::ViewDown => self.task_list_widget_state.scroll_down(),
                Action::ViewPageUp => self.task_list_widget_state.scroll_page_up(),
                Action::ViewPageDown => self.task_list_widget_state.scroll_page_down(),
                Action::ViewRight => self.task_list_widget_state.scroll_right(),
                Action::ViewLeft => self.task_list_widget_state.scroll_left(),
                // Commands
                Action::Help => self.show_help = !self.show_help,
                Action::Open => self.open_current_file(tui)?,
                Action::ReloadVault => {
                    self.task_mgr.reload(&self.config.tasks_config)?;
                    self.update_entries()?;
                }
                _ => (),
            }
        }

        Ok(None)
    }
    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        // If not focused, don't draw anything
        if !self.is_focused {
            return Ok(());
        }
        if self.entries_center_view.is_empty() {
            error!("Center view is empty"); // is it always an error ?
            self.update_entries()?;
            self.state_center_view.selected = Some(0);
        }
        let areas = Self::split_frame(area);
        Self::render_footer(areas.footer, frame);

        // Search Bar
        self.render_search_bar(frame, areas.search);

        // Current Path
        frame.render_widget(self.path_to_paragraph(), areas.path);

        let highlighted_style = *self
            .config
            .styles
            .get(&crate::app::Mode::Home)
            .unwrap()
            .get("highlighted_style")
            .unwrap();

        // Left Block
        let left_entries_list = Self::build_list(
            Self::apply_prefixes(&self.entries_left_view),
            Block::default().borders(Borders::RIGHT),
            highlighted_style,
        );
        let state = &mut self.state_left_view;
        left_entries_list.render(areas.previous, frame.buffer_mut(), state);

        // Center Block
        let lateral_entries_list = Self::build_list(
            Self::apply_prefixes(&self.entries_center_view),
            Block::default().borders(Borders::RIGHT),
            highlighted_style,
        );
        let state = &mut self.state_center_view;
        lateral_entries_list.render(areas.current, frame.buffer_mut(), state);

        // Right Block
        self.render_preview(frame, areas.preview, highlighted_style);

        // Help Menu
        if self.show_help {
            self.help_menu_widget.clone().render(
                area,
                frame.buffer_mut(),
                &mut self.help_menu_widget.state,
            );
        }
        if self.edit_task_bar.is_focused {
            self.render_edit_bar(frame, area);
        }

        Ok(())
    }
}
