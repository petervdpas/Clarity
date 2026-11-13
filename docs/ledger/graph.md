# Graph part

Status: partial. A `graph` part that shows records as nodes joined by edges, from two lists code fills or from the data model (records of several entities, references as edges), drawn by Clarity.UI on every stack.

Progress: 45%

## Code
- Spec: `crates/clarity-spec/src/widgets.rs` (`Graph`; events click, select, open on nodes :1171), `model.rs` (`GraphLayout`, `NodeShape`, `EdgeLine`; `graphModel` worked out when the project loads, never written into the source), `check.rs`, `code/check.rs`, `cxaml.rs`
- Generators: through window markup; a graph loses its picked row when its rows are replaced, as a table does (`rows_shown`, `crates/clarity-gen/src/lib.rs:617`)
- Runtimes: none of its own; the graph walks the records the window holds
- Graph core: crate `clarity-graph`, `runtimes/rust-graph/src/` (`read.rs`, `model.rs`, `hops.rs`, `layout.rs`: force, layered, radial, circle, deterministic); released by `.github/workflows/release-rust-graph.yml` before clarity-ui
- Clarity.UI: `runtimes/rust-ui/src/parts/data/graph.rs` (looks per kind, zoom and pan, dragged places kept, hops control, legend, `limit`)
- IDE: `app/src/lib/PageProperties.svelte` (inspector), `PartTile.svelte`, `PageDesigner.svelte` (palette); agent tool `crates/clarity-tools/src/structural/windows.rs`
- Docs: LANGUAGE.md section 6 (Graphs, A graph of the data model); manual `design.parts.md` (Data)

## Tests
- No conformance file.
- `runtimes/rust-graph/src/*.rs` (16 unit tests); rust-ui `tests/data/data_graph.rs`; spec `graphs.rs`; the every-part fixture
- `examples/network` (Rust desktop) in the example builds

## Open
- Looks bound to fields (size by a number, color by a value, label templates, edge thickness): NOT BUILT (GRAPH-PLAN.md:26). `NodeLook` is per kind only (`runtimes/rust-ui/src/parts/data/graph.rs:71-85`).
- Icons per node kind and label placement: NOT BUILT (GRAPH-PLAN.md:77, 86).
- A panel beside the graph for the picked node, references as links: NOT BUILT (GRAPH-PLAN.md:27, 86); the network example uses a slide-out with `opens`.
- Filters on any field: NOT BUILT (GRAPH-PLAN.md:28); only the legend's kinds.
- Edges through a link entity; choosing which relations to follow: NOT BUILT / PARTIAL (GRAPH-PLAN.md:32, 91).
- Property entities (open-ended typed properties as rows): NOT BUILT (GRAPH-PLAN.md:37-71).
- Optional controls, a node and edge count, keeping drags optional: PARTIAL (GRAPH-PLAN.md:79-80).
- Edge events and a focus-changed event: NOT BUILT (GRAPH-PLAN.md:81, 86).
- The modelling example uses the graph (step 4): NOT BUILT. User: "I'm going to need it in the data modelling app"
- Open questions (from the plan): whether data modelling is a graph of records or also of the model itself; whether people can add or link nodes in the graph (the plan only shows).

## Conflicts
- none

## Decisions
- 2026-11-10: generic: the shape is fixed (nodes and edges), what each holds is the app's, the look is the designer's. User: "I think the graph data struct is nice to have as part i can add to a app (data modelling)... but is has to be generic ... not the data part, because that's just a graph, but ui/design part should be rather flexible!"
- 2026-11-10: nodes and edges are records of any type; the part names which fields play which role. User: "you have to look that the information one can store on the nodes and edges can be flexible"
- 2026-11-10: data from code and from the data model. User: "Both"
- 2026-11-10: the data-model graph walks the records the window holds, not the database (the user chose "From held records").
- 2026-11-10: the core is its own crate `clarity-graph`, no NuGet (it ships inside Clarity.UI's library). User: "maybe it needs to be its own crate? And its own nuget."
- 2026-11-10: tried first in an example of its own. User: "I think its best to test this in its own example app before we go to the data modelling app"
- 2026-11-10: a dragged node stays where it was left, kept per person by window, graph and record; picking, hops and the legend move no node; a click runs `on click` or opens what `opens` names. User: "does not store positioning! And I dont get the ideas... cant go back! Add erd and class diagrams please", "clicking a node should show a slideout view with the information!"
- 2026-11-10: the modelling app does not shape the part. User: "I think telling it now is going to steer away from a generic structure"
- 2026-11-10: typed property values, so they hold object references (designed, not built). User: "typed values, so it holds object references too"

## Was
- GRAPH-PLAN.md steps 1-4, Open questions
- memory: graph-part-plan.md
