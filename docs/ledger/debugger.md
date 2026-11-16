# Debugger

Status: not built. Stop the real generated program at a line of Clarity, see its names, step and go on, the same on every stack and system, desktop and browser; later, debug the design.

Progress: 0%

## Code
- Spec: none
- Generators: none
- Runtimes: none
- Clarity.UI: none
- IDE: none
- Engines: Clarity engines run compiled or interpreted inside WebAssembly (`crates/clarity-wasm/src/interpreted.rs`); see [engines](engines.md)
- Docs: none

## Tests
- none

## Open
- Step 1, places and marks in the Rust generator, breakpoints, stepping and variables, the IDE's gutter and panels: NOT BUILT; see Design.
- Step 2, the same marks from the Go, C#, C and C++ generators: NOT BUILT; see Design.
- Step 3, stop on error, conditions, hit counts, log points: NOT BUILT; see Design.
- Step 4, the event log and the data view: NOT BUILT; see Design.
- Step 5, watches as expressions, stepping inside engines: NOT BUILT; see Design.
- Step 6, the browser target and VS Code: NOT BUILT; see Design.
- Step 7, debugging the design: NOT BUILT; depends on designer edits by part key, still PARTIAL ([ide-designer](ide-designer.md)); see Design.
- To settle when building: needs decision; see Design.

## Design
The person debugs the Clarity they wrote, never the generated Go or C#, on the real program on its stack, with its database, files and timing.

What the developer gets:
- Breakpoints: a click in the code editor's gutter; the app stops before that line.
- Conditions and hit counts: `when order.total > 1000`, or only the fifth time.
- Log points: a line that writes a text with slots (`{order.number}`) to the debug console instead of stopping.
- Stop on error: where a `refuse`, a failed save or a runtime error happens, before the message shows.
- Step over, into, out.
- Call stack: the handler at the bottom (`saveClicked` of OrderEditor, run by a click on saveButton), the functions and actions above it.
- Variables: locals and parameters, the window's data, the parts' state (`orderTable.selected`, `classPanel.visible`); a record opens to its fields, a list to its items.
- Watches: names and paths (`order.lines[1].price`); later any Clarity expression.
- Event log: which handlers ran, by which event, in order, with their time.
- Data view: while stopped, the app's database read only, through Clarity's own database layer.
- Engines: an engine runs inside WebAssembly, compiled or interpreted (`clarity-wasm`). An interpreted engine steps line by line through a mark in the interpreter it runs on; a compiled one is stepped over.

The pieces:
- Places: each statement gets a place number; a Debug build writes a table of places (file, line, column, the names in scope and their types) beside the program, so marks pass a number, not text. A normal build has no marks and no debug port, and pays nothing.
- Marks per stack: at a handler's start the generated code opens a frame (`debug_enter(handler, frame)`), before each statement it marks its place (`debug_at(place)`), at the end it closes the frame. The frame hands over its names' values as JSON only when asked. Rust, Go, C#, C and C++ each write these calls in their own way.
- Clarity.UI, behind its C interface, which every desktop stack loads: the breakpoints, the decision to stop (one compare per mark while nothing is set), the stopped state, the band on the window that says it is stopped in the debugger, and a Debug Adapter Protocol (DAP) server on 127.0.0.1, on a port only the IDE knows, with a token the IDE hands the program at its start. No firewall prompt for 127.0.0.1; alike on Linux, Windows and macOS.
- Browser target: the program runs on the server and the window in the browser, so the server's program stops and the browser shows the band; the IDE attaches to the server as to a desktop program.
- The IDE: Debug beside Run; the gutter; panels for the call stack, variables, watches, the event log and the data view; the debug console; the current line marked in the editor.

Debugging the design, once designer edits name parts by key (the designer, the code and the running app know a part by the same file and name):
- Pick in the app: a click on a part in the running app opens its `.cxaml` with that part selected.
- Part state: a part's value, visible, enabled, read-only, and what code set last; the window's data beside it.
- Event breakpoints: "stop when saveButton is clicked" stops in the handler its `onclick` names.
- Design trace: while stepping, the designer highlights each part the code touches.
- Live edits: a design edit goes to the running app, and Clarity.UI swaps in that element without a restart.

Tests: conformance cases every stack runs (a small program, the places its Debug build marks, and a DAP script: set a breakpoint, run, stopped where, these variables, step, into, out, go on); the examples' Debug builds driven under Xvfb with a scripted debugger; a test that a normal build has no mark and no port.

Order: 1 places and marks in the Rust generator with breakpoints, stepping, variables and the IDE's gutter and panels; 2 the other generators; 3 stop on error, conditions, hit counts, log points; 4 the event log and the data view; 5 watches as expressions, stepping inside engines; 6 the browser target and VS Code through its Clarity extension; 7 debugging the design.

To settle when building:
- How much slower a Debug build runs with a mark per statement, measured on the examples (the games above all).
- How a frame hands over values on each stack without copying while the app runs.
- Whether a stopped window may still repaint (the band drawn before the program waits, or a repaint from the library's own thread).
- The DAP crate on the Rust side, or a small one of Clarity's own: a new dependency, asked for first.

## Conflicts
- none

## Decisions
- 2026-11-05: debug the real program on its stack, not an imitation; the person debugs Clarity, never the generated Go or C#.
- 2026-11-05: a Debug build marks each statement; a normal build has none and pays nothing.
- 2026-11-05: written once in Clarity.UI's shared library behind its C interface; each generator only writes the marks.
- 2026-11-05: the IDE speaks the Debug Adapter Protocol over a local socket, so VS Code can attach later.
- 2026-11-05: stopping freezes only the app, and a band on its window says why.
- 2026-11-09: debugging the design follows from the change "no translator between design and code", after it. User: "you mean remove the translator between them... that might enable debugging design too?" and "yes, add those to the debugger plan... but that are consequences of the change"
- 2026-11-09: the debugger comes last in the agreed work order, then design debugging. User: "agree"

## Was
- DEBUGGER-PLAN.md, all of it (Goal, Decisions, "What the developer gets", "Debugging the design", "The pieces", "Every system", "Testing", order steps 1-7, "To settle when building"): now Design. Its engines row ("Clarity's engine interpreter") is rewritten for engines inside WebAssembly; its "UNIMPLEMENTED.md section 9" is designer edits by key ([ide-designer](ide-designer.md)).
- memory: debugger-plan.md, roadmap-order.md (item 12)
