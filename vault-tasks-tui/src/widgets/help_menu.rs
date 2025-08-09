use std::collections::HashSet;

use crossterm::event::KeyModifiers;
use layout::Flex;
use ratatui::{
    prelude::*,
    widgets::{Block, Cell, Clear, Row, Table},
};
use tracing::debug;
use tui_scrollview::{ScrollView, ScrollViewState};

use crate::{action::Action, app::Mode, config::Config};

#[derive(Default, Clone)]
pub struct HelpMenu<'a> {
    content: Table<'a>,
    content_size: Size,
    pub state: ScrollViewState,
}

impl HelpMenu<'_> {
    fn get_keys_for_action(config: &Config, app_mode: Mode, action: &Action) -> String {
        config
            .config
            .keybindings
            .get(&app_mode)
            .unwrap()
            .iter()
            .filter_map(|(k, v)| {
                if *v == *action {
                    let key = k.first().unwrap();
                    Some(if key.modifiers == KeyModifiers::NONE {
                        format!("<{}>", key.code)
                    } else {
                        format!("<{}-{}>", key.modifiers, key.code)
                    })
                } else {
                    None
                }
            })
            .collect::<Vec<String>>()
            .join(" | ")
    }
    pub fn new(app_mode: Mode, config: &Config) -> Self {
        let mut action_set = HashSet::<Action>::new();
        for kb in config.config.keybindings.get(&app_mode).unwrap().values() {
            action_set.insert(kb.clone());
        }
        let mut action_vec = action_set.iter().collect::<Vec<&Action>>();
        action_vec.sort();

        let header_height = 1;
        let header = ["Action", "Keys"]
            .into_iter()
            .map(Cell::from)
            .collect::<Row>()
            .style(Style::new().bold())
            .height(header_height);

        let rows = action_vec.iter().map(|action| {
            [
                action.to_string(),
                Self::get_keys_for_action(config, app_mode, action),
            ]
            .into_iter()
            .map(Cell::from)
            .collect::<Row>()
        });

        let lengths = action_set.iter().map(|action| {
            (
                action.to_string().len() as u16,
                Self::get_keys_for_action(config, app_mode, action).len() as u16,
            )
        });

        let longest = (
            lengths
                .clone()
                .max_by(|a, b| a.0.cmp(&b.0))
                .unwrap_or_default()
                .0,
            lengths.max_by(|a, b| a.1.cmp(&b.1)).unwrap_or_default().1,
        );

        let block = Block::bordered()
            .title("Help")
            .title_bottom(Line::from("Esc to close").right_aligned());
        let column_spacing = 4;
        let table = Table::new(
            rows,
            [Constraint::Length(longest.0), Constraint::Length(longest.1)],
        )
        .header(header)
        .column_spacing(column_spacing)
        .block(block);

        Self {
            state: ScrollViewState::new(),
            content: table,
            content_size: Size::new(
                longest
                    .0
                    .saturating_add(longest.1)
                    .saturating_add(column_spacing)
                    + 2, // +2 for block
                (action_vec.len() as u16).saturating_add(header_height) + 2, // +2 for block
            ),
        }
    }
    pub fn scroll_down(&mut self) {
        self.state.scroll_down();
    }
    pub fn scroll_up(&mut self) {
        self.state.scroll_up();
    }
}

impl StatefulWidget for HelpMenu<'_> {
    type State = ScrollViewState;
    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State)
    where
        Self: Sized,
    {
        let vertical =
            Layout::vertical([Constraint::Length(self.content_size.height)]).flex(Flex::End);
        let horizontal = Layout::horizontal([self.content_size.width]).flex(Flex::Start);
        let [area] = vertical.areas(area);
        let [area] = horizontal.areas(area);

        let mut scroll_view = ScrollView::new(self.content_size);
        debug!("{}", self.content_size);
        Widget::render(Clear, area, buf);
        scroll_view.render_widget(self.content, scroll_view.area());
        scroll_view.render(area, buf, state);
    }
}
