# Settings and secret settings

Status: built. One `settings` block of typed values kept per person on their machine (`settings.json`), with `secret` settings in the system's protected store and `folder` settings that place a file store.

Progress: 90%

## Code
- Spec: `settings` block (`crates/clarity-spec/src/parser.rs:625`, `check.rs:583-625`; the folder check :606-616), the record type `Settings` (`Entity.settings`, `FieldType::Language`, `Theme`, `Secret`, folder); Clarity's own settings `language`, `theme`, `agentReadsWhileClosed`, `agentChangesWhileClosed` (`parser.rs:628-634`, `OWN_SETTINGS` :676; the two agent settings refused when no command says `agent`, `check.rs:585-592`; see [app-mcp](app-mcp.md)); settings window template (`crates/clarity-spec/src/templates.rs:301`, :445); secret and unconnected keys in `translations.rs`
- Project file: `setting:name` in `auth` and `datasource` locations, `filestore x "setting:name"`; `keychain:` refused (`crates/clarity-project/src/lib.rs:748`, :757)
- Generators: settings program name `clarity_gen::settings_program`; secret locations written as `setting:<program>/<name>`; `keychain:` refused (`crates/clarity-gen/src/lib.rs:67`); a program without a secret setting gets no secrets code
- Runtimes: settings R `runtimes/rust/src/settings.rs`, Go `runtimes/go/clarity/settings.go`, C `runtimes/c/clarity/settings.c`, C++ `runtimes/cpp/clarity/settings.cpp`, C# `Clarity.Runtime/KeptSettings.cs` (`Merged` :88), `ISettings.cs`. Keeping settings merges into the file as it is on disk on every runtime. Secrets R `runtimes/rust/src/secrets.rs` (keyring; Win32 Credential Manager itself), Go `runtimes/go/secrets/secrets.go` (go-keyring), C `runtimes/c/secrets/secrets.c`, C++ `runtimes/cpp/secrets/secrets.cpp` (gio, wincred, Security), C# `Clarity.Runtime/Secrets.cs`
- Clarity.UI: password input with eye and copy (`runtimes/rust-ui/src/parts/text_box.rs`), folder part (`parts/inputs/folder.rs`)
- IDE: the settings block editor `app/src/lib/SettingsBlockEditor.svelte` (Settings in the project tree, mounted in `App.svelte:1716`; types from `dictionary.rs` SETTING_TYPES); New window "Settings window"; the IDE's own vault is separate and not used by generated apps; see [vault](vault.md)
- Docs: manual `data.settings.md` (Secret settings, Folder settings, Reference settings), `project.file.md`, `languages.md`

## Tests
- `conformance/settings.json` (cases fits, kept, file, folders, now, merge): run by R, Go, C, C++, C#
- `conformance/secret-store.json`: run by R (`runtimes/rust/tests/secrets.rs`), C# (`SecretsTests.cs`), gen `secrets.rs`; skipped by Go, C, C++ (they write the keychains themselves)
- `conformance/unconnected.json`: run by R, Go, C, C++, C#
- `crates/clarity-spec/tests/spec/settings.rs`, `secrets.rs`; `crates/clarity-project/tests/settings.rs`; gen `settings.rs`, `secrets.rs`, `app_settings.rs`, `reference_settings.rs`, `unconnected_csharp.rs`; real-store tests with `CLARITY_SECRET_STORE=1` (Linux with gnome-keyring)

## Open
- Secrets stored alike on every stack: PARTIAL. On macOS go-keyring stores `go-keyring-base64:` values no other runtime reads (`runtimes/go/secrets/secrets.go:11-40`).
- One name for secrets in the Credential Manager on every stack: UNTRIED on Windows ([os-checks](os-checks.md)).
- The `forget` option on a secret (kept in memory for the run only): NOT BUILT; see [encrypted-storage](encrypted-storage.md) Design "Forget".

## Conflicts
- In C# the person's keys have two writers in `settings.json`: Clarity.UI (`runtimes/rust-ui/src/window/mod.rs:103-108`) and `PersonKeys`, read once at start (`crates/clarity-gen/src/csharp/model.rs:287`, `runtimes/csharp/Clarity.Runtime/PersonKeys.cs:25`); see [frame-keys](frame-keys.md).
- `conformance/secret-store.json` is skipped by Go, C and C++.

