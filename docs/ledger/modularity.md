# Modularity: every specialized piece in modules of its own

Status: partial. The guide for moving each piece into one module per layer before it is built on; step 1 (sign-in) is in progress.

Progress: 10%

## Code
- Started (step 1): `crates/clarity-spec/src/check/signin.rs`, `crates/clarity-spec/src/templates/signin.rs`, `runtimes/rust-ui/src/window/host/signin.rs`, `runtimes/rust-ui/src/window/signin.rs`, `runtimes/rust-auth/src/attempt.rs` (ops `signinBegin`, `signinDecide`, `lib.rs:202, 209`), `runtimes/go/internal/authlib/authlib.go`, `app/src/lib/SigninView.svelte` + `signinView.ts` (committed 3eb6ef0c)
- Largest files still holding many pieces: `crates/clarity-spec/src/check.rs` (2,826), `parser.rs` (2,818), `widgets.rs` (2,547); `crates/clarity-gen/src/rust/mod.rs` (1,605); Clarity.UI `parts/data/table.rs` (2,287), `parts/data/canvas.rs` (1,956), `canvas/modes.rs` (1,077), `desktop/mod.rs` (1,513), `window/mod.rs` (1,347), `screen.rs` (1,228), `parts/text_editor.rs` (1,261); IDE `PageProperties.svelte` (2,096), `PageDesigner.svelte` (981)

## Tests
- Each move runs the moved module's tests before and after. Step 1's: `crates/clarity-spec/tests/signin_views.rs`, `runtimes/rust-auth/tests/attempt.rs`, `providers.rs`, `app/src/lib/signinView.test.ts`, `crates/clarity-gen/tests/signin_{rust,go,c,cpp,csharp}.rs`

## Open
Each step as a checklist; "settles" names the duplicates and stack differences the step would end.

### Step 1: sign-in, the generators included ([signin](signin.md))
- [x] Checker rules in `check/signin.rs`; New window's sign-in in `templates/signin.rs`.
- [x] Clarity.UI host `window/host/signin.rs`, window stack `window/signin.rs` ([clarity-ui](clarity-ui.md), [windows-dialogs](windows-dialogs.md)).
- [x] One attempt in Clarity.Auth, adopted by R, Go, C#, C, C++; Go's tests run against the library.
- [ ] Each generator's sign-in emitting in a module of its own: today `crates/clarity-gen/src/signin.rs` (150 lines) plus each stack's `model.rs` ([generation](generation.md), [stack-rust](stack-rust.md), [stack-go](stack-go.md), [stack-c](stack-c.md), [stack-cpp](stack-cpp.md), [stack-csharp](stack-csharp.md)).
- [ ] The providers inside these modules (`runtimes/rust-auth/src/providers.rs`, now `pub mod providers` at `lib.rs:12`).
- Settles: sign-in defaults (8 h, 5 tries, 15 min) written five times: `crates/clarity-spec/src/signin.rs:145-147`, `runtimes/rust-auth/src/attempt.rs:157`, `runtimes/rust/src/signin.rs:78`, `runtimes/go/clarity/signin.go:225-228`, `runtimes/cpp/clarity/people.hpp:69-72`.

### Step 2: shared runtime cores
- [ ] Row versions and undo ([row-versions](row-versions.md)). Settles: edit bus 500 per window on R, Go, C, C++ (`runtimes/rust/src/edits.rs:37`, `go/clarity/edits.go:40`, `c/clarity/edits.c:44`, `cpp/clarity/edits.cpp:15`) against none dropped on C# (`EditBus.cs:29-35`), and `conformance/edit-bus.json` has no limit case; undo's 100 steps written in each runtime.
- [ ] Migrations as the runtimes run them ([migrations](migrations.md)). Settles: `conformance/migrations.json` read by no R runtime test; C# runs it only on server databases, absent in CI.
- [ ] File sources on Clarity.Formats ([formats](formats.md)). Settles: `formats.json` skipped by R, Go, C, C++ runtime tests though their cli duration readers cite it (`runtimes/rust/src/cli.rs:320`, `go/clarity/cli.go:389`); Go's file-source tests skipped in CI by `-run Formats` (`.github/workflows/checks.yml:207`).
- [ ] Backups ([backups](backups.md)). Settles: "keep 10" written in each runtime and clarity-db.

### Step 3: the checker and the parser by area ([language](language.md))
- [ ] `check.rs` by area (pages and frames, data and bindings, code, toolboxes, content and views, settings, migrations), each when next worked on; `check/` exists with sign-in only.
- [ ] `parser.rs` by the same areas.
- Settles: highlighter words against the parsers (`crates/clarity-spec/src/highlight.rs:49-91`, `codec` missing; [ide-editors](ide-editors.md)).

