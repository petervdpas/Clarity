# Diagrams

Status: partial. ERD and class diagrams designed on Clarity's own canvas in the IDE and saved as `.mermaid` files; other kinds are not designed.

Progress: 50%

## Code
- Spec: `crates/clarity-spec/src/diagrams/` (mod.rs: kinds and their Mermaid headers :45-78; erd.rs, erd_model.rs, class.rs, class_model.rs, fields.rs, compare.rs (differences and proposals), views.rs (written from the project), files.rs). Models are read and written whole (`erd_model.rs:80, 184`)
- IDE: `app/src/lib/DiagramView.svelte` (Design and Text tabs, problems :338), `DiagramCanvas.svelte`, `diagramCanvas.ts`, `erdDesign.ts`, `classDesign.ts`, `ErdProperties.svelte`, `ClassProperties.svelte`, `FieldList.svelte`, `NewDiagramDialog.svelte` (offers ERD and class only, :28); Tauri `app/src-tauri/src/diagrams.rs`; mermaid.js 11.17.2 (`app/package.json:31`) draws the kinds Clarity does not design, read-only
- Places and bend points kept as `%% clarity:` comment lines, so the file stays plain Mermaid
- Agent tools: diagram_view, write_diagram, diagram_proposals, apply_diagram (`crates/clarity-tools/src/lib.rs`); see [mcp-tools](mcp-tools.md)
- Generated apps: none; diagrams are IDE only and not carried into programs
- Docs: manual/data.diagrams.md

## Tests
- `crates/clarity-spec/tests/diagrams.rs`, diagram_views.rs (examples' diagrams; `CLARITY_WRITE_DIAGRAMS=1 cargo test -p clarity-spec --test diagram_views`), diagram_files.rs, diagram_proposals.rs, class_diagrams.rs, class_model.rs, erd_model.rs
- `app/src/lib/diagramCanvas.test.ts`, erdDesign.test.ts, classDesign.test.ts

## Open
- State, sequence, activity and architecture diagrams: PARTIAL. Kinds and headers exist (`diagrams/mod.rs:45-78`); no parser, view or designer
- A diagram's problems in the project's Problems list: PARTIAL. Only in the diagram's own editor (`DiagramView.svelte:338`)
- `diagram_view` gives the class view too: PARTIAL. Data model only (`crates/clarity-tools/src/lib.rs:578-582`)
- `steps` between enum values (DG2, the state diagram's meaning): NOT BUILT; see [entities](entities.md)
- Mermaid diagrams shown inside generated apps: NOT BUILT (DESIGN 5.13 calls it a separate row); see [markdown](markdown.md)
- Graph-based modelling: see [graph](graph.md)

## Conflicts
- DESIGN 5.13 (:913) and DG1 (:2529) say nothing is built and mermaid.js draws the designer; ERD and class are built on Clarity's own canvas
- DESIGN 5.13 (:926) says New diagram copies in a `.mermaid` file; it offers a name, a kind and "from the project or a small example"
- DESIGN 5.13 (:984) names `clarity_spec::diagrams::edit`; it does not exist (models read and written whole)
- manual/data.diagrams.md:173-179: New diagram offers ERD and class only
- The Text tab opens the SourceEditor with `language="plain"` (`DiagramView.svelte:342`); DESIGN 5.13 says `language="mermaid"` with colors

## Decisions
- 2026-10-22: diagrams in Mermaid, the data dictionary the one source of truth; a diagram is a view or a proposal, never a second model (DESIGN 5.13)
- 2026-10-22: a visual designer, not text plus a drawing. User: "you do know that I eventually want a visual designer for diagrams eh? Where I set the diagram type and have elements I can put on a canvas, right?", "and it translates seamless to mermaid"
- 2026-10-22: Clarity's own canvas, the ERD first; places kept in `%% clarity:` lines
- 2026-10-22: `steps` on enums decided as language (DG2); syntax still open
- 2026-10-31: layered layout with Arrange; bend points on lines
- Apply never removes anything unasked: removals start unticked

## Was
- DESIGN 5.13, DG1, DG2, DG3
- memory: diagram-designer.md
