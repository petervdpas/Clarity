//! Clarity.Data (docs/ledger/runtime-data.md): the runtime data of the
//! programs Clarity generates, the decisions about records and their values
//! every stack makes while a program runs, made once. Each stack keeps its
//! own types and reads and writes its own database, files and screen; this
//! decides the shape.
//!
//! - `tree`: one value tree, numbers kept exactly as written and object names in their order.
//! - `kind`: what a program reads or writes, as the generator describes it once for every stack: a record of an entity, a list of them, or a plain value.
//! - `numbers`: exact decimal numbers as text.
//! - `fit`: a value fitted to its kind.
//! - `normal`: a value in its one plain form.
//!
//! Clarity.Formats (the formats' syntax, mappings and paths) is built on it.

pub mod fit;
pub mod kind;
pub mod normal;
pub mod numbers;
pub mod tree;
