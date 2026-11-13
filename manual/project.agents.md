# Apps for AI agents

How an app's commands become tools for AI agents, and what the person using the app allows.

## Offering commands

A command with the line `agent` in its body is a tool of the app's MCP service. An agent calls it by its words joined with hyphens (`project add` is `project-add`), with the command's arguments and options as its values. What the command prints is the answer the agent reads. A question the command asks with `ask` goes to the agent.

```clarity
command project add(name: text, folder: text)
    help @projectAddHelp
    agent
    let p = new Project
    p.name = name
    p.ledgerFolder = folder
    save p
    print @projectAdded
```

A command for agents needs its `help`: it is how an agent picks the command. Clarity works out whether a command changes anything (it saves, deletes, writes a file, sets a setting or sends something with an HTTP call), and the person is asked before every change.

## The Agents part

Once a command says `agent`, the project tree shows **Agents** beside the app, the README and Sign-in (before that, **+ Agents**). It holds:

- **Commands for agents**: the commands an agent may call, as it calls them.
- **While the app is closed**: what agents may do when nobody is there to allow it, until the person sets it in the app. **Agents may read** and **Agents may change** write the line `agents while closed read change` in the project file. Both are off without it.
- **At once**: how many agents the running app serves at the same time, 3 unless you change it (`agents at most 2` in the project file). One more is refused with the reason, and the app's window says it was turned away.
- **Guidance**: `agent/guidance.md`, which an agent gets when it connects, in four sections: what the app is for, the app's words, how work flows, and rules the app does not check. Each box says what belongs in it. Name the app's commands, entities and fields in backticks; the checker warns of a name the app does not have.

## The person's settings

The person using the app decides what agents may do while the app is closed, in the app's own settings. Add Clarity's own settings to the `settings` block and put them in the app's settings window, as `language` and `theme` are:

```clarity
settings
    language
    theme
    agentReadsWhileClosed
    agentChangesWhileClosed
```

They are checkboxes labeled with Clarity's words ("AI agents may read while the app is closed", "AI agents may change while the app is closed"), and they start as **While the app is closed** in the Agents part says.

## How an agent reaches the app

The app needs no command-line program for this: its own program answers the word `mcp`. In the agent's setup of MCP servers, add the program with `mcp` as its argument, such as `maintenance mcp` (for Claude Code: `claude mcp add maintenance -- /path/to/maintenance mcp`).

- **The app is open:** each call goes to it. The first read of a connection and every change ask the person: **Allow once**, **Allow for this session** or **Refuse**.
- **The app is closed:** nobody is asked. The person's settings allow reads and changes, else the call is refused with "open it to allow this".

Several apps can run at once: each answers only the agents that started its own program.

## What a change looks like

A change an agent makes is one step: all of it is kept, or none of it when the command refuses. The open window reads its data again, and a note says what the agent did, with **Undo**, until the person closes it. Undo takes the whole change back.

An app that hosts an agent in a chat of its own (Codex or Copilot, through ACP) is a later feature.

See also [Clarity in your own agent](ai.mcp.md), which is about Clarity itself as an MCP server for designing projects, and the [language reference](../docs/LANGUAGE.md), under Commands.
