use std::time::Instant;

use color_eyre::Result;
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Style, Stylize},
    text::Span,
    widgets::Paragraph,
};

use super::Component;

use crate::{action::Action, tui::Tui};

#[derive(Debug, Clone, PartialEq)]
pub struct FpsCounter {
    enabled: bool,
    last_tick_update: Instant,
    tick_count: u32,
    ticks_per_second: f64,

    last_frame_update: Instant,
    frame_count: u32,
    frames_per_second: f64,
}

impl Default for FpsCounter {
    fn default() -> Self {
        Self::new()
    }
}

impl FpsCounter {
    pub fn new() -> Self {
        Self {
            last_tick_update: Instant::now(),
            tick_count: 0,
            ticks_per_second: 0.0,
            last_frame_update: Instant::now(),
            frame_count: 0,
            frames_per_second: 0.0,
            enabled: false,
        }
    }

    fn app_tick(&mut self) {
        self.tick_count += 1;
        let now = Instant::now();
        let elapsed = (now - self.last_tick_update).as_secs_f64();
        if elapsed >= 1.0 {
            self.ticks_per_second = f64::from(self.tick_count) / elapsed;
            self.last_tick_update = now;
            self.tick_count = 0;
        }
    }

    fn render_tick(&mut self) {
        self.frame_count += 1;
        let now = Instant::now();
        let elapsed = (now - self.last_frame_update).as_secs_f64();
        if elapsed >= 1.0 {
            self.frames_per_second = f64::from(self.frame_count) / elapsed;
            self.last_frame_update = now;
            self.frame_count = 0;
        }
    }
}

impl Component for FpsCounter {
    fn update(&mut self, _tui: Option<&mut Tui>, action: Action) -> Result<Option<Action>> {
        if self.enabled {
            match action {
                Action::Tick => self.app_tick(),
                Action::Render => self.render_tick(),
                _ => {}
            }
        }
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        if self.enabled {
            let [top, _] =
                Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).areas(area);
            let message = format!(
                "{:.2} ticks/sec, {:.2} FPS",
                self.ticks_per_second, self.frames_per_second
            );
            let span = Span::styled(message, Style::new().dim());
            let paragraph = Paragraph::new(span).right_aligned();
            frame.render_widget(paragraph, top);
        }
        Ok(())
    }

    fn register_config_handler(&mut self, config: crate::config::Config) -> Result<()> {
        self.enabled = config.config.show_fps;
        Ok(())
    }
}
