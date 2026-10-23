<script lang="ts">
  // The project as one tree: the app, its windows, its data, its themes, its languages and resources,
  // its engines and the modules its files form.

  import type { Page } from "./generated/Page";
  import type { Preview } from "./generated/Preview";
  import { projectEngines, engineNote } from "./engines";
  import { commandLine } from "./commandLine";
  import { findApp } from "./frame";
  import { languageName } from "./translate";
  import { resourceKind, resourceUses } from "./resources";
  import { resourceKinds } from "./resourceKinds.svelte";
  import { tr } from "./model";
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
    onnewlanguage = null,
    onnewresource = null,
    onnewdiagram = null,
    onmenu = null,
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
    /** Asks for a new language and writes its translation file; null where the tree only shows. */
    onnewlanguage?: (() => void) | null;
    /** Asks for a new resource: its name, its type and maybe a file to copy in; null where the tree only shows. */
    onnewresource?: (() => void) | null;
    /** Asks for a new diagram: its name and kind; null where the tree only shows. */
    onnewdiagram?: (() => void) | null;
    /** A right click on a window, entity, enum, theme or engine; null where the tree only shows. */
    onmenu?: ((e: MouseEvent, item: Selected) => void) | null;
  } = $props();

  /** Hands a right click to `onmenu`, in place of the page's own menu. */
  function menu(e: MouseEvent, item: Selected) {
    if (!onmenu) return;
    e.preventDefault();
    onmenu(e, item);
  }

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
  const entities = $derived(files.flatMap((f) => f.entities.filter((e) => !e.settings).map((entity) => ({ entity, module: f.module?.name ?? "" }))));
  const enums = $derived(files.flatMap((f) => f.enums.map((decl) => ({ decl, module: f.module?.name ?? "" }))));
  const themes = $derived(files.flatMap((f) => f.themes.map((theme) => ({ theme, module: f.module?.name ?? "" }))));
  const engines = $derived(projectEngines(preview.project));
  // The outside libraries code calls (DESIGN 5.21), with their modules.
  const libraries = $derived(preview.project.files.flatMap((f) => f.libraries.map((library) => ({ library, module: f.module?.name ?? "" }))));
  // The custom parts outside libraries draw (DESIGN 5.19), with their modules.
  const customParts = $derived(preview.project.files.flatMap((f) => f.parts.map((part) => ({ part, module: f.module?.name ?? "" }))));
  function libraryNote(part: (typeof customParts)[number]["part"]): string {
    const l = part.library;
    if (!l) return "no library";
    if (l.kind === "native") return "C, C++";
    if (l.kind === "crate") return "Rust";
    if (l.kind === "go") return "Go";
    return l.kind === "nuget" ? "NuGet" : "DLL";
  }
  // The commands of the command-line program (DESIGN 5.20), by what is typed, with their module.
  const commands = $derived(
    preview.project.files.flatMap((f) => f.commands.map((c) => ({ line: commandLine(c.group?.name ?? null, c.name.name), module: f.module?.name ?? "" }))),
  );
  /** The language the texts are written in, and those the project translates them to. */
  const written = $derived(preview.project.locale.language ?? "en");
  const languages = $derived(Object.keys(preview.project.translations).filter((l) => l !== written).toSorted());
  /** The written diagrams of the diagrams folder. */
  const diagrams = $derived(preview.project.diagrams);
  const DIAGRAM_KINDS: Record<string, string> = { erd: "ERD", class: "class diagram", state: "state diagram", sequence: "sequence diagram", activity: "activity diagram", architecture: "architecture" };

  /** The files of the resources folder, by name. */
  const resources = $derived(preview.project.resources?.files.map((f) => f.name) ?? []);
  /** The name of a resource's type, as the backend lists the types. */
  function typeOf(name: string): string {
    const kind = resourceKind(name, resourceKinds.list);
    return resourceKinds.list.find((k) => k.kind === kind)?.name ?? "";
  }

  function fileName(path: string): string {
    return path.split(/[\\/]/).at(-1) ?? path;
  }
</script>