### Step 4: part families in the catalog, Clarity.UI's large files by job
- [ ] `widgets.rs` by family: text, inputs, containers, data, graphics, media, time, toolbox, sign-in ([parts-inputs](parts-inputs.md), [parts-containers](parts-containers.md), [parts-data](parts-data.md), [canvas-toolbox](canvas-toolbox.md), [timer](timer.md)).
- [ ] `table.rs`: columns and cells, sorting, paging, editing in place, adding and removing rows, reordering and drag ([parts-data](parts-data.md)).
- [ ] `canvas.rs` and `canvas/modes.rs`: drawing, zoom and grid, painting, vector, objects and connections, the toolbox ([canvas-toolbox](canvas-toolbox.md)).
- [ ] `desktop/mod.rs`, `window/mod.rs`, `screen.rs` ([clarity-ui](clarity-ui.md), [windows-dialogs](windows-dialogs.md)).
- [ ] `text_editor.rs`: editing, selection, undo, code colours, the Markdown preview ([parts-inputs](parts-inputs.md), [markdown](markdown.md)).
- Settles: part kinds and makers (`widgets.rs` against `runtimes/rust-ui/src/parts/mod.rs:584` and the `KINDS` lists, 70 and 70, no test); the 34 part properties code reads, in each generator (`rust/logic.rs:739`, `go/logic.rs:1558`, `cpp/logic.rs:1118`, `csharp/logic.rs:199`, c); color palette (`crates/clarity-spec/src/colors.rs:8`, `parts/inputs/color.rs:44`, C# `Palette.cs:13`); display formats (`parts/inputs/formats.rs`, `runtimes/web/src/screen/format.ts`, C# `Formats.cs`); code-language list (`widgets.rs:600`, `parts/code_colors.rs:9`); layout constants (`crates/clarity-spec/src/layout.rs` against `grid.rs:16-42`, `window/view.rs:28`, `window/messages.rs:20`, `parts/containers/popover.rs:39`, `runtimes/web/src/screen/grid.ts:9-55`; `tests/layout.rs` misses GAP, CELL_HEIGHTS, the popover column); ranges, display and list fields in rust-ui and `runtimes/web/src/screen/*.ts`.
- Settles conformance C# skips: accordions, dropdown-options, folding, steps, table-pages, trees, ranges, table-order (C# keeps `PartState.cs`, `DropdownOptions.cs`, `TablePages.cs`, `Trees.cs`, `Ranges.cs`; no test loads the files).
- C# dead copies to remove: `TablePages.cs`, `Ranges.cs`, `Palette.cs` ([stack-csharp](stack-csharp.md)).

### Step 5: the rest, piece by piece, right before it gets work
- [ ] Splash, app icon and version ([windows-dialogs](windows-dialogs.md), [generation](generation.md)).
- [ ] Audio ([audio](audio.md)); video ([video-camera](video-camera.md)).
- [ ] Image and toolboxes ([picture-functions](picture-functions.md), [canvas-toolbox](canvas-toolbox.md)). Settles: `picture-functions.json` run only by `runtimes/rust-image`, no stack wrapper.
- [ ] Graph ([graph](graph.md)).
- [ ] Charts and trends ([charts](charts.md)). Settles: C# `Charts.cs` (dead copy, `charts.json` unloaded); chart grouping in `runtimes/rust/src/charts.rs` and `runtimes/rust-ui/src/parts/data/groups.rs`.
- [ ] Engines ([engines](engines.md)).
- [ ] Custom parts and libraries ([custom-parts](custom-parts.md)). Settles: `custom-parts.json` unloaded on C#.
- [ ] Web API sources and mappings ([web-apis](web-apis.md)). Settles: C#'s own timeout text (`runtimes/csharp/Clarity.Runtime/Http.cs:260`; `http.json` checks only the start); 30 s written in each runtime.
- [ ] Translations ([translations](translations.md)); themes ([themes](themes.md)): each in rust-ui and `runtimes/web/src/screen/*.ts`.
- [ ] Secrets and the vault ([settings-secrets](settings-secrets.md), [vault](vault.md)). Settles: `secret-store.json` skipped by Go, C, C++.
- [ ] The command line ([cli-target](cli-target.md)). Settles: non-SQLite refusal worded twice per stack (`crates/clarity-gen/src/c/mod.rs:253` against `c/cli.rs:142`; `cpp/mod.rs:262` against `cpp/cli.rs:123`).
- [ ] Dialogs; content and views ([windows-dialogs](windows-dialogs.md)).
- [ ] The frame: menus, toolbars, ribbons, sidebars ([frame-keys](frame-keys.md)). Settles: `ribbons.json`, `sidebars.json` unloaded on C#; sidebars in rust-ui and `runtimes/web/src/frame/sidebars.ts`; `shortcut-keys.json` skipped by Go, C, C++ and by `runtimes/web/src/host/keys.ts`'s tests; key rules three times (`runtimes/rust/src/keys.rs:99`, C# `KeyRules.cs:79` counting UTF-16 units, `keys.ts:151`); the person's keys with two writers on C# (`runtimes/rust-ui/src/window/mod.rs:104-109` against `crates/clarity-gen/src/csharp/model.rs:287`, `PersonKeys.cs:25`); dead `KeyRules.cs`, `PersonKeys.Set`.
- [ ] Canvas modes ([canvas-toolbox](canvas-toolbox.md)).
- [ ] Timers and sounds ([timer](timer.md), [audio](audio.md)). Settles: melodies three times (`crates/clarity-spec/src/sound.rs:73`, `runtimes/rust-ui/src/melodies.rs:16`, dead C# `Melodies.cs:20`).
- [ ] Markdown ([markdown](markdown.md)). Settles: blocks in `runtimes/rust/src/markdown.rs` and `runtimes/web/src/screen/markdownBlocks.ts`.

## Conflicts
- UNIMPLEMENTED.md section 12 numbers the steps by the order before its rewrite (2 checker, 3 Clarity.UI files, 4 catalog, 5 cores); MODULAR-PLAN.md's order (below) is the one that holds.

## Decisions
- 2026-11-12, after sign-in work broke tests on five stacks found only at the end. User: "If modules arent modular enough TDD doesnt work!!!! you're not able to have enough overview", and, agreeing to give sign-in one module per layer before building its providers: "yes, do that first... and that needs to happen for every specialized (modular) piece in the ecosystem!!!"
- The rule (verbatim): A specialized piece (sign-in, formats, row versions, audio, a part family, ...) lives in one module per layer, each with its own tests, and logic every stack shares lives once:

| Layer | Where the piece lives |
|---|---|
| Spec | its own module for model and helpers, a checker module (`check/<piece>.rs`), a template module when New window makes it (`templates/<piece>.rs`) |
| Generators | the piece's emitting per stack in its own module, beside the stack's others |
| Shared runtime core | the decisions every stack makes, once, in a Rust crate with a C interface (as Clarity.Auth, Clarity.Formats); each stack's runtime only reads and writes its own database, files and screen through it |
| Clarity.UI | its own module of the window host and window stack, beside its parts |
| IDE | its own component and a TypeScript helper with node tests |
| Docs | its manual page (the manual holds the whole language), a conformance file where stacks must agree |

- A move keeps behaviour as it is and runs the module's tests before and after; a new feature of the piece then goes into its modules only.
- The rule of the order (the user, 2026-11-12, asked "is that the right order?" and chose "yes, rewrite the order"): a piece moves just before it is built on, never in one pass over everything; first what breaks across stacks, then what the next work touches.
- The order (verbatim):

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

## Was
- MODULAR-PLAN.md (the rule, the order steps 1-5, open questions)
- memory: modular-pieces.md

## Open questions
- From MODULAR-PLAN.md: whether the generators get a module per piece per stack now, or as each piece is touched.
- From MODULAR-PLAN.md: whether the IDE's large components (`PageProperties.svelte`, `PageDesigner.svelte`) split by piece in the same pass as their spec side.
- No step settles value semantics across stacks: ordered comparisons on C# (`crates/clarity-gen/src/csharp/logic.rs:1277`, `none < d` false, no `<` for text) against R, Go, C, C++ (`go/logic.rs:1917`, `runtimes/go/clarity/values.go:124`, `runtimes/c/clarity/values.h:37`, `runtimes/cpp/clarity/values.hpp:45`); decimal division (16 decimals on Go, C, C++; 28 digits on R, C#; divide by zero a value error on C, C++, a panic elsewhere); integer overflow in `+ - *`; decimal range; empty values first in comparisons but last in table order (`runtimes/rust/src/tables.rs:3`). Should step 2 take them as a shared core (and a conformance file), or a step of their own?
- No step names handler dispatch: `calls.json` has no C# test against `runtimes/csharp/Clarity.Runtime/Program/Members.cs:116`.
- No step names the web port: 8080 in `crates/clarity-project/src/lib.rs:349` (`DEFAULT_PORT`) and `app/src/lib/ProjectSettings.svelte:416, 422`.
