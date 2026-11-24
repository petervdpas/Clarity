# Your own agent in Clarity

Codex or GitHub Copilot in the Agent panel with your own subscription and no API key: setup, signing in, what it may do.

## What it is

The [Agent panel](ai.agent.md) can talk to an agent you already pay for: Codex with your ChatGPT plan, or GitHub Copilot with your GitHub plan. Clarity starts that vendor's program on your computer and the program signs in with the vendor's own login, so no API key is typed into Clarity and Clarity never sees your password. The agent works in the open project with Clarity's tools, as Clarity's own agent does.

## Setting it up

1. Install the vendor's program. Clarity does not install it for you.
   - Codex: the ACP adapter `codex-acp`, which brings Codex with it: `npm install -g @agentclientprotocol/codex-acp`. See [its page](https://github.com/agentclientprotocol/codex-acp).
   - GitHub Copilot: the Copilot CLI, the program `copilot`, by npm, WinGet, Homebrew or GitHub's install script. See [GitHub's install page](https://docs.github.com/en/copilot/how-tos/copilot-cli/set-up-copilot-cli/install-copilot-cli).
2. Pick it in the box at the top of the Agent panel, as **Codex** or **GitHub Copilot**. The panel lists each agent program installed on your computer; it needs no connection in the vault, which keeps only secrets.
3. To use another copy of the program, or one installed where Clarity does not look, choose it in **Clarity settings**, **Environment**, under **Agents**: pick a copy the scan found, or choose the program with **Browse...**. "The first one found" leaves it to Clarity.

Clarity looks where it looks for the build tools (the PATH and the folders the official installers use), and in the folders npm and the other installers put these programs in:

- Windows: npm's global folder `%APPDATA%\npm` (or the one `NPM_CONFIG_PREFIX` names), Node's own folder in Program Files and nvm for Windows' link, WinGet's links, pnpm, Scoop and Volta.
- Linux and macOS: npm's global folder `<prefix>/bin` (`/usr/local/bin`, the one `NPM_CONFIG_PREFIX` names, or `~/.npm-global/bin`), each Node version of nvm, newest first, `~/.local/bin` (the Copilot install script's), Homebrew's `/opt/homebrew/bin`, pnpm, Volta and Bun.

An agent program that is not installed is not listed; **Clarity settings**, **Environment** says where to install it.

A connection of kind `codex` or `copilot` made in the vault before keeps working, with its `command` field when it has one. A new one is not needed and the vault refuses it.

## Signing in

- Codex signs in from the panel the first time you use it, with your ChatGPT account. `codex-acp` offers a ChatGPT login; its page does not say whether it reuses a sign-in you already made with `codex login`.
- GitHub Copilot signs in before the conversation. When it says it is not signed in, the panel shows **Sign in**, which runs `copilot login --web-flow`, which signs in through GitHub in your browser. The Copilot CLI cannot sign in during a conversation. Signing in once in a terminal with `copilot login` does the same.

The program keeps its own sign-in. To sign out, use the vendor's program.

## What it may do

- Use Clarity's tools: look at the project, read and write its files (each write is checked at once), check it, generate the application's code, write migration steps, and show a window in Preview or a diagram when its turn ends.
- Write the project's files by the same rule as Clarity's own `write_file` tool. Modules, the project file, the README, the resources' texts and diagrams it writes with Clarity's tools, which check each write; windows, views, reports and the guidance only through their own tools. When it tries to write or edit one of these itself, that is refused and it is told which tool to use.
- Edit a custom part's or a library's code itself, the files in its folder, as `write_file` writes them; never the file Clarity writes there from the declaration, nor its build's output. Any other write, in the project's folder or outside it, is refused.
- Ask before it runs anything: every command the agent wants to run comes to you as **Allow** or **Deny**, and Deny is the default.

One exception: Codex runs commands that only read (listing or reading files, for one) inside its own read-only sandbox without asking. Anything that would change something still asks.

Once a chat has started, a row under the panel's header shows the program's own choices for that chat: normal or Plan, the model, and for Codex the reasoning effort. A choice holds for that chat only; **New** starts on the program's defaults again. Modes that would let the program change files without asking (Codex's Workspace access, Auto review and Full access, Copilot's Autopilot) are not offered.

**Undo** takes back a whole turn at once, whether the agent wrote through Clarity's tools or by itself. Files that are not text, such as pictures and sounds, cannot be put back; the panel names them when a turn changed one. A migration step follows the turn's changes to your entities once, when the turn ends.

## Claude

Claude is not offered in the panel this way until Anthropic allows other programs to use its sign-in. Until then, use Claude in the panel with an API key ([Connections](ai.connections.md)), or Claude Code in its own window with Clarity's tools through `clarity mcp` ([Clarity in your own agent](ai.mcp.md)).

## Watching outside agents

**Clarity settings**, section **Agent**, shows **Agent activity**: each tool call an outside agent makes, as it happens: which program, the tool, the file, and whether it worked. It never shows what a file holds, and it keeps only the last calls.

A program in the panel works only while you chat with it, and asks before every command it runs. The **MCP** switches in Clarity's status bar turn off an agent in its own window that reaches Clarity through `clarity mcp`, not the panel (see [Turning it off and watching it](ai.mcp.md#turning-it-off-and-watching-it)).
