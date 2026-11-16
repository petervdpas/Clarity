# DevTools

Status: built. Clarity settings > DevTools, only in an IDE run from its repository: a tab per piece of Clarity an app uses (the release or the cDdev's own build, with Build) and the Release planning tab, for review: the release checks with one button and one outcome line, the chain of steps a release would start, and the changes since each release by kind. The agent reads the same through MCP (`release_state`) and writes the plan; the tab writes none. An agent presses a Build through Clarity's MCP (`build_piece`).

Progress: 90%

## Code
- Core: `crates/clarity-gen/src/tools/devtools/` (`swap.rs` pieces, swaps, `build`, `behind`, `choices` putting native libraries together in `target/clarity-swap`; `jobs.rs` builds one at a time, lines read as a terminal shows them; `release.rs` `UNITS` in release order, `standing`, `bump`, `GUARDS`, `EXAMPLES`, `changes` by kind, `chain_of`/`chain`, `checks`, `fresh_checkout` in `target/fresh-checkout` building in `target/fresh-target` where the tests, spec checks and example builds run, `clean_tree` last, `record_check`/`check_records` in `target/release-checks.json`); MCP `build_piece`, `release_state`, `disk_state`, `remove_folder`, `write_release_plan` in `crates/clarity-mcp/src/devtools.rs` (clarity-cdev only, :20-28), compiled only with `debug_assertions` (guard `the_devtools_tools_are_never_compiled_into_a_release`); asks carry an `action` (build, remove) in `crates/clarity-project/src/dev_asks.rs`; CLI `clarity release fresh-checkout` / `clean-tree` (`crates/clarity-cli/src/main.rs`); swaps kept in environment.json (`environment.rs` `swaps`, `set_swaps`); Go module swapped by a `replace` line (`go/mod.rs`)
- Asks: `crates/clarity-project/src/dev_asks.rs` (files in `~/.clarity/dev-asks`); MCP tool `build_piece` (`crates/clarity-mcp/src/lib.rs`, debug builds, not in chat turns)
- IDE: `app/src-tauri/src/devtools/mod.rs` (tabs, swaps, `devtools_build`, `watch_asks`), `devtools/release.rs`, `devtools/disk.rs` (Disk tab, free space by sysinfo, Remove refused while a build runs); `app/src/lib/DiskSettings.svelte`; core `crates/clarity-gen/src/tools/devtools/disk.rs` (`places`, `size`, `remove`), `Swaps::shown` (a piece whose own build is behind shows by itself); `app/src/lib/DevToolsSettings.svelte`, `ReleasePlanning.svelte`, `BuildLines.svelte`, `devtoolsStore.svelte.ts` (an agent's ask allowed as Clarity settings > Agent says), `devtools.ts`
- An installed Clarity has no DevTools and uses the release; only a debug IDE or CLI reads the swaps

## Tests
- `crates/clarity-gen/tests/devtools.rs`, `disk.rs`, `swaps.rs`, `library_choices.rs`; `crates/clarity-mcp/tests/server.rs` (`build_piece_asks_the_ide_and_gives_its_answer`); `app/src/lib/devtools.test.ts`, `browserFiles.test.ts`

## Open
- Release planning lists notes marked partial and tests noted failing: NOT BUILT
- An MCP server that follows a new binary without a reconnect: NOT BUILT (offered 2026-11-15, not agreed)
- `build_piece` and `release_state` from a real MCP client (needs the local MCP entry removed and a reconnect), a full Run release checks with the fresh checkout's own build folder: UNTRIED
- The IDE's page checks (svelte-check, npm test) run in the tree, which has node_modules, not in the fresh checkout: PARTIAL
- `clarity libraries` (CLI) still rebuilds the libraries outside DevTools: OPEN, ask the user

## Conflicts
- none

## Decisions
- 2026-11-14: the IDE never builds Clarity's pieces at start; installed Clarity uses the release. User: "normally an app uses pre-build libs... not everybody is the developer of clarity", "the crates should be swappable!"
- 2026-11-14: tabs, core always on. User: "the tabs must able to add tabs for part the 'clarity dev' (going to call him/her cDdev from now on) knows have changes, but the core parts are always on"
- 2026-11-15: User: "Call it DevTools, and use use this for both the release planning and swapping. Every tab should have a build button for the own build to do the building of new stuff there! You should also use that button... and if a build is in progress the tab should show it!"
- 2026-11-15: a progress bar drawn over with `\r` is one line. User: "it shouldnt show more lines after each other"

- 2026-11-16: the checks run in a fresh checkout as on GitHub and end with a clean-tree check, after 1.2.0's checks failed on a test needing tags and stray test files went into the commit. User: "why you keep repeating the same mistakes over and over?", "can it be a button/command so I can be the instigator... I have a longer recollection than you!"

- 2026-11-16: the tab writes no plan and has no ticks, Bump or versions: user "not really made for humans, is it... untick all the boxes, really... no indicatore to focus on", "I never said the release plan should be written from inside... at the most you (mcp) can fetch it!", "I still want to be able to review things, so the tab stays", "a tree/list of chained (ci) events would be great!". The fresh checkout builds in its own folder: sharing target/ made cargo take the checkout's builds of workspace crates as the tree's (same names), and a test linked a stale library.

- 2026-11-16: Disk tab, auto-shown tabs and tab arrows. User: "build the disk tab... all libs that have that [!] should be added to the 'active' tabs... does that tab component have 'tab scrolling'? Two arrows (left and right)". A piece shows by itself only when its own build exists and is behind; never built stays under + Add a piece.

- 2026-11-16: DevTools tools never in a release MCP server; Disk and Release planning first and permanent, then pieces marked !, then the rest; arrows cycle the tabs; every piece tab closes (kept closed in the swaps until added again). User: "these actions (all on the DevTools page, should not go in the release mcp server!", "Put Disk and Release Planning as the first permanent tabs! Then the libs with exclamation marks! And something to cycle tabs", "tabs should be able to be removed too... all but the permanent ones!!". A removal an agent asks for is always asked of the person (unless never), even with agents allowed.

- 2026-11-16: by default only Disk, Release planning and the pieces marked ! (own build there and behind its source); core pieces have no tab of their own any more, a piece keeps its tab while added or swapped. User: "it default only has to show tab 'Disk', 'Release Planning' and any library or part with an exclamation mark!!". A piece never built does not show by itself (else removing target/release would open every native library's tab).

- 2026-11-16: a closed tab marked ! stays away only while Clarity runs (kept in the IDE, `hidden()` in `app/src-tauri/src/devtools/mod.rs`), never in environment.json. User: "restarting the ide does not show the parts with the exclamantion mark again!"

- 2026-11-16: a swap did not reach C and C++ apps: CMake kept the Clarity.UI folder of the first build (the release) over the one Clarity names each build; `runtimes/c|cpp/clarity.cmake` now let `CLARITY_UI` from the environment win (FORCE), and Rust's build.rs reruns when the library folder changes. Go loads from the named folder. Guards: `crates/clarity-gen/tests/swap_reaches_builds.rs` (real CMake, both runtimes), `rust::swapped_library`. User: "FIX ALL!!!!!"

- 2026-11-16: the release safeguard. A release check counts only on the tree it ran on (`release::tree`: HEAD plus a hash of the uncommitted changes and their contents, kept in each `CheckRecord`); `release_state` lists other trees' results as not run and says `planAllowed`; the MCP tool `write_release_plan` (clarity-cdev only) refuses while any check is held back, writes `releases/<stamp>.json` starting with the checks on main, never overwrites, and changes a failed release's commit message only once the checks passed again; a project hook (`.claude/settings.json`) refuses writing `releases/` by hand; Release planning shows "A plan can be written" or "Held back: N checks not passed on this tree". The stamp stays out of the plan: the Release Helper refuses unknown fields. User: "the ide and mcp should safeguard this!!!!", "yes, build 1 to 4".
- 2026-11-16: the checks run as on GitHub. A check counts on the tree it started on (for a check in the fresh checkout, the tree the checkout was made from, `release::checkout_tree`), never the tree when it ended; checks in the checkout clear what dev.sh sets (`CLARITY_RUNTIME`, `CLARITY_UI`, `CLARITY_UI_BROWSER`, `CLARITY_PROJECT`), use the real repository path, pass a test target's `required-features`, build `clarity-ui-ffi` before its tests (as checks.yml does), and give example builds the library the IDE builds apps with. Release planning has Stop (`release_stop`, `Jobs::stop`: the running check's process group ends, the waiting ones never start). User: "add the stop button too?".
- 2026-11-16: a check counts on what it reads, not the whole tree (`TreeState::fingerprint`): a crate's check reads every file but the `.rs` and `Cargo.toml` of the workspace crates outside its closure (`cargo metadata --offline`, its own and its tests' needs, theirs without tests'), with the versions its closure resolves to in place of Cargo.lock, and its command; the IDE pages' checks read every file but Rust and Cargo's; the fresh checkout and the clean tree read all of it. Results are written beside the file and moved over it, and the checks build without debug info (35 GB with it). User: "better do the fingerprints now!!".

## Was
- Clarity settings > Rebuild (`dev_rebuild.rs`, `target/dev-rebuild.json`), Clarity settings > Libraries (`libraries.rs`), the Swap section
