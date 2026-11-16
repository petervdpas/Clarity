# Modularity: every specialized piece in modules of its own

Status: partial. The guide for moving each piece into one module per layer before it is built on; step 1 (sign-in) is in progress.

Progress: 10%

## Code
- Step 1 done so far: `crates/clarity-spec/src/check/signin.rs`, `crates/clarity-spec/src/templates/signin.rs`, `runtimes/rust-ui/src/window/host/signin.rs`, `runtimes/rust-ui/src/window/signin.rs`, `runtimes/rust-auth/src/attempt.rs` (ops `signinBegin`, `signinDecide`, `lib.rs:202, 209`), adopted by `runtimes/rust/src/signin.rs`, `runtimes/go/clarity/signin.go` (through `runtimes/go/internal/authlib/authlib.go`), `runtimes/csharp/Clarity.Auth/Signins.cs`, `runtimes/c/auth/auth.c`, `runtimes/cpp/auth/auth.cpp`; `app/src/lib/SigninView.svelte` + `signinView.ts` (committed 3eb6ef0c)
- Not yet: no generator has a sign-in module (`crates/clarity-gen/src/signin.rs`, 150 lines, plus each stack's `model.rs`); `runtimes/rust-auth/src/providers.rs` is a module of Clarity.Auth (`lib.rs:12`)
- Largest files still holding many pieces (2026-11-16): `crates/clarity-spec/src/check.rs` (2,882), `parser.rs` (2,899), `widgets.rs` (2,611); `crates/clarity-gen/src/rust/mod.rs` (1,619); Clarity.UI (154 files, about 55,400 lines) `parts/data/table.rs` (2,405), `parts/data/canvas.rs` (1,956), `parts/data/canvas/modes.rs` (1,077), `desktop/mod.rs` (1,513), `window/mod.rs` (1,353), `screen.rs` (1,228), `parts/text_editor.rs` (1,261), `element.rs` (1,038), `access/mod.rs` (1,031); IDE `PageProperties.svelte` (2,181), `PageDesigner.svelte` (922)

## Tests
- Each move runs the moved module's tests before and after. Step 1's: `crates/clarity-spec/tests/signin_views.rs`, `runtimes/rust-auth/tests/attempt.rs`, `providers.rs`, `app/src/lib/signinView.test.ts`, `crates/clarity-gen/tests/signin_{rust,go,c,cpp,csharp}.rs`

## Open
- Step 1, each generator's sign-in in a module of its own (Design, "Step 1"): NOT BUILT.
- Step 1, the providers inside these modules (Design, "Step 1"): PARTIAL.
- Step 2, row versions and undo as a shared core (Design, "Step 2"): NOT BUILT.
- Step 2, migrations as the runtimes run them (Design, "Step 2"): NOT BUILT.
- Step 2, file sources on Clarity.Formats (Design, "Step 2"): NOT BUILT.
- Step 2, backups (Design, "Step 2"): NOT BUILT.
- Step 3, `check.rs` and `parser.rs` by area (Design, "Step 3"): NOT BUILT; `check/` holds sign-in only.
- Step 4, `widgets.rs` by family and Clarity.UI's large files by job (Design, "Step 4"): NOT BUILT.
- Step 5, the rest, piece by piece (Design, "Step 5"): NOT BUILT.
- Open questions (Design, "Open questions"): needs decision.

## Design
The rule and the rule of the order are under Decisions. "Settles" names the duplicates and stack differences each step would end.

Step 1: sign-in, the generators included ([signin](signin.md))
- Done: checker rules, New window's sign-in, the Clarity.UI host and window stack, one attempt in Clarity.Auth adopted by every runtime, Go's tests against the library (see Code).
- Next: each generator's sign-in emitting in a module of its own beside the stack's others ([generation](generation.md), [stack-rust](stack-rust.md), [stack-go](stack-go.md), [stack-c](stack-c.md), [stack-cpp](stack-cpp.md), [stack-csharp](stack-csharp.md)); the Rust generator's `mod.rs` is 1,619 lines.
- Then the providers, inside these modules.
- Settles: sign-in defaults (8 h, 5 tries, 15 min) written five times: `crates/clarity-spec/src/signin.rs:145-147`, `runtimes/rust-auth/src/attempt.rs:157`, `runtimes/rust/src/signin.rs:78`, `runtimes/go/clarity/signin.go:225-228`, `runtimes/cpp/clarity/people.hpp:69-72`.

Step 2: shared runtime cores, for decisions each runtime makes again (one decision written five times breaks five stacks at once)
- Row versions and undo ([row-versions](row-versions.md)). Settles: edit bus 500 per window on Rust, Go, C, C++ (`runtimes/rust/src/edits.rs:37`, `go/clarity/edits.go:40`, `c/clarity/edits.c:44`, `cpp/clarity/edits.cpp:15`) against none dropped on C# (`EditBus.cs:29-35`), `conformance/edit-bus.json` with no limit case; undo's 100 steps written in each runtime.
- Migrations as the runtimes run them ([migrations](migrations.md)). Settles: `conformance/migrations.json` read by no Rust runtime test; C# runs it only on server databases, absent in CI.
- File sources on Clarity.Formats ([formats](formats.md)). Settles: `formats.json` skipped by the Rust, Go, C, C++ runtime tests though their cli duration readers cite it (`runtimes/rust/src/cli.rs:320`, `go/clarity/cli.go:389`); Go's file-source tests skipped in CI by `-run Formats` (`.github/workflows/checks.yml:207`).
- Backups ([backups](backups.md)). Settles: "keep 10" written in each runtime and clarity-db.

Step 3: the checker and the parser by area ([language](language.md))
- `check.rs` (2,882) and `parser.rs` (2,899) by area: windows and frames, data and bindings, code, toolboxes, content and views, settings, migrations; each area when next worked on, the roadmap's areas first.
- Settles: highlighter words against the parsers (`crates/clarity-spec/src/highlight.rs:49-91`, `codec` missing; [ide-editors](ide-editors.md)).

Step 4: part families in the catalog, Clarity.UI's large files by job, each when its part next gets work
- `widgets.rs` (2,611) by family: text, inputs, containers, data, graphics, media, time, toolbox, sign-in ([parts-inputs](parts-inputs.md), [parts-containers](parts-containers.md), [parts-data](parts-data.md), [canvas-toolbox](canvas-toolbox.md), [timer](timer.md)).
- `parts/data/table.rs` (2,405): columns and cells, sorting, paging, editing in place, adding and removing rows, reordering and drag ([parts-data](parts-data.md)).
- `parts/data/canvas.rs` (1,956) and `canvas/modes.rs` (1,077): drawing, zoom and grid, painting, vector, objects and connections, the toolbox ([canvas-toolbox](canvas-toolbox.md)).
- `desktop/mod.rs` (1,513): the event loop, desktop windows and their sizes, the splash and sign-in windows, input, the screen reader.
- `window/mod.rs` (1,353): the window stack's opening and closing, dialogs, desktop windows of their own, the splash; sign-in is already out.
- `screen.rs` (1,228): layout, input routing, focus, overlays ([clarity-ui](clarity-ui.md), [windows-dialogs](windows-dialogs.md)).
- `parts/text_editor.rs` (1,261): editing, selection, undo, code colours, the Markdown preview ([parts-inputs](parts-inputs.md), [markdown](markdown.md)).
- Settles: part kinds and makers (`widgets.rs` against `runtimes/rust-ui/src/parts/mod.rs:584` and the `KINDS` lists, 70 and 70, no test); the 34 part properties code reads, in each generator (`rust/logic.rs:739`, `go/logic.rs:1558`, `cpp/logic.rs:1118`, `csharp/logic.rs:199`, c); color palette (`crates/clarity-spec/src/colors.rs:8`, `parts/inputs/color.rs:44`, C# `Palette.cs:13`); display formats (`parts/inputs/formats.rs`, `runtimes/web/src/screen/format.ts`, C# `Formats.cs`); code-language list (`widgets.rs:600`, `parts/code_colors.rs:9`); layout constants (`crates/clarity-spec/src/layout.rs` against `grid.rs:16-42`, `window/view.rs:28`, `window/messages.rs:20`, `parts/containers/popover.rs:39`, `runtimes/web/src/screen/grid.ts:9-55`; `tests/layout.rs` misses GAP, CELL_HEIGHTS, the popover column); ranges, display and list fields in rust-ui and `runtimes/web/src/screen/*.ts`.
- Settles the conformance files C# skips: accordions, dropdown-options, folding, steps, table-pages, trees, ranges, table-order (C# keeps `PartState.cs`, `DropdownOptions.cs`, `TablePages.cs`, `Trees.cs`, `Ranges.cs`; no test loads the files); dead C# copies to remove: `TablePages.cs`, `Ranges.cs`, `Palette.cs` ([stack-csharp](stack-csharp.md)).

Step 5: the rest, piece by piece, right before it gets work
- Splash, app icon and version ([windows-dialogs](windows-dialogs.md), [generation](generation.md)).
- Audio ([audio](audio.md)); video ([video-camera](video-camera.md)).
- Image and toolboxes ([picture-functions](picture-functions.md), [canvas-toolbox](canvas-toolbox.md)). Settles: `picture-functions.json` run only by `runtimes/rust-image`, no stack wrapper.
- Graph ([graph](graph.md)).
- Charts and trends ([charts](charts.md)). Settles: C# `Charts.cs` (dead copy, `charts.json` unloaded); chart grouping in `runtimes/rust/src/charts.rs` and `runtimes/rust-ui/src/parts/data/groups.rs`.
- Engines ([engines](engines.md)).
- Custom parts and libraries ([custom-parts](custom-parts.md)). Settles: `custom-parts.json` unloaded on C#.
- Web API sources and mappings ([web-apis](web-apis.md)). Settles: C#'s own timeout text (`runtimes/csharp/Clarity.Runtime/Http.cs:260`; `http.json` checks only the start); 30 s written in each runtime.
- Translations ([translations](translations.md)); themes ([themes](themes.md)): each in rust-ui and `runtimes/web/src/screen/*.ts`.
- Secrets and the vault ([settings-secrets](settings-secrets.md), [vault](vault.md)). Settles: `secret-store.json` skipped by Go, C, C++.
- The command line ([cli-target](cli-target.md)). Settles: the non-SQLite refusal worded twice per stack (`crates/clarity-gen/src/c/mod.rs:253` against `c/cli.rs:142`; `cpp/mod.rs:262` against `cpp/cli.rs:123`).
- Dialogs; content and views ([windows-dialogs](windows-dialogs.md)).
- The frame: menus, toolbars, ribbons, sidebars ([frame-keys](frame-keys.md)). Settles: `ribbons.json`, `sidebars.json` unloaded on C#; sidebars in rust-ui and `runtimes/web/src/frame/sidebars.ts`; `shortcut-keys.json` skipped by Go, C, C++ and by `runtimes/web/src/host/keys.ts`'s tests; key rules three times (`runtimes/rust/src/keys.rs:99`, C# `KeyRules.cs:79` counting UTF-16 units, `keys.ts:151`); the person's keys with two writers on C# (`runtimes/rust-ui/src/window/mod.rs:104-109` against `crates/clarity-gen/src/csharp/model.rs:287`, `PersonKeys.cs:25`); dead `KeyRules.cs`, `PersonKeys.Set`.
- Canvas modes ([canvas-toolbox](canvas-toolbox.md)).
- Timers and sounds ([timer](timer.md), [audio](audio.md)). Settles: melodies three times (`crates/clarity-spec/src/sound.rs:73`, `runtimes/rust-ui/src/melodies.rs:16`, dead C# `Melodies.cs:20`).
- Markdown ([markdown](markdown.md)). Settles: blocks in `runtimes/rust/src/markdown.rs` and `runtimes/web/src/screen/markdownBlocks.ts`.

Open questions
- Whether the generators get a module per piece per stack now, or as each piece is touched.
- Whether the IDE's large components (`PageProperties.svelte`, `PageDesigner.svelte`) split by piece in the same pass as their spec side.
- Clarity.UI's `element.rs` (1,038) and `access/mod.rs` (1,031) have passed 1,000 lines since the user's list of seven; whether they join step 4.
- No step settles value semantics across stacks: ordered comparisons on C# (`crates/clarity-gen/src/csharp/logic.rs:1277`, `none < d` false, no `<` for text) against Rust, Go, C, C++ (`go/logic.rs:1917`, `runtimes/go/clarity/values.go:124`, `runtimes/c/clarity/values.h:37`, `runtimes/cpp/clarity/values.hpp:45`); decimal division (16 decimals on Go, C, C++; 28 digits on Rust, C#; divide by zero a value error on C, C++, a panic elsewhere); integer overflow in `+ - *`; decimal range; empty values first in comparisons but last in table order (`runtimes/rust/src/tables.rs:3`). Should step 2 take them as a shared core (and a conformance file), or a step of their own?
- No step names handler dispatch: `calls.json` has no C# test against `runtimes/csharp/Clarity.Runtime/Program/Members.cs:116`.
- No step names the web port: 8080 in `crates/clarity-project/src/lib.rs:349` (`DEFAULT_PORT`) and `app/src/lib/ProjectSettings.svelte:416, 422`.

## Conflicts
- none

## Decisions
- 2026-11-12, after sign-in work broke tests on five stacks found only at the end. User: "If modules arent modular enough TDD doesnt work!!!! you're not able to have enough overview", and, agreeing to give sign-in one module per layer before building its providers: "yes, do that first... and that needs to happen for every specialized (modular) piece in the ecosystem!!!"
- The rule: a specialized piece (sign-in, formats, row versions, audio, a part family, ...) lives in one module per layer, each with its own tests, and logic every stack shares lives once:

| Layer | Where the piece lives |
|---|---|
| Spec | its own module for model and helpers, a checker module (`check/<piece>.rs`), a template module when New window makes it (`templates/<piece>.rs`) |
| Generators | the piece's emitting per stack in its own module, beside the stack's others |
| Shared runtime core | the decisions every stack makes, once, in a Rust crate with a C interface (as Clarity.Auth, Clarity.Formats); each stack's runtime only reads and writes its own database, files and screen through it |
| Clarity.UI | its own module of the window host and window stack, beside its parts |
| IDE | its own component and a TypeScript helper with node tests |
| Docs | its manual page (the manual holds the whole language), a conformance file where stacks must agree |

- A move keeps behaviour as it is and runs the module's tests before and after; a new feature of the piece then goes into its modules only.
- The rule of the order (the user, 2026-11-12, asked "is that the right order?" and chose "yes, rewrite the order"): a piece moves just before it is built on, never in one pass over everything; first what breaks across stacks, then what the next work touches. The order is Design, steps 1-5.
- 2026-11-12, on Clarity.UI's seven files over 1,000 lines. User: "yes, add them to the plan"

## Was
- MODULAR-PLAN.md (removed 2026-11-16): the rule, the order steps 1-5, open questions. Its Docs row "its section of LANGUAGE.md" dropped: LANGUAGE.md was removed 2026-11-13. Its file sizes brought to 2026-11-16.
- UNIMPLEMENTED.md section 12 (removed 2026-11-16), which numbered the steps by the order before its rewrite.
- memory: modular-pieces.md
