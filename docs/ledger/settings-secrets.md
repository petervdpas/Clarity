# Settings and secret settings

Status: built. One `settings` block of typed values kept per person on their machine (`settings.json`), with `secret` settings in the system's protected store and `folder` settings that place a file store.

## Code
- Spec: `settings` block (`crates/clarity-spec/src/parser.rs:624`, `check.rs:583-603`), the record type `Settings` (`Entity.settings`, `FieldType::Language`, `Theme`, `Secret`, folder); settings window template (`crates/clarity-spec/src/templates.rs:302`, :442); secret and unconnected keys in `translations.rs`
- Project file: `setting:name` in `auth` and `datasource` locations, `filestore x "setting:name"`; `keychain:` refused (`crates/clarity-project/src/lib.rs:607`)
- Generators: settings program name `clarity_gen::settings_program`; secret locations written as `setting:<program>/<name>`; `keychain:` refused (`crates/clarity-gen/src/lib.rs:67`); a program without a secret setting gets no secrets code
- Runtimes: settings R `runtimes/rust/src/settings.rs`, Go `runtimes/go/clarity/settings.go`, C `runtimes/c/clarity/settings.c`, C++ `runtimes/cpp/clarity/settings.cpp`, C# `Clarity.Runtime/KeptSettings.cs`, `ISettings.cs`. Secrets R `runtimes/rust/src/secrets.rs` (keyring; Win32 Credential Manager itself), Go `runtimes/go/secrets/secrets.go` (go-keyring), C `runtimes/c/secrets/secrets.c`, C++ `runtimes/cpp/secrets/secrets.cpp` (gio, wincred, Security), C# `Clarity.Runtime/Secrets.cs`
- Clarity.UI: password input with eye and copy (`runtimes/rust-ui/src/parts/text_box.rs`), folder part (`parts/inputs/folder.rs`)
- IDE: New window "Settings window"; the IDE's own vault is separate and not used by generated apps; see [vault](vault.md)
- Docs: manual `project.file.md`, `languages.md`; LANGUAGE.md section 4 "Settings" (:619), "Secret settings" (:641), "Folder settings" (:656)

## Tests
- `conformance/settings.json`: run by R, Go, C, C++, C#
- `conformance/secret-store.json`: run by R (`runtimes/rust/tests/secrets.rs`), C# (`SecretsTests.cs`), gen `secrets.rs`; skipped by Go, C, C++ (they write the keychains themselves)
- `conformance/unconnected.json`: run by R, Go, C, C++, C#
- `crates/clarity-spec/tests/settings.rs`, `secrets.rs`; `crates/clarity-project/tests/settings.rs`; gen `settings.rs`, `secrets.rs`, `app_settings.rs`, `reference_settings.rs`, `unconnected_csharp.rs`; real-store tests with `CLARITY_SECRET_STORE=1` (Linux with gnome-keyring)

## Open
- Secrets stored alike on every stack: PARTIAL. On macOS go-keyring stores `go-keyring-base64:` values no other runtime reads (`runtimes/go/secrets/secrets.go:11-40`).
- One name for secrets in the Credential Manager on every stack, needing a Rust runtime release: UNTRIED (WINDOWS-TESTS.md:29-35).
- The `forget` option on a secret (kept in memory for the run only): NOT BUILT; see [encrypted-storage](encrypted-storage.md).

## Conflicts
- DESIGN 5.17 (DESIGN.md:1148) says a reference is no setting; LANGUAGE.md:632 and the code allow `ref` settings (`crates/clarity-gen/tests/reference_settings.rs`).
- DESIGN 5.17 and 5.27 say the web renderer and Avalonia hold settings and the eye buttons; Clarity.UI and the runtimes do on every stack.
- In C# the person's keys have two writers in `settings.json`: Clarity.UI (`runtimes/rust-ui/src/window/mod.rs:104-109`) and `PersonKeys`, read once at start (`crates/clarity-gen/src/csharp/model.rs:287`, `runtimes/csharp/Clarity.Runtime/PersonKeys.cs:25`); see [frame-keys](frame-keys.md).
- `conformance/secret-store.json` is skipped by Go, C and C++ (CONFLICTING.md 3).

## Decisions
- 2026-10-23: one unnamed `settings` block per project, kept per person in the system's settings folder, never in the database; `language` and `theme` are Clarity's own lines (DESIGN 5.17, U1g).
- 2026-10-25: the settings folder is the project's name in snake case on every stack and the command line.
- 2026-10-25: a `secret` setting lives in the system's protected store, per program and per person, alike on every stack; never in `settings.json`; only a password input shows it (DESIGN 5.27).
- 2026-10-25: a secret signs or connects through `setting:name`; `keychain:NAME` is refused with that advice.
- 2026-10-25: a password input shows or copies its secret. User: "passwords, secrets and alike should have an option (open eye/closed eye) to show it or copy it"
- 2026-10-29: a store's folder chosen while the program runs is a `folder` setting, read at every use (X2).

## Was
- DESIGN 5.17, 5.27 (secrets, eye and copy; "When the databases do not open" is in [databases](databases.md)), roadmap U1g
- LANGUAGE.md section 4 "Settings", "Secret settings", "Folder settings"
- IMPLEMENTED.md 6 "Settings and secrets"
- UNIMPLEMENTED.md 6 "Secrets"; 13 (Credential Manager name)
- CONFLICTING.md 2 (C# keys writers), 3 (secret-store)
