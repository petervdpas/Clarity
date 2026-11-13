# IDE shell

Status: partial. The Tauri app's frame: ribbon, project tree, project browser, settings, status bar, keys, undo, manual window, dialogs.

Progress: 75%

## Code
- IDE: `app/` (Svelte in `app/src`, commands in `app/src-tauri/src`, registered in lib.rs:929-1076)
  - Ribbon tabs Project, Design, Preview, Vault, no menu bar (`app/src/App.svelte:108-114`, 2667 lines); a sidebar per tab; Problems list with one-click fixes; right-click menus (`lib/ContextMenu.svelte`); splitters whose sizes are kept (`lib/layout.svelte.ts`); toasts (`lib/Toasts.svelte`)
  - Project tree: App, README, Sign-in, Agents, Settings (the app's settings block), Windows, Views, Data, Mappings, Web APIs, Diagrams, Themes, i18n, Resources, Engines, Custom parts, Libraries, Commands, Modules, each with "+ new" (`lib/ProjectTree.svelte:170-476`); right-click Rename on a window or view (`App.svelte:1969`, `renamePage` :882)
  - Clarity settings > Rebuild, only in an IDE run from its repository: a tick per piece of Clarity built from its source (Clarity.UI's library and browser files, each `runtimes/*-ffi` library), kept in `target/dev-rebuild.json`; Restart (or starting the IDE with ticks) hides the IDE, builds what is ticked (each library as a release and as a debug build, which a C# Debug build links) with the splash showing each line, clears the ticks and loads the IDE again; a failed build keeps them and shows why (`app/src-tauri/src/dev_rebuild.rs`, `splash.rs` `open_building`/`line`, `lib/DevRebuildSettings.svelte`, `devRebuildStore.svelte.ts`)
  - Project browser: `lib/FileBrowser.svelte`, `fileBrowser.ts`, `fileBrowserStore.svelte.ts`; `app/src-tauri/src/browse.rs`; listing and project detection in `crates/clarity-project/src/browse.rs`; 10 recent projects and the last folder per use in `recent.json` (`crates/clarity-project/src/recent.rs`)
  - New project dialog: name, folder, backend, targets; no UI field (`lib/NewProjectDialog.svelte:5`)
  - Folder watch: outside changes reload (`app/src-tauri/src/lib.rs`, `watch`, `changes_project`); every project folder from one list, `clarity_spec::PROJECT_FOLDERS`, watched with its subfolders (fixed 2026-11-13: translations/, design/ and agent/ were not watched, so a translation added after a window stayed unseen until the project opened again)
  - Clarity settings: Display, Keys, Folders, Environment, Libraries, Agent, Logging, System, Rebuild (dev only, above) (`lib/AppSettingsView.svelte:27-37`; `folders.rs`, `logging.rs`, `system.rs`, `environment.rs`, `libraries.rs` with the factory reset :188, `lib/LibrariesSettings.svelte`, `libraries.ts`); Environment and Libraries are [build-tools](build-tools.md)
  - Keys: IDE commands per layer, rebind, swap, reset (`lib/KeysSettings.svelte`, `lib/keys.svelte.ts`, `runtimes/web/src/host/keys.ts:13-57`); WebView2's own browser keys turned off on Windows (`app/src-tauri/src/browser_keys.rs`)
  - Status bar: what is open, build state, error and warning counts, stack, load meters, last outside-agent call (`lib/StatusBar.svelte`, `statusBar.ts`)
  - Undo: project-wide undo and redo of edits, 200 steps, an agent turn is one step (`app/src-tauri/src/lib.rs:89`, `undo_edit` :693)
  - Manual: separate window, F1 opens the current tab's topic (`app/src/Manual.svelte`, `app/src-tauri/src/manual.rs`, topics in `manual/`); see [docs](docs.md)
  - Dialogs: confirm, message, prompt, choose (`lib/dialogs.ts`, `dialogStore.svelte.ts`, `DialogHost.svelte`, `Modal.svelte`)
  - Fail safe: drawing errors caught per area, panics reported (`app/src-tauri/src/fail_safe.rs`); splash at start (`splash.rs`)
  - Build and run buttons: Generate, Build, Run, Stop (`App.svelte:1540-1562`), `app/src-tauri/src/build.rs`; see [build-tools](build-tools.md)
- Docs: manual/start.overview.md, manual/app.keys.md

## Tests
- `app/src/lib/dialogs.test.ts` (no plugin ask/message, no window.alert/confirm/prompt, no backdrop outside Modal), fileBrowser.test.ts, statusBar.test.ts, libraries.test.ts, shortcutKeys.test.ts (loads conformance/shortcut-keys.json), manual.test.ts, ideTheme.test.ts, systemLoad.test.ts, version.test.ts
- `crates/clarity-project/tests/browse.rs`, recent.rs

## Open
- Translations of the IDE's own words: NOT BUILT. English in the Svelte files (ACP-PLAN.md:46 and the project rule)
- Project-wide search: NOT BUILT. Only Ctrl+F in editors and filters in translations, vault and manual
- The file browser starts in the open project's folder: PARTIAL (`app/src-tauri/src/browse.rs:56` starts at the last folder, then the asked one, then Home)
- System dialog plugin: PARTIAL. No screen uses it, but `tauri-plugin-dialog` is a dependency and initialised (`app/src-tauri/Cargo.toml:36`, `app/src-tauri/src/lib.rs:921`, `app/package.json:29`)
- Project browser manual pages (plan step 4): UNTRIED as a whole; only `manual/start.overview.md` names Recent projects and New project here
- No macOS IDE installer; see [release-ci](release-ci.md)

## Conflicts
- CLAUDE.md "Every text a person reads comes from the translations" vs the IDE's English words in `app/src/**/*.svelte`
- manual/start.overview.md:41 tree names (App, Settings, Windows, Data, Web APIs, Diagrams, Themes, Languages, Resources, Modules): the tree says i18n, not Languages; README, Sign-in, Agents, Views, Mappings, Engines, Custom parts, Libraries, Commands are missing; sidebar (Main window); :52 settings sections leave out Keys, Environment, Logging
- manual/app.keys.md:76-83 vs design.text.md:55-56: Tab is always four spaces (`lib/SourceEditor.svelte:72`); Escape closes the panel vs goes back to Properties

## Decisions
- 2026-10-20: the IDE works like C++ Builder, modern; every step is edited, built and fixed inside Clarity. User: "like C++ Builder but then modern"
- 2026-10-22: no system or browser dialogs; one dialogs module. User: "no js dialogs please (is rule!!!)", "make multi-purpose dialog module!"
- 2026-10-25: IDE views and data load on demand (mounted only while shown, `import()` for heavy pages) (memory: lazy-views.md)
- 2026-10-25: the IDE is the pilot of the keys model, keys always editable (DESIGN 5.28)
- 2026-11-10: Clarity's own file browser everywhere. User: "I want to be able to create and open a project from anywhere! Like Rider does... and open project dialog... (with new project) that can detect if a project is a clarity project"; "everywhere, write the plan first"
- 2026-11-10: recent projects and the last folder. User: "10 recent, remember last folder, build it after release"
- 2026-11-10: the New project dialog loses its UI field. User: "yes, after the release"
- 2026-11-10: a folder is a project only when it holds a `.clarity` file whose first line starts with `project` (no guessing)
- 2026-11-13: Clarity's own pieces are rebuilt by the IDE in development, never by an agent by hand. User: "the ide knows weather its dev or real... when dev it get a section in settings to tick the things that need rebuild and a restart button, that build and shows ide when done... it also resets all tick!"; "the splash shows the build!!! line for line"; an agent sets the ticks.

## Was
- S2, EN2, EN3, DESIGN 5.28 (the IDE pilot bullet; the rest is [frame-keys](frame-keys.md))
- PROJECT-BROWSER-PLAN.md steps 1-4
- memory: project-browser-plan.md, lazy-views.md
