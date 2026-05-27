#[derive(Default, Clone, Debug, Eq, PartialEq)]
pub struct TaskListState {
    offset: u16,
    viewport_height: u16,
    content_height: u16,
}

impl TaskListState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn offset(&self) -> u16 {
        self.offset
    }

    pub fn scroll_to_top(&mut self) {
        self.offset = 0;
    }

    pub fn scroll_up(&mut self) {
        self.offset = self.offset.saturating_sub(1);
    }

    pub fn scroll_down(&mut self) {
        self.offset = self.offset.saturating_add(1).min(self.max_offset());
    }

    pub fn scroll_page_up(&mut self) {
        let step = self.viewport_height.max(1);
        self.offset = self.offset.saturating_sub(step);
    }

    pub fn scroll_page_down(&mut self) {
        let step = self.viewport_height.max(1);
        self.offset = self.offset.saturating_add(step).min(self.max_offset());
    }

    pub fn scroll_left(&mut self) {}

    pub fn scroll_right(&mut self) {}

    fn max_offset(&self) -> u16 {
        self.content_height.saturating_sub(self.viewport_height)
    }

    pub fn update_bounds(&mut self, content_height: u16, viewport_height: u16) {
        self.content_height = content_height;
        self.viewport_height = viewport_height;
        self.offset = self.offset.min(self.max_offset());
    }
}
