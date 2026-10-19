<script lang="ts">
  // One module of the project: its file, what it declares, which other
  // modules it may use, and its functions and actions, written in Clarity.

  import { invoke } from "@tauri-apps/api/core";
  import SourceEditor from "./SourceEditor.svelte";
  import { projectEngines, signature } from "./engines";
  import type { CodeView } from "./generated/CodeView";
  import type { DeclTrial } from "./generated/DeclTrial";
  import type { Edit } from "./generated/Edit";
  import type { DataEvent } from "./generated/DataEvent";
  import type { Preview } from "./generated/Preview";
  import type { SpecFile } from "./generated/SpecFile";

  let {
    preview,
    file,
    onedit,
    onapply,
    onopen,
  }: {
    preview: Preview;
    file: SpecFile;
    onedit: (edit: Edit) => void;
    /** Applies an edit and throws with the reason when it is refused. */
    onapply: (edit: Edit) => Promise<void>;
    /** Opens a declaration by kind and name. */
    onopen: (kind: "window" | "app" | "entity" | "enum" | "theme", name: string) => void;
  } = $props();

  /** The engines this module declares, with the errors in their declarations. */
  const engines = $derived(projectEngines(preview.project).filter((e) => e.module === (file.module?.name ?? "")));

  /** The function or action open in the editor, by name. */
  let openFunction = $state<string | null>(null);
  const fn = $derived(file.functions.find((f) => f.name.name === openFunction) ?? null);

  const STARTERS = {
    function: (name: string) => `function ${name}(value: integer) -> integer\n    return value\n`,
    action: (name: string) => `action ${name}()\n    effects db.write\n`,
  } as const;

  /** Adds a function or action with a free name and opens it. */
  async function addFunction(kind: keyof typeof STARTERS) {
    const taken = new Set(file.functions.map((f) => f.name.name));
    let name = "new_" + kind;
    for (let n = 2; taken.has(name); n++) name = "new_" + kind + n;
    await onapply({ op: "appendDecl", text: STARTERS[kind](name) });
    openFunction = name;
  }

  /** The entity whose data events are open in the editor. */
  let openEvents = $state<string | null>(null);
  const eventsBlock = $derived(file.code.find((b) => b.name.name === openEvents) ?? null);

  const EVENT_TEXT: Record<DataEvent, string> = {
    beforeSave: "before save",
    afterSave: "after save",
    beforeDelete: "before delete",
    afterDelete: "after delete",
  };

  function eventsOf(entity: string): string {
    const block = file.code.find((b) => b.name.name === entity);
    return block ? block.events.map((e) => EVENT_TEXT[e.event]).join(", ") : "";
  }

  /** Writes the entity's code block with a first event and opens it. */
  async function addEvents(entity: string) {
    const text = `code ${entity}\n    before save\n        // Runs before a record is stored; refuse "text" stops the save.\n        return\n`;
    await onapply({ op: "appendDecl", text });
    openEvents = entity;
  }

  /** Saves the text and follows a rename. */
  async function saveFunction(item: number, text: string) {
    await onapply({ op: "replaceDecl", item, text });
    const renamed = /^(?:function|action)\s+([A-Za-z_]\w*)/.exec(text)?.[1];
    if (renamed) openFunction = renamed;
  }

  const uses = $derived(file.uses.map((u) => u.name));
  const others = $derived(
    preview.project.files.map((f) => f.module?.name).filter((n): n is string => !!n && n !== file.module?.name),
  );

  function setUse(name: string, on: boolean) {
    const next = on ? [...uses, name] : uses.filter((u) => u !== name);
    // Keep the order of the project's modules.
    onedit({ op: "setUses", uses: others.filter((o) => next.includes(o)) });
  }
</script>

