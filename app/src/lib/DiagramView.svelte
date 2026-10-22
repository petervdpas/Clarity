<script lang="ts">
  // A diagram of the diagrams folder, opened from the project tree (DESIGN
  // 5.13). An ERD or class diagram opens in the designer: a palette, a
  // canvas of Clarity's own and the properties of what is picked; every
  // change is written to its .mermaid file at once, as plain Mermaid. Other
  // kinds are drawn by Mermaid. The diagram and the project meet both
  // ways: what the diagram would add or change is marked and Apply makes
  // the ticked proposals, and From project writes the diagram again from
  // the project, keeping where its boxes stand. Each is one undo step.

  import { dialogs } from "./dialogStore.svelte";
  import { invoke } from "./backend";
  import ContextMenu from "./ContextMenu.svelte";
  import ClassProperties from "./ClassProperties.svelte";
  import DesignCanvas from "./DesignCanvas.svelte";
  import DiagramCanvas from "./DiagramCanvas.svelte";
  import ErdProperties from "./ErdProperties.svelte";
  import { CLASS_PALETTE, classEdit, classLinks, classShapes } from "./classDesign";
  import type { Box, CanvasEdit, CanvasPick } from "./designCanvas";
  import { ERD_PALETTE, erdEdit, erdLinks, erdShapes } from "./erdDesign";
  import RenameField from "./RenameField.svelte";
  import SourceEditor from "./SourceEditor.svelte";
  import { DIAGRAM_EXTENSION, marksOf } from "./diagramCanvas";
  import type { ClassDesign } from "./generated/ClassDesign";
  import type { ClassRead } from "./generated/ClassRead";
  import type { Diagnostic } from "./generated/Diagnostic";
  import type { Erd } from "./generated/Erd";
  import type { ErdRead } from "./generated/ErdRead";
  import type { Preview } from "./generated/Preview";
  import type { Proposal } from "./generated/Proposal";

  let {
    preview,
    name,
    onapplied,
    onopen,
    onrenamed = () => {},
    renaming = false,
    onrenaming = () => {},
  }: {
    preview: Preview;
    /** The diagram's file name. */
    name: string;
    onapplied: (preview: Preview) => void;
    /** Opens the entity or enum of `name`. */
    onopen: (name: string) => void;
    /** The diagram's file was renamed to `file`. */
    onrenamed?: (file: string, preview: Preview) => void;
    /** Rename was asked for from outside, as from the tree's menu: the name is readied, then `onrenaming` says so. */
    renaming?: boolean;
    onrenaming?: () => void;
  } = $props();

  /** Clarity's field types, offered when a field's type is typed. */
  const CLARITY_TYPES = ["text", "text(80)", "integer", "decimal(10, 2)", "money", "bool", "date", "datetime", "duration", "image", "email", "url"];

  let tab = $state<"design" | "text">("design");
  let error = $state<string | null>(null);

  /** The project's entities and enums: the boxes a click opens. */
  const known = $derived(
    preview.project.files.flatMap((f) => [...f.entities.map((e) => e.name.name), ...f.enums.map((e) => e.name.name)]),
  );
  const enums = $derived(preview.project.files.flatMap((f) => f.enums.map((e) => e.name.name)));

  // The diagram: its saved text, and the text as it is typed. The draft
  // follows the file whenever the file changes (an Undo, the agent, From
  // project), and keeps what is typed in between.
  const file = $derived(preview.project.diagrams.find((d) => d.name === name));
  let draft = $state("");
  let draftFrom = $state<string | null>(null);
  $effect.pre(() => {
    const saved = `${name}\u0000${file?.text ?? ""}`;
    if (saved !== draftFrom) {
      draftFrom = saved;
      draft = file?.text ?? "";
    }
  });

  const shown = $derived(draft);
  /** Which of Clarity's own designers draws it: ERDs, class diagrams; none for other kinds. */
  const designer = $derived(file?.kind === "erd" ? "erd" : file?.kind === "class" ? "class" : null);

  // What the canvas shows, read from the text as it stands.
  let erd = $state<Erd>({ entities: [], relations: [] });
  let classes = $state<ClassDesign>({ boxes: [], relations: [] });
  let picked = $state<CanvasPick>(null);
  $effect(() => {
    const text = shown;
    if (designer === "erd") {
      invoke<ErdRead>("read_erd", { text })
        .then((read) => (erd = read.erd))
        .catch((err) => (error = String(err)));
    } else if (designer === "class") {
      invoke<ClassRead>("read_classes", { text })
        .then((read) => (classes = read.design))
        .catch((err) => (error = String(err)));
    }
  });

  const shapes = $derived(designer === "class" ? classShapes(classes) : erdShapes(erd));
  const links = $derived(designer === "class" ? classLinks(classes) : erdLinks(erd));
  const entityNames = $derived(designer === "class" ? classes.boxes.filter((b) => b.kind === "entity").map((b) => b.name) : erd.entities.map((e) => e.name));
  const drawnEnums = $derived(designer === "class" ? classes.boxes.filter((b) => b.kind === "enum").map((b) => b.name) : []);

  const types = $derived([
    ...CLARITY_TYPES,
    ...new Set([...enums, ...drawnEnums]),
    ...entityNames.flatMap((e) => [`ref ${e}`, `list<${e}>`]),
  ]);

  /** An edit on the canvas, made in the diagram's model; gives what to pick after it. */
  function edit(change: CanvasEdit, laid: Box[]): CanvasPick | undefined {
    if (designer === "class") {
      const { next, pick } = classEdit(classes, change, laid);
      void changeClasses(next);
      return pick;
    }
    const { next, pick } = erdEdit(erd, change, laid);
    void changeErd(next);
    return pick;
  }

  /** The menu item that opens a box. */
  function openLabel(box: string): string {
    return enums.includes(box) ? "Open enum" : "Open entity";
  }

  /** A change of the ERD: shown at once, then written to its file as Mermaid, with Undo. */
  async function changeErd(next: Erd) {
    erd = next;
    try {
      error = null;
      const text = await invoke<string>("write_erd", { erd: next });
      draft = text;
      onapplied(await invoke<Preview>("save_diagram", { name, text }));
    } catch (err) {
      error = String(err);
    }
  }

  /** A change of the class diagram: shown at once, then written to its file as Mermaid, with Undo. */
  async function changeClasses(next: ClassDesign) {
    classes = next;
    try {
      error = null;
      const text = await invoke<string>("write_classes", { design: next });
      draft = text;
      onapplied(await invoke<Preview>("save_diagram", { name, text }));
    } catch (err) {
      error = String(err);
    }
  }

  // What the written diagram proposes, asked again after typing rests or the project changes.
  let proposals = $state<Proposal[]>([]);
  let ticked = $state<boolean[]>([]);
  $effect(() => {
    const text = draft;
    void preview;
    const timer = setTimeout(async () => {
      try {
        const found = await invoke<Proposal[]>("diagram_proposals", { text });
        proposals = found;
        ticked = found.map((p) => p.ticked);
      } catch (err) {
        error = String(err);
      }
    }, 300);
    return () => clearTimeout(timer);
  });
  const marks = $derived(marksOf(proposals));
  const tickedCount = $derived(ticked.filter(Boolean).length);
  let applying = $state(false);

  async function apply() {
    applying = true;
    try {
      error = null;
      const indexes = ticked.flatMap((t, i) => (t ? [i] : []));
      onapplied(await invoke<Preview>("apply_diagram", { text: draft, ticked: indexes }));
    } catch (err) {
      error = String(err);
    } finally {
      applying = false;
    }
  }

  /** The From project menu, under its button. */
  let fromMenu = $state<{ x: number; y: number } | null>(null);

  /** Writes the diagram again from the project, after asking, keeping where its boxes stand. */
  async function generate(onlyDrawn: boolean) {
    const what = onlyDrawn ? "the boxes it draws" : designer === "class" ? "everything in the project" : "all entities";
    const question = `Write ${name} again from the project, with ${what}? Where the boxes stand is kept; Undo takes it back.`;
    if (!(await dialogs.confirm({ text: question, action: "Write again" }))) return;
    try {
      error = null;
      onapplied(await invoke<Preview>("generate_diagram", { name, onlyDrawn }));
    } catch (err) {
      error = String(err);
    }
  }

  let renameField: RenameField | undefined = $state();

  // Rename asked for from the tree's menu: the name is ready to type over once the diagram is shown.
  $effect(() => {
    if (!renaming || !renameField) return;
    renameField.focus();
    onrenaming();
  });

  /** Renames the diagram's file; `.mermaid` is added when it is not typed. */
  async function rename(to: string) {
    try {
      error = null;
      const next = await invoke<Preview>("rename_diagram", { from: name, to });
      onrenamed(to.endsWith(`.${DIAGRAM_EXTENSION}`) ? to : `${to}.${DIAGRAM_EXTENSION}`, next);
    } catch (err) {
      error = String(err);
    }
  }

  function open(box: string) {
    if (known.includes(box)) onopen(box);
  }

  const KIND_NAMES: Record<string, string> = {
    erd: "ERD",
    class: "Class diagram",
    state: "State diagram",
    sequence: "Sequence diagram",
    activity: "Activity diagram",
    architecture: "Architecture",
  };
  const note = $derived(file?.kind ? `${KIND_NAMES[file.kind] ?? file.kind} in diagrams/` : "Drawn only: Clarity maps ERDs and class diagrams");
