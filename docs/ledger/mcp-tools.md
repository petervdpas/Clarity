# Shared tools and the MCP server

Status: partial. One set of design-and-generate tools for every agent (Clarity's own agent, the outside agents Codex and Copilot over ACP, any MCP client through `clarity mcp`); the questionnaire is not built.

Progress: 70%

## Code
- Spec: structural edits shared with the designer (`clarity_spec::edit`); field renames `crates/clarity-spec/src/field_rename.rs`; window and view renames `page_rename.rs` (`rename` kinds window and view, `crates/clarity-tools/src/lib.rs:341-348`); every edit fitted, parts cut to what holds them (`fit.rs`, called from `crates/clarity-tools/src/edits.rs:166-177`)
- Tools: `crates/clarity-tools/src/lib.rs` (64 tools, none builds, runs or touches a database; general tools :818-941; `write_file` refuses `.cxaml`, declaration-written files and `agent/guidance.md` :500-514; `manual` reads the manual's pages, the only documentation agents read; the `design` and `reference` tools went with DESIGN.md and LANGUAGE.md on 2026-11-13), `structural/model.rs`, `structural/windows.rs`, `structural/frame.rs`, `structural/guidance.rs` (`guidance`, `set_guidance`; see [app-mcp](app-mcp.md)), `structural/places.rs` (names to places), `edits.rs` (one edit path with the designer), `steps.rs` (automatic migration steps, folded while the local SQLite has applied fewer), `manual.rs`, `diagrams.rs`, `generate.rs`, `turn.rs`
- MCP server: `crates/clarity-mcp/src/lib.rs` on `rmcp` over stdio (`LEFT_OUT` :52, `WITHOUT_PROJECT` :56, `WHEN_OFF` :60, `for_turn` :144, call log without file contents :191-201)
- CLI: `clarity mcp [--turn <file>]` (`crates/clarity-cli/src/main.rs:121`, :339); the open project from `ide.json` (`crates/clarity-project/src/ide.rs`)
- Repo setup: `.mcp.json` starts `node mcp.mjs` as `clarity-cdev` (since 2026-11-17: Windows cannot start `./mcp.sh`, which now hands over to `mcp.mjs`) (everything the repository builds, the DevTools tools too), beside the installed Clarity's `clarity` (user scope); it builds `clarity-cli` when behind and runs `clarity mcp`. Two names since 2026-11-16: one shared name let hidden settings decide which ran (user: "isnt it better to make two... clarity and clarity-cdev?"); the debug build also names itself `clarity-cdev` in its server info and instructions (`ClarityServer::name`, `devtools::NAME`), a chat turn and the release stay `clarity` (test `the_developers_build_names_itself_clarity_cdev`)
- The repo build (clarity-cdev) also has the DevTools tools `build_piece`, `release_state`, `disk_state`, `remove_folder` and `write_release_plan` (`crates/clarity-mcp/src/devtools.rs:20-28`, compiled only with `debug_assertions`, `lib.rs:24-30`): the one exception to design and generate only, never in the installed `clarity`; see [devtools](devtools.md)
- Claude Code hook: `~/.claude/hooks/clarity-tools-only.sh`, user-level, outside the repository; denies Write/Edit/MultiEdit on `.clarity` and `.cxaml` outside the Clarity repo
- IDE: `app/src-tauri/src/agent.rs` keeps only build, backups, show window, show diagram
- Docs: manual/ai.mcp.md, manual/ai.agent.md

## Tests
- `crates/clarity-tools/src/lib.rs:1463` (`the_shared_tools_design_and_generate_only`); unit tests in `structural/model.rs`, `windows.rs`, `frame.rs`, `steps.rs`
- `crates/clarity-tools/tests/turn.rs`, `crates/clarity-mcp/tests/server.rs`, `crates/clarity-cli/tests/mcp.rs`
- conformance: none

## Open
- The questionnaire: NOT BUILT; see Design.
- Tool schemas listing the project's own entities, windows and parts as choices: PARTIAL, only fixed lists are enums; see Design.
- `diagram_view` gives the class view too: PARTIAL; see [diagrams](diagrams.md).
- The Claude Code hook: PARTIAL. Nothing in the repository installs or documents it; a shell write still slips past it.

## Design
The questionnaire. User: "a questionaire to tell the ai what components to use and a first explaining what the app should do?!", "agent asks the question to get its optimized prompt and it also saved in project. So a form pops up when a user asks to make him a new app."
- When the person asks an agent for a new app, the agent calls the questionnaire tool before it designs anything.
- The form asks first what the app should do, then which components to use. Components are everything Clarity offers (parts, custom parts, data sources, engines, devices, sound, video, pictures), read from Clarity's own catalog, so the form grows with Clarity.
- The form is Clarity's own window ("mcp opens clarity own window"): `clarity mcp` starts a separate form program on Clarity.UI and waits for its answers, so the CLI links no UI library. The IDE's agent shows the same form. Defined once in Clarity, its texts from translations.
- The tool turns the answers into the prompt it gives back to the agent, and saves them in the project's README.md under fixed headings, which `project_overview` reads.
- Only for new apps. A later change of goal is an edit of README.md (or an extra document) like any other file.
- The questionnaire also clarifies other things, with decision trees. User: "I want to offer forms (from the chat inside clarity or the mpc client (claude terminal)) to clarify things and fill out decision tree(s)". Any agent calls it when it needs to clarify something; a decision tree picks the next question from the answers so far.
- Where it shows: both. A simple question goes as the protocol's own form (MCP elicitation, ACP's equivalent), drawn by the client: flat fields (text, number, yes or no, a choice), one form per step of a tree, reaching a client without Clarity. When Clarity runs, the rich form opens in Clarity's own window as a centered pop-up over the IDE, like Clarity's dialogs, one step of a tree at a time, with the catalog and pictures. User: "follows standards and within clarity it can be front-runner by using centered popup screens/forms".
- The guidance wizard of [app-mcp](app-mcp.md) (step 5) is built on the same form.
- Open questions: who writes the decision trees (Clarity per kind of question: a new app, a new window, data; or the agent per case); whether the answers of a clarifying form are kept in the project or only go back to the agent; where the questionnaire falls beside [reports-pdf](reports-pdf.md) in the order of work (the user's choice).

Tool schemas that list the project's names. Each structural tool's schema lists what exists as an enum (the entities, windows and parts of the open project, besides the fixed part kinds and field types), so a small model picks a name instead of typing one. A name outside the list is refused with the names there are, never a guess.

## Conflicts
- The plain types window data can hold are written twice: `PLAIN_DATA_TYPES` in `crates/clarity-tools/src/structural/windows.rs` and `PLAIN_TYPES` in `runtimes/web/src/screen/windowData.ts`; equal today.

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
- 2026-11-12: a view's tells through the tools: `add_tell` and `remove_tell` on a view (edit `SetPageTell`), and `wire_event` with `view` wires a tell on the content's view line with a handler that takes the tell's values (edit `SetViewTell`). Asked for the maintenance review slide-out as a view that tells its window `saved`. Tests `a_view_tells_and_its_window_hears_it` (frame.rs), `the_designer_edits_what_a_view_tells_and_who_hears_it` (clarity-spec tests/content.rs).
- 2026-11-12: a refused edit names the file and line of the error it would add (`refusal` in `edits.rs`); `remove_data` of data the code still uses says the code uses it. Test `data_fields_and_tabs_go_on_by_name`.
- 2026-11-12: Clarity's own words of parts in a sidebar or footer (a tree's Expand and Collapse) were added but then warned "not used anywhere": the check looked only at a window's grid. It walks every list of parts now (`part_lists` in `translations.rs`). Test `a_tree_in_a_sidebar_uses_clarity_s_own_words` (clarity-spec tests/trees.rs).
- 2026-11-13: a view's tells BUILT in the designer (`app/src/lib/PageTells.svelte`, `ViewLineTells.svelte` in `PageProperties.svelte`; test `app/src/lib/viewTells.test.ts`); see [by-hand](by-hand.md).
- 2026-11-14: the server tells Clarity's icon in its info (`icons`: the 128x128 PNG and the SVG of `app/src-tauri/icons`, embedded as data URIs; `crates/clarity-mcp/src/lib.rs` `icons`; test `the_server_tells_clarity_s_icon` in `crates/clarity-mcp/tests/server.rs`), so a client such as VS Code shows it. User: "can a mcp server get an icon?", "yes, add it after the release".
- 2026-11-14: four tool fixes found reworking the maintenance example, each with a test: `add_enum` leaves no key of its starting value (`structural/model.rs` `without_key`); renaming an entity renames it where code names it as a type (`clarity-spec/src/rename.rs` `type_uses`, test `renames_an_entity_where_code_names_it_as_a_type`); an edit is refused only for an error in a file it writes, what it leaves elsewhere is made and reported (`edits.rs`, test `a_removal_that_leaves_code_behind_is_made_and_reported`); a step Clarity wrote with `migrate` under a title folds while no database ran it (`steps.rs` `written_by_clarity`, test `a_titled_step_takes_the_next_change_while_no_database_ran_it`). User: "fix 'em felix".
- 2026-11-14: a value of an enum is renamed everywhere it is named (`crates/clarity-spec/src/value_rename.rs`): its declaration, the defaults of fields, settings and command options, code typed as the value (bare or `Enum.Value`), a window's data default and a value a part's event gives a handler. The rename tool takes `kind: value`, `change_enum_value` with a new name is the same rename, and the IDE's enum editor renames through `rename_value`. The step says `rename value Enum.Old to New` and gives every stored record the new name, in a column of the enum and inside a list of its values (`crates/clarity-gen/src/schema.rs`, `crates/clarity-gen/tests/gen/value_renames.rs`); the IDE keeps its value renames for the next step (`app/src-tauri/src/migrations.rs`).

## Was
- DESIGN 17.3, roadmap A1, M8 (MCP half)
- MCP-PLAN.md: Goal, Decisions, Tools in the first version, Order of work steps 1-6, "Decided at their step" (built); step 7, "The questionnaire", "Forms to clarify, with decision trees" (now Design)
- AGENT-TOOLS-PLAN.md: step 1, "How it is checked", "Decided" (built); step 1's schemas listing existing names (now Design)
- memory: mcp-plan.md, agent-tools-plan.md, all-ai-use-tools.md
