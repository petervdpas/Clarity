# Clarity's own file browser

A plan, written 2026-11-10 at the user's request and agreed the same day. Steps 1 to 3 are built (2026-11-10); step 4, the manual, is open. The user: "I think you plan now, but not build... that'll be tomorrow".

## Goal

The user's words, about the New project dialog that took its folder from Clarity settings: "yeah, settings holds the path I see... but I want to be able to create and open a project from anywhere! Like Rider does... and open project dialog... (with new project) that can detect if a project is a clarity project".

Asked whether the address bar is for typing, the user said "type or paste... browse you mean?": browsing is the way, typing a path is a shortcut. Asked where it replaces the system's file window: "everywhere, write the plan first".

So the IDE gets one browser of its own, in a window of Clarity's (no system dialog, as with every other dialog in Clarity), and uses it wherever it now opens the system's file or folder window. Opening a project shows which folders hold a Clarity project, and a new project can be made from where you are.

## What it is

One window, used in four ways:

| Use | What it picks | What it shows |
|---|---|---|
| Open project | a Clarity project | every folder; projects marked |
| Choose a folder | a folder | folders only |
| Choose a file | an existing file of the kinds asked (a picture, an OpenAPI description, a database) | folders, and files of those kinds |
| Save as | a new file's place and name | folders, and files of that kind, with a name field |

Its parts:

- **Address bar** on top: where you are. Browsing is the way (click into a folder, Up to go back); typing or pasting a path and Enter jumps there, and a path that is no folder says so under the bar.
- **Up** and **Home** beside it. On Windows also the drives (C:, D:, ...), since there is no single root.
- **The list**: folders first, then files, by name. A double click goes into a folder or picks a file. Hidden files (a name starting with a dot, or hidden on Windows) are left out, with a switch to show them.
- **Places** on the left: Home, the projects folder of Clarity settings, the apps folder, and (when opening a project) **Recent projects**.
- **Buttons**: the use's own (Open, Choose, Save), Cancel, and when opening a project **New project here**, which opens the New project dialog with this folder filled in.

## A Clarity project, told exactly

As built: a project is a file (`shop.clarity`) beside the folder of its modules (`shop/`), often several side by side (as in `examples/`). So the list marks the project file itself with its name and stack, the folder beside it as "the modules of Shop", and a folder that holds project files as holding them.


No guessing (the house rule: determinism). A folder is shown as a Clarity project when it holds a project file:

- a file ending in `.clarity` whose first line, leaving out blank lines and comments, starts with `project` (`project Shop`), as the project file's reader takes it;
- the list shows its name (from that first line) and its stack, read with the same reader the IDE opens projects with; a project file that does not read shows as a project with its problem instead of a stack.

Such a folder shows the Clarity icon. Opening it opens its project file; with more than one project file in it, the folder opens to show them, each marked. A `.clarity` file that is a module, not a project file, shows as a plain file.

## Recent projects

The 10 projects opened last, newest first, kept in Clarity's own folder (`recent.json`, like `ide.json`), changed by read-modify-write. A project that is no longer there is left out of the list, not removed from the file. The IDE's start already remembers the last project; that becomes the top of this list.

## Where it replaces the system window

| Where | Now | Becomes |
|---|---|---|
| Project tab, Open | system Open, a `.clarity` file | Open project |
| New project dialog, Folder: Browse... | system folder window | Choose a folder |
| Clarity settings, Folders (projects, apps) | system folder window | Choose a folder |
| Clarity settings, Environment (a build tool's program) | system Open | Choose a file (any) |
| Project settings, app folder | system folder window | Choose a folder |
| Project settings, a SQLite database file | system Save | Save as (`.db`, `.sqlite`, `.sqlite3`) |
| Project settings, a file store's folder | system folder window | Choose a folder |
| Resources: a new resource, replacing one | system Open | Choose a file (the resource's kinds) |
| Web APIs: an OpenAPI description | system Open | Choose a file (`.yaml`, `.yml`, `.json`) |

Every one gets the browser's own start folder: the one it shows now (`defaultPath`), else the open project's folder, else Home.

## How it is built

- **Backend lists, frontend shows** (the house rule: backend drives frontend). A command in `crates/clarity-project` (and the IDE's command over it) lists a folder: its entries with name, path, folder or file, size, and for a project file its name, stack or problem; Up's path; on Windows the drives. Reading a project file uses the project reader that opening uses, so what the list says is what opening finds.
- **One Svelte part**, `FileBrowser.svelte`, with the four uses as a prop, called through one function (`browse({ use, title, start, kinds })`) that returns the path chosen or nothing, like Clarity's dialogs module does for its dialogs.
- **Every OS**: paths, Home and hidden files the same way on Linux, Windows and macOS; the drives only on Windows. A folder that cannot be read says so in the list instead of failing.
- **Texts**: the IDE's own words, as its other windows have them.
- **Tests first**: the listing (folders before files, hidden left out, kinds filtered, a project file told from a module, a broken project file shown with its problem, a folder that cannot be read), recent projects (newest first, one entry per project, missing ones left out), and the frontend's picking (double click, Up, typed path).

## Order of work

1. The listing and the project detection in `clarity-project`, with recent projects (tests first).
2. `FileBrowser.svelte` and `browse()`, with Open project and New project here; the New project dialog without its UI field.
3. Every other place in the table moves to it; the system dialog plugin is then no longer used by the IDE's own screens.
4. Manual: the Open and New project pages, and a word in each page whose Browse changed.

## Decided

- Recent projects and the last folder: the user said "10 recent, remember last folder, build it after release". So Recent projects keeps the last 10, and the browser starts each use where it was left last time (per use: Open project, a folder, a file kind); the table's start folder is for its first time.
- The New project dialog loses its UI field: every backend offers only Clarity.UI for a new project, so the field is a list of one. Asked, the user said "yes, after the release". Project settings keeps showing the UI as text, so an older project still says what it uses.

## Open questions

- Whether network places (smb, mounted drives) need anything beyond what the system already shows as folders.