</script>

<section class="diagram">
  <header>
    <RenameField bind:this={renameField} {name} title="Rename the diagram's file" onrename={rename} />
    <span class="note">{note}</span>
    {#if designer}
      <button
        onclick={(e) => {
          const r = e.currentTarget.getBoundingClientRect();
          fromMenu = { x: r.left, y: r.bottom + 2 };
        }}
        title="Write the diagram again from the project, keeping where its boxes stand">From project</button
      >
    {/if}
    <button onclick={() => renameField?.focus()} title="Rename the diagram's file">Rename</button>
    <div class="tabs" role="tablist">
      <button role="tab" aria-selected={tab === "design"} class:active={tab === "design"} onclick={() => (tab = "design")}>Design</button>
      <button role="tab" aria-selected={tab === "text"} class:active={tab === "text"} onclick={() => (tab = "text")}>Text</button>
    </div>
  </header>
  {#if error}<p class="error" role="alert">{error}</p>{/if}

  <div class="body" hidden={tab !== "design"}>
    {#if designer}
      <DesignCanvas
        {shapes}
        {links}
        palette={designer === "class" ? CLASS_PALETTE : ERD_PALETTE}
        {marks}
        bind:picked
        onedit={edit}
        onopen={open}
        canopen={(n) => known.includes(n)}
        {openLabel}
      />
    {:else}
      <DiagramCanvas text={shown} names={known} {marks} onopen={open} />
    {/if}
    <div class="side">
      {#if designer === "erd"}
        <ErdProperties {erd} bind:picked {types} onchange={changeErd} />
      {:else if designer === "class"}
        <ClassProperties design={classes} bind:picked {types} onchange={changeClasses} />
      {/if}
      <section class="proposals">
        <h3>Proposals <span class="count">{proposals.length}</span></h3>
        {#if proposals.length === 0}
          <p class="empty">{file?.kind === "erd" || file?.kind === "class" ? "The diagram matches the project." : "Nothing to propose."}</p>
        {:else}
          <ul>
            {#each proposals as p, i (p.subject + p.change)}
              <li class={p.change}>
                <label>
                  <input type="checkbox" bind:checked={ticked[i]} />
                  <span class="change">{p.change}</span>
                  <span class="what"><code>{p.subject}</code> {p.description}</span>
                </label>
              </li>
            {/each}
          </ul>
          <button class="primary" disabled={tickedCount === 0 || applying} onclick={apply} title="Make the ticked changes in the project; one Undo takes them back">
            {applying ? "Applying..." : `Apply ${tickedCount}`}
          </button>
        {/if}
      </section>
    </div>
  </div>

  <div class="body" hidden={tab !== "text"}>
    {#key file?.text}
      <SourceEditor
        load={async () => ({ text: file?.text ?? "", problems: await invoke<Diagnostic[]>("check_diagram", { text: file?.text ?? "" }) })}
        check={(text) => invoke<Diagnostic[]>("check_diagram", { text })}
        save={async (text) => onapplied(await invoke<Preview>("save_diagram", { name, text }))}
        reloadKey={`${name}:${file?.text ?? ""}`}
        language="plain"
        errorsBlockSave={false}
        ontext={(text) => (draft = text)}
      />
    {/key}
  </div>
</section>

{#if fromMenu}
  <ContextMenu
    x={fromMenu.x}
    y={fromMenu.y}
    items={[
      { label: designer === "class" ? "Everything in the project" : "All entities", run: () => generate(false) },
      { label: "Only the boxes drawn", run: () => generate(true) },
    ]}
    onclose={() => (fromMenu = null)}
  />
{/if}

<style>
  .diagram {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 0;
    min-width: 0;
  }

  header {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 12px;
    border-bottom: 1px solid var(--border);
    background: var(--panel);
  }

  .note {
    flex: 1;
    font-size: 12px;
    color: var(--text-muted);
  }

  .tabs {
    display: flex;
    gap: 2px;
  }

  .tabs button.active {
    background: var(--accent-soft);
    border-color: var(--accent);
  }

  .body {
    flex: 1;
    display: flex;
    min-height: 0;
  }

  .body[hidden] {
    display: none;
  }

  .side {
    width: 320px;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    border-left: 1px solid var(--border);
    background: var(--panel);
    overflow: auto;
  }

  .proposals {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 10px 12px;
    border-top: 1px solid var(--border);
  }

  h3 {
    margin: 0;
    font-size: 13px;
  }

  .count {
    color: var(--text-muted);
    font-weight: normal;
  }

  ul {
    margin: 0;
    padding: 0;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  li label {
    display: grid;
    grid-template-columns: auto auto 1fr;
    gap: 6px;
    align-items: baseline;
    font-size: 12px;
  }

  .change {
    font-family: var(--mono);
    font-size: 11px;
  }

  li.add .change {
    color: #2f8f4e;
  }

  li.change .change {
    color: #c08a12;
  }

  li.remove .change {
    color: var(--text-muted);
  }

  .what {
    min-width: 0;
    overflow-wrap: anywhere;
  }

  .empty {
    margin: 0;
    font-size: 12px;
    color: var(--text-muted);
  }

  .error {
    margin: 0;
    padding: 6px 12px;
    color: var(--error);
    font-size: 12px;
  }
</style>
