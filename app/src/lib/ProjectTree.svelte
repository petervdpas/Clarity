<script lang="ts">
  // The project as one tree: the app, its windows, its data, its themes, its languages, its engines and
  // the modules its files form.

  import type { Page } from "./generated/Page";
  import type { Preview } from "./generated/Preview";
  import { projectEngines } from "./engines";
  import { findApp } from "./frame";
  import type { Selected } from "./selection";

  let {
    preview,
    selected,
    full = true,
    onapp,
    onwindow,
    onselect,
    onnewwindow = null,
    onnewapp = null,
    onnewdata = null,
    onnewtheme = null,
    onnewengine = null,
  }: {
    preview: Preview;
    /** What is open: a window, the app, an entity, an enum or a module. */
    selected: Selected | null;
    /** Data and modules too, not only the app and its windows. */
    full?: boolean;
    onapp: () => void;
    onwindow: (page: Page) => void;
    onselect: (selected: Selected) => void;
    onnewwindow?: (() => void) | null;
    onnewapp?: (() => void) | null;
    /** Starts a new entity or enum. */
    onnewdata?: ((kind: "entity" | "enum") => void) | null;
    onnewtheme?: (() => void) | null;
    /** Starts a new engine; null where the tree only shows. */
    onnewengine?: (() => void) | null;
  } = $props();

  function is(kind: Selected["kind"], name: string): boolean {
    return selected?.kind === kind && selected.name === name;
  }

  const app = $derived(findApp(preview));
  const files = $derived(preview.project.files);
  /** Windows in file order, the main window first. */
  const windows = $derived.by(() => {
    const all = files.flatMap((f) => f.pages.map((page) => ({ page, module: f.module?.name ?? "" })));
    const main = app?.main?.name;
    return all.toSorted((a, b) => Number(b.page.name.name === main) - Number(a.page.name.name === main));
  });
  const entities = $derived(files.flatMap((f) => f.entities.map((entity) => ({ entity, module: f.module?.name ?? "" }))));
  const enums = $derived(files.flatMap((f) => f.enums.map((decl) => ({ decl, module: f.module?.name ?? "" }))));
  const themes = $derived(files.flatMap((f) => f.themes.map((theme) => ({ theme, module: f.module?.name ?? "" }))));
  const engines = $derived(projectEngines(preview.project));
  /** The language the texts are written in, and those the project translates them to. */
  const written = $derived(preview.project.locale.language ?? "en");
  const languages = $derived(Object.keys(preview.project.translations).toSorted());

  function fileName(path: string): string {
    return path.split(/[\\/]/).at(-1) ?? path;
  }
</script>

