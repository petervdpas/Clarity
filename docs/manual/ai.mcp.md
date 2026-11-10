# Clarity in your own agent

How an agent you already use (Claude Code, Codex, Copilot) designs the open project through `clarity mcp`, and how to add it.

## What it is

`clarity mcp` makes Clarity an MCP server: a program your agent starts and talks to through its standard input and output. The agent then works with Clarity's own tools, the ones the [Agent panel](ai.agent.md) uses, under your own subscription to that agent. Clarity needs no connection of its own for it, and the server needs no internet.

To have Codex or GitHub Copilot in Clarity's own Agent panel instead, see [Your own agent in Clarity](ai.your-agent.md).

The agent always works on the project open in Clarity. Open another project there and the agent follows it at its next step. With no project open, the tools tell the agent to ask you to open one; it can still read this manual and make a new project.

## What the agent can do

It designs and generates; building and running stay with you.

- Look at the project: its project file, its modules, its README and its problems.
- Read and write the project file, the modules, the windows and views in `design/`, and the Markdown files of the resources. Every write is checked at once.
- Look up the parts a window can hold, and read this manual, the language reference and the design document.
- Pick what the application uses of a web API.
- Put every change of your entities into a migration step at once, and rename an entity, an enum or a field so that the step keeps its data; the same as for [Clarity's own agent](ai.agent.md).
- Read and write diagrams, and apply what a diagram proposes when you ask for it.
- Make a new project. It opens only when you open it in Clarity.
- Generate the application's source code for its stack, as `clarity gen` does, and read the generated code.

It cannot build or run the application, take or restore backups, or show a window: press **Run** or **Build** in Clarity and tell the agent what the **Output** says.

## Adding Clarity to your agent

Each agent starts the command `clarity mcp`. Clarity's installer puts the `clarity` command beside Clarity itself:

- On Linux it is `/usr/bin/clarity`, on the PATH, so `clarity` is enough.
- On Windows it is in Clarity's folder, which is not on the PATH: `%LOCALAPPDATA%\Clarity IDE\clarity.exe` from the `.exe` installer, or `C:\Program Files\Clarity IDE\clarity.exe` from the `.msi`. Give that full path in place of `clarity` below; in JSON and TOML write each backslash twice (`"C:\\Program Files\\Clarity IDE\\clarity.exe"`).

Open the project in Clarity before you ask the agent for anything.

### Claude Code

```
claude mcp add --scope user clarity -- clarity mcp
```

`--scope user` makes Clarity available in every folder you start Claude Code in; leave it out to add it to the current folder only. Everything after `--` is the command Claude Code starts.

### Codex

```
codex mcp add clarity -- clarity mcp
```

Or write it in `~/.codex/config.toml` yourself:

```toml
[mcp_servers.clarity]
command = "clarity"
args = ["mcp"]
```

### GitHub Copilot in VS Code

Add Clarity to `.vscode/mcp.json` in your workspace:

```json
{
  "servers": {
    "clarity": {
      "type": "stdio",
      "command": "clarity",
      "args": ["mcp"]
    }
  }
}
```

For every workspace, write it in `~/.copilot/mcp-config.json` (or `mcp-config.json` in the folder `COPILOT_HOME` names), whose list is called `mcpServers`:

```json
{
  "mcpServers": {
    "clarity": {
      "command": "clarity",
      "args": ["mcp"]
    }
  }
}
```

## Asking well

Ask as you would in the Agent panel; see [The agent](ai.agent.md). The agent reads the project and the manual pages a task needs before it writes, and Clarity's designer follows each file it writes. Changes it makes are plain file changes: Clarity's Undo does not take back an outside agent's turn, so keep your project in version control.
