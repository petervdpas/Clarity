# Markdown

Status: built. The `markdown` part shows Markdown from a resource file (per language through a key, or one `source`) or a bound text; a memo with `markdown` writes it beside a preview.

Progress: 90%

## Code
- Spec: `crates/clarity-spec/src/widgets.rs` (`Markdown` kind; memo property `Markdown` :489; event `Link` in `enum Event` :222 for `clarity:` links), `resources.rs` (Markdown files among the resources, sources and keys checked by kind), `translations.rs` (`key resource "file.md"`, :1002)
- Generators: each resource key becomes a text per language (the Markdown) in the program's files (`crates/clarity-gen/src/web_browser.rs`)
- Runtimes: the block reader R `runtimes/rust/src/markdown.rs` (`read` :281, `spans` :208), used by Clarity.UI for every stack; `runtimes/web/src/screen/markdownBlocks.ts` for the IDE
- Clarity.UI: `runtimes/rust-ui/src/parts/containers/markdown.rs` (a page: headings, quotes, lists, links, a bar that drags and pages; uses `clarity_runtime::markdown`), `parts/markdown_memo.rs` (Write and Preview)
- IDE: `app/src/lib/ResourceView.svelte` (Markdown view of a resource), `ReadmeView.svelte`; `app/src/lib/markdown.ts` is the IDE's own reader for the agent and manual
- Docs: manual `parts.text.md` (`markdown`), `data.pictures-files.md` (Markdown files), `languages.md` (Keys that point at resources)

## Tests
- `conformance/markdown.json`: run by R runtime (`runtimes/rust/tests/markdown.rs`), rust-ui (`tests/parts/parts_markdown.rs`), `runtimes/web` (`markdownBlocks.test.ts`); Go, C, C++, C# have no reader of their own (their programs draw through Clarity.UI)
- spec `markdown.rs`, `markdown_memo.rs`, `resources.rs`; gen `resources.rs`
- rust-ui `tests/parts/parts_markdown.rs`, `parts_markdown_link_cursor.rs`

## Open
- Real italic faces: PARTIAL. Italic is a skewed regular face (`runtimes/rust-ui/src/parts/containers/markdown.rs:475`, `draw_slanted` :492; DESIGN.md:1978).
- Mermaid diagrams shown inside generated apps: NOT BUILT (DESIGN.md:916).

## Conflicts
- Markdown blocks decided twice: `runtimes/rust/src/markdown.rs` and `runtimes/web/src/screen/markdownBlocks.ts`; equal on the cases.

## Decisions
- 2026-10-22: long texts are Markdown files among the resources; a key's value in a language may point at one (`gameRules resource "rules-nl.md"`), never a long text in a language file.
- 2026-10-22: one Markdown subset on every stack; links open in the browser only for http, https and mailto.

## Was
- Roadmap: R1 (the markdown part; the resources folder itself is [translations](translations.md))
