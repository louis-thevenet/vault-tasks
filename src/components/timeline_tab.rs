use ratatui::widgets::{Paragraph, Widget};

use super::Component;

pub struct TimelineTab {}

impl TimelineTab {
    pub fn new() -> Self {
        Self {}
    }
}
impl Component for TimelineTab {
    fn draw(
        &mut self,
        frame: &mut ratatui::Frame,
        area: ratatui::prelude::Rect,
    ) -> color_eyre::eyre::Result<()> {
        Paragraph::new("test").render(area, frame.buffer_mut());
        Ok(())
    }
}