<div class="tree">
  {#if app}
    <button class="row top" class:current={selected?.kind === "app"} onclick={onapp} title="The app: its frame and main window">
      <span class="badge app">app</span>
      {app.title ?? app.name.name}
    </button>
  {:else if onnewapp}
    <button class="row add" onclick={onnewapp}>+ New app</button>
  {/if}

  <details open>
    <summary>Windows <span class="count">{windows.length}</span></summary>
    {#each windows as { page, module } (page.name.name)}
      <button
        class="row"
        class:current={is("window", page.name.name)}
        title="{page.name.name} in module {module}"
        onclick={() => onwindow(page)}
      >
        {page.name.name}
        {#if page.name.name === app?.main?.name}<span class="note">main</span>{/if}
        {#if page.kind !== "window"}<span class="note">{page.kind}</span>{/if}
      </button>
    {:else}
      <p class="empty">No windows yet.</p>
    {/each}
    {#if onnewwindow}
      <button class="row add" onclick={onnewwindow}>+ New window</button>
    {/if}
  </details>

  {#if full}
    <details open>
      <summary>Data <span class="count">{entities.length + enums.length}</span></summary>
      {#each entities as { entity, module } (entity.name.name)}
        <button class="row" class:current={is("entity", entity.name.name)} title="in module {module}" onclick={() => onselect({ kind: "entity", name: entity.name.name })}>
          {entity.name.name}
          <span class="note">{entity.fields.length} fields</span>
        </button>
      {/each}
      {#each enums as { decl, module } (decl.name.name)}
        <button class="row" class:current={is("enum", decl.name.name)} title="in module {module}" onclick={() => onselect({ kind: "enum", name: decl.name.name })}>
          {decl.name.name}
          <span class="note">enum, {decl.variants.length} values</span>
        </button>
      {/each}
      {#if entities.length === 0 && enums.length === 0}
        <p class="empty">No data yet.</p>
      {/if}
      {#if onnewdata}
        <button class="row add" onclick={() => onnewdata("entity")}>+ New entity</button>
        <button class="row add" onclick={() => onnewdata("enum")}>+ New enum</button>
      {/if}
    </details>

    <details open>
      <summary>Themes <span class="count">{themes.length + 1}</span></summary>
      <button
        class="row"
        class:current={is("theme", preview.baseTheme.name.name)}
        title="Clarity's own theme, the base every theme changes"
        onclick={() => onselect({ kind: "theme", name: preview.baseTheme.name.name })}
      >
        {preview.baseTheme.name.name}
        <span class="note">built in</span>
      </button>
      {#each themes as { theme, module } (theme.name.name)}
        {@const name = theme.name.name}
        <button class="row" class:current={is("theme", name)} title="in module {module}" onclick={() => onselect({ kind: "theme", name })}>
          {name}
          {#if preview.project.themes[0] === name}
            <span class="note">offered, starts with it</span>
          {:else if preview.project.themes.includes(name)}
            <span class="note">offered</span>
          {/if}
        </button>
      {/each}
      {#if onnewtheme}
        <button class="row add" onclick={onnewtheme}>+ New theme</button>
      {/if}
    </details>

    <details open>
      <summary>i18n <span class="count">{languages.length + 1}</span></summary>
      <button class="row" class:current={is("language", written)} title="The language the texts are written in" onclick={() => onselect({ kind: "language", name: written })}>
        {written}
        <span class="note">as written</span>
      </button>
      {#each languages as language (language)}
        <button class="row" class:current={is("language", language)} title="translations/{language}.clarity" onclick={() => onselect({ kind: "language", name: language })}>
          {language}
          <span class="note">translation</span>
        </button>
      {/each}
      {#if full}
        <button class="row add" class:current={is("language", "")} onclick={() => onselect({ kind: "language", name: "" })}>+ New language</button>
      {/if}
    </details>

    <details open={engines.length > 0}>
      <summary>Engines <span class="count">{engines.length}</span></summary>
      {#each engines as { engine, module, problems } (engine.name.name)}
        <button
          class="row"
          class:current={is("engine", engine.name.name)}
          title={problems.length > 0 ? problems.join("\n") : `${engine.file} in module ${module}`}
          onclick={() => onselect({ kind: "engine", name: engine.name.name })}
        >
          {engine.name.name}
          {#if problems.length > 0}
            <span class="note problem">does not fit its file</span>
          {:else}
            <span class="note">{engine.functions.length === 1 ? "1 function" : `${engine.functions.length} functions`}</span>
          {/if}
        </button>
      {:else}
        <p class="empty">No engines yet: processing compiled to WebAssembly, which code calls.</p>
      {/each}
      {#if onnewengine}
        <button class="row add" onclick={onnewengine}>+ New engine</button>
      {/if}
    </details>

    <details>
      <summary>Modules <span class="count">{files.length}</span></summary>
      {#each files as f (f.path)}
        {@const name = f.module?.name ?? ""}
        <button class="row module" class:current={is("module", name)} title={f.path} onclick={() => onselect({ kind: "module", name })}>
          <span>{name || "(no module)"}</span>
          <code>{fileName(f.path)}</code>
          {#if f.uses.length > 0}
            <span class="uses">uses {f.uses.map((u) => u.name).join(", ")}</span>
          {/if}
        </button>
      {/each}
    </details>
  {/if}
</div>

<style>
  .tree {
    display: flex;
    flex-direction: column;
    gap: 2px;
    font-size: 13px;
  }

  details {
    margin-top: 4px;
  }

  summary {
    cursor: default;
    padding: 3px 4px;
    font-size: 12px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--text-muted);
  }

  .count,
  .note {
    font-weight: 400;
    font-size: 11px;
    color: var(--text-muted);
    text-transform: none;
    letter-spacing: 0;
  }

  .note {
    margin-left: 6px;
  }

  .row {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    text-align: left;
    border: none;
    border-radius: var(--radius);
    background: transparent;
    padding: 4px 8px 4px 20px;
    font-size: 13px;
  }

  .row.top {
    padding-left: 6px;
    font-weight: 600;
  }

  .row:hover {
    background: var(--panel-alt);
  }

  .row.current {
    background: var(--accent-soft);
  }

  .row.add {
    color: var(--accent);
    font-size: 12px;
  }

  .badge {
    font-size: 10px;
    text-transform: uppercase;
    border-radius: 3px;
    padding: 1px 5px;
    border: 1px solid var(--brand);
    color: var(--brand);
  }

  code {
    font-family: var(--mono);
    font-size: 11px;
    color: var(--text-muted);
  }

  .row.module {
    flex-wrap: wrap;
    gap: 2px 8px;
  }

  .uses {
    flex-basis: 100%;
    font-size: 11px;
    color: var(--text-muted);
  }

  .note.problem {
    color: var(--error);
  }

  .empty {
    margin: 2px 0 4px 20px;
    font-size: 12px;
    color: var(--text-muted);
  }
</style>
