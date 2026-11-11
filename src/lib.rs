//! User-defined ⌘K commands, each typing its text into the current tab.
// wit-bindgen's flattened `notify` import has 14 params.
#![allow(clippy::too_many_arguments)]

wit_bindgen::generate!({ path: "wit/haunt-plugin.wit", world: "plugin" });

use std::sync::Mutex;

use haunt::plugin::log::log;
use haunt::plugin::types::LogLevel;
use haunt::plugin::{palette, typing};
use serde::Deserialize;

/// The palette holds 64 entries per plugin.
const MAX_COMMANDS: usize = 64;
const MAX_NAME: usize = 64;
/// Haunt types at most this much at once.
const MAX_TEXT: usize = 4096;

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
struct Command {
    id: String,
    name: String,
    text: String,
    #[serde(default)]
    enter: bool,
}

static COMMANDS: Mutex<Vec<Command>> = Mutex::new(Vec::new());

/// The valid commands from the settings JSON, with a reason for each one dropped.
fn parse(settings: &str) -> (Vec<Command>, Vec<String>) {
    #[derive(Deserialize)]
    struct Settings {
        #[serde(default)]
        commands: Vec<serde_json::Value>,
    }
    let raw = match serde_json::from_str::<Settings>(settings) {
        Ok(s) => s.commands,
        Err(e) => return (Vec::new(), vec![format!("settings: {e}")]),
    };
    let (mut out, mut dropped) = (Vec::<Command>::new(), Vec::new());
    for (i, v) in raw.into_iter().enumerate() {
        let c: Command = match serde_json::from_value(v) {
            Ok(c) => c,
            Err(e) => {
                dropped.push(format!("command {}: {e}", i + 1));
                continue;
            }
        };
        let why = if !valid_id(&c.id) {
            Some("id must be 1 to 32 lowercase letters, digits and '-'")
        } else if out.iter().any(|o| o.id == c.id) {
            Some("another command has this id")
        } else if c.name.trim().is_empty() || c.name.chars().count() > MAX_NAME {
            Some("name must be 1 to 64 characters")
        } else if c.name.chars().any(hidden) {
            Some("name has a control or invisible character")
        } else if c.text.is_empty() {
            Some("text is empty")
        } else if c.text.len() > MAX_TEXT {
            Some("text is over 4096 bytes")
        } else if c.text.chars().any(|ch| ch != '\t' && hidden(ch)) {
            Some("text has a newline, escape or other control or invisible character (a tab is fine)")
        } else if out.len() == MAX_COMMANDS {
            Some("more than 64 commands")
        } else {
            None
        };
        match why {
            Some(why) => dropped.push(format!("command {} ({}): {why}", i + 1, c.id)),
            None => out.push(c),
        }
    }
    (out, dropped)
}

/// Roughly what Haunt refuses to type or show: controls, format characters such as bidi overrides and
/// zero-width ones, and any whitespace but the plain space.
fn hidden(c: char) -> bool {
    c != ' ' && (c.is_control() || c.is_whitespace() || matches!(c as u32, 0xAD | 0x200B..=0x200F | 0x202A..=0x202E | 0x2060..=0x206F | 0xFEFF))
}

