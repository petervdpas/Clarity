//! The one interface every format of the catalog implements
//! (docs/FORMATS-PLAN.md, "A format is its syntax only"): its entry, its
//! bytes read into the document model (doc.rs) and written from it, and
//! its schemas read into the engine's rules (describe.rs). Everything else
//! (paths, records, fitting, file-source tables, write-back, describe) is
//! the engine's, written once for every format.

use crate::describe::Described;
use crate::doc::Document;
use crate::tree::Tree;
use crate::{FormatInfo, Problem, ProblemKind};

pub trait Format: Sync {
    /// Its catalog entry: name, title, what it holds and its options.
    fn info(&self) -> FormatInfo;

    /// `input` as a document. `names` name what the file leaves unnamed
    /// (the columns of a flat file without a header row), in order.
    fn parse(&self, input: &[u8], options: &[(String, Tree)], names: &[String]) -> Result<Document, Problem>;

    /// The document as the format's bytes: a document read written back as
    /// it was where nothing changed, a new one by the options.
    fn write(&self, doc: &Document, options: &[(String, Tree)]) -> Result<Vec<u8>, Problem>;

    /// A schema of one of its `schema_kinds` as the engine's rules: the
    /// tables it declares, typed (an XSD for XML).
    fn schema(&self, _text: &str) -> Result<Vec<Described>, Problem> {
        Err(Problem::new(ProblemKind::UnknownFormat, format!("{} has no schema to import from", self.info().title)))
    }
}
