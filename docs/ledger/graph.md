# Graph part

Status: partial. A `graph` part that shows records as nodes joined by edges, from two lists code fills or from the data model (records of several entities, references as edges), drawn by Clarity.UI on every stack.

Progress: 45%

## Code
- Spec: `crates/clarity-spec/src/widgets.rs` (`Graph`; events click, select, open on nodes :1175), `model.rs` (`GraphLayout`, `NodeShape`, `EdgeLine`; `graphModel` worked out when the project loads, never written into the source), `check.rs`, `code/check.rs`, `cxaml.rs`
- Generators: through window markup; a graph loses its picked row when its rows are replaced, as a table does (`rows_shown`, `crates/clarity-gen/src/lib.rs:638`)
- Runtimes: none of its own; the graph walks the records the window holds
- Graph core: crate `clarity-graph`, `runtimes/rust-graph/src/` (`read.rs`, `model.rs`, `hops.rs`, `layout.rs`: force, layered, radial, circle, deterministic); released by `.github/workflows/release-rust-graph.yml` before clarity-ui
- Clarity.UI: `runtimes/rust-ui/src/parts/data/graph.rs` (looks per kind, zoom and pan, dragged places kept, hops control, legend, `limit`)
- IDE: `app/src/lib/PageProperties.svelte` (inspector), `PartTile.svelte`, `PageDesigner.svelte` (palette); agent tool `crates/clarity-tools/src/structural/windows.rs`
- Docs: manual `parts.data.md` (`graph`)

## Tests
- No conformance file.
- `runtimes/rust-graph/src/*.rs` (16 unit tests); rust-ui `tests/data/data_graph.rs`; spec `graphs.rs`; the every-part fixture
- `examples/network` (Rust desktop) in the example builds

## Open
- Looks bound to fields: NOT BUILT; see Design "Looks bound to fields".
- Icons per node kind and label placement: NOT BUILT; see Design "Looks per kind".
- A picked node's references shown as links: NOT BUILT; see Design "References as links".
- Filters on any field: NOT BUILT; see Design "Filters". Only the legend's kinds are built.
- Edges through a link entity; choosing which relations to follow: NOT BUILT / PARTIAL; see Design "Lists of the data model".
- Property entities: NOT BUILT; see Design "Open-ended information".
- Controls each optional, keeping drags optional: PARTIAL; see Design "Controls".
- Edge events: NOT BUILT; see Design "Edge events".
- The modelling example uses the graph (step 4): NOT BUILT; see Design "The data modelling app".
- Open questions: see Design "Open questions".

## Design

### Looks bound to fields
- A node's size by a number field, its color by a field's value, its label from a text with field slots, an edge's thickness by a number field.
- Today `nodeLooks` and `edgeLooks` are per kind only (`runtimes/rust-ui/src/parts/data/graph.rs:73-77`).

### Looks per kind
- Beside shape, color and size (built): an icon per node kind, and how its label shows (beside, inside, on hover, cut at a length).
- Per edge kind, beside line, color and arrow (built): whether its label shows.

### References as links
- A node's reference fields (one, or a list, to records of any entity) show as links where the picked node's fields show (pointing at it, or the slide-out `opens` names): one opens the record, as `on open` does, or picks it in the graph.

### Filters
- A filter that tests any field of the nodes or edges, beside the legend's kinds.

### Lists of the data model
- A relation through a link entity becomes one edge, carrying the link's fields as the edge's information.
- Which relations of the `lists` entities the graph follows is set on the part (today every reference between the entities shown is an edge).

### Open-ended information
For information not known in advance (the user: graphs as "'unstructured' information structures"), a node's information can be rows of a property entity, one row per piece of information. The user's decision: "typed values, so it holds object references too".

```clarity
entity Thing
    name: text
    kind: text

enum ValueType
    text
    number
    date
    yesNo
    record

entity ThingProperty
    thing: ref Thing, cascade
    name: text
    type: ValueType
    text: text
    number: decimal
    date: datetime
    yesNo: bool
    entity: text
    record: integer
```

- Any node gets any information without a change to the model: a row `(thing, "owner", "Bob")`.
- The part names the property entity and its three fields (the node it belongs to, the name, the value). The graph shows a node's properties as its own fields: pointing at it tells them, and looks and filters use them like fields.
- An edge has properties the same way, through a property entity that refers to the edge.
- `type` says which column holds the value; the others stay empty, so a number sorts as a number and a date as a date.
- A value of type `record` refers to a record of any entity (`entity` names it, `record` holds its id): the node's store of object references. It shows as a link (see "References as links"), and as an edge to that record's node when the record is in the graph.
- Such a reference cannot be a foreign key; the graph shows one to a removed record as gone.

### Controls
- Each control optional on the part: zoom and center, hops (built on every graph), the legend (built, optional), the count of nodes and edges (built beside hops).
- Whether a dragged place is kept is an option (today always kept).

### Edge events
- An edge clicked or double clicked runs a handler, as a node's `on click`, `on select` and `on open` do (built for nodes only, `crates/clarity-spec/src/widgets.rs:1187`).

### The data modelling app
- Step 4: the data modelling example uses the graph. The user: "I'm going to need it in the data modelling app". The records of the model are the graph: domains, data objects, class definitions and properties, enums, events, adapter tables and API views, with the relations between them as edges, followed from the record on screen.
- The modelling app does not shape the part (Decisions 2026-11-10); it fits onto steps 1 and 2 and the property entity.

### Open questions
- Is "data modelling" a graph of the records, or also a graph of the model itself (entities and their relations, as the ERD has)? The design covers the records; the model would be a graph of two lists or a third source.
- Can people add or link nodes in the graph (an editor), or does it only show and navigate? The design only shows.
- Are property names free, or from a list per kind?
- Does removing a record also remove the property rows that refer to it?

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
- GRAPH-PLAN.md "Goal" (in Decisions), steps 1-3 (built; their "left for later" lists now Design), step 4 (Design "The data modelling app"), "Open questions" (Design "Open questions")
- GRAPH-PLAN.md "The data" (looks bound to fields, the panel, filters, link-entity edges: now Design; "an entity and a focused record" is outdated, the code has `lists`, `pickedKind`, `pickedId`), "Storing open-ended information" (Design "Open-ended information"), "The design" (Design "Looks per kind", "Controls", "Edge events"; the focus-changed event is built as `on select`)
- memory: graph-part-plan.md
