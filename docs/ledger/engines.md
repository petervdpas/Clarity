# Engines

Status: partial. WebAssembly modules every stack hosts alike: brought along (`engine X from "x.wasm"`) or written in Clarity and compiled at generation; on the backend or beside the screen.

Progress: 65%

## Code
- Spec: `crates/clarity-spec/src/engines.rs` (imports, exports, limits :17-26, `SCREEN_INTERFACE`), `code/parse.rs:60`, `code/mod.rs` (`EngineDecl`), hook names in `names.rs` (`HOOKS`)
- Compiler: `crates/clarity-wasm` (`src/lower.rs`, `emit.rs`, `interpreted.rs`, `merge.rs`; at the crate root `ir/` and `support/`, built to `support.wasm` and `interpreter.wasm`)
- Generators: `compile_engines` in `crates/clarity-gen/src/lib.rs:845`; each stack embeds the files
- Runtimes: R `runtimes/rust/src/engine.rs` (wasmi 2); Go `runtimes/go/engine` (wazero 1.12); C `runtimes/c/engine` + `third_party/wasm3`; C++ `runtimes/cpp/engine`; C# `runtimes/csharp/Clarity.Runtime.Engines` (Wasmtime 48)
- Clarity.UI: `runtimes/rust-ui/src/engines.rs` (beside the screen), `dialogs/long_call.rs` (progress and Stop)
- Engine library: `runtimes/rust-engine` (crate `clarity-engine` 0.3.1, for engine authors)
- Project: `crates/clarity-project/src/engines.rs` (Rust engine crate in `engines/<name>/`, stubs, build)
- IDE: `app/src/lib/EngineEditor.svelte`, `app/src-tauri/src/engines.rs`
- Docs: manual `code.engines.md`

## Tests
- `conformance/engines.json`, `progress.json`, `screen-engines.json`: run by R, Go, C, C++, C#
- `conformance/clarity-engines.json`: run by `crates/clarity-wasm` only
- `crates/clarity-wasm/tests/`: conformance.rs, noughts.rs, progress.rs, sky.rs
- `crates/clarity-spec/tests/`: engines.rs, screen_engines.rs, clarity_engines.rs; `crates/clarity-gen/tests/`: engines.rs, screen_engines.rs, clarity_engines.rs; `crates/clarity-project/tests/engines.rs`

## Open
- Engines written in Clarity: PARTIAL. "not compiled yet" for date, datetime, duration, file values (`lower.rs:25, 36`); statements other than let, assign, if, for, return, call, tell, refuse (:246); keys with values (:295); `a to b` outside `for` (:220-228); `/` giving a decimal (:405); built-ins other than count, sin, cos, round, random, progress (:432); record fields of date or ref type (:484).
- Picture functions in engines, a video frame as a picture: NOT BUILT (VIDEO-PLAN.md:99).
- `clarity engines build`, `clarity watch`, templates for C, C++, TinyGo, Zig, AssemblyScript: PARTIAL. Only the Rust engine flow (N6).
- WIT and the component model instead of JSON: open, not decided (N6).
- `progress n of total` as a statement in handlers and actions: NOT BUILT. `progress(...)` outside an engine is an error (`code/check.rs:4466`).

## Conflicts
- `manual/code.engines.md:79` says "Keys it says"; the section is "Texts it gives".

## Decisions
- 2026-10-20: an engine computes, the application does all input and output; imports are services only (clock, random, log, progress), none for data.
- 2026-10-20: JSON over plain memory, not the component model, because wazero, wasm3 and Wasmtime for .NET do not run components.
- 2026-10-20: beside the screen runs in a worker, not the page's thread.
- 2026-10-20: no `on close` hook.
- 2026-10-20: engines open and build inside the IDE, like C++ Builder. User: "the whole flow/ux is not really intuitive or friendly"
- 2026-10-26: engines written in Clarity compile to WebAssembly (transpiled per stack from 2026-10-21 until then); one beside the screen needs no words of its own.
- 2026-10-27: compiled or interpreted, chosen per project (`engines interpreted|compiled`), the user's idea.

## Was
- DESIGN 5.11; 5.12 (Progress of a long call, Engines written in Clarity N2, Engines beside the screen N5, In order)
- Roadmap N1, N2, N2a, N3, N4, N5, N5a, N5b, N6, P
- IMPLEMENTED.md section 3 "Engines", section 8 (engine hosts); UNIMPLEMENTED.md section 3, section 1 (`progress n of total`)
- memory: cpp-builder-ide.md (engines part)
