use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Paragraph, Widget, WidgetRef},
};
use tui_input::Input;

#[derive(Default)]
pub struct SearchBar {
    pub input: Input,
    pub is_focused: bool,
}

impl WidgetRef for SearchBar {
    fn render_ref(&self, area: Rect, buf: &mut Buffer) {
        let width = area.width.max(3) - 3; // 2 for borders, 1 for cursor
        let scroll = self.input.visual_scroll(width as usize);
        Paragraph::new(self.input.value())
            .style(Style::reset())
            .block(Block::bordered().style(Style::new().fg(if self.is_focused {
                Color::Rgb(255, 153, 0)
            } else {
                Color::default()
            })))
            .scroll((0, scroll as u16))
            .render(area, buf);
    }
}
