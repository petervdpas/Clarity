//! What the inputs' tests give their parts: the shared test host
//! (`crate::testing::TestHost`) with what inputs read beyond it: the
//! choices of a dropdown or radio group, what a typed or search dropdown
//! offers, the types of values, the person's formats, the application's
//! resources and the stored files of records. Everything else is the
//! shared host's, reached through it as `host.set(...)`, `host.logged()`.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::ops::Deref;
use std::rc::Rc;

use clarity_runtime::ui::{Level, PartState, Text};
use serde_json::Value;

use crate::markup::{HandlerCall, PartSpec};
use crate::parts::{Choice, FieldFacts, Formats, PartHost};
use crate::testing::TestHost;

#[derive(Default)]
pub struct InputsHost {
    pub inner: TestHost,
    /// What a dropdown or radio group bound to a path offers.
    pub choices: RefCell<BTreeMap<String, Vec<Choice>>>,
    /// What a typed or search dropdown bound to a path offers; none: its choices.
    pub offers: RefCell<BTreeMap<String, Vec<Choice>>>,
    /// The type of the value at a path, by its name.
    pub types: RefCell<BTreeMap<String, String>>,
    pub formats: RefCell<Formats>,
    /// The application's resources by their names.
    pub resources: RefCell<BTreeMap<String, Vec<u8>>>,
    /// The stored files of records' fields, by the field's path, or by the store's path.
    pub files: RefCell<BTreeMap<String, Vec<u8>>>,
    /// The messages said, as `level: text`.
    pub said: RefCell<Vec<String>>,
}

impl InputsHost {
    pub fn new() -> Rc<InputsHost> {
        Rc::new(InputsHost::default())
    }

    /// What a dropdown or radio group bound to `path` offers: values with their labels.
    pub fn offer(&self, path: &str, choices: &[(Value, &str)]) {
        let made = choices.iter().map(|(value, label)| Choice { value: value.clone(), label: label.to_string(), cells: None }).collect();
        self.choices.borrow_mut().insert(path.to_string(), made);
    }

    /// The facts of the field at `path`.
    pub fn facts(&self, path: &str, facts: FieldFacts) {
        self.inner.fields.borrow_mut().insert(path.to_string(), facts);
    }

    /// The person's language and currency.
    pub fn use_formats(&self, language: &str, currency: Option<&str>) {
        *self.formats.borrow_mut() = Formats { language: Some(language.to_string()), currency: currency.map(str::to_string) };
        self.inner.change();
    }
}

impl Deref for InputsHost {
    type Target = TestHost;

    fn deref(&self) -> &TestHost {
        &self.inner
    }
}

impl PartHost for InputsHost {
    fn read(&self, path: &str) -> Option<Value> {
        self.inner.read(path)
    }

    fn write(&self, part: &str, path: &str, value: Value) {
        self.inner.write(part, path, value)
    }

    fn edited(&self, part: &str) {
        self.inner.edited(part)
    }

    fn state(&self, name: Option<&str>) -> Option<PartState> {
        self.inner.state(name)
    }

    fn change_state(&self, name: &str, change: &mut dyn FnMut(&mut PartState)) {
        self.inner.change_state(name, change)
    }

    fn run(&self, part: Option<&str>, call: &HandlerCall) -> bool {
        self.inner.run(part, call)
    }

    fn text(&self, written: Option<&str>) -> String {
        self.inner.text(written)
    }

    fn type_of(&self, path: &str) -> Option<String> {
        self.types.borrow().get(path).cloned()
    }

    fn choices(&self, path: &str) -> Vec<Choice> {
        self.choices.borrow().get(path).cloned().unwrap_or_default()
    }

    fn offered(&self, _part: &PartSpec, path: &str) -> Vec<Choice> {
        match self.offers.borrow().get(path) {
            Some(offered) => offered.clone(),
            None => self.choices(path),
        }
    }

    fn field(&self, path: &str) -> Option<FieldFacts> {
        self.inner.field(path)
    }

    fn formats(&self) -> Formats {
        self.formats.borrow().clone()
    }

    fn open_address(&self, address: &str) {
        self.inner.open_address(address)
    }

    fn resource(&self, name: &str) -> Option<Vec<u8>> {
        self.resources.borrow().get(name).cloned()
    }

    fn changes(&self) -> u64 {
        self.inner.changes()
    }

    fn fetch(&self, bind: &str, store_path: Option<&str>) -> Option<Vec<u8>> {
        let files = self.files.borrow();
        store_path.and_then(|p| files.get(p)).or_else(|| files.get(bind)).cloned()
    }

    fn say(&self, level: Level, text: Text) {
        self.said.borrow_mut().push(format!("{level:?}: {}", text.show()));
    }

    fn bound_of(&self, part: &str) -> Option<String> {
        self.inner.bound_of(part)
    }
}
