#![allow(dead_code)]
use std::{
    collections::{hash_map::Entry, HashMap},
    env,
    fs::{create_dir_all, File},
    io::Write,
    path::PathBuf,
};

use chrono::NaiveTime;
use std::{fmt::Display, time::Duration};
use strum::{EnumIter, FromRepr};

use crate::core::TasksConfig;
use crate::widgets::timer::TimerWidget;
use crate::{action::Action, app::Mode, cli::Cli};
use color_eyre::{eyre::bail, Result};
use config::ConfigError;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use derive_deref::{Deref, DerefMut};
use directories::ProjectDirs;
use lazy_static::lazy_static;
use ratatui::style::{Color, Modifier, Style};
use serde::{de::Deserializer, Deserialize};
use tracing::{debug, info};

const CONFIG: &str = include_str!("../.config/config.toml");

#[derive(Clone, Debug, Deserialize, Default)]
pub struct AppConfig {
    #[serde(default)]
    pub config_dir: PathBuf,
    #[serde(default)]
    pub data_dir: PathBuf,
    #[serde(default)]
    pub show_fps: bool,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub config: AppConfig,
    #[serde(default)]
    pub keybindings: KeyBindings,
    #[serde(default)]
    pub styles: Styles,
    #[serde(default)]
    pub tasks_config: TasksConfig,
    #[serde(default)]
    pub time_management_methods_settings: HashMap<MethodsAvailable, Vec<MethodSettingsEntry>>,
}

lazy_static! {
    pub static ref PROJECT_NAME: String = env!("CARGO_CRATE_NAME").to_uppercase();
    pub static ref DATA_FOLDER: Option<PathBuf> =
        env::var(format!("{}_DATA", PROJECT_NAME.clone()))
            .ok()
            .map(PathBuf::from);
    pub static ref CONFIG_FOLDER: Option<PathBuf> =
        env::var(format!("{}_CONFIG", PROJECT_NAME.clone()))
            .ok()
            .map(PathBuf::from);
}

impl Default for Config {
    fn default() -> Self {
        let mut config: Self = toml::from_str(CONFIG).unwrap();
        if cfg!(test) {
            config.tasks_config.vault_path = PathBuf::from("./test-vault");
        }
        config
    }
}
impl Config {
    pub fn new(args: &Cli) -> Result<Self, config::ConfigError> {
        let default_config: Self = Self::default();

        let data_dir = get_data_dir();
        let config_path = args.config_path.clone().unwrap_or_else(get_config_dir);

        // A config file was provided
        let builder = if config_path.is_file() {
            config::Config::builder()
                .set_default("data_dir", data_dir.to_str().unwrap())?
                .add_source(config::File::from(config_path))
        } else {
            let mut builder = config::Config::builder()
                .set_default("data_dir", data_dir.to_str().unwrap())?
                .set_default("config_dir", config_path.to_str().unwrap())?;

            let config_files = [
                ("config.json5", config::FileFormat::Json5),
                ("config.json", config::FileFormat::Json),
                ("config.yaml", config::FileFormat::Yaml),
                ("config.toml", config::FileFormat::Toml),
                ("config.ini", config::FileFormat::Ini),
            ];
            let mut found_config = false;
            for (file, format) in &config_files {
                let source = config::File::from(config_path.join(file))
                    .format(*format)
                    .required(false);
                builder = builder.add_source(source);
                if config_path.join(file).exists() {
                    found_config = true;
                }
            }
            if !found_config && !cfg!(test) {
                info!(
                    "No configuration file found.\nCreate one at {config_path:?} or generate one using `vault-tasks generate-config`");
            }
            builder
        };

        let mut cfg: Self = builder.build()?.try_deserialize()?;

        for (mode, default_bindings) in default_config.keybindings.iter() {
            let user_bindings = cfg.keybindings.entry(*mode).or_default();
            for (key, cmd) in default_bindings {
                user_bindings
                    .entry(key.clone())
                    .or_insert_with(|| cmd.clone());
            }
        }
        for (mode, default_styles) in default_config.styles.iter() {
            let user_styles = cfg.styles.entry(*mode).or_default();
            for (style_key, style) in default_styles {
                user_styles.entry(style_key.clone()).or_insert(*style);
            }
        }
        if let Entry::Vacant(e) = cfg
            .time_management_methods_settings
            .entry(MethodsAvailable::Pomodoro)
        {
            e.insert(
                default_config
                    .time_management_methods_settings
                    .get(&MethodsAvailable::Pomodoro)
                    .unwrap()
                    .clone(),
            );
        }
        if let Entry::Vacant(e) = cfg
            .time_management_methods_settings
            .entry(MethodsAvailable::FlowTime)
        {
            e.insert(
                default_config
                    .time_management_methods_settings
                    .get(&MethodsAvailable::FlowTime)
                    .unwrap()
                    .clone(),
            );
        }

        if let Some(path) = &args.vault_path {
            cfg.tasks_config.vault_path.clone_from(path);
        }

        cfg.config.show_fps = args.show_fps;

        cfg.check_config()?;
        debug!("{cfg:#?}");
        Ok(cfg)
    }
    fn check_config(&mut self) -> Result<(), ConfigError> {
        if self
            .tasks_config
            .vault_path
            .to_str()
            .is_some_and(str::is_empty)
        {
            return Err(ConfigError::Message(
                "No vault path provided (use `--vault-path <PATH>`) and no default path set in config file".to_string(),
            ));
        }
        if !self.tasks_config.vault_path.exists() && !cfg!(test) {
            return Err(ConfigError::Message(format!(
                "Vault path does not exist: {:?}",
                self.tasks_config.vault_path
            )));
        }

        if self.tasks_config.indent_length == 0 {
            self.tasks_config.indent_length = Self::default().tasks_config.indent_length;
        }
        Ok(())
    }

