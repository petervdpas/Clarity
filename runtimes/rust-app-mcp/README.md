# Clarity.AppMcp

The MCP service of the programs Clarity generates (docs/ledger/app-mcp.md): an app offers its commands with an `agent` line to AI agents as tools, and the person using it grants each call. One core, used by every stack.

- `Server::new(tool_list, options)` serves the tool list clarity-gen writes: `initialize` gives the app's guidance as the instructions, `tools/list` the tools, `tools/call` checks the arguments against the tool's inputs, gets the grant and runs the command through the program's `Host`.
- Grants: while the app is open the person is asked; a change at every call unless allowed for the session (that tool, that connection), a read once per connection. While it is closed nobody is asked: the person's settings (`closedReads`, `closedChanges`) allow, else the call is refused.
- A command that asks gets its answer from the agent through the client's elicitation; a client without it gets no answer.
- `discovery`: `running.json`, written by a running app beside its own settings, so each app has its own; readable by the person only.

The service speaks lines of JSON (`Lines`): standard input and output for `<app> mcp`. Rust programs use this crate; Go, C, C++ and C# programs its C table (`clarity-app-mcp-ffi`, `clarity_app_mcp_api()`). Every stack runs `conformance/app-mcp.json`.
