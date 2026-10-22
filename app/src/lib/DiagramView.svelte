<script lang="ts">
  // A diagram, opened from Diagrams in the project tree (DESIGN 5.13). The
  // Data model is written from the data dictionary and always true: a
  // click on a box opens the entity. A written ERD opens in the designer:
  // a palette, a canvas of Clarity's own and the properties of what is
  // picked; every change is written to its .mmd file at once, as plain
  // Mermaid. Other diagrams are drawn by Mermaid. A written diagram is a
  // proposal: what it would add or change is marked, and Apply makes the
  // ticked proposals as one undo step.

  import { invoke } from "./backend";
  import DiagramCanvas from "./DiagramCanvas.svelte";
  import ErdCanvas, { type ErdPick } from "./ErdCanvas.svelte";
  import ErdProperties from "./ErdProperties.svelte";
  import SourceEditor from "./SourceEditor.svelte";
  import { marksOf } from "./diagramCanvas";
  import type { Diagnostic } from "./generated/Diagnostic";
  import type { Erd } from "./generated/Erd";
  import type { ErdRead } from "./generated/ErdRead";
  import type { Preview } from "./generated/Preview";
  import type { Proposal } from "./generated/Proposal";
  import type { View } from "./generated/View";

  let {
    preview,
    kind,
    name,
    onapplied,
    onopen,
  }: {
    preview: Preview;
    /** A view written from the project, or a written diagram of the diagrams folder. */
    kind: "view" | "diagram";
    /** The view's name, or the diagram's file name. */
    name: string;
    onapplied: (preview: Preview) => void;
    /** Opens the entity or enum of `name`. */
    onopen: (name: string) => void;
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

  // The Data model, written again whenever the project changes.
  let view = $state<View | null>(null);
  $effect(() => {
    void preview;
    if (kind !== "view") return;
    invoke<View>("data_model_view")
      .then((v) => (view = v))
      .catch((err) => (error = String(err)));
  });

  // A written diagram: its saved text, and the text as it is typed.
  const file = $derived(kind === "diagram" ? preview.project.diagrams.find((d) => d.name === name) : undefined);
  let draft = $state("");
  let draftOf = $state("");
  $effect.pre(() => {
    if (kind === "diagram" && name !== draftOf) {
      draftOf = name;
      draft = file?.text ?? "";
    }
  });

  const shown = $derived(kind === "view" ? (view?.text ?? "") : draft);
  /** Whether the designer of Clarity's own draws it: the Data model and every ERD. */
  const designed = $derived(kind === "view" || file?.kind === "erd");

  // The ERD the canvas shows, read from the text as it stands.
  let erd = $state<Erd>({ entities: [], relations: [] });
  let picked = $state<ErdPick>(null);
  $effect(() => {
    const text = shown;
    if (!designed) return;
    invoke<ErdRead>("read_erd", { text })
      .then((read) => (erd = read.erd))
      .catch((err) => (error = String(err)));
  });

  const types = $derived([
    ...CLARITY_TYPES,
    ...enums,
    ...erd.entities.flatMap((e) => [`ref ${e.name}`, `list<${e.name}>`]),
  ]);

  /** A change on the canvas: shown at once, then written to the diagram's file as Mermaid, with Undo. */
  async function change(next: Erd) {
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

  // What the written diagram proposes, asked again after typing rests or the project changes.
  let proposals = $state<Proposal[]>([]);
  let ticked = $state<boolean[]>([]);
  $effect(() => {
    const text = draft;
    void preview;
    if (kind !== "diagram") return;
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
  const note = $derived(
    kind === "view"
      ? "Written from the data dictionary; click a box to open it"
      : file?.kind
        ? `${KIND_NAMES[file.kind] ?? file.kind} in diagrams/`
        : "Drawn only: Clarity maps ERDs and class diagrams",
  );
</script>

<section class="diagram">
  <header>
    <h2>{kind === "view" ? "Data model" : name}</h2>
    <span class="note">{note}</span>
    <div class="tabs" role="tablist">
      <button role="tab" aria-selected={tab === "design"} class:active={tab === "design"} onclick={() => (tab = "design")}>Design</button>
      <button role="tab" aria-selected={tab === "text"} class:active={tab === "text"} onclick={() => (tab = "text")}>Text</button>
    </div>
  </header>
  {#if error}<p class="error" role="alert">{error}</p>{/if}

  <div class="body" hidden={tab !== "design"}>
    {#if designed}
      <ErdCanvas {erd} {marks} readonly={kind === "view"} bind:picked onchange={change} onopen={open} />
    {:else}
      <DiagramCanvas text={shown} names={known} {marks} onopen={open} />
    {/if}
    {#if kind === "diagram"}
      <div class="side">
        {#if designed}
          <ErdProperties {erd} bind:picked {types} onchange={change} />
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
    {/if}
  </div>

  <div class="body" hidden={tab !== "text"}>
    {#if kind === "view"}
      <SourceEditor
        load={async () => ({ text: view?.text ?? "", problems: [] })}
        check={async () => []}
        save={async () => {}}
        reloadKey={view?.text ?? ""}
        language="plain"
        readonly
      />
    {:else}
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
    {/if}
  </div>
</section>

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

  h2 {
    margin: 0;
    font-size: 15px;
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