    pub fn generate_config(path: Option<PathBuf>) -> Result<()> {
        let config_dir = path.unwrap_or_else(get_config_dir);
        let dest = config_dir.join("config.toml");
        if create_dir_all(config_dir).is_err() {
            bail!("Failed to create config directory at {dest:?}".to_owned());
        }
        if let Ok(mut file) = File::create(dest.clone()) {
            if file.write_all(CONFIG.as_bytes()).is_err() {
                bail!("Failed to write default config at {dest:?}".to_owned());
            }
        } else {
            bail!("Failed to create default config at {dest:?}".to_owned());
        }
        println!("Configuration has been created at {dest:?}. You can fill the `vault-path` value to set a default vault.");
        Ok(())
    }
}

pub fn get_data_dir() -> PathBuf {
    let directory = DATA_FOLDER.clone().map_or(
        {
            project_directory().map_or_else(
                || PathBuf::from(".").join(".data"),
                |proj_dirs| proj_dirs.data_local_dir().to_path_buf(),
            )
        },
        |s| s,
    );
    directory
}

pub fn get_config_dir() -> PathBuf {
    let directory = CONFIG_FOLDER.clone().map_or_else(
        || {
            project_directory().map_or_else(
                || PathBuf::from(".").join(".config"),
                |proj_dirs| proj_dirs.config_local_dir().to_path_buf(),
            )
        },
        |s| s,
    );
    directory
}

fn project_directory() -> Option<ProjectDirs> {
    ProjectDirs::from("com", "kdheepak", env!("CARGO_PKG_NAME"))
}

#[derive(Clone, Debug, Default, Deref, DerefMut)]
pub struct KeyBindings(pub HashMap<Mode, HashMap<Vec<KeyEvent>, Action>>);

impl<'de> Deserialize<'de> for KeyBindings {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let parsed_map = HashMap::<Mode, HashMap<String, Action>>::deserialize(deserializer)?;

        let keybindings = parsed_map
            .into_iter()
            .map(|(mode, inner_map)| {
                let converted_inner_map = inner_map
                    .into_iter()
                    .map(|(key_str, cmd)| (parse_key_sequence(&key_str).unwrap(), cmd))
                    .collect();
                (mode, converted_inner_map)
            })
            .collect();

