# Files and pictures

Status: built. File stores, pictures in records (`image`), files in records (`file`), files in code (`files f in store`, read, write), `zip`, and folder settings that place a store.

Progress: 90%

## Code
- Project file: `filestore name "folder" | "env:NAME" | "setting:name"` (`crates/clarity-project/src/file.rs:571`)
- Spec: types `image`, `file`, modifier `store` (`crates/clarity-spec/src/parser.rs:725-858`, `check.rs:2478`; a folder setting `check.rs:606-616`); `files f in store where ...`, `read text|bytes of f`, `write t to store "path"`, `zip(paths, texts)` (`code/check.rs:2154`), effects `files.read`, `files.write`
- Generators: `check_stores` (`crates/clarity-gen/src/lib.rs:207`), folder stores :276-297, `generate` :881; a store's folder setting held for the running handler through `held_setting` (lib.rs:352); per stack `logic.rs`
- Runtimes: R `runtimes/rust/src/{files,picture,record_file,zip,store}.rs`; Go `runtimes/go/clarity/{files,picture,record_file,zip}.go`; C `runtimes/c/clarity/{files.c,picture.c,record_file.c}`, `clarity_zip` (`code.h:105`); C++ `runtimes/cpp/clarity/{files,picture,record_file}.cpp`; C# `Clarity.Runtime/{Files,Pictures,RecordFiles,Zip}.cs`. Holding the handler's settings for a folder store: R `hold_settings`, `hold_setting` (`runtimes/rust/src/lib.rs:591-596`), Go `HoldSettings` (`picture.go:239`), C `clarity_hold_settings`, `clarity_hold_setting` (`files.c:50-54`), C++ `Db::hold_settings` (`files.cpp:34-36`), C# `HoldSettings` (`Db.cs:120`)
- Clarity.UI: `runtimes/rust-ui/src/parts/containers/image.rs`, `parts/inputs/picture_field.rs`, `parts/inputs/file.rs`, `parts/inputs/folder.rs`; system pickers `runtimes/rust-ui/src/desktop/files/` (`mod.rs`, `portal.rs`, `windows.rs`)
- IDE: file stores in `app/src/lib/ProjectSettings.svelte`; a field's store in `app/src/lib/EntityEditor.svelte`
- Docs: manual `data.pictures-files.md` (Pictures in records, Files in records, File stores, Files in code), `data.settings.md` (Folder settings), `code.queries.md` (Files in a file store)

## Tests
- `conformance/files.json`, `pictures.json`, `record-files.json`, `zip.json`: run by R, Go, C, C++, C#
- `crates/clarity-spec/tests/files.rs`, `pictures.rs`, `file_fields.rs`
- `crates/clarity-gen/tests/files.rs`, `pictures.rs`, `file_fields.rs`, `folders.rs` (the folder setting held)
- `conformance/settings.json` case `now` (a folder store reads the setting as the handler holds it): run by R, Go, C, C++, C#
- `runtimes/rust-ui/tests/inputs/inputs_file.rs`, `inputs_picture_field.rs`

## Open
- The folder picker on Windows and macOS: UNTRIED (WINDOWS-TESTS.md:65; folder settings :64).
- The file part runs no change handlers on open and save a copy: by design (`runtimes/rust-ui/src/parts/inputs/file.rs:1-9`).
- The folder part has no picker in a browser; the path is typed (`parts/inputs/folder.rs:8`).
- A `video` field kept in a file store: NOT BUILT; see [video-camera](video-camera.md).
- File stores on a server (`filestore shared sftp "sftp://..."`): NOT BUILT, noted only (APP-CAPABILITIES-PLAN.md step 3 "Later on the same line").
- Encrypted file stores: NOT BUILT; see [encrypted-storage](encrypted-storage.md).
- Picture functions (resize, crop, ...): see [picture-functions](picture-functions.md).

## Conflicts
- Generated apps open the system's folder and file pickers (`runtimes/rust-ui/src/desktop/files/mod.rs`: osascript, the Windows COM dialog, the portal), and manual `parts.input.md:202` and `data.settings.md:137` describe the folder picker as intended; CLAUDE.md says no dialog of the system, in the IDE and in generated apps. Needs a decision.

## Decisions
- 2026-10-18: `image` kept in the database or, with `store name`, as a file in a file store named by the SHA-256 of its bytes; files in a store are never deleted (X2).
- 2026-10-18: reading records never carries pictures; the screen fetches each on its own (X2).
- 2026-10-18: code reaches files only in the project's file stores, written like `find`; a path leaving the store is refused (X2).
- 2026-10-29: copying a picture or file between records works between the database and a store; a file of a store stays linked only in the same store (X2).
- 2026-10-29: a store's folder chosen while the program runs is a setting of type `folder`, not a click target (X2); see [settings-secrets](settings-secrets.md).
- 2026-11-12: a file store reads its folder setting as the running handler holds it; see [settings-secrets](settings-secrets.md).

## Was
- DESIGN roadmap X2 ("Pictures in records", "File access in code", "Copying a picture or a file", "Choosing a store's folder")
- LANGUAGE.md section 4 "Pictures in records", "Files in records", "Folder settings"; section 10 "Files"
- IMPLEMENTED.md 6 "Files and formats" (file stores), 4 (known limits of file and folder parts)
- CONFLICTING.md 1 (system pickers), 6 (`none`)
