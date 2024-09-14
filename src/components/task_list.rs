use color_eyre::Result;
use ratatui::{
    layout::Rect,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use super::Component;

#[derive(Default)]
pub struct TaskList {
    // command_tx: Option<UnboundedSender<Action>>,
    // config: Config,
    // task_mgr: TaskManager,
}
impl TaskList {}
impl Component for TaskList {
    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        // let items = self
        //     .task_mgr
        //     .tasks
        //     .iter()
        //     .map(|file_entry| FileEntryWidget::new(file_entry.clone(), self.config.clone()))
        //     .collect::<Vec<FileEntryWidget>>();
        // let count = items.len();
        // let builder = ListBuilder::new(move |context| {
        //     let item = items[context.index].clone();
        //     let main_axis_size = item.get_height();
        //     (item, main_axis_size)
        // });

        // let list = ListView::new(builder, count);
        // frame.render_stateful_widget(list, area, &mut ListState::default());

        let surrounding_block = Block::default().borders(Borders::ALL).title("Center Menu");
        frame.render_widget(
            Paragraph::new("Here goes the content of the selected item").block(surrounding_block),
            area,
        );
        Ok(())
    }
}
