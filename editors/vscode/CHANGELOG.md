# Changelog

## 0.1.9

- Colors external libraries: `library` with `cmake` and its `function` lines.
- Colors the `sound` part.

## 0.1.8

- Colors `crate` and `go`, the library lines of custom parts written in Rust and Go.

## 0.1.7

- Colors the words of custom parts: `part` with `native`, `nuget`, `dll`, `binds`, `tells`, `hears` and `preview`. A canvas no longer has `renderer`.

## 0.1.6

- Colors the words of command-line programs: `cli`, `command`, `option`, `print` and `exit`.

## 0.1.5

- Colors `search`, the field modifier for full-text search, and `matches`, which finds the records holding every word typed.

## 0.1.4

- Colors the words of the newer parts and settings: `accordion`, `chart`, `slider`, `spinbox`, `toggle`, `tree`, `treetable`, `tags`, `popover`, `settings` and others, and `color` as a field type.

## 0.1.3

- Colors the `markdown` part, and `resource` in a language file: a key that points at a file of the resources, `gameRules resource "rules-nl.md"`.

## 0.1.2

- The README says to search for **Clarity Designer**, the extension's own name, in the Extensions view.

## 0.1.1

- The first version published by the release pipeline (`editors/vscode/v*` tags). The extension itself is as in 0.1.0.

## 0.1.0

- Colors for `.clarity` files: words of the language, types, names, texts with their `{slots}`, `@keys`, numbers and comments; in a language file, each key beside its text. The grammar is written from the highlighter of the Clarity designer, so both color alike.
- `//` comments a line; brackets and quotes close themselves; a block line indents the next.
- Snippets for a window, an entity, an enum, a handler, a function, an engine written in Clarity and a language file.
- A file icon for `.clarity` files.
