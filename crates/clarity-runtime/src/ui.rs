//! The binding between a window on the screen and its handlers in the
//! backend. The screen sends a window's state with each event; the handler
//! runs on it and the reply carries the new state, the messages to show and
//! the windows to open or close. The screen holds no behavior of its own.

use std::collections::BTreeMap;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::{Error, Result};

/// What a handler set on a part. Anything unset keeps the design.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PartState {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visible: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub caption: Option<String>,
}

/// One open window: its data by name, its parts' properties and the
/// selected row of each table or list.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct WindowState {
    pub window: String,
    pub data: Map<String, Value>,
    #[serde(default)]
    pub parts: BTreeMap<String, PartState>,
    #[serde(default)]
    pub selected: BTreeMap<String, usize>,
}

impl WindowState {
    pub fn new(window: &str, data: Map<String, Value>) -> Self {
        WindowState {
            window: window.to_string(),
            data,
            ..WindowState::default()
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Message {
    pub level: Level,
    pub text: String,
}

/// `open Window with name = value, ...`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OpenRequest {
    pub window: String,
    pub with: Map<String, Value>,
}

/// What the screen does after a handler ran.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Reply {
    pub state: WindowState,
    pub messages: Vec<Message>,
    pub open: Vec<OpenRequest>,
    pub close: bool,
}

/// What a handler can do to the screen.
#[derive(Debug, Default)]
pub struct Ui {
    parts: BTreeMap<String, PartState>,
    selected: BTreeMap<String, usize>,
    messages: Vec<Message>,
    open: Vec<OpenRequest>,
    pub close: bool,
}

impl Ui {
    pub fn from_state(state: &WindowState) -> Ui {
        Ui {
            parts: state.parts.clone(),
            selected: state.selected.clone(),
            ..Ui::default()
        }
    }

    pub fn visible(&self, part: &str) -> bool {
        self.parts.get(part).and_then(|p| p.visible).unwrap_or(true)
    }

    pub fn set_visible(&mut self, part: &str, value: bool) {
        self.parts.entry(part.to_string()).or_default().visible = Some(value);
    }

    pub fn enabled(&self, part: &str) -> bool {
        self.parts.get(part).and_then(|p| p.enabled).unwrap_or(true)
    }

    pub fn set_enabled(&mut self, part: &str, value: bool) {
        self.parts.entry(part.to_string()).or_default().enabled = Some(value);
    }

    /// The caption a handler set, or the one from the design.
    pub fn caption(&self, part: &str, design: &str) -> String {
        self.parts
            .get(part)
            .and_then(|p| p.caption.clone())
            .unwrap_or_else(|| design.to_string())
    }

    pub fn set_caption(&mut self, part: &str, value: String) {
        self.parts.entry(part.to_string()).or_default().caption = Some(value);
    }

    /// The selected record of a table or list showing `rows`.
    pub fn selected<R: Clone>(&self, part: &str, rows: &[R]) -> Result<R> {
        self.selected
            .get(part)
            .and_then(|i| rows.get(*i))
            .cloned()
            .ok_or_else(|| Error::Invalid(format!("no row of {part} is selected")))
    }

    pub fn message(&mut self, level: Level, text: String) {
        self.messages.push(Message { level, text });
    }

    pub fn open(&mut self, window: &str, with: Vec<(&str, Value)>) {
        self.open.push(OpenRequest {
            window: window.to_string(),
            with: with.into_iter().map(|(k, v)| (k.to_string(), v)).collect(),
        });
    }

    /// The reply for the window `window`, with its data after the handler.
    pub fn reply(self, window: &str, data: Map<String, Value>) -> Reply {
        Reply {
            state: WindowState {
                window: window.to_string(),
                data,
                parts: self.parts,
                selected: self.selected,
            },
            messages: self.messages,
            open: self.open,
            close: self.close,
        }
    }
}

/// A window's typed data as the map the screen holds.
pub fn to_data<T: Serialize>(window: &T) -> Result<Map<String, Value>> {
    match serde_json::to_value(window) {
        Ok(Value::Object(map)) => Ok(map),
        Ok(_) => Err(Error::Invalid("window data must be named values".into())),
        Err(err) => Err(Error::Invalid(err.to_string())),
    }
}

/// The screen's data of a window as the window's typed data.
pub fn from_data<T: DeserializeOwned>(data: Map<String, Value>) -> Result<T> {
    serde_json::from_value(Value::Object(data)).map_err(|err| Error::Invalid(err.to_string()))
}

/// A value as the screen holds it, for `open Window with name = value`.
pub fn to_value<T: Serialize>(value: &T) -> Result<Value> {
    serde_json::to_value(value).map_err(|err| Error::Invalid(err.to_string()))
}

/// A record a reference field can point at, as a dropdown shows it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RefOption {
    pub id: crate::Id,
    pub text: String,
}
