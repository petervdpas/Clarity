# Outside agents in the panel (ACP)

Status: partial. Codex and Copilot run in the IDE's agent panel on the person's own subscription and login, through the Agent Client Protocol, with Clarity's tools.

Progress: 70%

## Code
- Spec: none
- Generators: none
- Runtimes: none
- Clarity.UI: none
- ACP client: `crates/clarity-acp/src/lib.rs` (JSON-RPC lines over stdio, written by hand; fs reads and writes inside the project only :444-503; `clarity_s_own` refuses `.clarity`, `.cxaml`, README.md, `resources/*.md` :636; no terminal :293-299, :471), `crates/clarity-acp/src/vendors.rs` (Codex `codex-acp`, Copilot `copilot --acp --stdio`; `install_page` :42; `ask_first_mode` Codex read-only :84; `safe_choices` :110)
- IDE: `app/src-tauri/src/acp.rs` (Allow/Deny per permission request; tools through `clarity mcp --turn <file>` :177; `agent_sign_in`); `app/src/lib/AgentPanel.svelte`, `OutsideAgentsSettings.svelte` (on/off and the activity list)
- The turn as one Undo step (snapshot and compare): `crates/clarity-tools/src/turn.rs`
- Tools offered: [mcp-tools](mcp-tools.md)
- Docs: manual/ai.your-agent.md

## Tests
- `crates/clarity-acp/tests/client.rs` (against a fake agent program, no vendor login), `crates/clarity-acp/tests/vendors.rs`
- `crates/clarity-tools/tests/turn.rs`
- `crates/clarity-project/tests/activity.rs` (`a_writer_waits_while_the_log_keeps_being_written` :122, the activity log's lock wait)
- conformance: none

## Open
- Codex and Copilot checked by hand against the real programs: UNTRIED. Open notes in `crates/clarity-acp/src/vendors.rs:84`.
- Claude Code as an outside agent: NOT BUILT, waits on Anthropic's position on subscription sign-in. Until then Claude works through `clarity mcp` in Claude Code, or as [agent](agent.md) with an API key.
- A missing program links to Clarity's manual page: PARTIAL. It links the vendor's install page (`vendors.rs:42-51`).
- Panel, picker and dialog texts from translations: NOT BUILT (see [ide-shell](ide-shell.md)).

## Conflicts
- none

## Decisions
- 2026-11-09: the panel talks to an agent the person already pays for, with that vendor's login, no API key in Clarity. User: "but I asked for an easier way to use ai in clarity, without and api key!!!!!!!!", and "but I want it inside the ide!!"
- 2026-11-09: ACP spoken by Clarity itself with serde_json, no ACP crate (the `agent-client-protocol` crate turned on serde_json `preserve_order` workspace-wide and failed eight test targets in v0.36.0). User: first "use the crate, standards are better", then "go non standard use json_something then!"
- 2026-11-09: outside agents get `clarity mcp`'s tools plus show window and show diagram; their own edits allowed inside the project, checked and undone with the turn; Claude not offered yet. User: "also preview and diagrams, edits allowed, wait for anthropic (api-key for now)"
- 2026-11-09: a setting to turn outside agents off, and an activity list fed by the log `clarity mcp` writes (no file contents). User: "Is it something I turn on and off in the ide settings? Can I monitor it somehow?", then "yes, add both to the plan and build them"
- 2026-11-12: a writer of the activity log (`crates/clarity-project/src/activity.rs`, `turn`) waits for its turn until nothing was written to the log for `LOCK_WAIT`, not 2 seconds from when it asked: under the checks' load a busy log was taken for a hung one and a call left out (`calls_at_the_same_moment_each_get_their_own_number` failed in the Workspace job). Test `a_writer_waits_while_the_log_keeps_being_written`. User: "why does this always happen at this step in this job?????"
- 2026-11-09: the IDE must run; an agent works only on the project the running IDE has open (see [mcp-tools](mcp-tools.md)). User: "the clarity project that is currently open in the ide!"
- 2026-11-09: Codex starts in its read-only sandbox (`INITIAL_AGENT_MODE=read-only`); everything else comes to Clarity's Allow/Deny, default Deny. User: "Accept, read-only sandbox"
- 2026-11-10: every AI changes Clarity's own files only through the tools; outside agents are refused when they write them and get no mode that writes past the tools. User: "all ai must use tools!"

## Was
- ACP-PLAN.md steps 1-7, "How it is built", "Decided"
- memory: acp-plan.md
