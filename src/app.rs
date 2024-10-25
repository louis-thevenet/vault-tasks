use color_eyre::Result;
use crossterm::event::KeyEvent;
use ratatui::prelude::Rect;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tracing::{debug, error, info};

use crate::{
    action::Action,
    cli::{Cli, Commands},
    components::{
        explorer_tab::ExplorerTab, filter_tab::FilterTab, fps::FpsCounter, home::Home, Component,
    },
    config::Config,
    tui::{Event, Tui},
};

struct InitialState {
    tab: Action,
}

pub struct App {
    config: Config,
    initial_state: InitialState,
    tick_rate: f64,
    frame_rate: f64,
    components: Vec<Box<dyn Component>>,
    should_quit: bool,
    should_suspend: bool,
    mode: Mode,
    last_tick_key_events: Vec<KeyEvent>,
    action_tx: mpsc::UnboundedSender<Action>,
    action_rx: mpsc::UnboundedReceiver<Action>,
}

#[derive(Default, Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Mode {
    #[default]
    Home,
    Explorer,
    Filter,
}

impl App {
    pub fn new(args: &Cli) -> Result<Self> {
        let config = Config::new(args)?;
        let initial_state = Self::get_initial_state(args);
        let (action_tx, action_rx) = mpsc::unbounded_channel();
        Ok(Self {
            tick_rate: args.tick_rate,
            frame_rate: args.frame_rate,
            components: vec![
                Box::new(Home::new()),
                Box::<FpsCounter>::default(),
                Box::new(ExplorerTab::new()),
                Box::new(FilterTab::new()),
            ],
            should_quit: false,
            should_suspend: false,
            config,
            mode: Mode::Home,
            last_tick_key_events: Vec::new(),
            action_tx,
            action_rx,
            initial_state,
        })
    }
    fn get_initial_state(args: &Cli) -> InitialState {
        let tab = match args.command {
            Some(Commands::Filter) => Action::FocusFilter,
            Some(Commands::Explorer | Commands::GenerateConfig { path: _ }) | None => {
                Action::FocusExplorer
            }
            _ => {
                error!("Unhandled command: {:?}", args.command);
                Action::FocusExplorer
            }
        };
        InitialState { tab }
    }
    pub async fn run(&mut self) -> Result<()> {
        let mut tui = Tui::new()?
            // .mouse(true) // uncomment this line to enable mouse support
            .tick_rate(self.tick_rate)
            .frame_rate(self.frame_rate);
        tui.enter()?;

        for component in &mut self.components {
            component.register_action_handler(self.action_tx.clone())?;
        }
        for component in &mut self.components {
            component.register_config_handler(self.config.clone())?;
        }
        for component in &mut self.components {
            component.init(tui.size()?)?;
        }

        let action_tx = self.action_tx.clone();

        action_tx.send(self.initial_state.tab.clone())?;

        loop {
            self.handle_events(&mut tui).await?;
            self.handle_actions(&mut tui)?;
            if self.should_suspend {
                tui.suspend()?;
                action_tx.send(Action::Resume)?;
                action_tx.send(Action::ClearScreen)?;
                // tui.mouse(true);
                tui.enter()?;
            } else if self.should_quit {
                tui.stop()?;
                break;
            }
        }
        tui.exit()?;
        Ok(())
    }

    async fn handle_events(&mut self, tui: &mut Tui) -> Result<()> {
        let Some(event) = tui.next_event().await else {
            return Ok(());
        };
        let action_tx = self.action_tx.clone();
        match event {
            Event::Quit => action_tx.send(Action::Quit)?,
            Event::Tick => action_tx.send(Action::Tick)?,
            Event::Render => action_tx.send(Action::Render)?,
            Event::Resize(x, y) => action_tx.send(Action::Resize(x, y))?,
            Event::Key(key) => self.handle_key_event(key)?,
            _ => {}
        }
        for component in &mut self.components {
            if let Some(action) = component.handle_events(Some(event.clone()))? {
                action_tx.send(action)?;
            }
        }
        Ok(())
    }

    fn handle_key_event(&mut self, key: KeyEvent) -> Result<()> {
        let action_tx = self.action_tx.clone();
        let Some(keymap) = self.config.keybindings.get(&self.mode) else {
            return Ok(());
        };
        if let Some(action) = keymap.get(&vec![key]) {
            info!("Got action: {action:?}");
            // Look for components in editing mode
            for component in &self.components {
                // Is it in editing mode and is the action in the escape list ?
                if component.blocking_mode() && !component.escape_blocking_mode().contains(action) {
                    info!("Action was sent as raw key");
                    action_tx.send(Action::Key(key))?;
                    return Ok(());
                }
            }
            action_tx.send(action.clone())?;
        } else {
            // If there is a component in editing mode, send the raw key
            if self.components.iter().any(|c| c.blocking_mode()) {
                info!("Got raw key: {key:?}");
                action_tx.send(Action::Key(key))?;
                return Ok(());
            }

            // If the key was not handled as a single key action,
            // then consider it for multi-key combinations.
            self.last_tick_key_events.push(key);

            // Check for multi-key combinations
            if let Some(action) = keymap.get(&self.last_tick_key_events) {
                info!("Got action: {action:?}");
                action_tx.send(action.clone())?;
            }
        }
        Ok(())
    }

    fn handle_actions(&mut self, tui: &mut Tui) -> Result<()> {
        while let Ok(action) = self.action_rx.try_recv() {
            if action != Action::Tick && action != Action::Render {
                debug!("Action: {action:?}");
            }
            match action {
                Action::FocusExplorer => self.mode = Mode::Explorer,
                Action::FocusFilter => self.mode = Mode::Filter,
                Action::Tick => {
                    self.last_tick_key_events.drain(..);
                }
                Action::Quit => self.should_quit = true,
                Action::Suspend => self.should_suspend = true,
                Action::Resume => self.should_suspend = false,
                Action::ClearScreen => tui.terminal.clear()?,
                Action::Resize(w, h) => self.handle_resize(tui, w, h)?,
                Action::Render => self.render(tui)?,
                _ => {}
            }
            for component in &mut self.components {
                if let Some(action) = component.update(Some(tui), action.clone())? {
                    self.action_tx.send(action)?;
                };
            }
        }
        Ok(())
    }

    fn handle_resize(&mut self, tui: &mut Tui, w: u16, h: u16) -> Result<()> {
        tui.resize(Rect::new(0, 0, w, h))?;
        self.render(tui)?;
        Ok(())
    }

    fn render(&mut self, tui: &mut Tui) -> Result<()> {
        tui.draw(|frame| {
            for component in &mut self.components {
                if let Err(err) = component.draw(frame, frame.area()) {
                    let _ = self
                        .action_tx
                        .send(Action::Error(format!("Failed to draw: {err:?}")));
                }
            }
        })?;
        Ok(())
    }
}
