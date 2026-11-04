# @clarity/ui

What the IDE's front end (`app/`) shares of Clarity's model: its types, what it reads of a project (pages, frames, themes, formats, translations) and a few parts of its own pages. The designer and the preview no longer draw windows with it: they draw with Clarity.UI's browser build, as every generated program does (DESIGN 8.1, 7.9).

| Folder | What it holds |
|---|---|
| `src/screen/` | What the IDE reads of a project: formats, translations, themes, the window's data, pages and frames, the grid's sizes (`grid.ts`, kept equal to `clarity_spec::layout` by a test). |
| `src/parts/` | Parts of the IDE's own pages: the splitter, the error guards, icons, markdown, secret buttons. |
| `src/frame/` | A sidebar's widths and the page's own menu, as the IDE uses them. |
| `src/host/` | The transport to the IDE (`backend.ts`) and its keys. |
| `src/generated/` | The model's types, written by ts-rs from the Rust crates (`cargo test`). |

The package has no dependencies of its own yet: it uses the IDE's (`app/`), linked by `npm install` in `app/` (`runtimes/web/node_modules`). Its tests run with the IDE's: `npm test` in `app/`.

The IDE imports it as `@clarity/ui/...`.
