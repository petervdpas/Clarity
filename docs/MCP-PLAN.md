# Clarity as an MCP server

A plan, worked out with the user 2026-11-09. Steps 1 to 6 are built (2026-11-09); step 7, the questionnaire, is next.

## Goal

The user's words: "I think we need to make a clarity-mcp!"

And on what an agent may do through it: "I want an ai only be able to design and generate! build and run is for the user... later I might add those too. I want to add another tool for the ai in conjunction with clarity... a questionaire to tell the ai what components to use and a first explaining what the app should do?!"

So a developer starts `clarity mcp` from the agent they already pay for (Claude Code, Codex, Copilot, Claude Desktop). That agent then designs a Clarity project and generates its code using Clarity's own tools, under its own vendor's sign-in. Clarity does not depend on any vendor letting it register.

## Decisions

- **A crate plus a CLI command.** `crates/clarity-mcp` holds the server; the `clarity` CLI starts it as `clarity mcp [project file]`, speaking MCP over stdio. It ships with the IDE's CLI and works on Linux, Windows and macOS without the IDE open.
- **One set of tools for both agents.** `ProjectTools` moves out of `app/src-tauri/src/agent.rs` into a shared crate, so the IDE's agent and the MCP server call the same code. The IDE keeps only what needs the IDE (Preview, the Output panel, Allow/Deny).
- **Design and generate only.** The agent can read, write, check and generate. It never builds, runs, restores or deletes. Build and run stay with the user.
- **Which project.** The user's words: "the clarity project that is currently open in the ide!". The server works on the project open in the Clarity IDE, nothing else. The IDE records its open project in its app settings and `clarity mcp` reads it on every call, so switching projects in the IDE switches the agent too. With no project open, the tools answer that a project must be opened in Clarity first.
- **Model changes go through migrations.** The user's words on `migrate`: "keep it... and mcp must do app model changes over migrations!" Asked how, the user chose: "b, write the step automatically". When the agent changes the app's data model (entities, their fields) through any tool, the server writes the migration step for that change itself, as the Migrate button does, so the model and the steps are never out of step. `migrate` stays as a tool too (for a title, or a source the agent names).
- **No guessed renames.** The user's words: "no heuristics only determinism!". The server's steps never use `proposed_renames`. A rename reaches a step only when the agent asks for it by name with a `rename` tool (entity, enum, and field, which `rename.rs` does not cover yet); the tool renames every reference through the parsed model and the step records it as a rename, so the column keeps its data. A field that is removed and another added is exactly that in the step: a drop and an add.
- **Steps fold while unapplied.** The server keeps adding to its own latest step as long as no database has applied it, so one task leaves one step.
- **Offline.** The server needs no internet; it reads the manual, design and language reference built into the CLI.

## Tools in the first version

Taken from the IDE agent's tools, same names and inputs:

| Tool | What it does |
|---|---|
| `project_overview` | The project file, its modules, README and problems |
| `read_file`, `write_file` | Read or write a project file (Clarity modules, parts, README) |
| `check` | The project's problems |
| `parts` | The parts a window can hold, with properties, sizes and events |
| `manual`, `reference`, `design` | Sections of the manual, LANGUAGE.md and DESIGN.md |
| `api`, `pick_api` | An OpenAPI data source's operations; pick which ones the project uses |
| `migrate` | Write the next migration step from the project's entities (writes a file, runs nothing) |
| `diagram_view`, `write_diagram`, `diagram_proposals`, `apply_diagram` | Mermaid diagrams of the project |
| `new_project` | Make a new project in the projects folder |
| `generate` (new) | Generate the project's source tree for its stack, as `clarity gen` does; no build |
| `read_generated` | Read a generated file |

Left out for now: `build`, `build_output`, `list_backups`, `backup_data`, `restore_backup` (they execute or touch data), and `show_window`, `show_diagram` (they need the IDE).

## The questionnaire

The user's words: "agent asks the question to get its optimized prompt and it also saved in project. So a form pops up when a user asks to make him a new app."

And on the details: "1. yes, 2. it uses clarity's own form and translates that to chat/prompt, 3 components means everything clarity has to offer, 4 form is only for new apps... goal changes are manipulations of the readme (or extra document)"

- When the user asks the agent for a new app, the agent calls a questionnaire tool before it designs anything.
- A form pops up: first what the app should do, then which components to use. Components means everything Clarity has to offer (parts, custom parts, data sources, engines, devices, sound, video, pictures and so on), taken from Clarity's own catalog so the form grows with Clarity.
- The form is Clarity's own window: `clarity mcp` opens it (Clarity.UI), whatever agent is calling; the IDE's agent shows the same form. The user's words: "mcp opens clarity own window". Defined once in Clarity. The tool translates the answers into the optimized prompt it gives back to the agent, and the agent designs from that prompt.
- The answers are saved in the project's README.md under fixed headings, which `project_overview` already reads.
- The form is only for new apps. When the goal changes later, the agent edits README.md (or an extra document) like any other file.
- The questions and the form texts come from translations, like every other text in Clarity.

## Order of work

Agreed 2026-11-09 (user: "agree"), interleaved with the `.cxaml` refactor ("Next: no translator between design and code" in CXAML-SOURCE-PLAN.md) because two steps touch pages:

1. Move `ProjectTools` and its tests into a shared crate; the IDE uses it from there (TDD, IDE agent tests stay green). Its instructions teach `.cxaml`, which they do not yet.
2. Add the `generate` tool.
3. The `.cxaml` refactor (CXAML-SOURCE-PLAN.md), so renames and pages work on the model.
4. The `rename` tool (fields added to `rename.rs`, on model renames), automatic folded steps without guessed renames.
5. `crates/clarity-mcp`: stdio server, tool list from the shared tools minus the left-out ones, a test that lists every offered tool (like `no_tool_executes_anything`).
6. `clarity mcp` in the CLI; the IDE records its open project (one key, read-modify-write); manual page `ai.mcp.md` with setup for Claude Code, Codex and Copilot (in English, as the whole manual is).
7. The questionnaire tool and Clarity's form (a `.cxaml` page, so after the refactor). Where it falls beside the clarity-pdf crate (REPORT-PLAN.md) is the user's choice.

## Decided at their step

Asked 2026-11-09, after v0.34.0:

- The protocol: the user first chose "Own JSON-RPC (Recommended)", then asked "I know what I chose, but isnt using standards better?" and, told tokio is already in the workspace through the IDE, said "tokio is already in eh". So `clarity-mcp` uses `rmcp`, the official Rust MCP SDK, over stdio.
- The form: the user chose "Separate form program (Recommended)". `clarity mcp` starts a small program on Clarity.UI and waits for its answers; the CLI links no UI library.
- Folding steps: the user chose "Local DB check (Recommended)". The agent's latest step is folded while the source's local SQLite file is missing or has applied fewer steps (`clarity_schema.version`); for a server database, which cannot be seen, every change gets a new step.
- Which agents: the user chose "Both agents (Recommended)". Automatic steps and the `rename` tool work the same for the IDE's agent and through `clarity mcp`.