        Ok(Self(keybindings))
    }
}

fn parse_key_event(raw: &str) -> Result<KeyEvent, String> {
    let raw_lower = raw.to_ascii_lowercase();
    let (remaining, modifiers) = extract_modifiers(&raw_lower);
    parse_key_code_with_modifiers(remaining, modifiers)
}

fn extract_modifiers(raw: &str) -> (&str, KeyModifiers) {
    let mut modifiers = KeyModifiers::empty();
    let mut current = raw;

    loop {
        match current {
            rest if rest.starts_with("ctrl-") => {
                modifiers.insert(KeyModifiers::CONTROL);
                current = &rest[5..];
            }
            rest if rest.starts_with("alt-") => {
                modifiers.insert(KeyModifiers::ALT);
                current = &rest[4..];
            }
            rest if rest.starts_with("shift-") => {
                modifiers.insert(KeyModifiers::SHIFT);
                current = &rest[6..];
            }
            _ => break, // break out of the loop if no known prefix is detected
        };
    }

    (current, modifiers)
}

fn parse_key_code_with_modifiers(
    raw: &str,
    mut modifiers: KeyModifiers,
) -> Result<KeyEvent, String> {
    let c = match raw {
        "esc" => KeyCode::Esc,
        "enter" => KeyCode::Enter,
        "left" => KeyCode::Left,
        "right" => KeyCode::Right,
        "up" => KeyCode::Up,
        "down" => KeyCode::Down,
        "home" => KeyCode::Home,
        "end" => KeyCode::End,
        "pageup" => KeyCode::PageUp,
        "pagedown" => KeyCode::PageDown,
        "backtab" => {
            modifiers.insert(KeyModifiers::SHIFT);
            KeyCode::BackTab
        }
        "backspace" => KeyCode::Backspace,
        "delete" => KeyCode::Delete,
        "insert" => KeyCode::Insert,
        "f1" => KeyCode::F(1),
        "f2" => KeyCode::F(2),
        "f3" => KeyCode::F(3),
        "f4" => KeyCode::F(4),
        "f5" => KeyCode::F(5),
        "f6" => KeyCode::F(6),
        "f7" => KeyCode::F(7),
        "f8" => KeyCode::F(8),
        "f9" => KeyCode::F(9),
        "f10" => KeyCode::F(10),
        "f11" => KeyCode::F(11),
        "f12" => KeyCode::F(12),
        "space" => KeyCode::Char(' '),
        "hyphen" | "minus" => KeyCode::Char('-'),
        "tab" => KeyCode::Tab,
        c if c.len() == 1 => {
            let mut c = c.chars().next().unwrap();
            if modifiers.contains(KeyModifiers::SHIFT) {
                c = c.to_ascii_uppercase();
            }
            KeyCode::Char(c)
        }
        _ => return Err(format!("Unable to parse {raw}")),
    };
    Ok(KeyEvent::new(c, modifiers))
}
pub fn key_event_to_string(key_event: &KeyEvent) -> String {
    let char;
    let key_code = match key_event.code {
        KeyCode::Backspace => "backspace",
        KeyCode::Enter => "enter",
        KeyCode::Left => "left",
        KeyCode::Right => "right",
        KeyCode::Up => "up",
        KeyCode::Down => "down",
        KeyCode::Home => "home",
        KeyCode::End => "end",
        KeyCode::PageUp => "pageup",
        KeyCode::PageDown => "pagedown",
        KeyCode::Tab => "tab",
        KeyCode::BackTab => "backtab",
        KeyCode::Delete => "delete",
        KeyCode::Insert => "insert",
        KeyCode::F(c) => {
            char = format!("f({c})");
            &char
        }
        KeyCode::Char(' ') => "space",
        KeyCode::Char(c) => {
            char = c.to_string();
            &char
        }
        KeyCode::Esc => "esc",
        KeyCode::Null => "",
        KeyCode::CapsLock => "",
        KeyCode::Menu => "",
        KeyCode::ScrollLock => "",
        KeyCode::Media(_) => "",
        KeyCode::NumLock => "",
        KeyCode::PrintScreen => "",
        KeyCode::Pause => "",
        KeyCode::KeypadBegin => "",
        KeyCode::Modifier(_) => "",
    };

    let mut modifiers = Vec::with_capacity(3);

    if key_event.modifiers.intersects(KeyModifiers::CONTROL) {
        modifiers.push("ctrl");
    }

    if key_event.modifiers.intersects(KeyModifiers::SHIFT) {
        modifiers.push("shift");
    }

    if key_event.modifiers.intersects(KeyModifiers::ALT) {
        modifiers.push("alt");
    }

    let mut key = modifiers.join("-");

    if !key.is_empty() {
        key.push('-');
    }
    key.push_str(key_code);

    key
}

