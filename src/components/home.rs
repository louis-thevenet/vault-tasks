use color_eyre::Result;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List};
use tokio::sync::mpsc::UnboundedSender;

use super::Component;
use crate::task_core::TaskManager;
use crate::{action::Action, config::Config};

#[derive(Default)]
pub struct Home {
    command_tx: Option<UnboundedSender<Action>>,
    // config: Config,
    task_mgr: TaskManager,
    selected_header_path: Vec<String>,
    selected_entry: usize,
}

impl Home {
    pub fn new() -> Self {
        Default::default()
    }
}

impl Component for Home {
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<()> {
        self.command_tx = Some(tx);
        Ok(())
    }

    fn register_config_handler(&mut self, config: Config) -> Result<()> {
        // self.config = config;
        self.task_mgr = TaskManager::load_from_config(&config);
        Ok(())
    }

    // fn update(&mut self, action: Action) -> Result<Option<Action>> {
    //     match action {
    //         Action::Tick => {
    //             // add any logic here that should run on every tick
    //         }
    //         Action::Render => {
    //             // add any logic here that should run on every render
    //         }
    //         _ => {}
    //     }
    //     Ok(None)
    // }

    fn draw(&mut self, frame: &mut Frame, _area: Rect) -> Result<()> {
        self.selected_entry = (self.selected_entry + 1) % 5;

        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![Constraint::Percentage(20), Constraint::Percentage(80)])
            .split(frame.area());

        // Lateral Menu
        let surrounding_lateral_block =
            Block::default().borders(Borders::ALL).title("Lateral Menu");
        let (mut item_prefixes, items) =
            match self.task_mgr.get_entries(self.selected_header_path.clone()) {
                Ok(items) => items,
                Err(e) => (vec![String::new()], vec![e.to_string()]),
            };

        item_prefixes[self.selected_entry] = format!("> {}", item_prefixes[self.selected_entry]); // after we picked it for the preview
        let displayed_entries: Vec<String> = items
            .iter()
            .enumerate()
            .map(|(i, item)| format!("{} {}", item_prefixes[i], item))
            .collect();

        let lateral_entries_list = List::new(displayed_entries).block(surrounding_lateral_block);
        frame.render_widget(lateral_entries_list, layout[0]);

        // Center View
        let surrounding_center_block = Block::default().borders(Borders::ALL).title("Center View");
        let mut path_to_preview = self.selected_header_path.clone();
        path_to_preview.push(items[self.selected_entry].clone());
        let (preview_items_prefixes, preview_items_list) =
            match self.task_mgr.get_entries(path_to_preview) {
                Ok(items) => items,
                Err(e) => (vec![String::new()], vec![e.to_string()]),
            };

        let displayed_preview_entries: Vec<String> = preview_items_list
            .iter()
            .enumerate()
            .map(|(i, item)| format!("{} {}", preview_items_prefixes[i], item))
            .collect();

        let preview_entries = List::new(displayed_preview_entries).block(surrounding_center_block);
        frame.render_widget(preview_entries, layout[1]);

        Ok(())
    }
}
