//! Keys set by the person (DESIGN 5.28), as every renderer sets them
//! (conformance/shortcut-keys.json): a command's keys are the person's or the
//! design's; a key taken in a place they share swaps; Reset swaps back; the
//! keys the system and the inputs need are refused. Keys are written as
//! Clarity writes them: `Ctrl+Shift+N`. The person's keys are kept in the
//! settings file as `keys`, by command, only where they differ from the
//! design's; a command of the app's frame is known by its name, a window's
//! own by `Window.name`.

use std::collections::BTreeMap;

use serde_json::{Map, Value};

/// The person's keys by command.
pub type Choices = BTreeMap<String, Vec<String>>;

/// A command: its id, the places it is active in ("" is the app's frame, a
/// window by its name) and the keys its design gives it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Command {
    pub id: String,
    pub layers: Vec<String>,
    pub keys: Vec<String>,
}

/// The rules of setting keys, alike on every renderer.
pub struct KeyRules;

impl KeyRules {
    /// A command's keys: the person's, or the design's.
    pub fn keys_of(command: &Command, choices: &Choices) -> Vec<String> {
        choices.get(&command.id).cloned().unwrap_or_else(|| command.keys.clone())
    }

    fn share(a: &Command, b: &Command) -> bool {
        a.layers.iter().any(|l| b.layers.contains(l))
    }

    /// Gives `id` the key: a command that has it in a place they share gets
    /// the keys this one had (the second value names it); what is kept is
    /// only what differs from the design.
    pub fn assign(commands: &[Command], choices: &Choices, id: &str, key: &str) -> (Choices, Option<String>) {
        let Some(command) = commands.iter().find(|c| c.id == id) else {
            return (choices.clone(), None);
        };
        let mut keys: BTreeMap<&str, Vec<String>> = commands.iter().map(|c| (c.id.as_str(), Self::keys_of(c, choices))).collect();
        let old = keys[id].clone();
        let other = commands.iter().find(|c| c.id != id && Self::share(c, command) && keys[c.id.as_str()].iter().any(|k| k == key));
        if let Some(other) = other {
            let theirs = keys[other.id.as_str()].clone();
            let mut next: Vec<String> = theirs.iter().filter(|k| *k != key).cloned().collect();
            next.extend(old.iter().filter(|k| !theirs.contains(k)).cloned());
            keys.insert(other.id.as_str(), next);
        }
        keys.insert(id, vec![key.to_string()]);
        let next = commands.iter().filter(|c| keys[c.id.as_str()] != c.keys).map(|c| (c.id.clone(), keys[c.id.as_str()].clone())).collect();
        (next, other.map(|o| o.id.clone()))
    }

    /// Puts a command's keys back to the design's: a command that took one
    /// of them in a swap gets its own back.
    pub fn reset(commands: &[Command], choices: &Choices, id: &str) -> Choices {
        let mut next = choices.clone();
        let Some(command) = commands.iter().find(|c| c.id == id) else {
            return next;
        };
        for key in &command.keys {
            next = Self::assign(commands, &next, id, key).0;
        }
        next.remove(id);
        next
    }

    /// The commands of other places that have the key too: allowed, and the
    /// inner one goes first where both are active.
    pub fn also_has(commands: &[Command], choices: &Choices, id: &str, key: &str) -> Vec<String> {
        let Some(command) = commands.iter().find(|c| c.id == id) else {
            return Vec::new();
        };
        commands
            .iter()
            .filter(|c| c.id != id && !Self::share(c, command) && Self::keys_of(c, choices).iter().any(|k| k == key))
            .map(|c| c.id.clone())
            .collect()
    }

    /// The modifiers of a key and the key itself: `Ctrl++` is Ctrl and `+`.
    pub fn parts(key: &str) -> (Vec<String>, String) {
        if let Some(held) = key.strip_suffix("++") {
            return (held.split('+').filter(|m| !m.is_empty()).map(String::from).collect(), "+".to_string());
        }
        let mut all: Vec<String> = key.split('+').map(String::from).collect();
        let last = all.pop().unwrap_or_default();
        (all, last)
    }

    /// Why a key cannot be a command's: `typing`, `tab`, `enter` or `escape`;
    /// none when it can.
    pub fn refused(key: &str) -> Option<&'static str> {
        let (held, last) = Self::parts(key);
        match last.as_str() {
            "Tab" => return Some("tab"),
            "Enter" => return Some("enter"),
            "Escape" => return Some("escape"),
            _ => {}
        }
        let plain = !held.iter().any(|m| matches!(m.as_str(), "Ctrl" | "Alt" | "Meta"));
        (plain && (last.chars().count() == 1 || last == "Space")).then_some("typing")
    }
}

/// The person's keys, as the settings file keeps them under `keys`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PersonKeys {
    pub choices: Choices,
}

impl PersonKeys {
    /// What the settings kept, as lists of keys only: anything else is left out.
    pub fn read(settings: &Map<String, Value>) -> PersonKeys {
        let mut choices = Choices::new();
        if let Some(Value::Object(kept)) = settings.get("keys") {
            for (id, value) in kept {
                let Value::Array(list) = value else { continue };
                let keys: Option<Vec<String>> = list.iter().map(|k| k.as_str().map(String::from)).collect();
                if let Some(keys) = keys {
                    choices.insert(id.clone(), keys);
                }
            }
        }
        PersonKeys { choices }
    }

    /// The keys a command answers: the person's, or the one its design gives.
    pub fn of(&self, id: &str, designed: Option<&str>) -> Vec<String> {
        match self.choices.get(id) {
            Some(keys) => keys.clone(),
            None => designed.filter(|d| !d.is_empty()).map(|d| vec![d.to_string()]).unwrap_or_default(),
        }
    }

    /// Puts the person's keys in `settings`, in place of what they kept before.
    pub fn keep(&self, settings: &mut Map<String, Value>) {
        let keys: Map<String, Value> = self.choices.iter().map(|(id, keys)| (id.clone(), Value::from(keys.clone()))).collect();
        settings.insert("keys".to_string(), Value::Object(keys));
    }
}
