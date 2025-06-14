use super::Component;
use crate::{action::Action, app::Mode, config::Config, tui::Tui};
use color_eyre::Result;
use ratatui::{prelude::*, widgets::Tabs};
use strum::{Display, EnumCount, EnumIter, FromRepr, IntoEnumIterator};
use tokio::sync::mpsc::UnboundedSender;
use tracing::error;

#[derive(Default)]
pub struct Home {
    command_tx: Option<UnboundedSender<Action>>,
    config: Config,
    selected_tab: SelectedTab,
}

impl Home {
    pub fn new() -> Self {
        Self::default()
    }

    fn send_new_focused_tab_command(&self) {
        if let Some(tx) = &self.command_tx {
            if let Err(e) = tx.send(match self.selected_tab {
                SelectedTab::Explorer => Action::Focus(Mode::Explorer),
                SelectedTab::Filter => Action::Focus(Mode::Filter),
                SelectedTab::TimeManagement => Action::Focus(Mode::TimeManagement),
                SelectedTab::Calendar => Action::Focus(Mode::Calendar),
            }) {
                error!("Could not focus selected tab: {e}");
            }
        }
    }
    pub fn next_tab(&mut self) {
        self.selected_tab = self.selected_tab.next();
        self.send_new_focused_tab_command();
    }

    pub fn previous_tab(&mut self) {
        self.selected_tab = self.selected_tab.previous();
        self.send_new_focused_tab_command();
    }
    fn render_tabs(&self, area: Rect, buf: &mut Buffer) {
        let titles = SelectedTab::iter().map(SelectedTab::title);

        let highlight_style = *self
            .config
            .styles
            .get(&crate::app::Mode::Home)
            .unwrap()
            .get("highlighted_style")
            .unwrap();

        let selected_tab_index = self.selected_tab as usize;
        Tabs::new(titles)
            .select(selected_tab_index)
            .highlight_style(highlight_style)
            .padding("", "")
            .divider(" ")
            .render(area, buf);
    }

    pub fn render_footer(area: Rect, frame: &mut Frame) {
        Line::raw("Change tab: Shift+<hl|◄►> | Quit: q | Help: ?")
            .centered()
            .render(area, frame.buffer_mut());
    }
}
impl Component for Home {
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<()> {
        self.command_tx = Some(tx);
        self.send_new_focused_tab_command();
        Ok(())
    }

    fn register_config_handler(&mut self, config: Config) -> Result<()> {
        self.config = config;
        Ok(())
    }

    fn update(&mut self, _tui: Option<&mut Tui>, action: Action) -> Result<Option<Action>> {
        match action {
            Action::TabRight => self.next_tab(),
            Action::TabLeft => self.previous_tab(),
            Action::Focus(Mode::Explorer) => self.selected_tab = SelectedTab::Explorer,
            Action::Focus(Mode::Filter) => self.selected_tab = SelectedTab::Filter,
            Action::Focus(Mode::TimeManagement) => self.selected_tab = SelectedTab::TimeManagement,
            Action::Focus(Mode::Calendar) => self.selected_tab = SelectedTab::Calendar,
            _ => (),
        }
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        use Constraint::{Length, Min};
        let vertical = Layout::vertical([Length(1), Min(0), Length(1)]);
        let [header_area, _inner_area, footer_area] = vertical.areas(area);

        self.render_tabs(header_area, frame.buffer_mut());
        Self::render_footer(footer_area, frame);
        Ok(())
    }
}

#[derive(Default, Clone, Copy, Display, FromRepr, EnumIter, EnumCount)]
enum SelectedTab {
    #[default]
    #[strum(to_string = "Explorer")]
    Explorer,
    #[strum(to_string = "Filter")]
    Filter,
    #[strum(to_string = "Calendar")]
    Calendar,
    #[strum(to_string = "Time Management")]
    TimeManagement,
}

impl SelectedTab {
    /// Get the previous tab, wrapping around if there is no previous tab.
    fn previous(self) -> Self {
        let current_index: usize = self as usize;
        let previous_index = current_index.wrapping_sub(1) % Self::COUNT;
        Self::from_repr(previous_index).unwrap_or(self)
    }

    /// Get the next tab, wrapping around if there is no next tab.
    fn next(self) -> Self {
        let current_index = self as usize;
        let next_index = current_index.wrapping_add(1) % Self::COUNT;
        Self::from_repr(next_index).unwrap_or(self)
    }
    fn title(self) -> Line<'static> {
        format!("  {self}  ").into()
    }
}

#[cfg(test)]
mod tests {
    use insta::assert_snapshot;
    use ratatui::{Terminal, backend::TestBackend};
    use tokio::sync::mpsc::unbounded_channel;

    use crate::{
        components::{Component, home::Home},
        config::Config,
    };

    #[test]
    fn test_render_home_component() {
        let mut home = Home::new();
        let (tx, _rx) = unbounded_channel();

        home.register_action_handler(tx).unwrap();
        home.register_config_handler(Config::default()).unwrap();

        let mut terminal = Terminal::new(TestBackend::new(80, 20)).unwrap();

        terminal
            .draw(|frame| home.draw(frame, frame.area()).unwrap())
            .unwrap();
        assert_snapshot!(terminal.backend());
    }
}
