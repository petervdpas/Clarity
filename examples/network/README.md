# Network

Graphs of records, to try the graph part out (docs/GRAPH-PLAN.md). One window with two tabs.

## What you see and can do

**People, organisations and projects** shows them as one graph, drawn from the data model:

- Each record is a node of its entity's kind: organisations as boxes, projects as diamonds, people as circles. The legend's chips at the top left hide and show each kind.
- The references between them are the edges: a person's organisation, a project's lead and organisation. Nothing in the window names them; the data model gives them.
- A click on a node picks it and slides out its details at the right: the record, in the panel of its kind. The graph writes the picked node's entity and id into `kindPicked` and `idPicked`; the `nodePicked` handler reads that record.
- With a node picked, the graph shows only what lies two hops from it; the control at the top left shows fewer or more. Picking, hops and the legend move no node.
- Drag a node where you want it: it stays there, also the next time the window opens.
- The buttons at the top right zoom, and show it whole again. The wheel pans, Ctrl and the wheel zoom.

**Ideas** holds ideas and the links between them: a graph of two lists beside the two tables that edit them.

- The graph binds the ideas and names the links as its edges: each link's `source` and `target` are the ideas it joins, its `relation` the edge's label.
- An idea's `kind` gives its look: a topic is a box, a question a diamond.
- Pointing at an idea tells its note; pointing at a link tells its relation, the ideas it joins and its strength.
- Adding, changing or removing a row of either table changes the graph at once.

A new database starts with a few records of each (`startingRecords`), so both graphs have something to show.

The `diagrams` folder holds the data model as an ERD and as a class diagram.

## Stack

Rust, desktop. An example builds only for the stack its project file names.

## What it shows

- A graph of the data model (`lists`): records of several entities, joined by their references, with hops, a legend, the picked node in data and a slide-out of its details (`opens`).
- A graph of one list (`bind` and `edges`): any records as nodes, any records as edges, with the fields that play a part named and looks per kind.
- Actions that make and save records, run from a window's open handler.
