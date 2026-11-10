//! Clarity.Graph (docs/GRAPH-PLAN.md): the graph behind the `graph` part,
//! without drawing. `read` reads one from two lists of records, `model`
//! from the data model's records and references, `hops` keeps the nodes
//! near one, and `layout` places them, the same way every time.

pub mod hops;
pub mod layout;
pub mod model;
pub mod read;