<div class="view">
  <header>
    <h2>{file.module?.name ?? "(no module)"}</h2>
    <span class="muted">module, file <code>{file.path}</code></span>
  </header>

  <section>
    <h3>Uses</h3>
    <p class="muted">A module may only refer to what the modules it uses declare.</p>
    {#each others as name (name)}
      <label class="check">
        <input type="checkbox" checked={uses.includes(name)} onchange={(e) => setUse(name, e.currentTarget.checked)} />
        {name}
      </label>
    {:else}
      <p class="muted">There are no other modules.</p>
    {/each}
  </section>

  <section>
    <h3>Declares</h3>
    <ul>
      {#each file.apps as a (a.name.name)}
        <li><button onclick={() => onopen("app", a.name.name)}><span class="kind">app</span>{a.name.name}</button></li>
      {/each}
      {#each file.pages as p (p.name.name)}
        <li><button onclick={() => onopen("window", p.name.name)}><span class="kind">{p.kind}</span>{p.name.name}</button></li>
      {/each}
      {#each file.entities as e (e.name.name)}
        <li><button onclick={() => onopen("entity", e.name.name)}><span class="kind">entity</span>{e.name.name}</button></li>
      {/each}
      {#each file.enums as e (e.name.name)}
        <li><button onclick={() => onopen("enum", e.name.name)}><span class="kind">enum</span>{e.name.name}</button></li>
      {/each}
    </ul>
  </section>

  <section>
    <h3>Functions and actions</h3>
    <p class="muted">
      A function computes a value and has no effects. An action may change data, and declares it with <code>effects</code>.
      Handlers and other actions call them by name.
    </p>
    <ul>
      {#each file.functions as f (f.name.name)}
        <li>
          <button class:active={openFunction === f.name.name} onclick={() => (openFunction = f.name.name)}>
            <span class="kind">{f.kind}</span>{f.name.name}
          </button>
        </li>
      {/each}
    </ul>
    <div class="buttons">
      <button onclick={() => addFunction("function")}>New function</button>
      <button onclick={() => addFunction("action")}>New action</button>
    </div>
    {#if fn}
      {@const item = fn.span.start}
      <div class="function-editor">
        <SourceEditor
          load={() => invoke<CodeView>("decl_text", { path: file.path, item })}
          check={async (text) => (await invoke<DeclTrial>("try_decl", { path: file.path, item, text })).problems}
          save={(text) => saveFunction(item, text)}
          reloadKey={file.path + "\n" + item + "\n" + file.version}
          onclose={() => (openFunction = null)}
        />
      </div>
    {/if}
  </section>

  {#if engines.length > 0}
    <section>
      <h3>Engines</h3>
      <p class="muted">
        Processing compiled to WebAssembly, its file in the <code>engines</code> folder. Code calls a function as
        <code>Engine.function(...)</code>; the engine only computes, and code hands it the records it needs.
      </p>
      {#each engines as { engine, problems } (engine.name.name)}
        <div class="engine">
          <div><span class="kind">engine</span><strong>{engine.name.name}</strong> <span class="muted">from <code>{engine.file}</code></span></div>
          {#each problems as problem (problem)}
            <p class="problem">{problem}</p>
          {/each}
          <ul>
            {#each engine.functions as f (f.name.name)}
              <li class="signature"><code>{signature(f)}</code></li>
            {/each}
          </ul>
        </div>
      {/each}
    </section>
  {/if}

  <section>
    <h3>Themes</h3>
    <p class="muted">
      How the application looks. Each theme opens in its own view, also from Themes in the project tree, where New theme starts one.
    </p>
    <ul>
      {#each file.themes as t (t.name.name)}
        <li>
          <button onclick={() => onopen("theme", t.name.name)}>
            <span class="kind">theme</span>{t.name.name}{#if preview.project.themes.includes(t.name.name)}<span class="muted">{preview.project.themes[0] === t.name.name ? "offered, starts with it" : "offered"}</span>{/if}
          </button>
        </li>
      {:else}
        <li class="muted">This module declares no themes.</li>
      {/each}
    </ul>
  </section>

  {#if file.entities.length > 0}
    <section>
      <h3>Data events</h3>
      <p class="muted">
        Code that runs whenever a record is saved or deleted, from any window or action: <code>before save</code>,
        <code>after save</code>, <code>before delete</code> and <code>after delete</code>. <code>refuse "text"</code> stops the save or
        delete, and nothing of it is kept.
      </p>
      <ul>
        {#each file.entities as e (e.name.name)}
          {@const events = eventsOf(e.name.name)}
          <li>
            {#if events}
              <button class:active={openEvents === e.name.name} onclick={() => (openEvents = e.name.name)}>
                <span class="kind">events</span>{e.name.name}<span class="muted">{events}</span>
              </button>
            {:else}
              <button onclick={() => addEvents(e.name.name)}>
                <span class="kind">entity</span>{e.name.name}<span class="muted">Add events</span>
              </button>
            {/if}
          </li>
        {/each}
      </ul>
      {#if eventsBlock}
        {@const item = eventsBlock.span.start}
        <div class="function-editor">
          <SourceEditor
            load={() => invoke<CodeView>("decl_text", { path: file.path, item })}
            check={async (text) => (await invoke<DeclTrial>("try_decl", { path: file.path, item, text })).problems}
            save={(text) => onapply({ op: "replaceDecl", item, text })}
            reloadKey={file.path + "\n" + item + "\n" + file.version}
            onclose={() => (openEvents = null)}
          />
        </div>
      {/if}
    </section>
  {/if}
</div>

<style>
  .view {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  h2 {
    margin: 0 0 2px;
    font-size: 18px;
  }

  h3 {
    margin: 0 0 6px;
    font-size: 12px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-muted);
  }

  section {
    background: var(--panel);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 12px 14px;
  }

  .muted {
    color: var(--text-muted);
    font-size: 12px;
    margin: 0 0 6px;
  }

  code {
    font-family: var(--mono);
  }

  .check {
    display: flex;
    gap: 8px;
    align-items: center;
    padding: 2px 0;
  }

  ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  li button {
    display: flex;
    gap: 10px;
    align-items: center;
    width: 100%;
    text-align: left;
    border: none;
    background: transparent;
    padding: 4px 6px;
    border-radius: var(--radius);
  }

  li button:hover {
    background: var(--panel-alt);
  }

  .kind {
    min-width: 56px;
    font-size: 10px;
    text-transform: uppercase;
    color: var(--text-muted);
  }

  li button.active {
    background: var(--panel-alt);
    font-weight: 600;
  }

  .engine + .engine {
    margin-top: 10px;
  }

  .signature {
    padding: 2px 6px 2px 72px;
  }

  .problem {
    color: var(--error);
    font-size: 12px;
    margin: 4px 0;
  }

  .buttons {
    display: flex;
    gap: 8px;
    margin: 8px 0;
  }

  .function-editor {
    display: flex;
    height: 380px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    overflow: hidden;
  }
</style>