pub fn parse_key_sequence(raw: &str) -> Result<Vec<KeyEvent>, String> {
    if raw.chars().filter(|c| *c == '>').count() != raw.chars().filter(|c| *c == '<').count() {
        return Err(format!("Unable to parse `{raw}`"));
    }
    let raw = if raw.contains("><") {
        raw
    } else {
        let raw = raw.strip_prefix('<').unwrap_or(raw);
        let raw = raw.strip_prefix('>').unwrap_or(raw);
        raw
    };

    raw.split("><")
        .map(|seq| {
            seq.strip_prefix('<')
                .map_or_else(|| seq.strip_suffix('>').map_or(seq, |s| s), |s| s)
        })
        .map(parse_key_event)
        .collect()
}

#[derive(Clone, Debug, Default, Deref, DerefMut)]
pub struct Styles(pub HashMap<Mode, HashMap<String, Style>>);

impl<'de> Deserialize<'de> for Styles {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let parsed_map = HashMap::<Mode, HashMap<String, String>>::deserialize(deserializer)?;

        let styles = parsed_map
            .into_iter()
            .map(|(mode, inner_map)| {
                let converted_inner_map = inner_map
                    .into_iter()
                    .map(|(str, style)| (str, parse_style(&style)))
                    .collect();
                (mode, converted_inner_map)
            })
            .collect();

        Ok(Self(styles))
    }
}

pub fn parse_style(line: &str) -> Style {
    let (foreground, background) =
        line.split_at(line.to_lowercase().find("on ").unwrap_or(line.len()));
    let foreground = process_color_string(foreground);
    let background = process_color_string(&background.replace("on ", ""));

    let mut style = Style::default();
    if let Some(fg) = parse_color(&foreground.0) {
        style = style.fg(fg);
    }
    if let Some(bg) = parse_color(&background.0) {
        style = style.bg(bg);
    }
    style = style.add_modifier(foreground.1 | background.1);
    style
}

fn process_color_string(color_str: &str) -> (String, Modifier) {
    let color = color_str
        .replace("grey", "gray")
        .replace("bright ", "")
        .replace("bold ", "")
        .replace("underline ", "")
        .replace("inverse ", "");

    let mut modifiers = Modifier::empty();
    if color_str.contains("underline") {
        modifiers |= Modifier::UNDERLINED;
    }
    if color_str.contains("bold") {
        modifiers |= Modifier::BOLD;
    }
    if color_str.contains("inverse") {
        modifiers |= Modifier::REVERSED;
    }

    (color, modifiers)
}

