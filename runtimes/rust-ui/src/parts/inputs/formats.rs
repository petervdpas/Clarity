//! Showing and reading values in the person's locale: Clarity's one way,
//! in the runtime (`clarity_runtime::shown`), which Clarity.UI's parts and
//! Clarity.Pdf's reports share (conformance/formats.json). Here only what
//! asks the window's host.

pub use clarity_runtime::shown::*;

use crate::parts::{FieldFacts, PartHost};

/// The format of the value at `path`: as the host knows its field, else by its type; none for a value that is no number or date.
pub fn format_of(host: &dyn PartHost, path: &str) -> Option<FieldFormat> {
    if let Some(known) = host.field(path).and_then(|f: FieldFacts| f.format).and_then(|f| FieldFormat::parse(&f)) {
        return Some(known);
    }
    host.type_of(path).and_then(|t| FormatKind::of_type(&t)).map(FieldFormat::new)
}

/// The person's formats as the host gives them.
pub fn formats_of(host: &dyn PartHost) -> Formats {
    let given = host.formats();
    Formats::new(given.language.as_deref(), given.currency.as_deref())
}

