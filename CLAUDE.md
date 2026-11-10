# Clarity: rules for Claude Code

## Generated-file guards

Clarity checks generated files against their source in tests that fail when a file is behind. A release's checks fail on any of them, so:

- After changing what a generated file is written from (a part kind, keyword, field, format, model type, diagram or theme), write the file again and run its test.
- Before writing a release plan (`releases/*.json`), write and test every guard.

The guards, each written with its environment variable and then run without it:

| Writes | Command |
| --- | --- |
| `editors/vscode/syntaxes/clarity.tmLanguage.json` | `CLARITY_WRITE_GRAMMAR=1 cargo test -p clarity-spec --test highlight` |
| `runtimes/rust-ui/catalog/*.json` | `CLARITY_WRITE_DEFAULTS=1 cargo test -p clarity-gen --test web_windows` |
| `runtimes/web/src/generated/*.ts` | `cargo test -p clarity-spec --lib` (ts-rs writes them) |
| the examples' diagrams | `CLARITY_WRITE_DIAGRAMS=1 cargo test -p clarity-spec --test diagram_views` |
| the servers' files | `CLARITY_WRITE_DEFAULTS=1 cargo test -p clarity-gen --test servers` |
| Clarity.UI's theme | `cargo test -p clarity-spec --test clarity_ui_theme` |
| the picture cases | `CLARITY_WRITE_PICTURES=1 cargo test -p clarity-image --test pictures` |

A guard added later is found with `grep -rhn "CLARITY_WRITE_[A-Z_]*" crates runtimes --include=*.rs -o | sort -u`.

When a check fails, run only what failed, not whole suites.
