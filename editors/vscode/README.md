# Clarity Designer

Colors, snippets and a file icon for Clarity files (`.clarity`), the language of the [Clarity](https://github.com/petervdpas/Clarity) designer: design an application once, and Clarity generates it for Rust, Go, C#, C or C++.

## Features

- **Colors** as the Clarity designer gives them: words of the language, types, names, texts with their `{slots}`, `@keys`, numbers and comments. A word set or called (`title = ...`, `size(3)`) is a name, whatever it is elsewhere. In a language file, each key stands beside its text.
- **Editing**: `//` comments a line out (Ctrl+/), brackets and quotes close themselves, and a block line (`page`, `handler`, `if`, ...) indents the next.
- **Snippets**: `page`, `entity`, `enum`, `code`, `handler`, `function`, `if`, `for`, `engine` (an engine written in Clarity) and `translation` (a language file).
- **A file icon** for `.clarity` files.

```clarity
engine Dice
    uses random
    state per app
    keep rolled: integer = 0

    function roll(sides: integer) -> integer
        rolled = rolled + 1
        return random(sides)
```

The language is described in the [language reference](https://github.com/petervdpas/Clarity/blob/main/docs/LANGUAGE.md).

## Install

From the Marketplace: search for **Clarity** in the Extensions view, or

    code --install-extension petervdpas.clarity-designer

From a package, such as one attached to a release:

    code --install-extension clarity-designer-0.1.0.vsix

## For the repository

The grammar, `syntaxes/clarity.tmLanguage.json`, is written from the highlighter of the Clarity designer (`clarity_spec::highlight`), so both color Clarity alike. It is not edited by hand: a test checks it, and writes it again with

    CLARITY_WRITE_GRAMMAR=1 cargo test -p clarity-spec --test highlight

Another test checks that every snippet is Clarity. To try the extension from this folder, link it (`ln -s "$PWD" ~/.vscode/extensions/clarity-designer`) or package it (`npm run package`) and install the `.vsix`. A tag `editors/vscode/vX.Y.Z` publishes it to the Marketplace and Open VSX (`.github/workflows/release-vscode.yml`), and every release of Clarity carries the `.vsix` beside its installers (`release.yml`).

JetBrains editors read the same grammar through their TextMate Bundles setting: add this folder there.
