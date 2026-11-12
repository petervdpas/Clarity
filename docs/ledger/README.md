# Development ledger

One small entry per piece of Clarity. Read this index, then only the entries the work touches. The ledger replaces the plan documents, `docs/DESIGN.md` and the one-off status documents; `docs/LANGUAGE.md` (the language reference) and `manual/` (the user manual) stay as they are.

## How to keep it

- Every entry has the same headings, in this order: Status, Code, Tests, Open, Conflicts, Decisions, Was. At most 80 lines (modularity and docs may run longer).
- Status is one of: **built**, **partial**, **not built**, **needs decision**. Open items say NOT BUILT, PARTIAL or UNTRIED (built but never run where it was promised).
- Code wins: an entry says what the code does. When code and entry disagree, fix the entry, or put the difference under Conflicts until it is settled.
- Work on a piece updates its entry in the same piece of work: a built item leaves Open, a settled conflict leaves Conflicts, a new decision goes under Decisions with the date and the user's words verbatim.
- A new piece gets a new entry and a line in this index; a fact lives in one entry, others link to it.
- Was lists the old references an entry replaces ("DESIGN 7.9", "FORMATS-PLAN.md step 2"), so an old code comment can be traced here with `grep -l "DESIGN 7.9" docs/ledger/*.md`.

Plain ASCII, no em-dashes, short lines.

## Index

### Cross-cutting

| Piece | Status | First open item |
|---|---|---|
| [ledger](ledger.md) | partial | old references, the `design` tool, deleting the old documents |
| [vision](vision.md) | partial | what Clarity is for and the rules every piece follows |
| [modularity](modularity.md) | partial | the guide: one module per layer, step by step, with what each step settles |
| [docs](docs.md) | partial | LANGUAGE.md, the manual and READMEs against the code |
| [os-checks](os-checks.md) | partial | what has never run on Windows or macOS |

### Language and core

| Piece | Status | First open item |
|---|---|---|
| [language](language.md) | partial | `while`, `break`, `continue`, `match` |
| [translations](translations.md) | partial | `context` for one text with two meanings |
| [engines](engines.md) | partial | engines in Clarity: dates, text functions, more statements |
| [queries](queries.md) | built | none |
| [project-file](project-file.md) | built | none |
| [custom-parts](custom-parts.md) | built | `preview` picture on the design canvas |

### Data

| Piece | Status | First open item |
|---|---|---|
| [entities](entities.md) | partial | `list<Entity>` on stored entities, `owned`, `pattern` |
| [databases](databases.md) | partial | server databases on C and C++ |
| [migrations](migrations.md) | partial | fill expression when a field becomes required |
| [row-versions](row-versions.md) | partial | the IDE's data browser through the version check |
| [backups](backups.md) | partial | server databases in a portable form |
| [files-pictures](files-pictures.md) | built | none |
| [formats](formats.md) | partial | file sources in a data browser |
| [web-apis](web-apis.md) | partial | REST entities mapped onto endpoints |
| [settings-secrets](settings-secrets.md) | built | Go's secrets on macOS read by other stacks |
| [signin](signin.md) | partial | providers (github, microsoft, openid) in generated programs |
| [encrypted-storage](encrypted-storage.md) | not built | `encrypted "setting:name"` on sources and stores |
| [mail](mail.md) | not built | `smtp` and `imap` sources |
| [background-jobs](background-jobs.md) | not built | `at most N`, `start ... on done` |

### Clarity.UI and parts

| Piece | Status | First open item |
|---|---|---|
| [clarity-ui](clarity-ui.md) | built | grid reflow on narrow windows |
| [windows-dialogs](windows-dialogs.md) | built | the app's icon in the designer |
| [frame-keys](frame-keys.md) | partial | Ctrl as Cmd on macOS |
| [themes](themes.md) | built | none |
| [parts-inputs](parts-inputs.md) | built | dropdown search on labels, accent-blind sorting |
| [parts-containers](parts-containers.md) | built | menu key opens a part's context menu |
| [parts-data](parts-data.md) | built | moving several rows, dragging between parts |
| [charts](charts.md) | built | chart grouping in one place |
| [graph](graph.md) | partial | looks bound to fields; the modelling example |
| [markdown](markdown.md) | built | real italic faces |
| [timer](timer.md) | partial | `once`, `start()`, `stop()`, timers.json |

### Media

| Piece | Status | First open item |
|---|---|---|
| [canvas-toolbox](canvas-toolbox.md) | partial | `surface` read by nothing; layer opacity |
| [picture-functions](picture-functions.md) | partial | Go, C, C++ web-only and cli programs |
| [audio](audio.md) | built | conformance/sound.json |
| [video-camera](video-camera.md) | partial | video fields; camera and recording |

### Generation and stacks

| Piece | Status | First open item |
|---|---|---|
| [generation](generation.md) | partial | `clarity gen --verify` |
| [web-target](web-target.md) | built | untried in Edge and on Windows |
| [cli-target](cli-target.md) | built | picture functions refused on Go, C, C++ |
| [build-tools](build-tools.md) | partial | library check on Windows and macOS |
| [offline](offline.md) | not built | `clarity prepare`, `clarity vendor` |
| [release-ci](release-ci.md) | partial | a Windows job in checks.yml |
| [examples](examples.md) | built | maintenance not yet tried on a real ledger |
| [stack-rust](stack-rust.md) | built | list fields not loaded |
| [stack-go](stack-go.md) | built | file-source tests skipped in CI |
| [stack-c](stack-c.md) | built | server databases |
| [stack-cpp](stack-cpp.md) | built | server databases |
| [stack-csharp](stack-csharp.md) | built | comparisons, skipped conformance files, dead copies |

### IDE

| Piece | Status | First open item |
|---|---|---|
| [ide-shell](ide-shell.md) | partial | translations of the IDE's own words |
| [ide-designer](ide-designer.md) | partial | edits named by key, not by file place |
| [ide-editors](ide-editors.md) | partial | code behind in the person's own editor |
| [ide-data](ide-data.md) | partial | a data browser |
| [diagrams](diagrams.md) | partial | state, sequence, activity, architecture |
| [vault](vault.md) | built | untried keys on Windows |

### AI

| Piece | Status | First open item |
|---|---|---|
| [agent](agent.md) | partial | local models (ollama, openai-compatible) |
| [acp](acp.md) | partial | Codex and Copilot checked by hand |
| [mcp-tools](mcp-tools.md) | partial | the `design` tool reads the ledger; the questionnaire |

### Planned, nothing built

| Piece | Status | First open item |
|---|---|---|
| [hardware](hardware.md) | not built | serial devices with tells and hears |
| [parts-library](parts-library.md) | not built | packages of Clarity parts |
| [reports-pdf](reports-pdf.md) | not built | clarity-pdf on Typst |
| [debugger](debugger.md) | not built | marks per statement in the Rust generator |
