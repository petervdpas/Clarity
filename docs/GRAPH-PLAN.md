# Graph part plan

A part that shows a graph of nodes and edges in an app, for data modelling and anything else that is a graph. Written 2026-11-10; steps 1 to 3 built 2026-11-10, unreleased.

## Goal

The user's words, shown a node-link view in another program: "I think the graph data struct is nice to have as part i can add to a app (data modelling)... but is has to be generic ... not the data part, because that's just a graph, but ui/design part should be rather flexible!"

The user, starting step 1: "for now it needs to be as generic as possible. I [see] graphs as 'unstructured' information structures!" And: "closest to life in data modelling". How the data modelling app uses it is decided when step 3 comes.

So the graph's shape stays fixed (nodes joined by edges), what each holds is the app's, and the look is what the designer shapes.

## The data

The user's words: "you have to look that the information one can store on the nodes and edges can be flexible".

So a node and an edge are records of any type the app declares, with whatever fields it needs. The part only has to know which fields play which role:

- **Node:** which field is its id, its label and its kind. Every other field is the node's information.
- **Edge:** which field is its source and its target, and optionally its label and kind. Every other field is the edge's information.

A node can point to data: it holds a store of object references. So among a node's fields can be references: one reference, or a list of them, to records of any entity. From the data model these are the record's own reference fields; from code, reference fields on the declared node type.

The information is used, not only kept:

- **Looks bind to fields:** a node's size by a number field, its color by a field's value, its label from a text with field slots, an edge's thickness by a number.
- **Showing it:** on hover, and in a panel beside the graph for the picked node or edge, with the fields the part names. A node's references show there as links: one opens the record, as a node's double click does, or focuses the graph on it.
- **Filtering on it:** the legend filters by kind, and a filter can test any field.

Where they come from, the user's decision ("Both"):

- **From the data model, without code.** The part names an entity and a focused record: records become nodes (kind = their entity, information = their fields), relation fields become edges (label = the field; a relation through a link entity carries the link's fields as the edge's information). It follows the relations a number of hops from the focused record, both ways. Which entities and relations it follows is set on the part.
- **From code.** The part binds to two lists the app's code fills, of the node and edge types the app declares, for any graph that is not records and relations.

A graph larger than the part's limit is cut, whole nodes first, never an edge to a node that is not shown, and the part says it was cut.

## Storing open-ended information

Nodes and edges are ordinary entities, so each is a table; their fields are columns, fixed when the entity is declared. For information that is not known in advance (the user: graphs as "'unstructured' information structures"), the user asked to add the property entity option: a node's information as rows of a property entity, one row per piece of information.

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
- The graph shows a node's properties as its own fields: pointing at it tells them, and looks and filters can use them like fields. The part names the property entity and its three fields (the node it belongs to, the name, the value).
- An edge can have properties the same way, with a property entity that refers to the edge.
- Values are typed, the user's decision: "typed values, so it holds object references too". `type` says which column holds the value; the others stay empty, so a number sorts and compares as a number and a date as a date.
- A value of type `record` refers to a record of any entity: `entity` names the entity, `record` holds its id. This is the node's store of object references: a node holds as many as it has property rows of that type, each to a record of any kind. The graph shows one as a link that opens the record or focuses the graph on it, and can draw it as an edge to that record's node when the record is in the graph.
- A reference to any entity cannot be a foreign key, so the database does not keep it from pointing at a record that was removed. The graph shows such a reference as gone; whether removing a record also removes the property rows that refer to it is open.
- Open: whether property names are free or come from a list per kind.

## The design

Set in the designer, defined in Clarity like every part look, drawn by Clarity.UI on every stack:

