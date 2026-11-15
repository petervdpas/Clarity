# DevTools

Status: built. Clarity settings > DevTools, only in an IDE run from its repository: a tab per piece of Clarity an app uses (the release or the cDdev's own build, with Build) and the Release planning tab, which writes the Release Helper's plan from the repository as it is. An agent presses a Build through Clarity's MCP (`build_piece`).

Progress: 90%

## Code
- Core: `crates/clarity-gen/src/tools/devtools/` (`swap.rs` pieces, swaps, `build`, `behind`, `choices` putting native libraries together in `target/clarity-swap`; `jobs.rs` builds one at a time, lines read as a terminal shows them; `release.rs` `UNITS` in release order, `standing`, `bump`, `GUARDS`, `EXAMPLES`, `checks`, `plan`, `write_plan`); swaps kept in environment.json (`environment.rs` `swaps`, `set_swaps`); Go module swapped by a `replace` line (`go/mod.rs`)
- Asks: `crates/clarity-project/src/dev_asks.rs` (files in `~/.clarity/dev-asks`); MCP tool `build_piece` (`crates/clarity-mcp/src/lib.rs`, debug builds, not in chat turns)
- IDE: `app/src-tauri/src/devtools/mod.rs` (tabs, swaps, `devtools_build`, `watch_asks`), `devtools/release.rs`; `app/src/lib/DevToolsSettings.svelte`, `ReleasePlanning.svelte`, `BuildLines.svelte`, `devtoolsStore.svelte.ts` (an agent's ask allowed as Clarity settings > Agent says), `devtools.ts`
- An installed Clarity has no DevTools and uses the release; only a debug IDE or CLI reads the swaps

## Tests
- `crates/clarity-gen/tests/devtools.rs`, `swaps.rs`, `library_choices.rs`; `crates/clarity-mcp/tests/server.rs` (`build_piece_asks_the_ide_and_gives_its_answer`); `app/src/lib/devtools.test.ts`, `browserFiles.test.ts`

## Open
- Release planning lists notes marked partial and tests noted failing: NOT BUILT
- Release planning from MCP (standing, run checks, write plan): NOT BUILT (asked about 2026-11-15, not agreed)
- An MCP server that follows a new binary without a reconnect: NOT BUILT (offered 2026-11-15, not agreed)
- `build_piece` from a real MCP client, Release planning writing a real plan: UNTRIED

## Conflicts
- none

## Decisions
- 2026-11-14: the IDE never builds Clarity's pieces at start; installed Clarity uses the release. User: "normally an app uses pre-build libs... not everybody is the developer of clarity", "the crates should be swappable!"
- 2026-11-14: tabs, core always on. User: "the tabs must able to add tabs for part the 'clarity dev' (going to call him/her cDdev from now on) knows have changes, but the core parts are always on"
- 2026-11-15: User: "Call it DevTools, and use use this for both the release planning and swapping. Every tab should have a build button for the own build to do the building of new stuff there! You should also use that button... and if a build is in progress the tab should show it!"
- 2026-11-15: a progress bar drawn over with `\r` is one line. User: "it shouldnt show more lines after each other"

## Was
- Clarity settings > Rebuild (`dev_rebuild.rs`, `target/dev-rebuild.json`), Clarity settings > Libraries (`libraries.rs`), the Swap section
