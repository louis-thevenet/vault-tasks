use color_eyre::eyre::bail;
use color_eyre::Result;
use crossterm::event::Event;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};
use tokio::sync::mpsc::UnboundedSender;
use tracing::{debug, error, info};

use tui_input::backend::crossterm::EventHandler;
use tui_widget_list::{ListBuilder, ListState, ListView};
use tui_widgets::scrollview::ScrollViewState;

use super::Component;

use crate::task_core::filter::parse_search_input;
use crate::task_core::vault_data::VaultData;
use crate::task_core::{TaskManager, WARNING_EMOJI};
use crate::tui::Tui;
use crate::widgets::search_bar::SearchBar;
use crate::widgets::task_list::TaskList;
use crate::{action::Action, config::Config};

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
    search_bar_widget: SearchBar<'a>,
    task_list_widget_state: ScrollViewState,
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
        self.select_previous_left_entry();
        Ok(())
    }
    fn select_previous_left_entry(&mut self) {
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
    }
    /// Updates left and center entries.
    fn update_entries(&mut self) -> Result<()> {
        debug!("Updating entries");

        if self.current_path.is_empty() {
            // Vault root
            self.entries_left_view = vec![];
        } else {
            self.entries_left_view = match self
                .task_mgr
                .get_explorer_entries(&self.current_path[0..self.current_path.len() - 1])
            {
                Ok(res) => res,
                Err(e) => vec![(String::from(WARNING_EMOJI), (e.to_string()))],
            };
        }
        self.entries_center_view = match self.task_mgr.get_explorer_entries(&self.current_path) {
            Ok(res) => res,
            Err(e) => {
                self.leave_selected_entry()?;
                vec![(String::from(WARNING_EMOJI), e.to_string())]
            }
        };
        self.update_preview();
        Ok(())
    }

    fn get_preview_path(&self) -> Result<Vec<String>> {
        let mut path_to_preview = self.current_path.clone();
        if self.entries_center_view.is_empty() {
            bail!("Center view is empty for {:?}", self.current_path)
        }
        match self
            .entries_center_view
            .get(self.state_center_view.selected.unwrap_or_default())
        {
            Some(res) => path_to_preview.push(res.clone().1),
            None => bail!(
                "Index ({:?}) of selected entry out of range {:?}",
                self.state_center_view.selected,
                self.entries_center_view
            ),
        }
        Ok(path_to_preview)
    }

    fn update_preview(&mut self) {
        debug!("Updating preview");
        let Ok(path_to_preview) = self.get_preview_path() else {
            return;
        };

        self.entries_right_view = match self.task_mgr.get_vault_data_from_path(&path_to_preview) {
            Ok(res) => res,
            Err(e) => vec![VaultData::Directory(e.to_string(), vec![])],
        };
        self.task_list_widget_state.scroll_up();
    }
    fn build_list(
        entries_to_display: Vec<String>,
        surrouding_block: Block<'_>,
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

    fn open_current_file(&self, tui_opt: Option<&mut Tui>) -> Result<()> {
        let Some(tui) = tui_opt else {
            bail!("Could not open current entry, Tui was None")
        };
        let mut path = self.config.tasks_config.vault_path.clone();
        for e in &self
            .get_preview_path()
            .unwrap_or_else(|_| self.current_path.clone())
        {
            if path
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
            {
                break;
            }
            path.push(e);
        }

        info!("Opening {:?} in default editor.", path);
        if let Some(tx) = &self.command_tx {
            tui.exit()?;
            edit::edit_file(path)?;
            tui.enter()?;
            tx.send(Action::ClearScreen)?;
        } else {
            bail!("Failed to open current path")
        }
        if let Some(tx) = self.command_tx.clone() {
            tx.send(Action::ReloadVault)?;
        }
        Ok(())
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
}

impl<'a> Component for ExplorerTab<'a> {
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<()> {
        self.command_tx = Some(tx);
        Ok(())
    }

    fn register_config_handler(&mut self, config: Config) -> Result<()> {
        self.task_mgr = TaskManager::load_from_config(&config)?;
        self.config = config;
        self.search_bar_widget.input = self.search_bar_widget.input.clone().with_value(
            self.config
                .tasks_config
                .explorer_default_search_string
                .clone(),
        );
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

    fn update(&mut self, tui: Option<&mut Tui>, action: Action) -> Result<Option<Action>> {
        if self.is_focused {
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
                    self.update_entries()?;
                    if self.state_left_view.selected.unwrap_or_default()
                        >= self.entries_left_view.len()
                    {
                        self.state_left_view.select(None);
                    } else {
                        self.select_previous_left_entry();
                    }
                    if self.state_center_view.selected.unwrap_or_default()
                        >= self.entries_center_view.len()
                    {
                        self.state_center_view.select(Some(0));
                    }
                    self.update_preview();
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
                Action::Cancel | Action::Left | Action::Escape => self.leave_selected_entry()?,
                Action::Open => self.open_current_file(tui)?,
                Action::ReloadVault => {
                    self.task_mgr.reload(&self.config)?;
                    self.update_entries()?;
                }
                Action::ViewUp => self.task_list_widget_state.scroll_up(),
                Action::ViewDown => self.task_list_widget_state.scroll_down(),
                Action::ViewPageUp => self.task_list_widget_state.scroll_page_up(),
                Action::ViewPageDown => self.task_list_widget_state.scroll_page_down(),
                Action::ViewRight => self.task_list_widget_state.scroll_right(),
                Action::ViewLeft => self.task_list_widget_state.scroll_left(),
                Action::Help => todo!(),
                _ => (),
            }
        } else {
            match action {
                Action::FocusExplorer => {
                    self.is_focused = true;
                }
                Action::ReloadVault => {
                    self.task_mgr.reload(&self.config)?;
                    self.update_entries()?;
                }
                _ => (),
            }
        }

        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
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
        if self.search_bar_widget.is_focused {
            let width = areas.search.width.max(3) - 3; // 2 for borders, 1 for cursor
            let scroll = self.search_bar_widget.input.visual_scroll(width as usize);

            // Make the cursor visible and ask tui-rs to put it at the specified coordinates after rendering
            frame.set_cursor_position((
                // Put cursor past the end of the input text
                areas.search.x.saturating_add(
                    ((self.search_bar_widget.input.visual_cursor()).max(scroll) - scroll) as u16,
                ) + 1,
                // Move one line down, from the border to the input line
                areas.search.y + 1,
            ));
        }

        self.search_bar_widget.block = Some(Block::bordered().title("Search").style(
            if self.search_bar_widget.is_focused {
                *self
                    .config
                    .styles
                    .get(&crate::app::Mode::Explorer)
                    .unwrap()
                    .get("highlighted_searchbar")
                    .unwrap()
            } else {
                Style::new()
            },
        ));
        self.search_bar_widget
            .render(areas.search, frame.buffer_mut());

        // Current path
        frame.render_widget(self.path_to_paragraph(), areas.path);

        let highlighted_style = *self
            .config
            .styles
            .get(&crate::app::Mode::Explorer)
            .unwrap()
            .get("highlighted_entry")
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
        // If we have tasks, then render a TaskList widget
        match self.entries_right_view.first() {
            Some(VaultData::Task(_) | VaultData::Header(_, _, _)) => {
                TaskList::new(&self.config, &self.entries_right_view, false)
                    .header_style(
                        *self
                            .config
                            .styles
                            .get(&crate::app::Mode::Explorer)
                            .unwrap()
                            .get("preview_headers")
                            .unwrap(),
                    )
                    .render(
                        areas.preview,
                        frame.buffer_mut(),
                        &mut self.task_list_widget_state,
                    );
            }
            // Else render a ListView widget
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
                highlighted_style,
            )
            .render(areas.preview, frame.buffer_mut(), &mut ListState::default()),
            None => (),
        }

        Ok(())
    }
}