- **Per node kind:** shape, size, color, icon, and how the label shows (beside, inside, on hover, cut at a length).
- **Per edge kind:** line style, color, arrow, and whether the label shows.
- **Layout:** force, layered, radial or circle. Nodes can be dragged; whether a dragged place is kept is an option.
- **Controls, each optional:** zoom and center (the zoom bar the diagram designer has), focus with hops (- and +), a legend that filters by kind, the count of nodes and edges.
- **Events:** a node or edge clicked or double clicked, and the focus changed, so the app can open the record behind a node.

## Steps

1. The part with code-given nodes and edges: the layouts, looks per kind, zoom and center, events. Every stack.
   Built: the `graph` part (LANGUAGE.md "Graphs"): two lists of any records, the role fields (`nodeKey`, `nodeLabel`, `nodeKind`, `edgeFrom`, `edgeTo`, `edgeLabel`, `edgeKind`), the four layouts (deterministic), `nodeLooks` (shape, color, size) and `edgeLooks` (line, color, arrow) per kind, `limit`, zoom and pan, dragging nodes, `select` and `open`, and pointing at a node or edge telling its other fields (`show`). Drawn by Clarity.UI, so every stack has it; checked, in the designer's inspector and palette, in the every-part fixture. Left for later from the design above: looks bound to fields (size by a number), an icon and label placement per kind, the panel beside the graph with references as links, edge events.
2. The data model as the source: entity, focus, hops and the relations to follow, with the hops control and the legend filter.

   Decided: the graph walks records the window holds, not the database (the user chose "From held records": no database code per stack; the window's lists load every record when it opens). And the graph's own logic is a crate of its own, `clarity-graph` (runtimes/rust-graph: reading, the data model's edges, hops, the layouts), released before clarity-ui; the user asked "maybe it needs to be its own crate? And its own nuget." and chose the crate without a NuGet, as the part ships inside Clarity.UI's library.

   Built: `lists` (records of several entities, each its entity's kind, labelled by the first text field, joined by references between the entities shown; worked out when the project loads as `graphModel`, never written into the source), `pickedKind` and `pickedId` (the picked node's entity and id in data, which code sets to pick), `hops` with its control and `legend` with its chips on every graph, the force layout packing groups that share no edge side by side, and the inspector's settings for all of it. Not built from the design above: following a relation through a link entity as an edge.
3. An example app of its own tries the graph out first, before the data modelling app. The user: "I think its best to test this in its own example app before we go to the data modelling app".

   Built: `examples/network` (Rust desktop, the user's choice). Map: people, organisations and projects as a graph of the data model, with hops, a legend and the picked node in data. Ideas: ideas and their links as a graph of two lists, beside the tables that edit them. A new database starts with a few records (`startingRecords`). Builds without warnings; in the example builds of the checks.

   After the user tried it: "does not store positioning! And I dont get the ideas... cant go back! Add erd and class diagrams please", and "clicking a node should show a slideout view with the information!". So a dragged node stays where it was left, kept per person by window and graph name and by its record (not its row), and picking, hops and the legend no longer move nodes (the whole graph is laid out; they only hide some of it). A graph is clickable: every click on a node runs `on click` or opens what `opens` names, as a button's. The example became one window with two tabs, each saying what it is for, a slide-out with the picked record's details, and an ERD and class diagram (held to the project by a test).
4. The data modelling example uses it. The user's words: "I'm going to need it in the data modelling app". There the records of the model are the graph: domains, data objects, class definitions and properties, enums, events, adapter tables and API views, with the relations between them as edges, followed from the record on screen. Steps 1 and 2 are built so this window needs no code. The modelling app does not shape the part: the user, on deciding its needs early: "I think telling it now is going to steer away from a generic structure". So steps 1 and 2 and the property entity are designed for any graph, and step 3 fits the modelling app onto them.

## Open questions

- Is "data modelling" a graph of the records (as in the screenshot), or also a graph of the model itself (entities and their relations, as the ERD has)? The plan covers the records; the model would be a code-given graph or a third source.
- Can the user add or link nodes in the graph (an editor), or does it only show and navigate? The plan only shows.
