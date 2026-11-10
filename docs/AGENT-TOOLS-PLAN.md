# Tools that make any agent better

A plan, written 2026-11-10. Step 1 is built (2026-11-10, unreleased): 42 structural tools in crates/clarity-tools/src/structural (model, windows, frame), one edit path shared with the designer (crates/clarity-tools/src/edits.rs), and `write_file` refusing `.cxaml`. Steps 2 and 3 wait.

## Goal

The user's question, asked in another session: "say I want to connect a local llm ollama (or what's it called) that actually isnt that strong on coding. Can I hand it tools that it becomes better at the tasks without changing the llm?" And on why it came here: "I've sent it here bc you/me was busy with mpc and I believe this is in the same line!".

So the answer is in the tools, not the model: tools that do the Clarity for the agent, so a weak model only has to choose what to change, never how to write it. They go into the shared tool set (crates/clarity-tools), so every agent gains at once: Clarity's own agent, Codex and Copilot in the chat panel (docs/ACP-PLAN.md), Claude Code and others through `clarity mcp` (docs/MCP-PLAN.md), and a local model once Clarity can talk to one (DESIGN roadmap A2).

## What the agents have now

- They change the project by writing whole files with `write_file`: correct Clarity syntax and `.cxaml` markup are theirs to get right, and the check after each write tells them what they got wrong.
- The designer already changes the project through structural edits (`clarity_spec::edit::Edit`): AddEntity, AddField, AddEnum, AddVariant, AddWindow, AddPage, PlaceField, CreateWidget, UpdateWidget, AddMenu, AddTab, AddData, SetGrid, Remove and Batch, each one checked. They name what they change by its place in the text (`entity: u32`), as the designer knows it.
- `rename` is the one structural tool agents have (entity, enum, field, by name).
- Clarity's own agent talks only to Anthropic models (`clarity-agent`, one `Model` trait with one method, `reply`; `anthropic.rs` is about 200 lines).

## Steps

1. **Structural edit tools, by name.** Tools such as `add_entity`, `add_field` (entity, name, type, modifiers), `add_enum` and `add_value`, `add_window` (with a template), `add_part` (window, kind, place, binding), `change_part`, `place_field`, `add_menu_item`, `remove`. A thin layer turns names (`Customer`, `customerTable`) into the places the edits want, and every tool's schema lists what is allowed as an enum: the part kinds, the field types, the entities, windows and parts that exist. Each call is checked like a designer edit, so a tool either makes a valid change or says why not, with the names there are to choose from. These help every model, Claude included. `write_file` stays for what the tools do not cover (code bodies, texts).
2. **A smaller tool set for small models.** A connection can say it is small: the agent then gets the structural tools and the reading tools only, shorter instructions, and the relevant manual section put into the prompt for the task instead of the whole index. Its context window is set as large as the model allows (Ollama's default is small and must be raised).
3. **Local models, as DESIGN's A2 has it.** `ollama` first, then `openai` for any OpenAI-compatible server (llama.cpp server, LM Studio, vLLM), each a `Model` in `clarity-agent` beside `anthropic.rs`; a connection of that kind needs an address and a model, no key. Testing a connection checks that the model answers and can call tools.

## How it is checked

- Each structural tool has tests on a project: the change it makes, a name that does not exist refused with the names there are, a value outside its enum refused.
- The shared tool list test (`the_shared_tools_design_and_generate_only`) and the IDE's (`only_the_build_tool_executes_and_it_asks_first`) grow with the new tools, which write project files only and run nothing.
- For step 3, a faked server per kind, as A2 says.

## Decided

- Errors list the names there are; they do not guess which one was meant ("did you mean ..."): the house rule of determinism.
- Windows only through the tools, and what comes first: the user said "add and change only through tools, start with entities, fields and parts". So agents add and change windows and views only with the structural tools; writing a `.cxaml` file with `write_file` is refused, with the tools to use instead. Then, before it is tried: "before we test this this, the toolset must cover clarity as a whole". So the first version covers every structural edit the designer makes (the data model, windows, views and parts, the app and its frame, events and handlers), not entities, fields and parts alone.
- Steps 2 and 3 wait: the user said "3 not yet... couldnt get the pi give any performance, need a better machine first!". A local model needs a stronger machine than the Raspberry Pi; step 2 only serves small local models, so it waits with it. Step 1 is the work, and it helps the agents used today.

- One ERD per project, the user's words: "it should make one erd it it doesnt exist, and add or change if it does". So the instructions send data changes into the project's ERD (made as `diagrams/data.mermaid` from the whole model when there is none), and `write_diagram` refuses a second ERD, naming the one there is.
- The normal app-building flow, the user's words: "it didnt add the field in the form!!!... I think ai, should learn the 'normal' app building flow?". So the instructions say a data change is done only when the windows show it: each new field goes on the forms of its entity with `place_field`, the agent names the lists that could show it, then shows the window.
- Every AI uses the tools, the user's words: "all ai must use tools!". Codex and Copilot are refused when they write Clarity's own files themselves, get no mode that writes past the tools, and a Claude Code hook refuses its own writes of `.clarity` and `.cxaml` files.

## Open questions

None left for step 1.
