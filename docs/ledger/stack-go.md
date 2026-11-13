# Go stack

Status: built. `stack go ...`: a Go module on `github.com/petervdpas/Clarity/runtimes/go`, drawn by Clarity.UI through cgo.

Progress: 90%

## Code
- Generators: `crates/clarity-gen/src/go/` (logic.rs 2268 lines, mod.rs 1386, parts.rs 478, model.rs 362, cli.rs 264, names.rs 167); `go/runtime.sum` (prefilled go.sum), `go/modules.txt` (the requirements each runtime package group needs, recorded by a go_build test)
- Runtimes: Go `runtimes/go` (module v0.62.0, `runtimes/go/clarity/version.go:7`, tagged; go 1.25.0; modernc.org/sqlite 1.54, wazero 1.12 in `go.mod`). Packages:
  - `clarity`: records, Db, Query/Cond, dialect, edits and undo (undo.go), backup, signin, settings, files, picture, record_file, file_source, formats, http, zip, cli, canvas, data_folder, list_fields, part rules (tables, trees, ranges, steps, charts, dropdown_options, reorder); agents.go, agents_host.go, agents_library.go: Clarity.AppMcp through cgo, `<program> mcp` (see [app-mcp](app-mcp.md))
  - `shell`: answers the screen's commands, serves the web target (program.go, serve.go)
  - `ui`: the desktop on Clarity.UI through cgo (native.go, callbacks.c, custom.go, desktop.go, exports.go, files.go, program.go)
  - `engine`: wazero host, screen engines, progress
  - `secrets` (go-keyring), `postgresql`, `mysql`, `sqlserver`
  - `internal/authlib`: Clarity.Auth through cgo, tag `clarityauth`; `clarity/formats_library.go`: Clarity.Formats through cgo, tag `clarityformats`
- Desktop program: `main.go` runs the window structs (handlers as methods) through package `ui`; Clarity.UI's library found where CLARITY_UI names (`go/mod.rs:1-12`)
- Web: browser files, `--serve` with both targets (`go/mod.rs:975-977`); see [web-target](web-target.md)
- While working on the runtime, a go.work beside the module builds on the repository's runtime (`go/mod.rs:10-12`)
- Docs: manual/stacks.go.md

## Tests
- Runtime tests: `runtimes/go/clarity/*_test.go`, `shell/*_test.go` (agents_test.go among them), `ui/ui_test.go`, `engine/*_test.go`, `secrets/secrets_test.go`
- Conformance run by Go: every runtime file except `shortcut-keys`, `secret-store` (Go writes the keychain itself), `formats` (display formats), `picture-functions` (CONFLICTING 3); `app-mcp` is read by `clarity/agents_test.go`
- Generator: `crates/clarity-gen/tests/go.rs`, versions_go.rs, signin_go.rs; build: go_build.rs (ignored)
- Examples on this stack: calculator, dial, pills, records, videoplayer, waves (`go desktop`), stock (`go desktop cli`), invaders (`go web`)

## Open
- Go's file-source tests: SKIPPED in CI. They carry tag `clarityformats` but CI runs `-run Formats` (`.github/workflows/checks.yml:207`, `runtimes/go/clarity/file_source_test.go`)
- `files f in store where ...` with a condition that can fail: PARTIAL, refused on Go only (`crates/clarity-gen/src/go/logic.rs:701`)
- Secrets on macOS: PARTIAL. go-keyring stores `go-keyring-base64:` values no other runtime reads (`runtimes/go/secrets/secrets.go:11-40`); see [settings-secrets](settings-secrets.md)
- Picture functions in a web-only or cli Go program: refused (`pictures_without_library`, `crates/clarity-gen/src/lib.rs:993`); see [picture-functions](picture-functions.md)
- `list<Entity>` fields: NOT BUILT (`go/logic.rs:1546`)

## Conflicts
- Decimals keep 16 places (shopspring default); R and C# keep 28 (CONFLICTING 2). Decimal range unlimited; C/C++ about 38 digits, R/C# about 28
- Dividing a decimal by zero panics; C and C++ give a value error
- Integer overflow in `+ - *` wraps; C and C++ are undefined

## Decisions
- 2026-11-03: Wails desktop dropped; Go draws with the one Clarity.UI library through cgo; see [clarity-ui](clarity-ui.md)
- Release: the user tags `runtimes/go/vX.Y.Z`; the Go proxy reads the tag, no workflow; sums into `go/runtime.sum` after the tag check, see [release-ci](release-ci.md)

## Was
- G3a, G3b, G3c (formats and translations; themes part is [themes](themes.md)), DESIGN 7.1 (Go row)
