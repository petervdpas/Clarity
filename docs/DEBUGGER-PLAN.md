# The debugger

A plan, agreed 2026-11-05; nothing is built yet.

## Goal

Inside the IDE a developer stops their running app at a line of Clarity, sees what its names hold, steps on line by line, and goes on, the same way on every stack (Rust, Go, C#, C, C++), on Linux, Windows and macOS, desktop and browser. They debug the Clarity they wrote, never the generated Go or C#.

## Decisions

- **The real program, not an imitation.** Clarity has an interpreter for engines only; a whole app runs as generated code on its stack, with its database, files and timing. The debugger stops that program, so what it shows is what really happens.
- **A debug build marks each line.** With Debug (beside Run), the generator writes the same program with a mark before each statement of a handler, function and action: its place in the Clarity file and the names in scope there. A normal build has none, so a released app pays nothing.
- **Written once, in the shared library.** The marks call into Clarity.UI's C interface, which every stack already loads. The library keeps the breakpoints, decides whether to stop, and talks to the IDE; each generator only writes the marks in its language.
- **The IDE speaks a standard protocol.** The library talks the Debug Adapter Protocol (DAP) to the IDE over a local socket, so the IDE's panels follow a known shape, and VS Code's Clarity extension can debug the same way later.
- **Stopping freezes only the app.** A stopped handler holds the app as any debugger does; the window shows a band that says it is stopped in the debugger, so a person who looks at it knows why it does not answer.

## What the developer gets

| Feature | What it does |
|---|---|
| Breakpoints | A click in the gutter of the code editor; the app stops before that line. |
| Conditions and hit counts | `when order.total > 1000`, or only the fifth time. |
| Log points | A line that writes a text with slots (`{order.number}`) to the debug console instead of stopping. |
| Stop on error | Stops where a `refuse`, a failed save or a runtime error happens, before the message shows. |
| Step over, into, out | Line by line, into a function or action it calls, back out to the caller. |
| Call stack | The handler at the bottom (`saveClicked` of OrderEditor, run by a click on saveButton), the functions and actions above it. |
| Variables | The locals and parameters; the window's data; the parts' state (`orderTable.selected`, `classPanel.visible`); a record opens to its fields, a list to its items. |
| Watches | Names and paths to keep in view (`order.lines[1].price`); later any Clarity expression. |
| Event log | Which handlers ran, by which event, in order, with their time: the window's story while it was not stopped. |
| Data view | While stopped, the app's database read only (the tables the entities are kept in), through Clarity's own database layer. |
| Engines | An interpreted engine steps line by line too, through Clarity's engine interpreter; a compiled one is stepped over. |

## The pieces

- **Places.** Each statement gets a place number; the generator writes a table of places (file, line, column, the names in scope and their types) beside the program, so marks pass a number, not text.
- **Marks per stack.** At a handler's start the generated code opens a frame (`debug_enter(handler, frame)`), before each statement it marks its place (`debug_at(place)`), at the end it closes the frame. The frame hands the library the values of its names when asked, as JSON, so nothing is copied while the app runs freely. Rust, Go, C#, C and C++ each write these calls in their own way; the protocol behind them is one.
- **In Clarity.UI**: the breakpoints, the decision to stop (one compare per mark while nothing is set), the stopped state, the DAP server on a local port that only the IDE knows, and the band on the window.
- **Browser target**: the program runs on the server and the window in the browser, so the server's program stops and the browser shows the band; the IDE attaches to the server as to a desktop program.
- **The IDE**: Debug beside Run; the gutter in the code editor; panels for the call stack, variables, watches, the event log and the data view; the debug console; the current line marked in the editor, which opens the right file.

## Every system

- The debug port listens on 127.0.0.1 only, with a token the IDE hands the program at its start, so no other program on the machine attaches.
- Windows, macOS and Linux alike, since the library does it; firewall prompts do not come for a port on 127.0.0.1.

## Testing

- **Conformance cases** every stack runs: a small program, the places its debug build marks, and a script of the protocol (set a breakpoint, run, stopped where, these variables with these values, step, step into, out, go on), so each generator is held to the same answers.
- **Headless**: the examples' debug builds driven under Xvfb with a scripted debugger, test first.
- A normal build has no mark and no debug port: a test holds it to that.

## Order

1. **Places and marks** in one generator (Rust), with breakpoints, stepping and variables over DAP, and the IDE's gutter and panels.
2. **The other generators**: Go, C#, C and C++ write the same marks; the conformance cases hold them alike.
3. **Stop on error, conditions, hit counts, log points.**
4. **The event log and the data view.**
5. **Watches as expressions**, and stepping through interpreted engines.
6. **The browser target**, and debugging from VS Code through its Clarity extension.

## To settle when building

- How much slower a debug build runs with a mark per statement, measured on the examples (the games above all).
- How a frame hands over values on each stack without copying while the app runs.
- Whether a stopped handler's window may still repaint (the band drawn before the program waits, or a repaint from the library's own thread).
- The protocol crate on the Rust side (a DAP implementation, or a small one of Clarity's own): a new dependency, asked for first.
