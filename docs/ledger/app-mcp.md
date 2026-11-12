# Apps as MCP servers

Status: not built. A program Clarity generates offers its actions to an AI agent over MCP while it runs, and the person using it grants each outside change; beside it, a command line of the app for when it is closed. First user: the maintenance example (see [examples](examples.md)).

## Code
- none yet. Clarity's own MCP server, for the IDE's project, is a different piece (see [mcp-tools](mcp-tools.md)); its server code (`crates/clarity-mcp`) may be a model, not a dependency of generated programs.
- cli targets exist on every stack (see [cli-target](cli-target.md)).

## Tests
- none yet

## Open
- The whole piece: NOT BUILT. Sketch, to be agreed before building:
  - The app declares which of its actions an agent may call (an `action` with its effects, as in the language today), with a description people and agents read from the translations.
  - The running desktop app serves them over MCP; every call asks the person in Clarity's own dialog what it would change: allow once, allow for this session, refuse. A refused call tells the agent so. The change shows live in the open windows.
  - The command line of the same app (desktop + cli in the project file) runs the same actions when the app is not running.
  - Every stack, every system.
- clarity-b5's proposals (2026-11-12, not agreed):
  1. Tools are `command`s, not actions: plain-typed arguments and options give the input schema (min, max, default), `print` the result, `refuse` a tool error, `ask` MCP elicitation; help from the translations. One declaration is a terminal command when the app is closed and a tool while it runs. Opt in per command with a line such as `agent`, off by default. The checker's effects (db.write, files.write, net) tell the grant dialog what kind of change comes.
  2. One shared Rust core with a C interface (like clarity-auth, clarity-formats): protocol, transport, discovery, session grants. clarity-gen writes the tool list once for all stacks; each stack adds one callback that runs a command by name with JSON arguments (its cli dispatch). The desktop program links its commands too.
  3. Discovery: the agent starts `<app> mcp` (stdio), as `clarity mcp` reads ide.json. A running desktop app writes `running.json` (local socket or pipe, a token, readable by the person only) beside settings.json and removes it at exit; `<app> mcp` forwards to it when there, else answers from the closed app.
  4. While the app is closed, changes from `<app> mcp` are refused ("open Shop to allow this") unless the person turned on a setting that allows them, off by default; no queue of waiting changes. A person typing a command needs no grant.
  5. A call reaches the desktop host like a click: Clarity.UI's dialog (allow once, for this session, refuse) with the command's help, arguments and effects; then one transaction, one undo step, windows reload, a note "Claude added customer Ada" with Undo. A session grant is keyed by the connection, not the agent's claimed name, and ends with it. With sign-in the agent acts as the signed-in person.
  6. Reads need no grant per call; an "agent connected" mark with a log of calls, and an option to ask for reads too.
  7. Modules: spec (the `agent` line and checks), gen (tool list writer, each stack's callback), the Rust core, Clarity.UI (grant dialog, activity mark), runtimes (dispatch), conformance (a tool list file and a grant flow every stack runs). Maintenance's first commands (import, review report, set an entry's status) are useful in the terminal before any MCP exists.
  8. The app's rules for agents, in two kinds. Rules that must hold are enforced where the app already checks them (`require`, `ensure`, `allow when`, field constraints such as `required`, `unique`, `min`, `max`): a call that breaks one is refused with the reason, and each tool's description lists them, generated from the declarations. Guidance no check can say ("an entry goes from draft to review to done", "review notes are Markdown") is written in the app's design: an `agent` block in a module, its texts from the translations; the generator gives it to the MCP service as the server's instructions, with a generated summary of the app's data and commands.
- Open questions: how a command-line change, made while nobody watches, reaches the person's grant (a list of changes waiting at the next start, or no grant needed when the app is closed); how an agent finds a running app (a file like the IDE's `~/.clarity/ide.json`, per app); whether a grant for the session is kept per agent; whether reads need a grant too.

## Conflicts
- none

## Decisions
- 2026-11-12: outside changes to an app's data need the person's grant. User: "me, as a second user, should be made aware that updates from outside are made and should give a grant for that... this will probably come down that apps themselves can become mcp-servers?!"; then chose "Both": a command line for when the app is closed, MCP with grants while it runs.
- 2026-11-12: the app holds its own rules for its MCP service. User asked: "think that question requires to also create something that hold the rules for the specific app to feed the mcp-service?"; on the answer (enforced rules plus written guidance, proposal 8): "add it yourself... then we make a release and when done we implement this... I think this is a very useful feature when a developer can incorporate an app specific mcp-server in the app". Built after the next release.

## Was
- none
