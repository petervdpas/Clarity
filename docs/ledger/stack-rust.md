# Rust stack

Status: built. `stack rust ...`: a Cargo program on the crate `clarity-runtime`, drawn by Clarity.UI's ready-made library.

Progress: 95%

## Code
- Spec: `crates/clarity-project/src/file.rs:401-473` (stack line); `ui library` / `ui crate` read and dropped (file.rs:366-368)
- Generators: `crates/clarity-gen/src/rust/` (logic.rs 2219 lines, mod.rs 1540, parts.rs 690, model.rs 436, cli.rs 319, names.rs 122)
- Runtimes: R `runtimes/rust` (crate `clarity-runtime` 0.56.0, `runtimes/rust/Cargo.toml:6`); modules in `src/`: lib.rs, program.rs, native_ui.rs, wire.rs, serve.rs, ui.rs, query.rs, sql.rs, dialect.rs, sqlite.rs, postgres.rs, mysql.rs, mssql.rs, edits.rs, backup.rs, signin.rs, settings.rs, secrets.rs, files.rs, picture.rs, record_file.rs, file_source.rs, formats.rs, http.rs, engine.rs, zip.rs, cli.rs, and the part rules (tables, trees, ranges, steps, charts, dropdown_options, reorder, markdown, keys)
- Cargo features: sqlite (default), postgres, mysql, mssql, engines (wasmi), http (ureq, rustls), pictures (Clarity.Image), formats (`runtimes/rust/Cargo.toml:20-32`)
- Desktop program: `main.rs` runs `clarity_runtime::native_ui::run` (feature `native-ui`, `rust/mod.rs:108, 641`); build.rs links the Clarity.UI library from the folder CLARITY_UI names; audio and video handed over by `native_ui::use_audio` / `use_video` (`rust/mod.rs:636-639`)
- Web: same crate, browser files embedded; `--serve` serves when both targets are set (`rust/mod.rs:816`); see [web-target](web-target.md)
- Cli: own crate without Clarity.UI (`rust/cli.rs`); see [cli-target](cli-target.md)
- Custom parts: a crate of the project implementing the trait Clarity writes into its `src/clarity.rs` (`rust/parts.rs:1-6`); see [custom-parts](custom-parts.md)
- While working on the runtime, the project's `.cargo/config.toml` patches the crate with the repository's (`rust/mod.rs:6-8`)
- Docs: manual/stacks.rust.md

## Tests
- Runtime tests: `runtimes/rust/tests/` (50 files: undo.rs, versions.rs, signin.rs, queries.rs, ...)
- Conformance run by R: every runtime file except `migrations` (no runtime test reads it) and `formats` (display formats; CONFLICTING 3); `picture-functions` only in `runtimes/rust-image`
- Generator: `crates/clarity-gen/tests/rust.rs`, rust_clarity.rs, rust_clarity_parts.rs, rust_ui_library.rs, rust_window_model.rs, rust_local_crates.rs, versions_rust.rs, signin_rust.rs
- Builds (ignored, need cargo): build.rs, rust_clarity_build.rs, examples_build.rs
- Examples on this stack: dashboard, modelling, network, shop (all `rust desktop`); no example uses rust web

## Open
- `list<Entity>` fields: NOT BUILT. "list fields like ... are not loaded yet" (`rust/logic.rs:903`); see [entities](entities.md)
- TLS to PostgreSQL and MySQL: PARTIAL. No TLS (`runtimes/rust/src/postgres.rs:144`, `Cargo.toml:57`); SQL Server uses rustls; see [databases](databases.md)
- Integer overflow in `+ - *`: panics in debug, wraps in release; no shared rule (CONFLICTING 2)

## Conflicts
- Decimal division keeps up to 28 digits (`rust/logic.rs:1488`); Go, C, C++ keep 16 (CONFLICTING 2)
- Dividing a decimal by zero panics; C and C++ give a value error (`runtimes/c/clarity/decimal.c:114`)
- `runtimes/rust/README.md:26` shows `clarity-runtime = "0.10.0"`; the crate is 0.56.0
- Runtime changed by the sign-in work without a version bump (`runtimes/rust/Cargo.toml` 0.56.0; CONFLICTING 5)
- `crates/clarity-gen/tests/build.rs:3` says it compiles Tauri; there is no Tauri in generated programs
- DESIGN 1.4 says the runtime is copied in and the shell is Tauri; the crate comes from crates.io (`rust/mod.rs:6-8`)

## Decisions
- 2026-11-02: Tauri desktop dropped; a desktop is Clarity.UI drawing natively. User: "to me desktop is nothing web, get it?"
- 2026-11-08: Rust desktops link Clarity.UI's ready-made library; since v0.30.0 every one does and the `ui` line is ignored (file.rs:366)
- Crates publish from tags `runtimes/rust/v*` through GitHub Actions; see [release-ci](release-ci.md)

## Was
- DESIGN 1.4, DESIGN 7.1 (Rust row), G1a, G1b
- DESIGN 5.39 is [clarity-ui](clarity-ui.md) and [build-tools](build-tools.md)
