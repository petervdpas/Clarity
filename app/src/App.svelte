<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { ask, open as openDialog, save as saveDialog } from "@tauri-apps/plugin-dialog";
  import { onMount } from "svelte";
  import FrameView from "./lib/FrameView.svelte";
  import NewWindowDialog from "./lib/NewWindowDialog.svelte";
  import PageDesigner from "./lib/PageDesigner.svelte";
  import PageView from "./lib/PageView.svelte";
  import ProjectTree from "./lib/ProjectTree.svelte";
  import RibbonButton from "./lib/RibbonButton.svelte";
  import Splitter from "./lib/Splitter.svelte";
  import { layout } from "./lib/layout.svelte";
  import EntityEditor from "./lib/EntityEditor.svelte";
  import EnumEditor from "./lib/EnumEditor.svelte";
  import ModuleView from "./lib/ModuleView.svelte";
  import ThemeView from "./lib/ThemeView.svelte";
  import NameDialog from "./lib/NameDialog.svelte";
  import NewProjectDialog from "./lib/NewProjectDialog.svelte";
  import ProjectSettings from "./lib/ProjectSettings.svelte";
  import type { Catalog } from "./lib/generated/Catalog";
  import type { ProjectFile } from "./lib/generated/ProjectFile";
  import type { Settings } from "./lib/generated/Settings";
  import type { Stack } from "./lib/generated/Stack";
  import type { BuildDone } from "./lib/generated/BuildDone";
  import type { Selected } from "./lib/selection";
  import type { Edit } from "./lib/generated/Edit";
  import type { RenameKind } from "./lib/generated/RenameKind";
  import { applyEdit } from "./lib/applyEdit";
  import type { Diagnostic } from "./lib/generated/Diagnostic";
  import type { HistoryCounts } from "./lib/generated/HistoryCounts";
  import type { Page } from "./lib/generated/Page";
  import type { PageTemplate } from "./lib/generated/PageTemplate";
  import type { Preview } from "./lib/generated/Preview";
  import type { SpecFile } from "./lib/generated/SpecFile";
  import { pageDesigner } from "./lib/pageDesigner.svelte";
  import { findApp, frameForPage } from "./lib/frame";
  import { appAsPage, findPage, pageData } from "./lib/pages";
  import { themeArea, themeOf } from "./lib/model";
  import ThemeStyles from "./lib/ThemeStyles.svelte";

  const LAST_PROJECT_KEY = "clarity.lastProject";

  /** What is open. Following a button in the preview pushes a window. */
  type Open = Selected;

  /** The ribbon tabs. Design and Preview are also the two modes. */
  type Tab = "project" | "design" | "preview";
  const TABS: { key: Tab; name: string }[] = [
    { key: "project", name: "Project" },
    { key: "design", name: "Design" },
    { key: "preview", name: "Preview" },
  ];

  /** What the sidebar shows while designing. */
  type DesignSide = "elements" | "parts" | "outline" | "settings";
  const DESIGN_SIDES: { key: DesignSide; name: string }[] = [
    { key: "elements", name: "Elements" },
    { key: "parts", name: "Parts" },
    { key: "outline", name: "Window" },
    { key: "settings", name: "Settings" },
  ];

  let openFile = $state<string | null>(null);
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
  /** The project file, which the Project tab shows and edits. */
  let projectFile = $state<ProjectFile | null>(null);
  /** The folder and the choices, while the New project dialog is open. */
  let newProjectIn = $state<{ folder: string; catalog: Catalog } | null>(null);
  let newProjectError = $state<string | null>(null);
  /** Bumped when a settings change is refused, so the editor shows the file again. */
  let settingsRevision = $state(0);

  /** How many lines of toolchain output the output panel keeps. */
  const BUILD_LOG_LINES = 400;
  let building = $state(false);
  let running = $state(false);
  let buildLog = $state<string[]>([]);
  let buildDone = $state<BuildDone | null>(null);
  let showOutput = $state(false);
  let outputPre: HTMLElement | undefined = $state();

  // The output follows the newest line.
  $effect(() => {
    if (buildLog.length > 0 && outputPre) outputPre.scrollTop = outputPre.scrollHeight;
  });

  /** Generates and builds the application; with `run`, starts it. */
  async function buildApp(release: boolean, run: boolean) {
    actionError = null;
    buildLog = [];
    buildDone = null;
    showOutput = true;
    building = true;
    try {
      await invoke("build_app", { release, run });
    } catch (err) {
      building = false;
      buildDone = { ok: false, message: String(err), binary: null };
    }
  }

  async function stopApp() {
    await invoke("stop_app");
    running = false;
  }

  const current = $derived(stack.at(-1) ?? null);
  const app = $derived(preview ? findApp(preview) : null);
  /** The app, when it is the open element (it is designed like a window frame). */
  const openApp = $derived(current?.kind === "app" ? app : null);
  const page = $derived(preview && current?.kind === "window" ? findPage(preview, current.name) : undefined);
  const files = $derived(preview?.project.files ?? []);
  const entity = $derived(current?.kind === "entity" ? files.flatMap((f) => f.entities).find((e) => e.name.name === current.name) : undefined);
  const enumDecl = $derived(current?.kind === "enum" ? files.flatMap((f) => f.enums).find((e) => e.name.name === current.name) : undefined);
  /** The open theme and the module that declares it; no module for Clarity's own. */
  const openTheme = $derived.by(() => {
    if (!preview || current?.kind !== "theme") return null;
    if (current.name === preview.baseTheme.name.name) return { theme: preview.baseTheme, file: null };
    const declaring = files.find((f) => f.themes.some((t) => t.name.name === current.name));
    const theme = declaring?.themes.find((t) => t.name.name === current.name);
    return declaring && theme ? { theme, file: declaring } : null;
  });
  const moduleFile = $derived(current?.kind === "module" ? files.find((f) => (f.module?.name ?? "") === current.name) : undefined);
  const appPage = $derived(app ? appAsPage(app) : null);
  const file = $derived(
    preview?.project.files.find(
      (f) =>
        (page && f.pages.some((p) => p.name.name === page.name.name)) ||
        (openApp && f.apps.some((a) => a.name.name === openApp.name.name)) ||
        (entity && f.entities.some((e) => e.name.name === entity.name.name)) ||
        (enumDecl && f.enums.some((e) => e.name.name === enumDecl.name.name)) ||
        f === moduleFile,
    ),
  );
  const designing = $derived(tab === "design" && (page !== undefined || openApp !== null));
  // Widgets in the app's sidebars and footer see the app's data next to the window's.
  const appData = $derived(preview && appPage ? pageData(preview, appPage) : {});
  const errors = $derived(preview?.project.diagnostics.filter((d) => d.severity === "error") ?? []);
  const warnings = $derived(preview?.project.diagnostics.filter((d) => d.severity === "warning") ?? []);
  const projectProblems = $derived(projectFile?.problems ?? []);
  const errorCount = $derived(errors.length + projectProblems.length);
  const stackSummary = $derived.by(() => {
    const settings = projectFile?.settings;
    if (!projectFile || !settings) return "(the project file cannot be read)";
    const { catalog } = projectFile;
    const backend = catalog.backends.find((b) => b.id === settings.stack.backend)?.label;
    const ui = catalog.uis.find((u) => u.id === settings.stack.ui)?.label;
    const targets = catalog.targets.filter((t) => settings.stack.targets.includes(t.id)).map((t) => t.label);
    return `${backend}, ${ui}, ${targets.join(" and ")}`;
  });
  const projectName = $derived(app?.title ?? app?.name.name ?? (openFile ? fileName(openFile) : ""));

  // Entering design mode shows the parts.
  $effect.pre(() => {
    if (designing) designSide = "parts";
  });

  function readLastProject(): string | null {
    try {
      return localStorage.getItem(LAST_PROJECT_KEY);
    } catch {
      return null;
    }
  }

  function writeLastProject(dir: string) {
    try {
      localStorage.setItem(LAST_PROJECT_KEY, dir);
    } catch {
      // Remembering the project is a convenience only.
    }
  }

  /** The window the project starts with: the app's main window, or the first one. */
  function startWindow(p: Preview): Open[] {
    const main = findApp(p)?.main?.name;
    const first = main ?? p.project.files.flatMap((f) => f.pages)[0]?.name.name;
    return first ? [{ kind: "window", name: first }] : [];
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
    const kept = stack.filter((o) => exists(next, o));
    stack = kept.length > 0 ? kept : startWindow(next);
    refreshHistory();
  }

  /** Whether a selection still exists in a reloaded project. */
  function exists(p: Preview, o: Open): boolean {
    const all = p.project.files;
    switch (o.kind) {
      case "app":
        return findApp(p) !== null;
      case "window":
        return findPage(p, o.name) !== undefined;
      case "entity":
        return all.some((f) => f.entities.some((e) => e.name.name === o.name));
      case "enum":
        return all.some((f) => f.enums.some((e) => e.name.name === o.name));
      case "theme":
        return o.name === p.baseTheme.name.name || all.some((f) => f.themes.some((t) => t.name.name === o.name));
      case "module":
        return all.some((f) => (f.module?.name ?? "") === o.name);
      case "settings":
        return true;
    }
  }

  async function loadProjectFile() {
    try {
      projectFile = await invoke<ProjectFile>("project_file");
    } catch {
      projectFile = null;
    }
  }

  async function saveSettings(settings: Settings) {
    if (!projectFile) return;
    actionError = null;
    try {
      projectFile = await invoke<ProjectFile>("save_settings", { version: projectFile.version, settings });
    } catch (err) {
      actionError = String(err);
      // Show the file as it is, not the refused change.
      settingsRevision++;
      await loadProjectFile();
    }
    refreshHistory();
  }

  function selectSettings() {
    select({ kind: "settings", name: "" });
    tab = "project";
  }

  function select(o: Open) {
    stack = [o];
    pageDesigner.selected = null;
  }

  /** Applies an edit from the data or module editors. */
  async function editData(edit: Edit) {
    if (!file) return;
    actionError = null;
    try {
      applyPreview((await applyEdit(file, edit)).preview);
    } catch (err) {
      actionError = String(err);
    }
  }

  /** Renames an entity or enum and every reference to it, in all files. */
  async function renameData(kind: RenameKind, name: string, newName: string) {
    actionError = null;
    try {
      applyPreview(await invoke<Preview>("rename", { kind, name, newName }));
      select({ kind, name: newName });
    } catch (err) {
      actionError = String(err);
    }
  }

  /** The file a new entity or enum goes into, while its dialog is open. */
  let newData = $state<{ kind: "entity" | "enum"; file: SpecFile } | null>(null);
  let newDataError = $state<string | null>(null);

  function startNewData(kind: "entity" | "enum") {
    const target = file ?? files.find((f) => f.entities.length > 0) ?? files[0];
    if (target) {
      newDataError = null;
      newData = { kind, file: target };
    }
  }

  /** Adds a theme with a free name, a full copy of Clarity's own, and opens it.
      It goes into the module that already declares themes, or the first one. */
  async function addTheme() {
    if (!preview) return;
    const target = files.find((f) => f.themes.length > 0) ?? files[0];
    if (!target) return;
    const taken = new Set(files.flatMap((f) => f.themes.map((t) => t.name.name)));
    let name = "Theme";
    for (let n = 2; taken.has(name); n++) name = "Theme" + n;
    actionError = null;
    try {
      const text = await invoke<string>("theme_starter", { name });
      applyPreview((await applyEdit(target, { op: "appendDecl", text })).preview);
      select({ kind: "theme", name });
    } catch (err) {
      actionError = String(err);
    }
  }

  async function addData(target: SpecFile, name: string) {
    if (!newData) return;
    const kind = newData.kind;
    try {
      const result = await applyEdit(target, kind === "entity" ? { op: "addEntity", name } : { op: "addEnum", name });
      applyPreview(result.preview);
      newData = null;
      select({ kind, name });
    } catch (err) {
      newDataError = String(err);
    }
  }

  /** Project files are .clarity files, like the modules in their folder. */
  const PROJECT_FILTER = [{ name: "Clarity project", extensions: ["clarity"] }];

  async function openProject(path: string) {
    const trimmed = path.trim();
    if (!trimmed) return;
    try {
      const result = await invoke<Preview>("open_project", { path: trimmed });
      loadError = null;
      openFile = trimmed;
      writeLastProject(trimmed);
      stack = [];
      applyPreview(result);
      await loadProjectFile();
    } catch (err) {
      loadError = String(err);
    }
  }

  /** When Check last ran, shown in the problems list; flashing while new. */
  let checkedAt = $state<string | null>(null);
  let checkFlash = $state(false);

  /** Reads every file again and checks the project, keeping what is open
      and the undo history. The designer checks after every edit and when
      a file changes on disk; Check is for being sure. */
  async function checkProject() {
    try {
      applyPreview(await invoke<Preview>("check_project"));
      await loadProjectFile();
      loadError = null;
      checkedAt = new Date().toLocaleTimeString();
      checkFlash = false;
      requestAnimationFrame(() => (checkFlash = true));
    } catch (err) {
      loadError = String(err);
    }
  }

  async function chooseProject() {
    const file = await openDialog({ title: "Open a Clarity project", filters: PROJECT_FILTER });
    if (typeof file === "string") openProject(file);
  }

  async function newProject() {
    const chosen = await saveDialog({
      title: "New project: its file, with a folder of the same name next to it",
      defaultPath: "project.clarity",
      filters: PROJECT_FILTER,
    });
    if (typeof chosen !== "string") return;
    const file = chosen.endsWith(".clarity") ? chosen : chosen + ".clarity";
    try {
      newProjectError = null;
      newProjectIn = { folder: file, catalog: await invoke<Catalog>("stack_catalog") };
    } catch (err) {
      loadError = String(err);
    }
  }

  async function createProject(file: string, chosen: Stack) {
    try {
      const result = await invoke<Preview>("new_project", { path: file, stack: chosen });
      newProjectIn = null;
      loadError = null;
      openFile = file;
      writeLastProject(file);
      stack = [];
      applyPreview(result);
      await loadProjectFile();
      tab = "design";
    } catch (err) {
      newProjectError = String(err);
    }
  }

  function selectApp() {
    if (app) stack = [{ kind: "app", name: app.name.name }];
    pageDesigner.selected = null;
    tab = "design";
  }

  function selectPage(p: Page) {
    stack = [{ kind: "window", name: p.name.name }];
    pageDesigner.selected = null;
  }

  /** Follows a button or menu item: opens the target window on top of the current one. */
  function navigate(target: string) {
    if (!preview) return;
    if (target === "close") {
      if (stack.length > 1) stack = stack.slice(0, -1);
      return;
    }
    if (findPage(preview, target)) stack = [...stack, { kind: "window", name: target }];
  }

  async function addWindow(target: SpecFile, name: string, template: PageTemplate) {
    try {
      const result = await applyEdit(target, { op: "addWindow", name, template });
      applyPreview(result.preview);
      newWindowIn = null;
      newWindowError = null;
      stack = [{ kind: "window", name }];
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
    // A change can be to the project file too.
    await loadProjectFile();
  }

  /** Deletes the picked part, or the open window when nothing is picked. */
  async function deletePicked() {
    if (!file) return;
    actionError = null;
    let item = pageDesigner.selected;
    if (item === null) {
      const target = openApp ?? page ?? entity ?? enumDecl;
      if (!target) return;
      const noun = openApp ? "the app" : page ? "the window" : entity ? "the entity" : "the enum";
      if (!(await ask(`Delete ${noun} ${target.name.name}?`, { title: "Clarity", kind: "warning" }))) return;
      item = target.span.start;
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
    const unlistenProject = listen<ProjectFile>("project-updated", (e) => (projectFile = e.payload));
    const unlistenLine = listen<string>("build-line", (e) => {
      buildLog = [...buildLog.slice(-(BUILD_LOG_LINES - 1)), e.payload];
    });
    const unlistenDone = listen<BuildDone>("build-done", (e) => {
      building = false;
      buildDone = e.payload;
      running = e.payload.ok && e.payload.message === "running";
    });
    invoke<string | null>("initial_project").then((dir) => {
      const start = dir ?? readLastProject();
      if (start) openProject(start);
    });
    return () => {
      unlisten.then((stop) => stop());
      unlistenProject.then((stop) => stop());
      unlistenLine.then((stop) => stop());
      unlistenDone.then((stop) => stop());
    };
  });
</script>

<svelte:window {onkeydown} />

{#if preview}<ThemeStyles {preview} />{/if}

{#snippet tree(full: boolean)}
  {#if preview}
    <ProjectTree
      {preview}
      selected={current}
      {full}
      onapp={selectApp}
      onwindow={selectPage}
      onselect={select}
      onnewdata={full ? startNewData : null}
      onnewtheme={full ? addTheme : null}
      onnewwindow={full ? startNewWindow : null}
      onnewapp={full && preview.project.files[0] ? () => preview && addApp(preview.project.files[0]) : null}
    />
  {/if}
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
            <RibbonButton icon="new" label="New" title="Create a project: a .clarity file with its folder" onclick={newProject} />
            <RibbonButton icon="open" label="Open" title="Open a project file (.clarity)" onclick={chooseProject} />
            <RibbonButton icon="reload" label="Reload" disabled={!openFile} onclick={() => openFile && openProject(openFile)} />
            <RibbonButton icon="settings" label="Settings" title="Stack and data sources" disabled={!preview} active={current?.kind === "settings"} onclick={selectSettings} />
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
            <RibbonButton icon="check" label={errorCount === 0 ? "No errors" : errorCount === 1 ? "1 error" : `${errorCount} errors`} title="Read every file again and check the whole project" disabled={!preview} onclick={checkProject} />
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
            <RibbonButton
              icon="text"
              label="Text"
              title="Show this window as Clarity text; the designer follows what you type"
              disabled={!designing}
              active={pageDesigner.code.open && pageDesigner.code.view === "design"}
              onclick={() => pageDesigner.togglePanel("design")}
            />
            <RibbonButton
              icon="code"
              label="Code"
              title="Show the code behind of this window"
              disabled={!designing}
              active={pageDesigner.code.open && pageDesigner.code.view === "code"}
              onclick={() => pageDesigner.togglePanel("code")}
            />
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
            <RibbonButton icon="window" label="Main window" disabled={!preview} onclick={() => preview && (stack = startWindow(preview))} />
            <RibbonButton icon="back" label="Back" disabled={stack.length < 2} onclick={() => (stack = stack.slice(0, -1))} />
          </div>
          <div class="group-name">Preview</div>
        </div>
        <div class="group">
          <div class="buttons">
            <RibbonButton
              icon="play"
              label={building ? "Building..." : "Run"}
              title="Generate the application, build it with its toolchain and start it"
              disabled={!preview || building || errorCount > 0}
              onclick={() => buildApp(false, true)}
            />
            <RibbonButton
              icon="build"
              label="Build"
              title="Build a release program of the application"
              disabled={!preview || building || errorCount > 0}
              onclick={() => buildApp(true, false)}
            />
            <RibbonButton icon="stop" label="Stop" title="Close the running application" disabled={!running} onclick={stopApp} />
          </div>
          <div class="group-name">Application</div>
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

  <div class="shell" style:grid-template-columns="{layout.sizes.sidebar}px auto minmax(0, 1fr)">
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
              <dt>File</dt>
              <dd class="path" title={openFile}>{openFile}</dd>
              <dt>Main window</dt>
              <dd>{app?.main?.name ?? "(none)"}</dd>
              <dt>Stack</dt>
              <dd><button class="link" title="Open the project settings" onclick={selectSettings}>{stackSummary}</button></dd>
              <dt>Data</dt>
              <dd>
                {#if projectFile?.settings}
                  {projectFile.settings.datasources.map((s) => s.name).join(", ") || "(none)"}
                {:else}
                  -
                {/if}
              </dd>
            </dl>
          {:else}
            <p class="none">No project is open. Use New or Open in the ribbon.</p>
          {/if}
        </section>
        {#if preview}
          <section class="side-section fill">
            {@render tree(true)}
          </section>
        {/if}
      {:else if tab === "design" && (!designing || designSide === "elements")}
        <section class="side-section fill">
          {@render tree(true)}
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
        <section class="side-section fill">
          <h2>Run a window</h2>
          {@render tree(false)}
        </section>
      {/if}

      {#if preview}
        <section class="diagnostics" class:flash={checkFlash} onanimationend={() => (checkFlash = false)}>
          <h2>
            Problems
            <span class="count error">{errorCount}</span>
            <span class="count warning">{warnings.length}</span>
            {#if checkedAt}<span class="checked">checked the project and {preview.project.files.length} modules at {checkedAt}</span>{/if}
          </h2>
          <ul>
            {#each projectProblems as message, i (i)}
              <li class="error">
                <button class="link" onclick={selectSettings}><code>{openFile ? fileName(openFile) : "project"}</code></button>
                <span>{message}</span>
              </li>
            {/each}
            {#each preview.project.diagnostics as d, i (i)}
              <li class={d.severity}>
                <code>{location(d)}</code>
                <span>{d.message}</span>
              </li>
            {:else}
              {#if projectProblems.length === 0}<li class="ok">No problems</li>{/if}
            {/each}
          </ul>
        </section>
      {/if}
    </aside>
    <Splitter axis="x" label="Width of the sidebar" onmove={(d) => layout.resize("sidebar", d)} />

    <div class="work">
      <main>
        {#if !preview}
          <div class="placeholder">
            <p>Open a project, or create a new one, with the buttons on the Project tab.</p>
          </div>
        {:else if current?.kind === "settings" && projectFile && openFile}
          <ProjectSettings project={projectFile} file={openFile} revision={settingsRevision} onsave={saveSettings} {preview} onapplied={applyPreview} onopentheme={(name) => select({ kind: "theme", name })} />
        {:else if entity && file}
          <EntityEditor
            {preview}
            {entity}
            module={file.module?.name ?? ""}
            onedit={editData}
            ondelete={deletePicked}
            onrename={(newName) => renameData("entity", entity.name.name, newName)}
          />
        {:else if enumDecl && file}
          <EnumEditor
            decl={enumDecl}
            module={file.module?.name ?? ""}
            onedit={editData}
            ondelete={deletePicked}
            onrename={(newName) => renameData("enum", enumDecl.name.name, newName)}
          />
        {:else if openTheme}
          <ThemeView
            {preview}
            theme={openTheme.theme}
            file={openTheme.file}
            onapply={async (edit) => {
              if (openTheme.file) applyPreview((await applyEdit(openTheme.file, edit)).preview);
            }}
            onopensettings={selectSettings}
          />
        {:else if moduleFile}
          <ModuleView
            {preview}
            file={moduleFile}
            onedit={editData}
            onapply={async (edit) => applyPreview((await applyEdit(moduleFile, edit)).preview)}
            onopen={(kind, name) => select({ kind, name })}
          />
        {:else if tab === "design" && openApp && appPage && file}
          <PageDesigner {preview} page={appPage} app={openApp} {file} onapplied={applyPreview} />
        {:else if tab === "design" && page && file}
          <PageDesigner {preview} {page} {file} onapplied={applyPreview} />
        {:else if page}
          <div class="window {themeArea(themeOf(preview)).class}" data-scheme={themeArea(themeOf(preview)).scheme}>
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
      {#if showOutput}
        <Splitter axis="y" label="Height of the output" onmove={(d) => layout.resize("output", -d)} />
        <!-- The toolchain's output while the application is generated and built. -->
        <section class="output-panel" aria-label="Build output" style:height="{layout.sizes.output}px">
          <header>
            <strong>Output</strong>
            {#if building}
              <span class="status">Building...</span>
            {:else if buildDone}
              <span class="status" class:failed={!buildDone.ok}>{buildDone.ok ? (buildDone.message === "running" ? "Running" : buildDone.message) : "Failed: " + buildDone.message}</span>
            {/if}
            <button onclick={() => (showOutput = false)} title="Hide the output">&times;</button>
          </header>
          <pre bind:this={outputPre}>{buildLog.join("\n")}</pre>
        </section>
      {/if}
      <!-- The code behind panel slides out over the main area. -->
      <div id="code-drawer" class="code-drawer"></div>
    </div>
  </div>
</div>

{#if newData && preview}
  <NameDialog
    {preview}
    title={newData.kind === "entity" ? "New entity" : "New enum"}
    placeholder={newData.kind === "entity" ? "Invoice" : "Priority"}
    action={newData.kind === "entity" ? "Add entity" : "Add enum"}
    file={newData.file}
    error={newDataError}
    onsubmit={addData}
    oncancel={() => (newData = null)}
  />
{/if}

{#if newProjectIn}
  <NewProjectDialog
    catalog={newProjectIn.catalog}
    folder={newProjectIn.folder}
    error={newProjectError}
    onsubmit={(chosen) => newProjectIn && createProject(newProjectIn.folder, chosen)}
    oncancel={() => (newProjectIn = null)}
  />
{/if}

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
    grid-template-rows: minmax(0, 1fr);
    min-height: 0;
  }

  /* Everything right of the sidebar: the main area, the docked output below
     it, and the code panel that slides out over it. */
  .work {
    position: relative;
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
  }

  .output-panel {
    flex: none;
    display: flex;
    flex-direction: column;
    background: var(--panel);
    border-top: 1px solid var(--border-strong);
  }

  .output-panel header {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 6px 12px;
    border-bottom: 1px solid var(--border);
  }

  .output-panel .status {
    flex: 1;
    font-size: 12px;
    color: var(--text-muted);
  }

  .output-panel .status.failed {
    color: var(--error);
  }

  .output-panel pre {
    flex: 1;
    margin: 0;
    padding: 8px 12px;
    overflow: auto;
    font-family: var(--mono);
    font-size: 12px;
    white-space: pre-wrap;
  }

  .code-drawer {
    position: absolute;
    inset: 0;
    pointer-events: none;
    z-index: 20;
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
    flex: none;
    padding: 10px 14px;
    border-bottom: 1px solid var(--border);
    overflow: auto;
  }

  /* The tree takes the room the other sections leave, and scrolls in it. */
  .side-section.fill {
    flex: 1 1 0;
    min-height: 0;
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

  .diagnostics.flash {
    animation: checked 1.2s ease-out;
  }

  @keyframes checked {
    from {
      background: var(--accent-soft);
    }
  }

  .checked {
    margin-left: auto;
    font-weight: 400;
    text-transform: none;
    letter-spacing: 0;
    color: var(--text-muted);
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

  button.link {
    border: none;
    background: none;
    padding: 0;
    color: inherit;
    font: inherit;
    text-align: left;
    cursor: pointer;
    text-decoration: underline dotted;
  }

  main {
    flex: 1;
    min-height: 0;
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

  /* The preview window fills the main area and resizes with it, like the
     running application does. */
  .window {
    background: var(--panel);
    border: 1px solid var(--border-strong);
    border-radius: 8px;
    box-shadow: 0 8px 24px rgb(0 0 0 / 0.08);
    overflow: hidden;
    width: 100%;
    flex: 1;
    min-height: 360px;
    display: flex;
    flex-direction: column;
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
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    overflow: auto;
  }
</style>
