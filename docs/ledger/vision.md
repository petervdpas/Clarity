# Vision

Status: partial. What Clarity is for and the rules every piece of work follows; the code follows them with the gaps under Conflicts.

Progress: 90%

## Code
- Spec: none (cross-cutting)
- Docs: `CLAUDE.md` "Principles"; `README.md`

## Tests
- none of its own; the principles are enforced per piece (conformance files, fail-safe sweeps)

## Open
- The name: "Clarity" is also the Stacks smart-contract language; keep it or rename before publishing (DESIGN 20 question 1). Not decided.

## Conflicts
- DESIGN 1 (Purpose) still states the goal as keeping control over AI-written code through a rule engine; the built product is the language, the designer and the transpiler; the rule engine was struck (Decisions, 2026-11-12). README.md:5 says the same old goal ("keeps humans in control of AI-assisted codebases").
- memory clarity-direction.md names a shared web renderer and Avalonia as UIs; both were replaced by Clarity.UI (2026-11-02, 2026-11-03; see [clarity-ui](clarity-ui.md)).
- CLAUDE.md:21 says Clarity's own agent never executes anything; it builds and runs after asking (see [agent](agent.md)).
- DESIGN 1.2 says an AI's changes are undone by Undo; an outside agent's turn is not (see [mcp-tools](mcp-tools.md)).

## Decisions
- 2026-10-16: one language for UI and behavior, a designer first, transpiled to real code on each stack; form and browse are templates, not screen kinds. User: "its ui designer with later code behind", "clarity get transpiled into the real frontend and backend code"
- 2026-10-16: models are Clarion's data dictionary and Borland Turbo C++ Visual Edition (Resource Workshop, AppExpert, ClassExpert); text files, no binary project files, generated code never edited by hand.
- 2026-10-16: generation, checking and enforcement are deterministic; no AI model is needed for them; the AI proposes, people decide.
- 2026-10-17: four principles: modular, test first, one source of truth (DRY), backend drives frontend.
- 2026-10-20: the IDE feels like C++ Builder, made modern; everything is edited, built and fixed inside Clarity.
- 2026-10-30: the principles hold for all time. User: "for all time, like backend drives frontend, TDD, think modular, etc"
- 2026-10-30: think as the maker of a generic app builder: options, designer property, checks, docs, translations, every stack, examples. User: "can you be a bit more pro-active and thing generic app building application!!!!!"
- 2026-10-30: fail safe: guard the whole kind of failure, not one case. User: "I meant to make guards that guard any such issue... make code fail safe!!!!!!!!!"
- 2026-10-31: a runtime decides; a screen shows and tells (DESIGN 1.7). User: "I highly doubt whether the language stacks follow the rule of 'backend drives frontend' ... and then I'm not talking about undo alone"
- 2026-11-09: no heuristics, no guessing of intent. User: "no heuristics only determinism!"
- 2026-11-12: the first design's rules engine over hand-written code is struck: directives, rules and gates, tuning and waivers, language packs, the source index, `clarity init`/`adopt`/`fmt`/`tune`, `clarity lsp`, the control room, the first MCP tool list and workspace crates (DESIGN 3, 4, 9 to 18, roadmap M0 to M10). User: "strike first-design"
- 2026-11-12: start every design from what C++ Builder could do, in a generic modern form; never a fixed format. User: "remember it needs to be generic! No fixed format!!! What C++ builder could Clarity should be able to do but then in this day and age"

## Was
- DESIGN 1 (Purpose: the Clarion and Turbo C++ model), 1.1, 1.7 (the rule; the message list is [clarity-ui](clarity-ui.md)), 2 (Goals: one language, transpilation, designer, deterministic); DESIGN 20 question 1
- DESIGN 1 (the directive layer and rule engine paragraphs), 2 (rule, gate and tuning goals; non-goals), 3, 4, 5.8, 6.1, 6.3, 6.4, 8 (init, stub and LSP lines), 9, 9.1, 9.2, 9.3, 10, 11, 11.1-11.6, 12, 13, 14, 15, 15.1-15.3, 16, 17, 17.1-17.4, 18; DESIGN 20 questions 3, 6, 7, 8, 9, 10; roadmap M0-M10 (all struck)
- memory: clarity-direction.md, principles.md, think-generic-builder.md, determinism.md, cpp-builder-ide.md, fail-safe-code.md
