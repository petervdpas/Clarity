# Shared tools and the MCP server

Status: partial. One set of design-and-generate tools for every agent (the IDE's, Codex and Copilot over ACP, any MCP client through `clarity mcp`); the questionnaire is not built.

## Code
- Spec: structural edits shared with the designer (`clarity_spec::edit`); field renames `crates/clarity-spec/src/field_rename.rs`
- Tools: `crates/clarity-tools/src/lib.rs` (66 tools, none builds, runs or touches a database; general tools :807-920; `write_file` refuses `.cxaml` and declaration-written files :491-499; `design` reads `docs/DESIGN.md` by `include_str!` :33, `reference` reads `docs/LANGUAGE.md` :30), `structural/model.rs`, `structural/windows.rs`, `structural/frame.rs`, `structural/places.rs` (names to places), `edits.rs` (one edit path with the designer), `steps.rs` (automatic migration steps, folded while the local SQLite has applied fewer), `manual.rs`, `diagrams.rs`, `generate.rs`, `turn.rs`
- MCP server: `crates/clarity-mcp/src/lib.rs` on `rmcp` over stdio (`LEFT_OUT` :27, `WITHOUT_PROJECT` :31, `WHEN_OFF` :35, `for_turn` :101, call log without file contents :126)
- CLI: `clarity mcp [--turn <file>]` (`crates/clarity-cli/src/main.rs:318`); the open project from `ide.json` (`crates/clarity-project/src/ide.rs`)
- Claude Code hook: `~/.claude/hooks/clarity-tools-only.sh`, user-level, outside the repository; denies Write/Edit/MultiEdit on `.clarity` and `.cxaml` outside the Clarity repo
- IDE: `app/src-tauri/src/agent.rs` keeps only build, backups, show window, show diagram
- Docs: manual/ai.mcp.md, manual/ai.agent.md

## Tests
- `crates/clarity-tools/src/lib.rs:1430` (`the_shared_tools_design_and_generate_only`); unit tests in `structural/model.rs`, `windows.rs`, `frame.rs`, `steps.rs`
- `crates/clarity-tools/tests/turn.rs`, `crates/clarity-mcp/tests/server.rs`, `crates/clarity-cli/tests/mcp.rs`
- conformance: none

## Open
- `design` tool and the ledger: NOT BUILT. `crates/clarity-tools/src/lib.rs:33` embeds `docs/DESIGN.md` (and :1387 tests its roadmap section); removing DESIGN.md breaks the build until the tool reads the ledger.
- Questionnaire (MCP-PLAN step 7): NOT BUILT. A tool the agent calls for a new app; a separate form program on Clarity.UI started by `clarity mcp`; answers into README.md under fixed headings; same form in the IDE agent; texts from translations.
- Forms to clarify with decision trees: NOT BUILT. Simple questions as the protocol's own form (MCP elicitation, ACP's equivalent); rich form in Clarity's own centered pop-up when Clarity runs.
- Who writes the decision trees (Clarity per kind of question, or the agent per case): needs decision.
- Whether clarify-form answers are kept in the project or only go back to the agent: needs decision.
- Where the questionnaire falls beside clarity-pdf ([reports-pdf](reports-pdf.md)): the user's choice.
- A view's tells in the IDE designer (the view's Tells, the view line's handlers): NOT BUILT; the tools and edits have them.
- Tool schemas listing existing entities, windows and parts as choices: PARTIAL. Only fixed lists are enums.
- `diagram_view` gives the class view too: PARTIAL; see [diagrams](diagrams.md).
- The Claude Code hook: PARTIAL. Nothing in the repository installs or documents it; a shell write still slips past it.

## Conflicts
- The plain types window data can hold are written twice: `PLAIN_DATA_TYPES` in `crates/clarity-tools/src/structural/windows.rs` and `PLAIN_TYPES` in `runtimes/web/src/screen/windowData.ts`; equal today.
- DESIGN.md:2406-2413 (17.3) lists spec_for, directives_for, module_map, check(diff), tdd_state, explain_violation; `crates/clarity-mcp/src/lib.rs` serves the shared tools.
- DESIGN.md:2439 describes `clarity-tools` as running native toolchains; it holds the agents' design tools, and runs nothing.
- DESIGN roadmap A1 (DESIGN.md:2534) and M8 (:2547) show no status; the server is built.