fn parse_color(s: &str) -> Option<Color> {
    let s = s.trim_start();
    let s = s.trim_end();
    if s.contains("bright color") {
        let s = s.trim_start_matches("bright ");
        let c = s
            .trim_start_matches("color")
            .parse::<u8>()
            .unwrap_or_default();
        Some(Color::Indexed(c.wrapping_shl(8)))
    } else if s.contains("color") {
        let c = s
            .trim_start_matches("color")
            .parse::<u8>()
            .unwrap_or_default();
        Some(Color::Indexed(c))
    } else if s.contains("gray") {
        let c = 232
            + s.trim_start_matches("gray")
                .parse::<u8>()
                .unwrap_or_default();
        Some(Color::Indexed(c))
    } else if s
        .split_whitespace()
        .collect::<Vec<&str>>()
        .first()
        .is_some_and(|w| w.eq_ignore_ascii_case("rgb"))
    {
        let s = s.split_whitespace().collect::<Vec<&str>>();
        let red = s[1].parse::<u8>().unwrap_or_default();
        let green = s[2].parse::<u8>().unwrap_or_default();
        let blue = s[3].parse::<u8>().unwrap_or_default();
        Some(Color::Rgb(red, green, blue))
    } else if s == "bold black" {
        Some(Color::Indexed(8))
    } else if s == "bold red" {
        Some(Color::Indexed(9))
    } else if s == "bold green" {
        Some(Color::Indexed(10))
    } else if s == "bold yellow" {
        Some(Color::Indexed(11))
    } else if s == "bold blue" {
        Some(Color::Indexed(12))
    } else if s == "bold magenta" {
        Some(Color::Indexed(13))
    } else if s == "bold cyan" {
        Some(Color::Indexed(14))
    } else if s == "bold white" {
        Some(Color::Indexed(15))
    } else if s == "black" {
        Some(Color::Indexed(0))
    } else if s == "red" {
        Some(Color::Indexed(1))
    } else if s == "green" {
        Some(Color::Indexed(2))
    } else if s == "yellow" {
        Some(Color::Indexed(3))
    } else if s == "blue" {
        Some(Color::Indexed(4))
    } else if s == "magenta" {
        Some(Color::Indexed(5))
    } else if s == "cyan" {
        Some(Color::Indexed(6))
    } else if s == "white" {
        Some(Color::Indexed(7))
    } else {
        None
    }
}

#[derive(
    Default,
    Debug,
    Clone,
    Copy,
    FromRepr,
    EnumIter,
    strum_macros::Display,
    PartialEq,
    Eq,
    Hash,
    Deserialize,
)]
pub enum MethodsAvailable {
    #[default]
    #[strum(to_string = "Pomodoro")]
    Pomodoro,
    #[strum(to_string = "Flowtime")]
    FlowTime,
}

#[derive(Debug, Clone, Deserialize)]
/// Represents every value a method setting can be.
pub enum MethodSettingsValue {
    Bool(bool),
    Duration(Duration),
    Int(u32),
}
impl Display for MethodSettingsValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                MethodSettingsValue::Duration(duration) => TimerWidget::format_time_delta(
                    chrono::Duration::from_std(*duration).unwrap_or_default(),
                ),
                MethodSettingsValue::Int(n) => n.to_string(),
                MethodSettingsValue::Bool(b) => b.to_string(),
            }
        )
    }
}