fn valid_id(id: &str) -> bool {
    (1..=32).contains(&id.len()) && id.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

fn apply(settings: &str) {
    let (commands, dropped) = parse(settings);
    for why in dropped {
        log(LogLevel::Warn, &why);
    }
    for c in &commands {
        palette::command(&c.id, &c.name, None);
    }
    *COMMANDS.lock().unwrap() = commands;
}

struct Plugin;

impl Guest for Plugin {
    fn init(settings: String) {
        apply(&settings);
    }

    fn on_settings_changed(settings: String) {
        apply(&settings);
    }

    fn on_event(_event: AgentEvent) -> Option<Decoration> {
        None
    }

    fn palette_items(_mode: String, _query: String) -> Vec<PaletteItem> {
        Vec::new()
    }

    fn on_action(action_id: String, _payload: String) {
        let Some(c) = COMMANDS.lock().unwrap().iter().find(|c| c.id == action_id).cloned() else {
            return log(LogLevel::Warn, &format!("{action_id}: no such command (removed from the settings?)"));
        };
        if let Err(e) = typing::type_text(&c.text, c.enter) {
            log(LogLevel::Warn, &format!("{}: {e}", c.id));
        }
    }

    fn row_menu(_surface: String) -> Vec<MenuItem> {
        Vec::new()
    }

    fn on_row_moved(_surface: String, _group: String, _before: Option<String>) {}

    fn on_theme_changed() {}
}

export!(Plugin);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_commands_are_kept_in_order_with_enter_off_by_default() {
        let (c, dropped) = parse(
            r#"{"commands":[{"id":"deploy","name":"Deploy staging","text":"make deploy ENV=staging","enter":true},
                {"id":"st","name":"Git status","text":"git status"}]}"#,
        );
        assert!(dropped.is_empty(), "{dropped:?}");
        assert_eq!(c.iter().map(|c| (c.id.as_str(), c.enter)).collect::<Vec<_>>(), [("deploy", true), ("st", false)]);
    }

    #[test]
    fn bad_commands_are_dropped_with_a_reason() {
        let (c, dropped) = parse(
            r#"{"commands":[{"id":"ok","name":"Ok","text":"ls"},
                {"id":"ok","name":"Again","text":"pwd"},
                {"id":"Bad Id","name":"x","text":"ls"},
                {"id":"noname","name":"  ","text":"ls"},
                {"id":"notext","name":"x","text":""},
                {"id":"extra","name":"x","text":"ls","shell":"/bin/sh"},
                {"name":"no id","text":"ls"},
                {"id":"nl","name":"x","text":"ls\nrm -rf ~"},
                {"id":"esc","name":"x","text":"\u001b[201~rm"},
                {"id":"bidi","name":"Dep\u202eloy","text":"ls"},
                {"id":"tab","name":"Tab ok","text":"ls\t-la"}]}"#,
        );
        assert_eq!(c.iter().map(|c| c.id.as_str()).collect::<Vec<_>>(), ["ok", "tab"]);
        assert_eq!(dropped.len(), 9, "{dropped:?}");
        assert!(dropped[0].contains("another command has this id"));
        assert!(dropped[4].contains("unknown field"), "{}", dropped[4]);
        let long = format!(r#"{{"commands":[{{"id":"long","name":"x","text":"{}"}}]}}"#, "a".repeat(MAX_TEXT + 1));
        assert!(parse(&long).0.is_empty());
    }

    #[test]
    fn no_more_than_the_palette_holds() {
        let many: Vec<String> = (0..70).map(|i| format!(r#"{{"id":"c{i}","name":"C{i}","text":"echo {i}"}}"#)).collect();
        let (c, dropped) = parse(&format!(r#"{{"commands":[{}]}}"#, many.join(",")));
        assert_eq!((c.len(), dropped.len()), (MAX_COMMANDS, 6));
    }

    #[test]
    fn missing_or_broken_settings_mean_no_commands() {
        assert_eq!(parse("{}"), (Vec::new(), Vec::new()));
        let (c, dropped) = parse("not json");
        assert!(c.is_empty() && dropped.len() == 1);
    }

    #[test]
    fn manifest_asks_for_typing_only_and_defaults_to_no_commands() {
        let m = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/haunt-plugin.toml")).unwrap();
        let t: toml::Table = toml::from_str(&m).unwrap();
        assert_eq!(t["capabilities"].as_table().unwrap().keys().collect::<Vec<_>>(), ["pty_type"]);
        assert_eq!(t["settings"]["commands"]["default"].as_array().unwrap().len(), 0);
    }
}