## Decisions
- 2026-11-09: an MCP server in a crate plus `clarity mcp`. User: "I think we need to make a clarity-mcp!"
- 2026-11-09: agents design and generate only; build and run stay with the person. User: "I want an ai only be able to design and generate! build and run is for the user... later I might add those too."
- 2026-11-09: the server works on the project open in the IDE, read on every call. User: "the clarity project that is currently open in the ide!"
- 2026-11-09: model changes write their migration step automatically. User: "keep it... and mcp must do app model changes over migrations!", then "b, write the step automatically"
- 2026-11-09: no guessed renames; a rename only through the `rename` tool by name. User: "no heuristics only determinism!"
- 2026-11-09: `rmcp`, the official Rust SDK. User: "tokio is already in eh"
- 2026-11-09: steps fold while the local SQLite has applied fewer (server databases: a new step per change), for both agents. User chose "Local DB check (Recommended)" and "Both agents (Recommended)"
- 2026-11-09: the questionnaire is Clarity's own form, for new apps only; later goal changes edit README.md. User: "agent asks the question to get its optimized prompt and it also saved in project. So a form pops up when a user asks to make him a new app.", "1. yes, 2. it uses clarity's own form and translates that to chat/prompt, 3 components means everything clarity has to offer, 4 form is only for new apps... goal changes are manipulations of the readme (or extra document)", "mcp opens clarity own window"; chose "Separate form program (Recommended)"
- 2026-11-10: clarify forms from both the chat and MCP clients, protocol form and Clarity's window. User: "and like the questionaire for a new app, I want to offer forms (from the chat inside clarity or the mpc client (claude terminal)) to clarify things and fill out decision tree(s)", "then do both... for better ux. I think this is an area that needs to evolve ftm.", "follows standards and within clarity it can be front-runner by using centered popup screens/forms... see it as my view on improved ux and hid"
- 2026-11-10: structural tools by name so weaker models choose what, never how. User: "Can I hand it tools that it becomes better at the tasks without changing the llm?"
- 2026-11-10: windows and views only through the tools, covering every designer edit. User: "add and change only through tools, start with entities, fields and parts", then "before we test this this, the toolset must cover clarity as a whole"
- 2026-11-10: one ERD per project; data changes go into it. User: "it should make one erd it it doesnt exist, and add or change if it does"
- 2026-11-10: a data change is done when the windows show it (`place_field`). User: "it didnt add the field in the form!!!... I think ai, should learn the 'normal' app building flow?"
- 2026-11-10: every AI uses the tools (ACP refusals, the Claude Code hook). User: "all ai must use tools!"

- 2026-11-12: `add_data` declares what the designer's Data panel does: a record of an entity or a record type, a list of them, or a value (a plain type or an enum); it took only entities, which blocked the maintenance example. Test `data_fields_and_tabs_go_on_by_name`. User: "A, fix the tool... and we continue tomorrow, bc it wont be in this instance"

- 2026-11-12: `change_field` with only `length` (or only `precision` or `scale`) keeps the field's type and changes its size; it said "nothing changed". Test `entities_and_fields_by_name`.
- 2026-11-12: a view's tells through the tools: `add_tell` and `remove_tell` on a view (edit `SetPageTell`), and `wire_event` with `view` wires a tell on the content's view line with a handler that takes the tell's values (edit `SetViewTell`). Asked for the maintenance review slide-out as a view that tells its window `saved`. Tests `a_view_tells_and_its_window_hears_it` (frame.rs), `the_designer_edits_what_a_view_tells_and_who_hears_it` (clarity-spec tests/content.rs). The IDE designer has no property for either yet.
- 2026-11-12: a refused edit names the file and line of the error it would add (`refusal` in `edits.rs`); `remove_data` of data the code still uses says the code uses it. Test `data_fields_and_tabs_go_on_by_name`.
- 2026-11-12: Clarity's own words of parts in a sidebar or footer (a tree's Expand and Collapse) were added but then warned "not used anywhere": the check looked only at a window's grid. It walks every list of parts now (`part_lists` in `translations.rs`). Test `a_tree_in_a_sidebar_uses_clarity_s_own_words` (clarity-spec tests/trees.rs).

## Was
- DESIGN 17.3, roadmap A1, M8 (MCP half)
- MCP-PLAN.md steps 1-7, "The questionnaire", "Forms to clarify, with decision trees", "Decided at their step"
- AGENT-TOOLS-PLAN.md step 1, "Decided"
- memory: mcp-plan.md, agent-tools-plan.md, all-ai-use-tools.md
