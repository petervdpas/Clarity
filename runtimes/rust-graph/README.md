# Clarity.Graph

The graph behind Clarity's `graph` part (docs/GRAPH-PLAN.md), without any drawing:

- **Reading** a graph from two lists of records of any kind (`read`), naming which fields are a node's key, label and kind and an edge's ends, label and kind; or from the data model (`model`): records of several entities as nodes, their references to each other as edges.
- **Hops**: the nodes a number of edges from one node, and the graph of the nodes kept (`hops`).
- **Layouts** (`layout`): force, layered, radial and circle, each placing the same graph the same way every time.

Clarity.UI's graph part draws what this gives and handles the person's input; every stack has it through Clarity.UI.
