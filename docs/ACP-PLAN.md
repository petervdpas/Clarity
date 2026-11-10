# Your own agent in Clarity's chat panel

A plan, written 2026-11-09 at the user's request and agreed the same day. Steps 1 to 3, 5 and 6 are built (2026-11-09); step 4's check by hand against the real Codex and Copilot is open, and step 7 waits for Anthropic.

## Goal

The user's words: "but I asked for an easier way to use ai in clarity, without and api key!!!!!!!!", and on `clarity mcp`, where the chat happens in the vendor's own program: "but I want it inside the ide!!". Then: "yes, write the plan".

So the chat panel in the Clarity IDE talks to an agent the person already pays for (Codex with ChatGPT, GitHub Copilot, later Claude), signed in with that vendor's own login. No API key is typed into Clarity. The agent designs in the open project with Clarity's tools, as Clarity's own agent does.

## How it works

The panel speaks the Agent Client Protocol (ACP), the standard JetBrains IDEs and Zed use to embed outside agents. ACP is JSON-RPC over the agent program's stdin and stdout, like MCP but the other way round: Clarity is the client, the agent program is the server.

1. The person picks an agent in the panel. Clarity starts that vendor's program (`codex` through its ACP adapter or app server, `copilot --acp`) as a child process.
2. When the program is not signed in, the panel shows **Sign in**, which starts the vendor's own login (it opens the browser). Clarity never sees a password or token; the program keeps its own login.
3. Clarity opens a session and hands the agent Clarity's tools as an MCP server (ACP's `session/new` takes MCP servers). The agent then reads, writes, checks and generates through the same tools as Clarity's own agent and `clarity mcp`.
4. The agent's messages, the tool lines and its questions show in the panel as they do now.

## Agents

From the vendors' own pages, checked 2026-11-09 (sources below):

| Agent | Embedding with the person's subscription |
|---|---|
| Codex (ChatGPT) | Allowed: OpenAI invites builders to embed its app server with "Sign in with ChatGPT"; JetBrains and Xcode do. |
| GitHub Copilot | Allowed: `copilot --acp` is GitHub's own server mode for editors, on the CLI's login. Public preview. |
| Claude Code | Unclear: Anthropic's Agent SDK page says other products may not offer claude.ai login without approval; signing in to an unmodified Claude Code is allowed, and reports on embedding it disagree. Not offered until Anthropic's position is clear (or Anthropic approves); until then Claude works through `clarity mcp` in Claude Code's own window, or with an API key in the panel. |

Clarity's own agent with an API key stays as it is, as one of the choices.

## How it is built

