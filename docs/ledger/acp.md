# Outside agents in the panel (ACP)

Status: partial. The outside agents Codex and Copilot run in the IDE's agent panel on the person's own subscription and login, through the Agent Client Protocol, with Clarity's tools.

Progress: 70%

## Code
- Spec: none
- Generators: none
- Runtimes: none
- Clarity.UI: none
- ACP client: `crates/clarity-acp/src/lib.rs` (JSON-RPC lines over stdio, written by hand; fs reads and writes inside the project only, `Client::handle`; a write goes through the tools' one rule and write, `clarity_tools::files::write_beside_tools`, and an edit permission is rejected by the same rule, `changes_clarity_s_own` and `in_project`; no terminal), `crates/clarity-acp/src/vendors.rs` (Codex `codex-acp`, Copilot `copilot --acp --stdio`; `install_page`; `ask_first_mode` Codex read-only; `safe_choices`; the scan: `search_folders` (the build tools' `clarity_gen_core::tools::installers_path`, then `install_folders`: npm's global folders, nvm, Volta, pnpm, Bun, WinGet, Scoop, the install script's and Homebrew's), `scan`, `copies_in`, `installed` (the Environment choice, else the scan), `find` (an old vault connection's command, else the Environment choice, else the scan))
- IDE: `app/src-tauri/src/acp.rs` (Allow/Deny per permission request; tools through `clarity mcp --turn <file>`; `agent_sign_in`; `agents_installed` for the picker; `vendor_of`: a vault connection by its kind, else the agent of that key); `app/src/lib/AgentPanel.svelte` and `agent.ts` `agentPicks` (the vault's connections, then the agents installed), `AgentActivityList.svelte` (the activity list in Clarity settings, Agent)
- The chosen program: `crates/clarity-gen-core/src/tools/environment.rs` (`choose_agent`, `chosen_agent`; `agents` in environment.json, apart from the build tools); `app/src-tauri/src/environment.rs` (`AgentView`, `choose_agent`); `app/src/lib/EnvironmentSettings.svelte` (Agents)
- The vault refuses a new connection of kind codex or copilot and reads old ones: `app/src-tauri/src/vault.rs` `save_connection`, `has_connection`
- The turn as one Undo step (snapshot and compare): `crates/clarity-tools/src/turn.rs`
- Tools offered: [mcp-tools](mcp-tools.md)
- Docs: manual/ai.your-agent.md, manual/ai.connections.md, manual/project.run-build.md (Environment, Agents)

## Tests
- `crates/clarity-acp/tests/client.rs` (against a fake agent program, no vendor login), `crates/clarity-acp/tests/vendors.rs` (the scan with fake folders: `a_scan_gives_each_agent_installed_with_its_program`, `the_agents_install_folders_on_each_system`, `the_agents_installed_count_the_chosen_program`, `a_chosen_path_is_used_as_it_is`)
- `crates/clarity-gen/tests/gen/environment.rs` (`an_agent_s_chosen_program_is_kept_apart_from_the_build_tools`)
- `app/src-tauri/src/acp.rs` (`the_panel_s_connection_names_its_outside_agent`), `app/src-tauri/src/vault.rs` (`outside_agents_need_no_new_connection_but_old_ones_keep_working`), `app/src/lib/agent.test.ts` (the agent programs found join the picker)
- `crates/clarity-tools/tests/turn.rs`
- `crates/clarity-project/tests/activity.rs` (`a_writer_waits_while_the_log_keeps_being_written` :122, the activity log's lock wait)
- conformance: none

## Open
- ACP agents found by a scan, not vault entries: DONE 2026-11-23; see Decisions. Not tried by hand on Windows and macOS.
- Codex and Copilot checked by hand against the real programs: UNTRIED; see Design.
- Claude Code as an outside agent: NOT BUILT; see Design.
- A missing program links to Clarity's manual page: PARTIAL. It links the vendor's install page (`Vendor::install_page`).
- Panel, picker and dialog texts from translations: NOT WANTED (the IDE stays English; see [ide-shell](ide-shell.md)).

## Design
The hand check. Codex and Copilot, each with its start command, its ask-first mode and its sign-in, tried against the real programs (the tests use a fake agent program). The open notes are in `crates/clarity-acp/src/vendors.rs` (`ask_first_mode`).

Claude Code as an outside agent. Not offered until Anthropic's position on subscription sign-in allows it (or Anthropic approves): its Agent SDK page says other products may not offer claude.ai login without approval, while signing in to an unmodified Claude Code is allowed, and reports on embedding disagree. Until then Claude works through `clarity mcp` in Claude Code's own window, or as [agent](agent.md) with an API key. Once allowed, it joins the agent picker as Codex and Copilot did, with the same refusals and Allow/Deny.

## Conflicts
- none

## Decisions
- 2026-11-23: ACP agents need no vault entry. User: "I was wondering if acp's really need vault entries (do they actually), but that the ide can figure out which are available (from some sort of scan -- registry)", then "yes, build it that way". The panel lists the ACP agents installed on the machine (the vendors' programs on the PATH and the usual install folders, as the build tools check finds tools); a program path other than the found one is set in the Environment settings beside the other tool paths (the registry of tool paths, docs/ledger/build-tools.md "Self-contained projects"); the vault keeps only secrets; a codex or copilot connection already in the vault keeps working.
- 2026-11-09: the panel talks to an agent the person already pays for, with that vendor's login, no API key in Clarity. User: "but I asked for an easier way to use ai in clarity, without and api key!!!!!!!!", and "but I want it inside the ide!!"
- 2026-11-09: ACP spoken by Clarity itself with serde_json, no ACP crate (the `agent-client-protocol` crate turned on serde_json `preserve_order` workspace-wide and failed eight test targets in v0.36.0). User: first "use the crate, standards are better", then "go non standard use json_something then!"
- 2026-11-09: outside agents get `clarity mcp`'s tools plus show window and show diagram; their own file edits inside the project, checked and undone with the turn, nothing outside it; Claude not offered yet. Since 2026-11-10 (below) their own writes of `.clarity`, `.cxaml`, README.md and `resources/*.md` are refused; since 2026-11-23 (below) by the tools' own rule. User: "also preview and diagrams, edits allowed, wait for anthropic (api-key for now)"
- 2026-11-09: a setting to turn outside agents off, and an activity list fed by the log `clarity mcp` writes (no file contents). User: "Is it something I turn on and off in the ide settings? Can I monitor it somehow?", then "yes, add both to the plan and build them"
- 2026-11-12: a writer of the activity log (`crates/clarity-project/src/activity.rs`, `turn`) waits for its turn until nothing was written to the log for `LOCK_WAIT`, not 2 seconds from when it asked: under the checks' load a busy log was taken for a hung one and a call left out (`calls_at_the_same_moment_each_get_their_own_number` failed in the Workspace job). Test `a_writer_waits_while_the_log_keeps_being_written`. User: "why does this always happen at this step in this job?????"
- 2026-11-09: the IDE must run; an agent works only on the project the running IDE has open (see [mcp-tools](mcp-tools.md)). User: "the clarity project that is currently open in the ide!"
- 2026-11-23: an outside agent's own writes follow the one rule of Clarity's tools (`clarity_tools::files::writable`, test `which_files_an_agent_writes_is_one_rule`): it writes a file of a custom part's or a library's folder as write_file writes it (`write_text`: atomic, a newline at the end); a module, the project file, README.md or a resource text is sent to write_file, a diagram to write_diagram, the guidance, reports, windows and a part's Clarity-written file to their tools, and anything else in the project (notes, an API description) is refused, as write_file refuses it. Before, its own policy blocked only `.clarity`, `.cxaml`, README.md and `resources/*.md`. User: "the must be only one way and 'ide/agents OR WHATEVER' can hook into it!!!"
- 2026-11-09: Codex starts in its read-only sandbox (`INITIAL_AGENT_MODE=read-only`); everything else comes to Clarity's Allow/Deny, default Deny. User: "Accept, read-only sandbox"
- 2026-11-10: every AI changes Clarity's own files only through the tools; outside agents are refused when they write them and get no mode that writes past the tools. User: "all ai must use tools!"
- 2026-11-23: the setting "Let outside agents work on my project" (2026-11-09, above) is removed; the status bar's MCP switches (see [mcp-tools](mcp-tools.md)) are the one place, and Agent activity stays. User: "Remove it". A panel agent is no longer held by any switch: it is the person's own chat, works only during a turn, asks before every command (Allow/Deny) and writes Clarity's files only through the tools; the switches hold agents in their own window through `clarity mcp`.

## Was
- ACP-PLAN.md: Goal, "How it works", "Agents", "How it is built", order steps 1-3, 5, 6, "Decided" (built; manual ai.your-agent.md); step 4 (hand check) and step 7 with the Claude row (now Design)
- memory: acp-plan.md