## Decisions
- 2026-10-23: one unnamed `settings` block per project, kept per person in the system's settings folder, never in the database; `language` and `theme` are Clarity's own lines (DESIGN 5.17, U1g).
- 2026-10-25: the settings folder is the project's name in snake case on every stack and the command line.
- 2026-10-25: a `secret` setting lives in the system's protected store, per program and per person, alike on every stack; never in `settings.json`; only a password input shows it (DESIGN 5.27).
- 2026-10-25: a secret signs or connects through `setting:name`; `keychain:NAME` is refused with that advice.
- 2026-10-25: a password input shows or copies its secret. User: "passwords, secrets and alike should have an option (open eye/closed eye) to show it or copy it"
- 2026-10-29: a store's folder chosen while the program runs is a `folder` setting, read at every use (X2).
- 2026-11-12: a file store reads its folder setting as the running handler has it: each call holds the settings its window's data began with, and code that sets a folder setting holds it at once (`hold_settings`, `hold_setting` on the Db of R, Go, C, C++, C#); settings are still kept only after the handler ends. The rule is `now` in `conformance/settings.json`, run by every runtime (R `settings.rs`, Go `settings_test.go`, C `settings_test.c`, C++ `settings_test.cpp`, C# `SettingsCaseTests.cs`); the generators write the holds through one helper (`held_setting`, `crates/clarity-gen/src/lib.rs`), pinned by gen `folders.rs`. C# keeps a setting the moment code changes it, so its generator holds nothing. Found by the maintenance example: picking a project set `ledgerFolder` and listed the ledger in one handler, which read the old folder. User chose "Hold settings in memory (Recommended)". Released in v0.42.0 (runtimes/rust v0.58.0, go v0.61.0, csharp v0.94.0).
- 2026-11-23: a setting is always one the person can reach. The checker warns about a setting of a project with windows that no part binds (in a page or a frame's sidebar or footer), no picker opens (`language`, `theme`) and no handler sets: "no window shows setting 'x', so the person using the app cannot set it; bind a part to settings.x, such as in a settings window (New window, Settings window) opened from a menu" (`check/settings.rs`, test `settings.rs` `a_setting_no_window_shows_is_warned_about`). A folder or file a project names is never a place on one machine: the project file refuses an absolute path, a `~` home path or a drive letter in a SQLite or file data source, a file store and a codec (`clarity_project::one_machine_problem` on `clarity_spec::files::one_machine`), and code refuses one at the start of a path it writes (`a_written_path_is_inside_its_store`); the output folder may still be absolute (pinned by `the_output_folder_is_never_the_project_itself`). User: "No, I mean its probably hardcoded in the app, but it needs settings!!! And settings should be accessible via a menu or something!!!", "hard coded folders or files are a problem when the examples run on a other environment". See [examples](examples.md).
- 2026-11-23: a relative file store and a relative folder setting (its default, or a relative folder the person typed) are in the program's data folder, found while it runs, exactly as a relative SQLite database is; nothing of the build machine is written into generated code. The generator writes a relative store as `data:<program>/<folder's name>` (`clarity_project::build_settings`, `store_kept`); each runtime finds the data folder (CLARITY_DATA, else the person's folder for application data under the program's name, else where the program runs) and the store in it: R `data_folder::folder`, `data_folder::store` (`runtimes/rust/src/data_folder.rs`), Go `DataFolder`, `DataFolderStore` (`data_folder.go`), C# `DataFolder.Folder`, `DataFolder.Store` (`Store.cs`), C `clarity_data_folder`, `clarity_data_folder_store` (`store.c`), C++ `data_folder`, `data_folder_store` (`store.cpp`); a folder setting's relative folder goes through `store_folder_in` with the data folder in place of the settings folder. `env:NAME` unset never stops the program: the store says so where it is used, and a database at it stops the start with the same words, Clarity's `variableNotSet` ("The environment variable {variable} must be set"); code listing an empty folder says `noFolderChosen` ("No folder was chosen"); both are in `RUNTIME_ERROR_KEYS`, in every example's and test project's language files. Pinned by `conformance/data-folder.json` (`folders`, `stores`) and `conformance/settings.json` (`folders`, now `dataFolder`), read by R, Go, C, C++, C#. Clarity's Run names in CLARITY_DATA the folder of the first own SQLite database, else of the first relative file store (`clarity_project::data_folder`). User: "hard coded folders or files are a problem when the examples run on a other environment", "check all examples for errors like pointing to a folder and not having that set in a setting".

## Was
- DESIGN 5.17, 5.27 (secrets, eye and copy; "When the databases do not open" is in [databases](databases.md)), roadmap U1g
- LANGUAGE.md section 4 "Settings", "Secret settings", "Folder settings"
- IMPLEMENTED.md 6 "Settings and secrets"
- UNIMPLEMENTED.md 6 "Secrets"; 13 (Credential Manager name)
- CONFLICTING.md 2 (C# keys writers), 3 (secret-store)
