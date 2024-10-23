use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    widgets::{Block, Paragraph, Widget, WidgetRef},
};
use tui_input::Input;

#[derive(Default, Clone)]
pub struct SearchBar<'a> {
    pub input: Input,
    pub is_focused: bool,
    pub block: Option<Block<'a>>,
}

impl<'a> WidgetRef for SearchBar<'a> {
    fn render_ref(&self, area: Rect, buf: &mut Buffer) {
        let width = area.width.max(3) - 3; // 2 for borders, 1 for cursor
        let scroll = self.input.visual_scroll(width as usize);
        let res = Paragraph::new(self.input.value())
            .style(Style::reset())
            .scroll((0, scroll as u16));

        if let Some(block) = &self.block {
            res.block(block.clone())
        } else {
            res
        }
        .render(area, buf);
    }
}
#[cfg(test)]
mod tests {
    use insta::assert_snapshot;
    use ratatui::{
        backend::TestBackend,
        layout::{Constraint, Layout},
        widgets::Block,
        Terminal,
    };
    use tui_input::Input;

    use crate::widgets::search_bar::SearchBar;

    #[test]
    fn test_render_search_bar() {
        let bar = SearchBar {
            input: Input::new("input".to_owned()),
            is_focused: true,
            block: Some(Block::bordered().title_top("test")),
        };
        let mut terminal = Terminal::new(TestBackend::new(80, 20)).unwrap();
        terminal
            .draw(|frame| frame.render_widget(&bar, frame.area()))
            .unwrap();
        assert_snapshot!(terminal.backend());
    }
    #[test]
    fn test_render_search_bar_line() {
        let input = Input::new("initial".to_owned());
        let bar = SearchBar {
            input,
            is_focused: true,
            block: Some(Block::bordered().title_top("test")),
        };
        let mut terminal = Terminal::new(TestBackend::new(80, 20)).unwrap();
        terminal
            .draw(|frame| {
                let [_, inner, _] = Layout::vertical([
                    Constraint::Percentage(40),
                    Constraint::Min(1),
                    Constraint::Percentage(40),
                ])
                .areas(frame.area());
                let [_, inner, _] = Layout::horizontal([
                    Constraint::Percentage(20),
                    Constraint::Min(10),
                    Constraint::Percentage(20),
                ])
                .areas(inner);

                frame.render_widget(&bar, inner);
            })
            .unwrap();
        assert_snapshot!(terminal.backend());
    }
}
