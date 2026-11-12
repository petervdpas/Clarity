# Modular plan: every specialized piece in modules of its own

Written 2026-11-12. The user, that day, after sign-in work broke tests on five stacks found only at the end: "If modules arent modular enough TDD doesnt work!!!! you're not able to have enough overview", and, agreeing to give sign-in one module per layer before building its providers: "yes, do that first... and that needs to happen for every specialized (modular) piece in the ecosystem!!!"

## The rule

A specialized piece (sign-in, formats, row versions, audio, a part family, ...) lives in one module per layer, each with its own tests, and logic every stack shares lives once:

| Layer | Where the piece lives |
|---|---|
| Spec | its own module for model and helpers, a checker module (`check/<piece>.rs`), a template module when New window makes it (`templates/<piece>.rs`) |
| Generators | the piece's emitting per stack in its own module, beside the stack's others |
| Shared runtime core | the decisions every stack makes, once, in a Rust crate with a C interface (as Clarity.Auth, Clarity.Formats); each stack's runtime only reads and writes its own database, files and screen through it |
| Clarity.UI | its own module of the window host and window stack, beside its parts |
| IDE | its own component and a TypeScript helper with node tests |
| Docs | its manual page, its section of LANGUAGE.md, a conformance file where stacks must agree |

A move keeps behaviour as it is and runs the module's tests before and after; a new feature of the piece then goes into its modules only.

## Order

The rule of the order (the user, 2026-11-12, asked "is that the right order?" and chose "yes, rewrite the order"): a piece moves just before it is built on, never in one pass over everything; first what breaks across stacks, then what the next work touches.

1. **Sign-in, the generators included** (in progress, 2026-11-12): checker rules in `check/signin.rs`, New window's sign-in in `templates/signin.rs`, the Clarity.UI host's sign-in in `window/host/signin.rs` and the window stack's in `window/signin.rs` (done); one sign-in attempt in Clarity.Auth (`attempt.rs`, ops `signinBegin` and `signinDecide`), adopted by the Rust, Go, C#, C and C++ runtimes (done; Go's sign-in tests run against the library itself, its stand-in removed). Next: each generator's sign-in emitting in a module of its own (the Rust generator's `mod.rs` is 1,400 lines), then the providers, inside these modules.
2. **Shared runtime cores** for decisions each runtime makes again: row versions and undo, migrations as the runtimes run them, file sources (on Clarity.Formats), backups. One decision written five times breaks five stacks at once.
3. **The checker and the parser by area**: `check.rs` (2,800 lines) and `parser.rs` (2,800) hold every area (pages and frames, data and bindings, code, toolboxes, content and views, settings, migrations); each area moves to its own module when it is next worked on, the roadmap's areas first.
4. **Part families in the catalog** (`widgets.rs`, 2,500 lines: text, inputs, containers, data, graphics, media, time, toolbox, sign-in) and **Clarity.UI's large files by job** (151 files, about 53,000 lines; the user, 2026-11-12, on the seven over 1,000 lines: "yes, add them to the plan"), each when its part next gets work:
   - `parts/data/table.rs` (2,287): columns and cells, sorting, paging, editing in place, adding and removing rows, reordering and drag, each a module of the table.
   - `parts/data/canvas.rs` (1,956) and `parts/data/canvas/modes.rs` (1,077): drawing, zoom and grid, painting, vector, objects and connections, the toolbox, each a module of the canvas.
   - `desktop/mod.rs` (1,428): the event loop, desktop windows and their sizes, the splash and sign-in windows, input, the screen reader.
   - `window/mod.rs` (1,242): the window stack's opening and closing, dialogs, desktop windows of their own, the splash; sign-in is already out (`window/signin.rs`).
   - `screen.rs` (1,228): layout, input routing, focus, overlays.
   - `parts/text_editor.rs` (1,198): editing, selection, undo, code colours, the Markdown preview.
5. **The rest, piece by piece, each right before it gets work**: splash, app icon and version, audio, video, image and toolboxes, graph, charts and trends, engines, custom parts and libraries, web API sources and mappings, translations, themes, secrets and the vault, the command line, dialogs, the frame (menus, toolbars, ribbons, sidebars), content and views, canvas modes, timers and sounds, markdown.

## Open questions

- Whether the generators get a module per piece per stack now, or as each piece is touched.
- Whether the IDE's large components (`PageProperties.svelte`, `PageDesigner.svelte`) split by piece in the same pass as their spec side.
