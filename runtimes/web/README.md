# @clarity/ui

The screen of Clarity's IDE: what the designer and the preview draw a project's windows with (DESIGN 7.8). Generated programs do not use it: they draw with Clarity.UI, on the desktop and in the browser (DESIGN 7.9).

| Folder | What it holds |
|---|---|
| `src/kinds/` | One module per kind of part (`Button.svelte`, `List.svelte`, `Splitter.svelte`, ...), each drawn from one interface, `PartContext` (`context.ts`); `index.ts` picks the module for a part. |
| `src/grid/` | The grid of a window or container (`GridBox.svelte`), the place of a part on it (`WidgetView.svelte`) and popovers. |
| `src/parts/` | Clarity's own inputs and views the kinds are built from (the table, the text, number and date inputs, the canvas, charts), with the same contract as `Clarity.Avalonia` (DESIGN 1.6). |
| `src/screen/` | What the screen reads and decides: formats, translations, themes, the window's data, the designer interface (`designer.svelte.ts`). |
| `src/host/` | The transport to the IDE (`backend.ts`), its keys, and the engines beside the screen. |
| `src/generated/` | The model's types, written by ts-rs from the Rust crates (`cargo test`). |

The package has no dependencies of its own yet: it uses the IDE's (`app/`), linked by `npm install` in `app/` (`runtimes/web/node_modules`). Its tests run with the IDE's: `npm test` in `app/`.

The IDE imports it as `@clarity/ui/...`.
