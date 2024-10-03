use color_eyre::eyre::bail;
use color_eyre::Result;
use crossterm::event::Event;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};
use tokio::sync::mpsc::UnboundedSender;
use tracing::{debug, error};

use tui_input::backend::crossterm::EventHandler;
use tui_widget_list::{ListBuilder, ListState, ListView};

use super::Component;

use crate::task_core::filter::parse_search_input;
use crate::task_core::vault_data::VaultData;
use crate::task_core::TaskManager;
use crate::widgets::search_bar::SearchBar;
use crate::widgets::task_list::TaskList;
use crate::{action::Action, config::Config};

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
    search_bar_widget: SearchBar<'a>,
}

impl<'a> ExplorerTab<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    fn enter_selected_entry(&mut self) -> Result<()> {
        // Update path with selected entry
        self.current_path.push(
            self.entries_center_view[self.state_center_view.selected.unwrap_or_default()]
                .1
                .clone(),
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
                    .into_iter()
                    .enumerate()
                    .find(|(_, entry)| &entry.1 == new_previous_entry)
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
            self.entries_left_view = vec![];
        } else {
            self.entries_left_view = self
                .task_mgr
                .get_explorer_entries(&self.current_path[0..self.current_path.len() - 1])?;
        }
        self.entries_center_view =
            if let Ok(res) = self.task_mgr.get_explorer_entries(&self.current_path) {
                res
            } else {
                self.leave_selected_entry()?;
                return Ok(());
            };
        self.update_preview();
        Ok(())
    }

    fn get_preview_path(&self) -> Result<Vec<String>> {
        let mut path_to_preview = self.current_path.clone();
        if self.entries_center_view.is_empty() {
            bail!("Error: No selected entry")
        }
        path_to_preview.push(
            self.entries_center_view[self.state_center_view.selected.unwrap_or_default()]
                .1
                .clone(),
        );
        Ok(path_to_preview)
    }

    fn update_preview(&mut self) {
        debug!("Updating preview");
        let Ok(path_to_preview) = self.get_preview_path() else {
            return;
        };

        self.entries_right_view = self
            .task_mgr
            .get_vault_data_from_path(&path_to_preview)
            .unwrap_or_default();
    }
    fn build_list(
        entries_to_display: Vec<String>,
        surrouding_block: Block<'_>,
    ) -> ListView<'_, Paragraph<'_>> {
        let item_count = entries_to_display.len();

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

        ListView::new(builder, item_count).block(surrouding_block)
    }
    fn apply_prefixes(entries: &[(String, String)]) -> Vec<String> {
        entries
            .iter()
            .map(|item| format!("{} {}", item.0, item.1))
            .collect()
    }
    pub fn render_footer(area: Rect, frame: &mut Frame) {
        Line::raw("Press hjkl|◄▼▲▶ to move")
            .centered()
            .render(area, frame.buffer_mut());
    }
}

impl<'a> Component for ExplorerTab<'a> {
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<()> {
        self.command_tx = Some(tx);
        Ok(())
    }

    fn register_config_handler(&mut self, config: Config) -> Result<()> {
        self.task_mgr = TaskManager::load_from_config(&config)?;
        self.config = config;
        self.task_mgr.current_filter = Some(parse_search_input(
            self.search_bar_widget.input.value(),
            &self.config,
        ));
        self.update_entries()?;
        self.state_center_view.selected = Some(0);
        Ok(())
    }

