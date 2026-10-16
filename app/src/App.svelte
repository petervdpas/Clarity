<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { ask, open as openFolder } from "@tauri-apps/plugin-dialog";
  import { onMount } from "svelte";
  import FrameView from "./lib/FrameView.svelte";
  import NewWindowDialog from "./lib/NewWindowDialog.svelte";
  import PageDesigner from "./lib/PageDesigner.svelte";
  import PageView from "./lib/PageView.svelte";
  import RibbonButton from "./lib/RibbonButton.svelte";
  import { applyEdit } from "./lib/applyEdit";
  import type { Diagnostic } from "./lib/generated/Diagnostic";
  import type { HistoryCounts } from "./lib/generated/HistoryCounts";
  import type { Page } from "./lib/generated/Page";
  import type { PageTemplate } from "./lib/generated/PageTemplate";
  import type { Preview } from "./lib/generated/Preview";
  import type { SpecFile } from "./lib/generated/SpecFile";
  import { pageDesigner } from "./lib/pageDesigner.svelte";
  import { findApp, frameForPage } from "./lib/frame";
  import { describeType } from "./lib/model";
  import { appAsPage, findPage, pageData } from "./lib/pages";

  const LAST_DIR_KEY = "clarity.preview.lastDir";

  /** One open window, or the app itself. Following a button pushes a window. */
  type Open = { name: string; app?: boolean };

  /** The ribbon tabs. Design and Preview are also the two modes. */
  type Tab = "project" | "design" | "preview";
  const TABS: { key: Tab; name: string }[] = [
    { key: "project", name: "Project" },
    { key: "design", name: "Design" },
    { key: "preview", name: "Preview" },
  ];

  /** What the sidebar shows while designing. */
  type DesignSide = "windows" | "parts" | "outline" | "settings";
  const DESIGN_SIDES: { key: DesignSide; name: string }[] = [
    { key: "windows", name: "Windows" },
    { key: "parts", name: "Parts" },
    { key: "outline", name: "Window" },
    { key: "settings", name: "Settings" },
  ];

  let openDir = $state<string | null>(null);
  let preview = $state<Preview | null>(null);
  let loadError = $state<string | null>(null);
  let stack = $state<Open[]>([]);
  let tab = $state<Tab>("preview");
  let designSide = $state<DesignSide>("parts");
  let history = $state<HistoryCounts>({ undo: 0, redo: 0 });
  let actionError = $state<string | null>(null);
  /** The file a new window goes into, while the New window dialog is open. */
  let newWindowIn = $state<SpecFile | null>(null);
  let newWindowError = $state<string | null>(null);

  const current = $derived(stack.at(-1) ?? null);
  const app = $derived(preview ? findApp(preview) : null);
  /** The app, when it is the open element (it is designed like a window frame). */
  const openApp = $derived(current?.app ? app : null);
  const page = $derived(preview && current && !current.app ? findPage(preview, current.name) : undefined);
  const appPage = $derived(app ? appAsPage(app) : null);
  const file = $derived(
    preview?.project.files.find(
      (f) =>
        (page && f.pages.some((p) => p.name.name === page.name.name)) ||
        (openApp && f.apps.some((a) => a.name.name === openApp.name.name)),
    ),
  );
  const designing = $derived(tab === "design" && (page !== undefined || openApp !== null));
  // Widgets in the app's sidebars and footer see the app's data next to the window's.
  const appData = $derived(preview && appPage ? pageData(preview, appPage) : {});
  const entities = $derived(preview?.project.files.flatMap((f) => f.entities) ?? []);
  const errors = $derived(preview?.project.diagnostics.filter((d) => d.severity === "error") ?? []);
  const warnings = $derived(preview?.project.diagnostics.filter((d) => d.severity === "warning") ?? []);
  const projectName = $derived(app?.title ?? app?.name.name ?? (openDir ? fileName(openDir) : ""));

  // Entering design mode shows the parts.
  $effect.pre(() => {
    if (designing) designSide = "parts";
  });

  function readLastDir(): string | null {
    try {
      return localStorage.getItem(LAST_DIR_KEY);
    } catch {
      return null;
    }
  }

  function writeLastDir(dir: string) {
    try {
      localStorage.setItem(LAST_DIR_KEY, dir);
    } catch {
      // Remembering the project is a convenience only.
    }
  }

  /** The window the project starts with: the app's main window, or the first one. */
  function startWindow(p: Preview): Open[] {
    const main = findApp(p)?.main?.name;
    const first = main ?? p.project.files.flatMap((f) => f.pages)[0]?.name.name;
    return first ? [{ name: first }] : [];
  }

  async function refreshHistory() {
    try {
      history = await invoke<HistoryCounts>("history_counts");
    } catch {
      history = { undo: 0, redo: 0 };
    }
  }

  function applyPreview(next: Preview) {
    preview = next;
    // Keep the open windows that still exist after an edit.
    const kept = stack.filter((o) => (o.app ? findApp(next) !== null : findPage(next, o.name)));
    stack = kept.length > 0 ? kept : startWindow(next);
    refreshHistory();
  }

  async function openProject(dir: string) {
    const trimmed = dir.trim();
    if (!trimmed) return;
    try {
      const result = await invoke<Preview>("open_spec_dir", { path: trimmed });
      loadError = null;
      openDir = trimmed;
      writeLastDir(trimmed);
      stack = [];
      applyPreview(result);
    } catch (err) {
      loadError = String(err);
    }
  }

  async function chooseProject() {
    const dir = await openFolder({ directory: true, title: "Open a Clarity project" });
    if (typeof dir === "string") openProject(dir);
  }

  async function newProject() {
    const dir = await openFolder({ directory: true, title: "Choose an empty folder for the new project" });
    if (typeof dir !== "string") return;
    try {
      const result = await invoke<Preview>("new_project", { path: dir });
      loadError = null;
      openDir = dir;
      writeLastDir(dir);
      stack = [];
      applyPreview(result);
      tab = "design";
    } catch (err) {
      loadError = String(err);
    }
  }

  function selectApp() {
    if (app) stack = [{ name: app.name.name, app: true }];
    pageDesigner.selected = null;
    tab = "design";
  }

  function selectPage(p: Page) {
    stack = [{ name: p.name.name }];
    pageDesigner.selected = null;
  }

  /** Follows a button or menu item: opens the target window on top of the current one. */
  function navigate(target: string) {
    if (!preview) return;
    if (target === "close") {
      if (stack.length > 1) stack = stack.slice(0, -1);
      return;
    }
    if (findPage(preview, target)) stack = [...stack, { name: target }];
  }

  async function addWindow(target: SpecFile, name: string, template: PageTemplate) {
    try {
      const result = await applyEdit(target, { op: "addWindow", name, template });
      applyPreview(result.preview);
      newWindowIn = null;
      newWindowError = null;
      stack = [{ name }];
      pageDesigner.selected = null;
      tab = "design";
    } catch (err) {
      newWindowError = String(err);
    }
  }

  function startNewWindow() {
    const target = file ?? preview?.project.files[0];
    if (target) {
      newWindowError = null;
      newWindowIn = target;
    }
  }

  async function addApp(target: SpecFile) {
    try {
      const name = target.module?.name ?? "App";
      const result = await applyEdit(target, { op: "addApp", name: name.charAt(0).toUpperCase() + name.slice(1) });
      applyPreview(result.preview);
      selectApp();
    } catch (err) {
      actionError = String(err);
    }
  }

  async function replay(command: "undo_edit" | "redo_edit") {
    actionError = null;
    try {
      applyPreview(await invoke<Preview>(command));
    } catch (err) {
      actionError = String(err);
    }
  }

  /** Deletes the picked part, or the open window when nothing is picked. */
  async function deletePicked() {
    if (!file) return;
    actionError = null;
    let item = pageDesigner.selected;
    if (item === null) {
      const subject = openApp ? `the app ${openApp.name.name}` : page ? `the window ${page.name.name}` : null;
      if (!subject) return;
      if (!(await ask(`Delete ${subject}?`, { title: "Clarity", kind: "warning" }))) return;
      item = openApp ? openApp.span.start : (page?.span.start ?? null);
      if (item === null) return;
    }
    try {
      const result = await applyEdit(file, { op: "remove", item });
      pageDesigner.selected = null;
      applyPreview(result.preview);
    } catch (err) {
      actionError = String(err);
    }
  }

  function onkeydown(e: KeyboardEvent) {
    if (tab !== "design" || !(e.ctrlKey || e.metaKey)) return;
    if ((e.target as HTMLElement | null)?.closest("input, textarea, select")) return;
    const key = e.key.toLowerCase();
    if (key === "z" && !e.shiftKey) {
      e.preventDefault();
      replay("undo_edit");
    } else if (key === "y" || (key === "z" && e.shiftKey)) {
      e.preventDefault();
      replay("redo_edit");
    }
  }

  function fileName(path: string): string {
    return path.split(/[\\/]/).at(-1) ?? path;
  }

  function location(d: Diagnostic): string {
    return `${fileName(d.path)}:${d.span.line}:${d.span.col}`;
  }

  onMount(() => {
    const unlisten = listen<Preview>("preview-updated", (e) => applyPreview(e.payload));
    invoke<string | null>("initial_spec_dir").then((dir) => {
      const start = dir ?? readLastDir();
      if (start) openProject(start);
    });
    return () => {
      unlisten.then((stop) => stop());
    };
  });
