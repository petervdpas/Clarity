<script lang="ts">
  // One module of the project: its file, what it declares and which other
  // modules it may use.

  import type { Edit } from "./generated/Edit";
  import type { Preview } from "./generated/Preview";
  import type { SpecFile } from "./generated/SpecFile";

  let {
    preview,
    file,
    onedit,
    onopen,
  }: {
    preview: Preview;
    file: SpecFile;
    onedit: (edit: Edit) => void;
    /** Opens a declaration by kind and name. */
    onopen: (kind: "window" | "app" | "entity" | "enum", name: string) => void;
  } = $props();

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
</div>

<style>
  .view {
    display: flex;
    flex-direction: column;
    gap: 16px;
    max-width: 640px;
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
</style>