    fn escape_editing_mode(&self) -> Vec<Action> {
        vec![Action::Enter, Action::Escape]
    }
    fn editing_mode(&self) -> bool {
        self.is_focused && self.search_bar_widget.is_focused
    }

    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        if self.is_focused {
            debug!("action in explorer:{action}");
            match action {
                Action::FocusFilter => self.is_focused = false,

                Action::Enter | Action::Escape if self.search_bar_widget.is_focused => {
                    self.search_bar_widget.is_focused = !self.search_bar_widget.is_focused;
                }
                Action::Search => {
                    self.search_bar_widget.is_focused = !self.search_bar_widget.is_focused;
                }
                Action::Key(key_event) if self.search_bar_widget.is_focused => {
                    self.search_bar_widget
                        .input
                        .handle_event(&Event::Key(key_event));

                    // Update search input in TaskManager
                    self.task_mgr.current_filter = Some(parse_search_input(
                        self.search_bar_widget.input.value(),
                        &self.config,
                    ));
                    self.current_path = vec![];
                    self.state_center_view.select(Some(0));
                    self.state_left_view.select(None);
                    self.update_entries()?;
                }

                Action::Up => {
                    self.state_center_view.previous();
                    self.update_preview();
                }
                Action::Down => {
                    self.state_center_view.next();
                    self.update_preview();
                }
                Action::Right | Action::Enter => self.enter_selected_entry()?,
                Action::Left | Action::Escape => self.leave_selected_entry()?,
                Action::Cancel => {
                    self.leave_selected_entry()?;
                }
                Action::Help => todo!(),
                _ => (),
            }
        } else if action == Action::FocusExplorer {
            self.is_focused = true;
        }
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, _area: Rect) -> Result<()> {
        if !self.is_focused {
            return Ok(());
        }
        if self.entries_center_view.is_empty() {
            error!("Center view is empty"); // is it always an error ?
            self.update_entries()?;
            self.state_center_view.selected = Some(0);
        }

        let vertical = Layout::vertical([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
            Constraint::Length(1),
        ]);
        let [_header_area, inner_area, footer_area, _tab_footer_area] =
            vertical.areas(frame.area());

        Self::render_footer(footer_area, frame);

        let [search_path_area, explorer_area] =
            Layout::vertical(vec![Constraint::Length(3), Constraint::Percentage(100)])
                .areas(inner_area);

        let [path_area, search_area] =
            Layout::horizontal(vec![Constraint::Percentage(70), Constraint::Percentage(30)])
                .areas(search_path_area);

        // Main Layout
        let [previous_area, current_area, preview_area] = Layout::horizontal(vec![
            Constraint::Percentage(10),
            Constraint::Percentage(30),
            Constraint::Percentage(60),
        ])
        .areas(explorer_area);

        // Search Bar
        if self.search_bar_widget.is_focused {
            let width = search_area.width.max(3) - 3; // 2 for borders, 1 for cursor
            let scroll = self.search_bar_widget.input.visual_scroll(width as usize);

            // Make the cursor visible and ask tui-rs to put it at the specified coordinates after rendering
            frame.set_cursor_position((
                // Put cursor past the end of the input text
                search_area.x.saturating_add(
                    ((self.search_bar_widget.input.visual_cursor()).max(scroll) - scroll) as u16,
                ) + 1,
                // Move one line down, from the border to the input line
                search_area.y + 1,
            ));
        }

        self.search_bar_widget.block = Some(Block::bordered().title("Search").style(
            Style::new().fg(if self.search_bar_widget.is_focused {
                Color::Rgb(255, 153, 0)
            } else {
                Color::default()
            }),
        ));
        self.search_bar_widget
            .render(search_area, frame.buffer_mut());

        // Current path
        frame.render_widget(
            Paragraph::new(format!("\n./{}", self.current_path.join("/"))),
            path_area,
        );

        // Left Block
        let left_entries_list = Self::build_list(
            Self::apply_prefixes(&self.entries_left_view),
            Block::default().borders(Borders::RIGHT),
        );
        let state = &mut self.state_left_view;
        left_entries_list.render(previous_area, frame.buffer_mut(), state);

        // Center Block
        let lateral_entries_list = Self::build_list(
            Self::apply_prefixes(&self.entries_center_view),
            Block::default().borders(Borders::RIGHT),
        );
        let state = &mut self.state_center_view;
        lateral_entries_list.render(current_area, frame.buffer_mut(), state);

        // Right Block

        match self.entries_right_view.first() {
            Some(VaultData::Task(_) | VaultData::Header(_, _, _)) => {
                TaskList::new(&self.config, &self.entries_right_view)
                    .render(preview_area, frame.buffer_mut());
            }
            Some(VaultData::Directory(_, _)) => Self::build_list(
                Self::apply_prefixes(
                    &self
                        .task_mgr
                        .get_explorer_entries(
                            &self
                                .get_preview_path()
                                .unwrap_or_else(|_| self.current_path.clone()),
                        )
                        .unwrap_or_default(),
                ),
                Block::new(),
            )
            .render(preview_area, frame.buffer_mut(), &mut ListState::default()),
            None => (),
        }

        Ok(())
    }
}
