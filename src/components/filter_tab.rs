use color_eyre::Result;
use crossterm::event::Event;
use ratatui::widgets::List;
use ratatui::{prelude::*, widgets::Block};
use tokio::sync::mpsc::UnboundedSender;
use tui_widgets::scrollview::ScrollViewState;

use super::Component;

use crate::task_core::filter::{filter_to_vec, parse_search_input};
use crate::task_core::task::Task;
use crate::task_core::vault_data::VaultData;
use crate::task_core::TaskManager;
use crate::tui::Tui;
use crate::widgets::search_bar::SearchBar;
use crate::widgets::task_list::TaskList;
use crate::{action::Action, config::Config};
use tui_input::backend::crossterm::EventHandler;

#[derive(Default)]
pub struct FilterTab<'a> {
    command_tx: Option<UnboundedSender<Action>>,
    config: Config,
    is_focused: bool,
    matching_entries: Vec<Task>,
    matching_tags: Vec<String>,
    search_bar_widget: SearchBar<'a>,
    task_mgr: TaskManager,
    task_list_widget_state: ScrollViewState,
}

impl<'a> FilterTab<'a> {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn render_footer(&self, area: Rect, frame: &mut Frame) {
        if self.search_bar_widget.is_focused {
            Line::raw("Press Enter to stop searching")
        } else {
            Line::raw("Press Enter to start searching")
        }
        .centered()
        .render(area, frame.buffer_mut());
    }
    fn update_matching_entries(&mut self) {
        let (search, has_state) =
            parse_search_input(self.search_bar_widget.input.value(), &self.config);

        // Filter tasks
        self.matching_entries = filter_to_vec(&self.task_mgr.tasks, &search, has_state);

        // Reset ScrollViewState
        self.task_list_widget_state.scroll_to_top();

        // Filter tags
        self.matching_tags = if search.tags.is_none() {
            self.task_mgr.tags.iter().cloned().collect::<Vec<String>>()
        } else {
            let search_tags = search.tags.unwrap_or_default();
            self.task_mgr
                .tags
                .iter()
                .filter(|t| {
                    search_tags
                        .clone()
                        .iter()
                        .any(|t2| t.to_lowercase().contains(&t2.to_lowercase()))
                })
                .cloned()
                .collect()
        };
        self.matching_tags.sort();
    }
}
impl<'a> Component for FilterTab<'a> {
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<()> {
        self.command_tx = Some(tx);
        Ok(())
    }

    fn register_config_handler(&mut self, config: Config) -> Result<()> {
        self.task_mgr = TaskManager::load_from_config(&config)?;
        self.config = config;
        self.search_bar_widget.is_focused = true; // Start with search bar focused
        self.search_bar_widget.input = self.search_bar_widget.input.clone().with_value(
            self.config
                .tasks_config
                .filter_default_search_string
                .clone(),
        );
        self.update_matching_entries();
        Ok(())
    }

    fn editing_mode(&self) -> bool {
        self.is_focused && self.search_bar_widget.is_focused
    }
    fn escape_editing_mode(&self) -> Vec<Action> {
        vec![Action::Enter, Action::Cancel, Action::Escape]
    }
    fn update(&mut self, _tui: Option<&mut Tui>, action: Action) -> Result<Option<Action>> {
        if self.is_focused {
            match action {
                Action::FocusExplorer => self.is_focused = false,
                Action::FocusFilter => self.is_focused = true,
                Action::Enter | Action::Search | Action::Cancel | Action::Escape => {
                    self.search_bar_widget.is_focused = !self.search_bar_widget.is_focused;
                }
                Action::Key(key) if self.search_bar_widget.is_focused => {
                    self.search_bar_widget.input.handle_event(&Event::Key(key));
                    self.update_matching_entries();
                }
                Action::ReloadVault => {
                    self.task_mgr.reload(&self.config)?;
                    self.update_matching_entries();
                }
                Action::ViewUp => self.task_list_widget_state.scroll_up(),
                Action::ViewDown => self.task_list_widget_state.scroll_down(),
                Action::ViewPageUp => self.task_list_widget_state.scroll_page_up(),
                Action::ViewPageDown => self.task_list_widget_state.scroll_page_down(),
                Action::ViewRight => self.task_list_widget_state.scroll_right(),
                Action::ViewLeft => self.task_list_widget_state.scroll_left(),
                _ => (),
            }
        } else {
            match action {
                Action::ReloadVault => {
                    self.task_mgr.reload(&self.config)?;
                    self.update_matching_entries();
                }
                Action::FocusExplorer => self.is_focused = false,
                Action::FocusFilter => self.is_focused = true,
                _ => (),
            }
        }
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        if !self.is_focused {
            return Ok(());
        }
        let vertical = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(1),
            Constraint::Length(1),
        ]);
        let [_header_area, search_area, content_area, footer_area, _tab_footer_areaa] =
            vertical.areas(area);

        let [tag_area, list_area] =
            Layout::horizontal([Constraint::Length(15), Constraint::Min(0)]).areas(content_area);

        self.render_footer(footer_area, frame);

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

        self.search_bar_widget.block = Some(Block::bordered().style(Style::new().fg(
            if self.search_bar_widget.is_focused {
                Color::Rgb(255, 153, 0)
            } else {
                Color::default()
            },
        )));
        self.search_bar_widget
            .render(search_area, frame.buffer_mut());

        let tag_list = List::new(self.matching_tags.iter().map(std::string::String::as_str))
            .block(Block::bordered().title("Found Tags"));

        let entries_list = TaskList::new(
            &self.config,
            &self
                .matching_entries
                .clone()
                .iter()
                .map(|t| VaultData::Task(t.clone()))
                .collect::<Vec<VaultData>>(),
            true,
        );

        Widget::render(tag_list, tag_area, frame.buffer_mut());

        entries_list.render(
            list_area,
            frame.buffer_mut(),
            &mut self.task_list_widget_state,
        );
        Ok(())
    }
}
