<script lang="ts">
  // One module of the project: its file, what it declares, which other
  // modules it may use, and its functions and actions, written in Clarity.

  import { invoke } from "@tauri-apps/api/core";
  import SourceEditor from "./SourceEditor.svelte";
  import { projectEngines } from "./engines";
  import { typeText } from "@clarity/ui/screen/windowData.ts";
  import type { FunctionDecl } from "@clarity/ui/generated/FunctionDecl.ts";
  import type { CodeView } from "@clarity/ui/generated/CodeView.ts";
  import type { DeclTrial } from "@clarity/ui/generated/DeclTrial.ts";
  import type { Edit } from "@clarity/ui/generated/Edit.ts";
  import type { DataEvent } from "@clarity/ui/generated/DataEvent.ts";
  import type { Preview } from "@clarity/ui/generated/Preview.ts";
  import type { SpecFile } from "@clarity/ui/generated/SpecFile.ts";

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
    onopen: (kind: "window" | "app" | "entity" | "enum" | "engine" | "part" | "library" | "theme", name: string) => void;
  } = $props();

  /** The engines this module declares, with the errors in their declarations. */
  const engines = $derived(projectEngines(preview.project).filter((e) => e.module === (file.module?.name ?? "")));

  /** The function or action open in the editor, by name; the first one until another is picked. */
  let openFunction = $state<string | null>(null);
  const fn = $derived(file.functions.find((f) => f.name.name === openFunction) ?? file.functions[0] ?? null);

  /** A function's name with its values and result, as the editor's title says which one it shows. */
  function signature(f: FunctionDecl): string {
    const params = f.params.map((p) => `${p.name.name}: ${typeText(p.ty.kind)}`).join(", ");
    return `${f.name.name}(${params})${f.returns ? ` -> ${typeText(f.returns.kind)}` : ""}`;
  }

  const STARTERS = {
    function: (name: string) => `function ${name}(value: integer) -> integer\n    return value\n`,
    action: (name: string) => `action ${name}()\n    effects db.write\n`,
  } as const;

  /** Adds a function or action with a free name and opens it. */
  async function addFunction(kind: keyof typeof STARTERS) {
    const taken = new Set(file.functions.map((f) => f.name.name));
    // camelCase, as every name in Clarity: newFunction, newAction2.
    const base = kind === "function" ? "newFunction" : "newAction";
    let name = base;
    for (let n = 2; taken.has(name); n++) name = base + n;
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
      {#each file.entities.filter((e) => !e.settings) as e (e.name.name)}
        <li><button onclick={() => onopen("entity", e.name.name)}><span class="kind">entity</span>{e.name.name}</button></li>
      {/each}
      {#each file.enums as e (e.name.name)}
        <li><button onclick={() => onopen("enum", e.name.name)}><span class="kind">enum</span>{e.name.name}</button></li>
      {/each}
      {#each file.libraries as l (l.name.name)}
        <li><button onclick={() => onopen("library", l.name.name)}><span class="kind">library</span>{l.name.name}</button></li>
      {/each}
      {#each file.parts as p (p.name.name)}
        <li><button onclick={() => onopen("part", p.name.name)}><span class="kind">part</span>{p.name.name}</button></li>
      {/each}
    </ul>
  </section>

  <section>
    <h3>Functions and actions</h3>
    <p class="muted">
      A function computes a value and has no effects. An action may change data, and declares it with <code>effects</code>.
      Handlers and other actions call them by name.
    </p>
    <!-- One widget: the functions and actions at the left, the one picked marked, its code at the right under its own name. -->
    <div class="functions">
      <div class="function-list">
        <ul>
          {#each file.functions as f (f.name.name)}
            <li>
              <button class:active={fn?.name.name === f.name.name} aria-current={fn?.name.name === f.name.name ? "true" : undefined} onclick={() => (openFunction = f.name.name)}>
                <span class="kind">{f.kind}</span>{f.name.name}
              </button>
            </li>
          {/each}
        </ul>
        <div class="buttons">
          <button onclick={() => addFunction("function")}>New function</button>
          <button onclick={() => addFunction("action")}>New action</button>
        </div>
      </div>
      <div class="function-editor">
        {#if fn}
          {@const item = fn.span.start}
          <header class="editor-title">
            <span class="kind">{fn.kind}</span>
            <code>{signature(fn)}</code>
          </header>
          <div class="editor-body">
            {#key item}
              <SourceEditor
                load={() => invoke<CodeView>("decl_text", { path: file.path, item })}
                check={async (text) => (await invoke<DeclTrial>("try_decl", { path: file.path, item, text })).problems}
                save={(text) => saveFunction(item, text)}
                reloadKey={file.path + "\n" + item + "\n" + file.version}
              />
            {/key}
          </div>
        {:else}
          <p class="muted pick">No functions or actions yet: New function and New action add one.</p>
        {/if}
      </div>
    </div>
  </section>

  {#if engines.length > 0}
    <section>
      <h3>Engines</h3>
      <p class="muted">
        Processing compiled to WebAssembly, its file in the <code>engines</code> folder. Code calls a function as
        <code>Engine.function(...)</code>; the engine only computes, and code hands it the records it needs.
      </p>
      <ul>
        {#each engines as { engine, problems } (engine.name.name)}
          <li>
            <button onclick={() => onopen("engine", engine.name.name)} title={problems.length > 0 ? problems.join("\n") : "Open the engine: its functions, settings and code"}>
              <span class="kind">engine</span>{engine.name.name}<span class="muted"
                >{problems.length > 0 ? "does not fit its file" : engine.functions.map((f) => f.name.name).join(", ")}</span
              >
            </button>
          </li>
        {/each}
      </ul>
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

  {#if file.entities.some((e) => !e.settings)}
    <section>
      <h3>Data events</h3>
      <p class="muted">
        Code that runs whenever a record is saved or deleted, from any window or action: <code>before save</code>,
        <code>after save</code>, <code>before delete</code> and <code>after delete</code>. <code>refuse "text"</code> stops the save or
        delete, and nothing of it is kept.
      </p>
      <ul>
        {#each file.entities.filter((e) => !e.settings) as e (e.name.name)}
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

  .buttons {
    display: flex;
    gap: 8px;
    margin: 8px 0;
  }

  .functions {
    display: grid;
    grid-template-columns: minmax(180px, 260px) minmax(0, 1fr);
    height: 420px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    overflow: hidden;
  }

  .function-list {
    display: flex;
    flex-direction: column;
    min-height: 0;
    border-right: 1px solid var(--border);
    background: var(--panel-alt);
  }

  .function-list ul {
    flex: 1;
    overflow: auto;
    padding: 4px;
  }

  .function-list li button.active {
    background: var(--panel);
    box-shadow: inset 3px 0 0 var(--accent);
  }

  .function-list .buttons {
    flex-direction: column;
    margin: 0;
    padding: 8px;
    border-top: 1px solid var(--border);
  }

  .function-editor {
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
  }

  .editor-title {
    display: flex;
    align-items: baseline;
    gap: 10px;
    padding: 8px 10px;
    border-bottom: 1px solid var(--border);
  }

  .editor-title code {
    font-weight: 600;
  }

  .editor-body {
    display: flex;
    flex: 1;
    min-height: 0;
  }

  .pick {
    padding: 16px;
  }
</style>