- **A turn is a snapshot of the project.** Outside agents also write files themselves, straight to disk, not only through Clarity's tools. So the IDE takes a snapshot of the project folder's text files when a turn starts (leaving out the output folder, data and build folders) and compares it when the turn ends: one Undo takes back everything the turn changed, however it was written, and the automatic migration step follows the turn's changes once at its end.
- **Clarity's tools come from `clarity mcp`.** The agent gets `clarity mcp`, started by the IDE with the CLI it ships, as its MCP server. Started by the IDE for a turn, it also offers show a window in Preview and show a diagram: those run nothing, and write what to show into a file the IDE reads when the turn ends, as Clarity's own agent shows at the end of its turn.
- **Outside agents on and off.** The user asked: "Is it something I turn on and off in the ide settings? Can I monitor it somehow?", and then "yes, add both to the plan and build them". A setting in the IDE, "Let outside agents work on my project", recorded beside the open project. When it is off, `clarity mcp` answers every project tool that Clarity does not allow outside agents now (the manual and references stay open). It is on by default: adding Clarity to an agent is already the person's own choice.
- **Agent activity.** A list in the IDE showing each tool call as it happens: which program called it (the MCP client's name), the tool, the file, and whether it worked. `clarity mcp` appends each call to a log in Clarity's folder, and the IDE follows that log. The chat panel's turns with Codex and Copilot show in the same list. The log is kept short (the last calls), and it never holds file contents.
- **The IDE must run.** Asked "if claude or codex need it, should the ide be running?": yes. Through `clarity mcp` an agent works only on the project an IDE that is running has open (docs/MCP-PLAN.md, "Which project"), so the IDE always sees what an agent does.
- **Nothing executes without the person.** Outside agents bring their own tools, among them running shell commands and editing files directly. Clarity's rule is that its agent runs nothing, and anything that executes asks first. So:
  - Clarity starts each agent in the mode where it must ask before running a command (each vendor has one), and answers every ACP permission request with Clarity's own Allow/Deny dialog (no native dialogs), defaulting to Deny.
  - File reads and writes the agent asks of the client (`fs/read_text_file`, `fs/write_text_file`) are kept inside the project folder and go through the same check and Undo as the write tool; a write elsewhere is refused.
  - ACP's terminal methods are not offered.
- **One agent program per panel.** It starts when the panel opens a conversation with it and stops when the panel closes or Clarity quits.
- **Missing program.** When the vendor's program is not installed, the panel says so and links to the manual page that tells how to install it; Clarity does not install vendor programs.
- **Every OS.** Programs are found on PATH or at a path set in Clarity's settings, the same on Linux, Windows and macOS.
- **Texts.** Every text in the panel, the agent picker and the dialogs comes from translations.

## Order of work

1. ACP client in a crate of its own (`crates/clarity-acp`): start a program, initialize, sign-in state, new session with MCP servers, prompt, stream updates, permission requests, cancel. Tested against a small fake agent program in the tests, so no vendor login is needed in CI.
2. The turn: a snapshot of the project folder and its comparison, as one change for Undo; `clarity mcp` started for a turn offers show a window and show a diagram, written to a file the IDE reads.
3. The panel: agent picker (Clarity with an API key, Codex, Copilot), Sign in, the conversation from ACP updates, Allow/Deny for permission requests, Undo per turn.
4. Codex and Copilot, each with its start command, its ask-first mode and its sign-in, checked by hand against the real programs.
5. Outside agents on and off (the setting, and `clarity mcp` honouring it) and the agent activity list (the log `clarity mcp` writes, the IDE's list following it).
6. Manual page `ai.your-agent.md` (setup, what the agent may do, what it asks).
7. Claude, once Anthropic's position allows it.

## Decided

- The ACP layer: first the user said "use the crate, standards are better", so `crates/clarity-acp` was built on the `agent-client-protocol` crate. That crate switches on serde_json's `preserve_order` for the whole workspace, which changed the key order of every JSON Clarity writes and failed eight test targets in the checks of v0.36.0. Told so, the user said "go non standard use json_something then!". So `crates/clarity-acp` speaks ACP (JSON-RPC lines over the program's stdin and stdout) itself, with serde_json, and depends on no ACP crate.
- The tools, the agent's edits and Claude: the user said "also preview and diagrams, edits allowed, wait for anthropic (api-key for now)". So:
  - The outside agent gets `clarity mcp`'s tools plus the IDE's two that run nothing: show a window in Preview and show a diagram.
  - Its own file edits are allowed inside the project folder, checked at once and undone with the turn; a write outside the folder is refused.
  - Claude is not offered as an outside agent until Anthropic's position allows it; until then Claude in the panel is Clarity's own agent with an API key.

- Codex's commands: Codex has no mode that asks before every command; in read-only it runs reading commands inside its own sandbox unasked, and everything else comes to Clarity's Allow/Deny. Asked, the user chose "Accept, read-only sandbox". So Codex starts in read-only (the codex-acp adapter, `INITIAL_AGENT_MODE=read-only`), and the panel says so when Codex is used.

## Open questions

None left; ask again when building shows something new.

## Sources

- GitHub Docs, Copilot CLI ACP server: https://docs.github.com/en/copilot/reference/copilot-cli-reference/acp-server
- GitHub changelog, ACP support in Copilot CLI: https://github.blog/changelog/2026-01-28-acp-support-in-copilot-cli-is-now-in-public-preview/
- OpenAI, Unlocking the Codex harness (App Server): https://openai.com/index/unlocking-the-codex-harness/
- JetBrains, Codex in JetBrains IDEs: https://blog.jetbrains.com/ai/2026/01/codex-in-jetbrains-ides/
- Claude Agent SDK, docs conflict on subscription sign-in: https://github.com/anthropics/claude-agent-sdk-typescript/issues/512
- The New Stack, Anthropic Agent SDK confusion: https://thenewstack.io/anthropic-agent-sdk-confusion/
- Agent Client Protocol explained: https://www.morphllm.com/agent-client-protocol
