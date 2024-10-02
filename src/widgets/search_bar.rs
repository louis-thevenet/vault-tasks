use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    widgets::{Block, Paragraph, Widget, WidgetRef},
};
use tui_input::Input;

#[derive(Default)]
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
