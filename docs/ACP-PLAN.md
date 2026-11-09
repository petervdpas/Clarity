# Your own agent in Clarity's chat panel

A plan, written 2026-11-09 at the user's request and agreed the same day; nothing is built yet.

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

## Proposed decisions

These are mine to propose; the user decides them (see Open questions).

- **The IDE serves the tools itself.** The MCP server handed to the agent is served by the running IDE, not a separate `clarity mcp` process. Then every write lands in the IDE's turn record: one Undo takes back the agent's turn, a window it made shows in Preview, and the automatic migration steps work as they do for Clarity's own agent. The tool list is the one `clarity mcp` offers (design and generate only).
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
2. The IDE serves its tools as an MCP server for the session (in-process, so writes reach the turn record), with the same tool list as `clarity mcp`.
3. The panel: agent picker (Clarity with an API key, Codex, Copilot), Sign in, the conversation from ACP updates, Allow/Deny for permission requests, Undo per turn.
4. Codex and Copilot, each with its start command, its ask-first mode and its sign-in, checked by hand against the real programs.
5. Manual page `ai.your-agent.md` (setup, what the agent may do, what it asks).
6. Claude, once Anthropic's position allows it.

## Decided

- The ACP layer: the user said "use the crate, standards are better". So `crates/clarity-acp` builds on the `agent-client-protocol` crate.
- The tools, the agent's edits and Claude: the user said "also preview and diagrams, edits allowed, wait for anthropic (api-key for now)". So:
  - The outside agent gets `clarity mcp`'s tools plus the IDE's two that run nothing: show a window in Preview and show a diagram.
  - Its own file edits are allowed inside the project folder, checked at once and undone with the turn; a write outside the folder is refused.
  - Claude is not offered as an outside agent until Anthropic's position allows it; until then Claude in the panel is Clarity's own agent with an API key.

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
