# The agent

How the Agent panel works: what the agent can do in your project, taking a turn back, stopping and starting over, what the token line means, and how to ask well.

## Opening the panel

Click **Agent** at the right of the tab row. The panel sits to the right of every tab and stays open the next time you start Clarity. The box at the top picks the connection the agent talks through; see [Connections](ai.connections.md). Type a request at the bottom and press Enter or **Send**; Shift+Enter starts a new line.

## What the agent can do

The agent is a model that knows Clarity and works on the open project through tools. Each tool use shows as a line in the conversation; click it to see what the tool gave back.

- Look at the project: its project file, its modules and its problems.
- Read and write the project file and the `.clarity` modules in the project's folder. Every write is checked at once, and the agent sees the errors it made.
- Check the whole project.
- Look up the parts a window can hold, with their properties.
- Write a migration step for the changes in your entities, as **Create step** does; see [Migrations](data.migrations.md).
- Make a new project and open it.
- Show a window in Preview when it is done.
- Read the design document, for what Clarity can do and what is planned.
- Read this manual and the language reference, the pages it needs for the task.

The agent cannot reach files outside the project's folder, and it writes only Clarity files. The designer follows its changes as they are written.

## Undo, Stop and New

Changes are written at once. One **Undo** takes a whole turn back, however many files it wrote. A project the agent made during a turn opens with a history of its own.

**Stop** ends a running turn after the step it is in. A turn also ends after 40 steps, and when it has spent its budget of tokens (200k at full price, unless its connection says otherwise; see [Connections](ai.connections.md)). The turn finishes the step it is in first, so nothing is left half done; ask again to go on.

**New** forgets the conversation and starts a fresh one. It is not available while a turn runs.

## The token line

Under the conversation, a line says what the conversation cost, in tokens:

```text
This turn 48.2k in (41k from the cache), 1.3k out. Session 210.4k in, 6.8k out.
```

- `in` is everything sent to the model: your words, the conversation so far, the instructions and the tool results.
- `from the cache` is the part of `in` the model had seen before and served from its cache, which costs a tenth of the full price or less, depending on the model.
- `out` is what the model wrote: its answers and its tool calls.

The turn counts restart with each request. The session counts add up the whole conversation; **New** sets them back to zero.

## Writing good requests

- Say what you want to see: "a window that lists open orders with a filter on customer", rather than "an orders screen".
- Name the entities and fields you mean when they exist already.
- Ask for one thing at a time, and look at it in the designer before the next.
- When something is wrong, say what you see and what you expected.
- Ask the agent what Clarity supports before asking it to build something unusual; it reads the design document and does not promise what is not built.

A long conversation sends more with every turn. When you move on to another task, click **New**.
