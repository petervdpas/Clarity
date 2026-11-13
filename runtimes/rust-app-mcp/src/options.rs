//! How a program serves: its version, whether the app is open (a person is
//! there to ask), what the person's settings allow while it is closed, and
//! the texts of a refusal, in the person's language (the program gives them
//! from its translations).

use serde_json::Value;

#[derive(Debug, Clone, PartialEq)]
pub struct Options {
    pub version: String,
    pub open: bool,
    pub closed_reads: bool,
    pub closed_changes: bool,
    /// What a call the person refused says.
    pub refused: String,
    /// What a call refused while the app is closed says.
    pub closed: String,
}

impl Default for Options {
    fn default() -> Options {
        Options {
            version: String::new(),
            open: true,
            closed_reads: false,
            closed_changes: false,
            refused: "The person using the app refused this.".into(),
            closed: "The app is not open; open it to allow this.".into(),
        }
    }
}

impl Options {
    /// `{version, open, closedReads, closedChanges, texts: {refused, closed}}`,
    /// each left out keeping its default.
    pub fn from_json(text: &str) -> Result<Options, String> {
        let value: Value = serde_json::from_str(text).map_err(|e| format!("the options are no JSON: {e}"))?;
        let mut out = Options::default();
        let flag = |key: &str, default: bool| value.get(key).and_then(Value::as_bool).unwrap_or(default);
        out.open = flag("open", out.open);
        out.closed_reads = flag("closedReads", out.closed_reads);
        out.closed_changes = flag("closedChanges", out.closed_changes);
        if let Some(version) = value.get("version").and_then(Value::as_str) {
            out.version = version.to_string();
        }
        let texts = &value["texts"];
        if let Some(refused) = texts.get("refused").and_then(Value::as_str) {
            out.refused = refused.to_string();
        }
        if let Some(closed) = texts.get("closed").and_then(Value::as_str) {
            out.closed = closed.to_string();
        }
        Ok(out)
    }
}