/// Represents an entry in the setting table of a method.
#[derive(Deserialize, Clone, Debug)]
pub struct MethodSettingsEntry {
    /// Name of the setting
    pub name: String,
    /// Setting value
    pub value: MethodSettingsValue,
    /// An hint on the setting
    pub hint: String,
}
impl MethodSettingsEntry {
    /// Parses an input string to a `MethodSettingValue`
    pub fn update(&self, input: &str) -> Result<Self> {
        debug!("New value input: {input}");
        let value = match self.value {
            MethodSettingsValue::Duration(_) => MethodSettingsValue::Duration(
                match NaiveTime::parse_from_str(input, "%H:%M:%S") {
                    Ok(t) => Ok(t),
                    Err(_) => NaiveTime::parse_from_str(&format!("0:{input}"), "%H:%M:%S"),
                }?
                .signed_duration_since(NaiveTime::default())
                .to_std()?,
            ),
            MethodSettingsValue::Int(_) => MethodSettingsValue::Int(input.parse::<u32>()?),
            MethodSettingsValue::Bool(_) => {
                MethodSettingsValue::Bool(input.to_lowercase() == "true")
            }
        };
        Ok(Self {
            name: self.name.clone(),
            value,
            hint: self.hint.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn test_parse_style_default() {
        let style = parse_style("");
        assert_eq!(style, Style::default());
    }

    #[test]
    fn test_parse_style_foreground() {
        let style = parse_style("red");
        assert_eq!(style.fg, Some(Color::Indexed(1)));
    }

    #[test]
    fn test_parse_style_background() {
        let style = parse_style("on blue");
        assert_eq!(style.bg, Some(Color::Indexed(4)));
    }

    #[test]
    fn test_parse_style_modifiers() {
        let style = parse_style("underline red on blue");
        assert_eq!(style.fg, Some(Color::Indexed(1)));
        assert_eq!(style.bg, Some(Color::Indexed(4)));
    }

    #[test]
    fn test_process_color_string() {
        let (color, modifiers) = process_color_string("underline bold inverse gray");
        assert_eq!(color, "gray");
        assert!(modifiers.contains(Modifier::UNDERLINED));
        assert!(modifiers.contains(Modifier::BOLD));
        assert!(modifiers.contains(Modifier::REVERSED));
    }

    #[test]
    fn test_parse_color_rgb() {
        let color = parse_color("rgb 255 000 128");
        assert_eq!(color, Some(Color::Rgb(255, 0, 128)));
    }

    #[test]
    fn test_parse_color_unknown() {
        let color = parse_color("unknown");
        assert_eq!(color, None);
    }

    #[test]
    fn test_config() {
        let c = Config::default();
        assert_eq!(
            c.keybindings
                .get(&Mode::Home)
                .unwrap()
                .get(&parse_key_sequence("<q>").unwrap_or_default())
                .unwrap(),
            &Action::Quit
        );
    }

    #[test]
    fn test_simple_keys() {
        assert_eq!(
            parse_key_event("a").unwrap(),
            KeyEvent::new(KeyCode::Char('a'), KeyModifiers::empty())
        );

        assert_eq!(
            parse_key_event("enter").unwrap(),
            KeyEvent::new(KeyCode::Enter, KeyModifiers::empty())
        );

        assert_eq!(
            parse_key_event("esc").unwrap(),
            KeyEvent::new(KeyCode::Esc, KeyModifiers::empty())
        );
    }

    #[test]
    fn test_with_modifiers() {
        assert_eq!(
            parse_key_event("ctrl-a").unwrap(),
            KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL)
        );

        assert_eq!(
            parse_key_event("alt-enter").unwrap(),
            KeyEvent::new(KeyCode::Enter, KeyModifiers::ALT)
        );

        assert_eq!(
            parse_key_event("shift-esc").unwrap(),
            KeyEvent::new(KeyCode::Esc, KeyModifiers::SHIFT)
        );
    }

    #[test]
    fn test_multiple_modifiers() {
        assert_eq!(
            parse_key_event("ctrl-alt-a").unwrap(),
            KeyEvent::new(
                KeyCode::Char('a'),
                KeyModifiers::CONTROL | KeyModifiers::ALT
            )
        );

        assert_eq!(
            parse_key_event("ctrl-shift-enter").unwrap(),
            KeyEvent::new(KeyCode::Enter, KeyModifiers::CONTROL | KeyModifiers::SHIFT)
        );
    }

    #[test]
    fn test_reverse_multiple_modifiers() {
        assert_eq!(
            key_event_to_string(&KeyEvent::new(
                KeyCode::Char('a'),
                KeyModifiers::CONTROL | KeyModifiers::ALT
            )),
            "ctrl-alt-a".to_string()
        );
    }

    #[test]
    fn test_invalid_keys() {
        assert!(parse_key_event("invalid-key").is_err());
        assert!(parse_key_event("ctrl-invalid-key").is_err());
    }

    #[test]
    fn test_case_insensitivity() {
        assert_eq!(
            parse_key_event("CTRL-a").unwrap(),
            KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL)
        );

        assert_eq!(
            parse_key_event("AlT-eNtEr").unwrap(),
            KeyEvent::new(KeyCode::Enter, KeyModifiers::ALT)
        );
    }
}
