# Learning by watching the agent

Status: not built. An agent builds an app in front of the developer: the
window opens in the designer, the parts appear on it one by one, the code
appears in its function, so a developer learns Clarity by watching it used.

Progress: 0%

## Code
- Spec: none
- Generators: none
- Runtimes: none
- Clarity.UI: none
- What exists to build on:
  - One checked edit per tool, with what it focuses: `crates/clarity-tools/src/edits.rs` (`apply`, `Applied.focus`)
  - An outside agent's turn asks the IDE to show a window, a diagram or a project, one JSON line each in the turn file: `crates/clarity-tools/src/turn.rs` (`show`, `shown`); read only when the turn ends: `app/src-tauri/src/acp.rs:141`
  - The IDE's own agent shows a window or a diagram: `app/src-tauri/src/agent.rs` (`show_window`, `IdeTools.shown`)
  - `clarity mcp` works on the project `ide.json` names, read on every call: `crates/clarity-project/src/ide.rs`; showing is left out without a turn: `crates/clarity-mcp/src/lib.rs` (`LEFT_OUT`, `TURN_TOOLS`)
  - The IDE reloads a project changed on disk: `app/src-tauri/src/lib.rs` (`watch`)
- IDE: none
- Docs: none

## Tests
- none

## Open
- Steps: every editing tool says what it changed and where: NOT BUILT; see Design, Steps.
- Steps reach the IDE while the agent works, from its own agent, an ACP agent and `clarity mcp`: NOT BUILT; see Design, Delivery.
- The IDE plays each step (designer or code editor, highlight, code typed in, tempo): NOT BUILT; see Design, Playing.
- Agents told to work in small visible steps and say why: NOT BUILT; see Design, Teaching.
- The order of the steps, a natural app design flow: NOT DESIGNED; see Design, open questions.
- Build and Run asked of the person from an outside agent: NOT BUILT; see [mcp-tools](mcp-tools.md) and Design, Build and Run.

## Design
What the user asked: "do you think its viable that the clarity mcp or acp
(not cdev) can execute or run a app building task like the old phantom
tools?", "so that a dev learns how to go use clarity... so that includes
entering code in functions", "agent draws window and puts parts on it...
visible", "write the plan".

The agent does the work through Clarity's tools as today; the IDE turns
each edit into something the developer sees, as it happens. The backend
says what changed; the IDE shows it. The agent never clicks on the screen.
For every agent of Clarity's: its own (agent.md), an outside agent in the
panel (acp.md) and an outside agent through `clarity mcp`; not cdev.

Steps. Each tool that edits (the structural tools through `edits::apply`,
`set_handler_code`, `write_file`, `add_window` and the rest) gives one step
after its edit: what it is (a window or view made, a part placed, a field
placed, a function's code written, a file written), where (the file, the
window, the part's key or the function, from `Applied.focus`), and the
sentence the tool already answers. `clarity_tools` makes the step from the
edit, one source for every agent; an agent cannot make one up.

Delivery. The IDE's own agent hands each step to the IDE as its tool call
ends. An ACP agent's `clarity mcp --turn <file>` writes each step as one
line in the turn file, beside the show lines, and the IDE reads the file
while the turn runs, not only at its end. An outside `clarity mcp` writes
each step to a file beside `ide.json`, only while `ide.json` names a
running IDE with that project open; the IDE watches it. With no IDE
running the edit is made as today and nothing is shown.

Playing. The IDE keeps the steps in a queue and plays them one at a time:
the Design tab opens on the step's window (the code editor on its
function), the new part appears with a short highlight, code is typed in
at reading speed, and a short pause comes before the next step. The reload
the file watcher would do is the step's own, never a second refresh. The
developer can stop following and keep working; the agent's edits then land
as today. A step that fails to show says so in its own place; the edit is
there regardless.

Teaching. Clarity's instructions to agents (`clarity_tools::instructions`)
say, when the developer is learning: one visible step per call (a part, a
field, a function's code), never a whole `.cxaml` through `write_file`; say
in the chat what each step does and why, pointing at the manual (`manual`);
leave Build and Run to the developer.

Build and Run. They stay with the person. An outside agent may ask the IDE
to build or run through the ask the IDE takes, as `build_piece` does for
cdev (`dev_asks`), honouring Clarity settings > Agent (ask, allow, never),
default ask; never a project path the agent gives. Designed in mcp-tools.md
when taken up.

Every system: the IDE's own feature, the same on Linux, Windows and macOS;
every text the developer reads (the switch, a step's line) from the
translations.

The order of the steps. User: "a natural app (functional) design flow needs
to be deducted", "The order has to SOMEWHAT feel like how a person
naturally builds...", "bottom up approach?", and on designing the window
first: "then the didnt do the requirements gathering properly!". Not a fixed
script: an order a person recognizes, never random (no code for a button
not there yet). The requirements first (what the app is for, who uses it,
what it keeps), then bottom-up: the data, the windows, their parts, the
code behind them.

Open questions (names and choices not from the user):
- Where the requirements are written down where the developer sees them:
  the project's README, its guidance (`set_guidance`), or both.
- This entry's name, "learn by watching".
- The switch that stops following: its name, where it sits, on by default or not.
- The tempo: the pause between steps and the typing speed, fixed or a setting.
- Undo: an ACP turn is one Undo step today (`turn.rs`); one step per edit or the turn as one.
- The steps file beside `ide.json`: its name and its line format.
- A `.cxaml` written whole by `write_file`: shown as the parts it adds, or as one file written.

## Conflicts
- none

## Decisions
- 2026-11-18: the design above is agreed, to be built later; its open questions are settled when it is taken up. User: "Agreed, for later".

## Was
- conversation 2026-11-18 ("the old phantom tools")