</script>

<svelte:window {onkeydown} />

{#snippet windowList(designList: boolean)}
  <nav>
    {#each preview?.project.files ?? [] as f (f.path)}
      <div class="module">
        <div class="module-name">
          {f.module?.name ?? "(no module)"} <span class="file">{fileName(f.path)}</span>
        </div>
        {#each f.apps as a (a.name.name)}
          <button class="screen-link" class:current={stack[0]?.app === true} onclick={selectApp}>
            <span class="pattern app">app</span>
            {a.name.name}
          </button>
        {/each}
        {#each f.pages as p (p.name.name)}
          <button class="screen-link" class:current={!stack[0]?.app && stack[0]?.name === p.name.name} onclick={() => selectPage(p)}>
            <span class="pattern page">{p.kind}</span>
            {p.name.name}
          </button>
        {/each}
        {#if f.pages.length === 0 && f.apps.length === 0}
          <div class="none">No windows</div>
        {/if}
        {#if designList}
          <button class="add-page" onclick={() => ((newWindowError = null), (newWindowIn = f))}>+ New window</button>
          {#if !app}
            <button class="add-page" onclick={() => addApp(f)}>+ New app</button>
          {/if}
        {/if}
      </div>
    {/each}
  </nav>
{/snippet}

<div class="app">
  <header class="ribbon">
    <div class="ribbon-tabs" role="tablist">
      <div class="brand"><span class="logo">C</span> Clarity</div>
      {#each TABS as t (t.key)}
        <button role="tab" aria-selected={tab === t.key} class:active={tab === t.key} onclick={() => (tab = t.key)}>{t.name}</button>
      {/each}
      {#if projectName}<span class="project-name">{projectName}</span>{/if}
    </div>
    <div class="ribbon-body">
      {#if tab === "project"}
        <div class="group">
          <div class="buttons">
            <RibbonButton icon="new" label="New" title="Create a project in an empty folder" onclick={newProject} />
            <RibbonButton icon="open" label="Open" title="Open a project folder" onclick={chooseProject} />
            <RibbonButton icon="reload" label="Reload" disabled={!openDir} onclick={() => openDir && openProject(openDir)} />
          </div>
          <div class="group-name">Project</div>
        </div>
        <div class="group">
          <div class="buttons">
            <RibbonButton icon="window" label="New window" disabled={!preview} onclick={startNewWindow} />
            <RibbonButton icon="app" label={app ? "App" : "New app"} disabled={!preview} onclick={() => (app ? selectApp() : preview && addApp(preview.project.files[0]))} />
          </div>
          <div class="group-name">Windows</div>
        </div>
        <div class="group">
          <div class="buttons">
            <RibbonButton icon="check" label={errors.length === 0 ? "No errors" : errors.length === 1 ? "1 error" : `${errors.length} errors`} disabled={!preview} onclick={() => openDir && openProject(openDir)} />
          </div>
          <div class="group-name">Check</div>
        </div>
      {:else if tab === "design"}
        <div class="group">
          <div class="buttons">
            <RibbonButton icon="undo" label="Undo" title="Undo (Ctrl+Z)" disabled={history.undo === 0} onclick={() => replay("undo_edit")} />
            <RibbonButton icon="redo" label="Redo" title="Redo (Ctrl+Y)" disabled={history.redo === 0} onclick={() => replay("redo_edit")} />
            <RibbonButton
              icon="delete"
              label="Delete"
              title={pageDesigner.selected !== null ? "Delete the picked part" : "Delete this window"}
              disabled={!designing}
              onclick={deletePicked}
            />
          </div>
          <div class="group-name">Edit</div>
        </div>
        <div class="group">
          <div class="buttons">
            <RibbonButton icon="window" label="New window" disabled={!preview} onclick={startNewWindow} />
            <RibbonButton icon="grid" label="Grid" active={pageDesigner.showGrid} onclick={() => (pageDesigner.showGrid = !pageDesigner.showGrid)} />
          </div>
          <div class="group-name">Window</div>
        </div>
        <div class="group">
          <div class="buttons">
            <RibbonButton
              icon="save"
              label={pageDesigner.saving ? "Saving..." : "Saved"}
              title="Every change is written to the project files as you make it"
              disabled={true}
              onclick={() => {}}
            />
          </div>
          <div class="group-name">File</div>
        </div>
      {:else}
        <div class="group">
          <div class="buttons">
            <RibbonButton icon="play" label="Main window" disabled={!preview} onclick={() => preview && (stack = startWindow(preview))} />
            <RibbonButton icon="back" label="Back" disabled={stack.length < 2} onclick={() => (stack = stack.slice(0, -1))} />
          </div>
          <div class="group-name">Run</div>
        </div>
        {#if stack.length > 1}
          <div class="breadcrumb">
            {#each stack as o, i (i)}
              {#if i > 0}<span class="sep">/</span>{/if}
              <button disabled={i === stack.length - 1} onclick={() => (stack = stack.slice(0, i + 1))}>{o.name}</button>
            {/each}
          </div>
        {/if}
      {/if}
      {#if actionError || loadError}
        <p class="ribbon-error" role="alert">
          {actionError ?? loadError}
          <button onclick={() => ((actionError = null), (loadError = null))}>Dismiss</button>
        </p>
      {/if}
    </div>
  </header>

  <div class="shell">
    <aside>
      {#if tab === "design" && designing}
        <div class="side-tabs" role="tablist">
          {#each DESIGN_SIDES as t (t.key)}
            <button role="tab" aria-selected={designSide === t.key} class:active={designSide === t.key} onclick={() => (designSide = t.key)}>{t.name}</button>
          {/each}
        </div>
      {/if}
      <!-- Designers put their parts and their window outline here. -->
      <div id="design-parts" class="design-sidebar" class:hidden={!designing || designSide !== "parts"}></div>
      <div id="design-outline" class="design-sidebar" class:hidden={!designing || designSide !== "outline"}></div>

      {#if tab === "project"}
        <section class="side-section">
          <h2>Project</h2>
          {#if preview}
            <dl>
              <dt>Name</dt>
              <dd>{projectName}</dd>
              <dt>Folder</dt>
              <dd class="path" title={openDir}>{openDir}</dd>
              <dt>Main window</dt>
              <dd>{app?.main?.name ?? "(none)"}</dd>
              <dt>Stack</dt>
              <dd>Rust backend, Tauri, Svelte</dd>
            </dl>
          {:else}
            <p class="none">No project is open. Use New or Open in the ribbon.</p>
          {/if}
        </section>
        {#if preview}
          <section class="side-section">
            <h2>Windows</h2>
            {@render windowList(true)}
          </section>
          <section class="side-section">
            <h2>Data</h2>
            {#each entities as entity (entity.name.name)}
              <details class="entity">
                <summary>{entity.name.name} <span class="file">{entity.fields.length} fields</span></summary>
                <ul>
                  {#each entity.fields as field (field.name.name)}
                    <li>{field.name.name} <code>{describeType(field)}</code></li>
                  {/each}
                </ul>
              </details>
            {:else}
              <p class="none">No entities yet.</p>
            {/each}
          </section>
        {/if}
      {:else if tab === "design" && (!designing || designSide === "windows")}
        <section class="side-section">
          <h2>Windows</h2>
          {@render windowList(true)}
        </section>
      {:else if tab === "design" && designSide === "settings"}
        <section class="side-section">
          <h2>Design settings</h2>
          <label class="check">
            <input type="checkbox" bind:checked={pageDesigner.showGrid} />
            Show the snap grid
          </label>
          <p class="none">Every change is written to the project files at once; Undo and Redo in the ribbon take changes back.</p>
        </section>
      {:else if tab === "preview"}
        <section class="side-section">
          <h2>Run a window</h2>
          {@render windowList(false)}
        </section>
      {/if}

      {#if preview}
        <section class="diagnostics">
          <h2>
            Problems
            <span class="count error">{errors.length}</span>
            <span class="count warning">{warnings.length}</span>
          </h2>
          <ul>
            {#each preview.project.diagnostics as d, i (i)}
              <li class={d.severity}>
                <code>{location(d)}</code>
                <span>{d.message}</span>
              </li>
            {:else}
              <li class="ok">No problems</li>
            {/each}
          </ul>
        </section>
      {/if}
    </aside>

    <main>
      {#if !preview}
        <div class="placeholder">
          <p>Open a project, or create a new one, with the buttons on the Project tab.</p>
        </div>
      {:else if tab === "design" && openApp && appPage && file}
        <PageDesigner {preview} page={appPage} app={openApp} {file} onapplied={applyPreview} />
      {:else if tab === "design" && page && file}
        <PageDesigner {preview} {page} {file} onapplied={applyPreview} />
      {:else if page}
        <div class="window">
          <div class="titlebar">
            <span>{page.title ?? page.name.name}</span>
            <code>{page.name.name}: {page.kind}</code>
          </div>
          <FrameView
            {preview}
            {page}
            frame={frameForPage(preview, page)}
            data={{ ...appData, ...pageData(preview, page) }}
            design={false}
            onnavigate={navigate}
          >
            <div class="client">
              <PageView {preview} {page} onnavigate={navigate} />
            </div>
          </FrameView>
        </div>
      {:else if openApp}
        <div class="placeholder"><p>The app is designed on the Design tab; Preview runs its main window.</p></div>
      {:else if current}
        <div class="placeholder">
          <p>Window <code>{current.name}</code> cannot be shown. See the problems list.</p>
        </div>
      {:else}
        <div class="placeholder"><p>This project has no windows yet. Add one with New window.</p></div>
      {/if}
    </main>
  </div>
</div>

{#if newWindowIn && preview}
  <NewWindowDialog
    {preview}
    file={newWindowIn}
    error={newWindowError}
    onsubmit={addWindow}
    oncancel={() => (newWindowIn = null)}
  />
{/if}

<style>
  .app {
    display: grid;
    grid-template-rows: auto minmax(0, 1fr);
    height: 100%;
  }

  .shell {
    display: grid;
    grid-template-columns: 300px minmax(0, 1fr);
    min-height: 0;
  }

  /* Ribbon */

  .ribbon {
    background: var(--panel);
    border-bottom: 1px solid var(--border-strong);
  }

  .ribbon-tabs {
    display: flex;
    align-items: stretch;
    gap: 2px;
    background: var(--titlebar);
    color: var(--titlebar-text);
    padding: 0 10px 0 0;
  }

  .ribbon-tabs button {
    border: none;
    border-radius: var(--radius) var(--radius) 0 0;
    background: transparent;
    color: var(--titlebar-text);
    opacity: 0.75;
    padding: 8px 16px 6px;
    margin-top: 6px;
  }

  .ribbon-tabs button.active {
    background: var(--panel);
    color: var(--text);
    opacity: 1;
  }

  .project-name {
    margin-left: auto;
    align-self: center;
    font-size: 12px;
    opacity: 0.8;
  }

  .ribbon-body {
    display: flex;
    align-items: stretch;
    gap: 4px;
    padding: 6px 10px 4px;
    min-height: 76px;
  }

  .group {
    display: flex;
    flex-direction: column;
    padding: 0 8px;
    border-right: 1px solid var(--border);
  }

  .group .buttons {
    display: flex;
    gap: 2px;
    flex: 1;
  }

  .group-name {
    text-align: center;
    font-size: 10px;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    padding-top: 2px;
  }

  .ribbon-error {
    margin: 0 0 0 auto;
    align-self: center;
    max-width: 420px;
    color: var(--error);
    font-size: 12px;
    display: flex;
    gap: 8px;
    align-items: center;
  }

  /* Sidebar sections */

  .side-section {
    padding: 10px 14px;
    border-bottom: 1px solid var(--border);
    overflow: auto;
  }

  .side-section h2 {
    margin: 0 0 6px;
    font-size: 12px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-muted);
  }

  .side-section dl {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    gap: 3px 10px;
    margin: 0;
    font-size: 12px;
  }

  .side-section dt {
    color: var(--text-muted);
  }

  .side-section dd {
    margin: 0;
  }

  .side-section dd.path {
    font-family: var(--mono);
    font-size: 11px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .side-section nav {
    padding: 0;
  }

  .entity summary {
    cursor: default;
    font-size: 13px;
    padding: 2px 0;
  }

  .entity ul {
    margin: 2px 0 6px;
    padding-left: 18px;
    font-size: 12px;
  }

  .entity code {
    font-family: var(--mono);
    font-size: 11px;
    color: var(--text-muted);
  }

  .check {
    display: flex;
    gap: 6px;
    align-items: center;
    font-size: 13px;
    margin-bottom: 8px;
  }

  aside {
    background: var(--panel);
    border-right: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    min-height: 0;
    overflow: hidden;
  }

  .side-tabs {
    display: flex;
    border-bottom: 1px solid var(--border);
    padding: 6px 8px 0;
    gap: 2px;
  }

  .side-tabs button {
    border: none;
    border-radius: var(--radius) var(--radius) 0 0;
    background: transparent;
    color: var(--text-muted);
    padding: 5px 12px;
  }

  .side-tabs button.active {
    background: var(--accent-soft);
    color: var(--text);
  }

  .design-sidebar.hidden {
    display: none;
  }

  .design-sidebar {
    flex: 1;
    overflow: auto;
    padding: 10px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 12px 14px;
    font-weight: 700;
    background: var(--titlebar);
    color: var(--titlebar-text);
  }


  .logo {
    display: inline-grid;
    place-items: center;
    width: 22px;
    height: 22px;
    border-radius: 4px;
    background: var(--brand);
    color: #20242c;
    font-weight: 800;
  }





  .load-error {
    color: var(--error);
    margin: 4px 0 0;
    font-size: 12px;
  }


  nav {
    flex: 1;
    overflow: auto;
    padding: 8px 0;
  }

  .module {
    padding: 4px 8px 8px;
  }

  .module-name {
    font-weight: 600;
    padding: 4px 6px;
  }

  .file {
    font-weight: 400;
    color: var(--text-muted);
    font-family: var(--mono);
    font-size: 11px;
  }

  .screen-link {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    text-align: left;
    border: none;
    background: transparent;
    padding: 5px 6px;
  }

  .screen-link:hover {
    background: var(--panel-alt);
  }

  .screen-link.current {
    background: var(--accent-soft);
  }

  .pattern {
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    border-radius: 3px;
    padding: 1px 5px;
    min-width: 52px;
    flex: none;
    text-align: center;
    background: var(--panel-alt);
    border: 1px solid var(--border);
    color: var(--text-muted);
  }

  .pattern.app {
    color: var(--brand);
    border-color: var(--brand);
  }

  .pattern.page {
    color: var(--accent);
    border-color: var(--accent);
  }

  .add-page {
    border: none;
    background: transparent;
    color: var(--accent);
    font-size: 12px;
    padding: 4px 6px;
  }





  .none {
    color: var(--text-muted);
    padding: 2px 6px;
    font-size: 12px;
  }

  .diagnostics {
    border-top: 1px solid var(--border);
    margin-top: auto;
    max-height: 40%;
    display: flex;
    flex-direction: column;
    min-height: 0;
  }

  .diagnostics h2 {
    font-size: 12px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    margin: 0;
    padding: 10px 14px 6px;
    display: flex;
    gap: 6px;
    align-items: center;
    color: var(--text-muted);
  }

  .count {
    border-radius: 8px;
    padding: 0 6px;
    font-size: 11px;
  }

  .count.error {
    background: var(--error-soft);
    color: var(--error);
  }

  .count.warning {
    background: var(--warning-soft);
    color: var(--warning);
  }

  .diagnostics ul {
    list-style: none;
    margin: 0;
    padding: 0 8px 10px;
    overflow: auto;
  }

  .diagnostics li {
    padding: 5px 6px;
    border-radius: 4px;
    display: flex;
    flex-direction: column;
    font-size: 12px;
  }

  .diagnostics li.error {
    background: var(--error-soft);
    margin-bottom: 4px;
  }

  .diagnostics li.warning {
    background: var(--warning-soft);
    margin-bottom: 4px;
  }

  .diagnostics li.ok {
    color: var(--text-muted);
  }

  .diagnostics code {
    font-family: var(--mono);
    font-size: 11px;
    color: var(--text-muted);
  }

  main {
    padding: 24px;
    overflow: auto;
    display: flex;
    flex-direction: column;
    gap: 12px;
    min-width: 0;
  }

  .placeholder {
    margin: auto;
    text-align: center;
    color: var(--text-muted);
  }






  .breadcrumb {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .breadcrumb button {
    border: none;
    background: transparent;
    color: var(--accent);
    padding: 2px 4px;
  }

  .breadcrumb button:disabled {
    color: var(--text);
    opacity: 1;
  }

  .sep {
    color: var(--text-muted);
  }

  .window {
    background: var(--panel);
    border: 1px solid var(--border-strong);
    border-radius: 8px;
    box-shadow: 0 8px 24px rgb(0 0 0 / 0.08);
    overflow: hidden;
    max-width: 1100px;
    width: 100%;
    /* Full height; the main area scrolls. */
    flex: none;
  }

  .titlebar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
    background: var(--titlebar);
    color: var(--titlebar-text);
    padding: 8px 14px;
    font-weight: 600;
  }

  .titlebar code {
    font-family: var(--mono);
    font-size: 11px;
    font-weight: 400;
    opacity: 0.7;
  }

  .client {
    padding: 16px;
  }
</style>