<div class="tree">
  {#if app}
    <button class="row top" class:current={selected?.kind === "app"} onclick={onapp} title="The app: its frame and main window">
      <span class="badge app">app</span>
      {app.title ? tr(preview, app.title) : app.name.name}
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
        oncontextmenu={(e) => menu(e, { kind: "window", name: page.name.name })}
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
        <button class="row" class:current={is("entity", entity.name.name)} title="in module {module}" onclick={() => onselect({ kind: "entity", name: entity.name.name })} oncontextmenu={(e) => menu(e, { kind: "entity", name: entity.name.name })}>
          {entity.name.name}
          <span class="note">{entity.fields.length} fields</span>
        </button>
      {/each}
      {#each enums as { decl, module } (decl.name.name)}
        <button class="row" class:current={is("enum", decl.name.name)} title="in module {module}" onclick={() => onselect({ kind: "enum", name: decl.name.name })} oncontextmenu={(e) => menu(e, { kind: "enum", name: decl.name.name })}>
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

    <!-- Diagrams of the diagrams folder: written by hand, by the agent or from the project; they propose changes. -->
    <details open>
      <summary>Diagrams <span class="count">{diagrams.length}</span></summary>
      {#each diagrams as d (d.name)}
        <button
          class="row"
          class:current={is("diagram", d.name)}
          title="diagrams/{d.name}"
          onclick={() => onselect({ kind: "diagram", name: d.name })}
          oncontextmenu={(e) => menu(e, { kind: "diagram", name: d.name })}
        >
          {d.name}
          <span class="note">{d.kind ? DIAGRAM_KINDS[d.kind] : "drawn only"}</span>
        </button>
      {/each}
      {#if onnewdiagram}
        <button class="row add" onclick={onnewdiagram}>+ New diagram</button>
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
        <button class="row" class:current={is("theme", name)} title="in module {module}" onclick={() => onselect({ kind: "theme", name })} oncontextmenu={(e) => menu(e, { kind: "theme", name })}>
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
        {languageName(written)}
        <span class="note">{written}, default</span>
      </button>
      {#each languages as language (language)}
        <button class="row" class:current={is("language", language)} title="translations/{language}.clarity" onclick={() => onselect({ kind: "language", name: language })}>
          {languageName(language)}
          <span class="note">{language}</span>
        </button>
      {/each}
      {#if onnewlanguage}
        <button class="row add" onclick={onnewlanguage}>+ New language</button>
      {/if}
    </details>

    <!-- The files the application carries; keys point each language at one, parts show one by its source. -->
    <details open={resources.length > 0}>
      <summary>Resources <span class="count">{resources.length}</span></summary>
      {#each resources as name (name)}
        {@const uses = resourceUses(preview.project.resource_keys, name)}
        <button class="row" class:current={is("resource", name)} title={uses.length > 0 ? uses.join("\n") : `resources/${name}`} onclick={() => onselect({ kind: "resource", name })}>
          {name}
          <span class="note">{typeOf(name)}</span>
        </button>
      {:else}
        <p class="empty">No resources yet: the Markdown, pictures, PDFs, drawings and other files the application carries.</p>
      {/each}
      {#if onnewresource}
        <button class="row add" onclick={onnewresource}>+ New resource</button>
      {/if}
    </details>

    <details open={engines.length > 0}>
      <summary>Engines <span class="count">{engines.length}</span></summary>
      {#each engines as { engine, module, problems } (engine.name.name)}
        <button
          class="row"
          class:current={is("engine", engine.name.name)}
          title={problems.length > 0 ? problems.join("\n") : engine.body ? `Written in Clarity, in module ${module}` : `${engine.file} in module ${module}`}
          onclick={() => onselect({ kind: "engine", name: engine.name.name })}
          oncontextmenu={(e) => menu(e, { kind: "engine", name: engine.name.name })}
        >
          {engine.name.name}
          {#if problems.length > 0}
            <span class="note problem">{engine.body ? "has problems" : "does not fit its file"}</span>
          {:else}
            <span class="note">{engineNote(engine)}</span>
          {/if}
        </button>
      {:else}
        <p class="empty">No engines yet: processing compiled to WebAssembly, which code calls.</p>
      {/each}
      {#if onnewengine}
        <button class="row add" onclick={onnewengine}>+ New engine</button>
      {/if}
    </details>

    <details open={customParts.length > 0}>
      <summary>Custom parts <span class="count">{customParts.length}</span></summary>
      {#each customParts as { part, module } (part.name.name)}
        <button class="row" class:current={is("part", part.name.name)} title="in module {module}" onclick={() => onselect({ kind: "part", name: part.name.name })}>
          {part.name.name}
          <span class="note">{libraryNote(part)}</span>
        </button>
      {:else}
        <p class="empty">No custom parts yet: parts an outside library draws, declared with <code>part Name</code> in a module.</p>
      {/each}
    </details>

    <details open={libraries.length > 0}>
      <summary>Libraries <span class="count">{libraries.length}</span></summary>
      {#each libraries as { library, module } (library.name.name)}
        <button class="row" class:current={is("library", library.name.name)} title="in module {module}" onclick={() => onselect({ kind: "library", name: library.name.name })}>
          {library.name.name}
          <span class="note">{library.source?.kind ?? "no package"}</span>
        </button>
      {:else}
        <p class="empty">No libraries yet: packages of the project's language that code calls, declared with <code>library Name</code>.</p>
      {/each}
    </details>

    {#if commands.length > 0}
      <details open>
        <summary>Commands <span class="count">{commands.length}</span></summary>
        {#each commands as { line, module } (line)}
          <button class="row" class:current={is("module", module)} title="in module {module}" onclick={() => onselect({ kind: "module", name: module })}>
            <code>{line}</code>
          </button>
        {/each}
      </details>
    {/if}

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
