<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { ask, open as openDialog, save as saveDialog } from "@tauri-apps/plugin-dialog";
  import { onMount, tick, untrack } from "svelte";
  // The installers' icon, from the same drawing.
  import logo from "../src-tauri/icons/icon.svg";
  import { loadPicturesWith } from "./lib/pictures.svelte";
  import { topicForTab } from "./lib/manual";
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
  import EngineEditor from "./lib/EngineEditor.svelte";
  import TranslationsPanel from "./lib/TranslationsPanel.svelte";
  import NewLanguageDialog from "./lib/NewLanguageDialog.svelte";
  import { languageChoice } from "./lib/i18n.svelte";
  import { languageName } from "./lib/translate";
  import { fixOf } from "./lib/codeParts";
  import { engineProblems, recordBuilt } from "./lib/engineBuilds.svelte";
  import type { EngineBuilt } from "./lib/generated/EngineBuilt";
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
  import { pageDesigner } from "./lib/pageDesignerStore.svelte";
  import { findApp, frameForPage } from "./lib/frame";
  import { CONTENT_PADDING, appAsPage, findPage, pageData, windowSize } from "./lib/pages";
  import ShownWindow from "./lib/ShownWindow.svelte";
  import VaultSide from "./lib/VaultSide.svelte";
  import VaultView from "./lib/VaultView.svelte";
  import { vault } from "./lib/vaultStore.svelte";
  import AgentPanel from "./lib/AgentPanel.svelte";
  import UnlockDialog from "./lib/UnlockDialog.svelte";
  import { agent } from "./lib/agentStore.svelte";
  import AppSettingsView from "./lib/AppSettingsView.svelte";
  import ContextMenu from "./lib/ContextMenu.svelte";
  import type { CodeView } from "./lib/generated/CodeView";
  import { themeArea, themeOf, tr } from "./lib/model";
  import ThemeStyles from "./lib/ThemeStyles.svelte";

  const LAST_PROJECT_KEY = "clarity.lastProject";

  /** What is open. Following a button in the preview pushes a window. */
  type Open = Selected;

  /** The ribbon tabs. Design and Preview are also the two modes. */
  type Tab = "project" | "design" | "preview" | "vault";
  const TABS: { key: Tab; name: string }[] = [
    { key: "project", name: "Project" },
    { key: "design", name: "Design" },
    { key: "preview", name: "Preview" },
    { key: "vault", name: "Vault" },
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
  // Clarity's vault is read again whenever its tab opens: it may have locked itself.
  $effect(() => {
    if (tab === "vault") vault.refresh();
  });
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
  /** The open engine and the module that declares it. */
  const openEngine = $derived.by(() => {
    if (current?.kind !== "engine") return null;
    const declaring = files.find((f) => f.engines.some((e) => e.name.name === current.name));
    const decl = declaring?.engines.find((e) => e.name.name === current.name);
    return declaring && decl ? { decl, file: declaring } : null;
  });
  /** A line of an engine's code to show, from the Problems list. */
  let engineGoTo = $state<{ line: number; n: number } | null>(null);
  const builtProblems = $derived(engineProblems());
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
  const projectName = $derived(app?.title && preview ? tr(preview, app.title) : (app?.name.name ?? (openFile ? fileName(openFile) : "")));

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

  /** The pictures the designer has loaded, by the names of the images folder. */
  let pictureNames: string | null = null;

  function applyPreview(next: Preview) {
    // Pictures load again when the images folder changes.
    const names = JSON.stringify(next.project.images?.map((i) => i.name) ?? []);
    if (names !== pictureNames) {
      pictureNames = names;
      loadPicturesWith((name) => invoke<string>("project_picture", { name }));
    }
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
      case "engine":
        return all.some((f) => f.engines.some((e) => e.name.name === o.name));
      case "language":
        return o.name === (p.project.locale.language ?? "en") || o.name in p.project.translations;
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

  /** Opens an engine's code at a line its build reported. */
  function showEngineLine(engine: string, line: number) {
    select({ kind: "engine", name: engine });
    engineGoTo = { line, n: (engineGoTo?.n ?? 0) + 1 };
  }

  async function deleteEngine(target: SpecFile, name: string, item: number) {
    if (!(await ask(`Delete engine ${name}? Its file and its code stay in the engines folder.`, { title: "Clarity", kind: "warning" }))) return;
    actionError = null;
    try {
      applyPreview((await applyEdit(target, { op: "remove", item })).preview);
    } catch (err) {
      actionError = String(err);
    }
  }

  /** Where the last choice of what a new engine is written in is kept. */
  const ENGINE_LANGUAGE_KEY = "clarity-new-engine-language";

  function lastEngineLanguage(): string | undefined {
    try {
      return localStorage.getItem(ENGINE_LANGUAGE_KEY) ?? undefined;
    } catch {
      return undefined;
    }
  }

  /** The module a new engine goes into, while its dialog is open. */
  let newEngine = $state<SpecFile | null>(null);
  let newEngineError = $state<string | null>(null);

  function startNewEngine() {
    const target = files.find((f) => f.engines.length > 0) ?? file ?? files[0];
    if (target) {
      newEngineError = null;
      newEngine = target;
    }
  }

  /**
   * Declares the engine with one function to start from and opens it:
   * written in Clarity, or in Rust built to WebAssembly, whose code with a
   * stub it writes and builds.
   */
  async function addEngine(target: SpecFile, name: string, how: string) {
    // The next new engine starts with the same choice.
    try {
      localStorage.setItem(ENGINE_LANGUAGE_KEY, how);
    } catch {
      // Not kept: the dialog starts with Clarity.
    }
    const stem = name.replace(/([a-z0-9])([A-Z])/g, "$1_$2").toLowerCase();
    const text =
      how === "rust"
        ? `engine ${name} from ${JSON.stringify(stem + ".wasm")}\n    function compute(value: integer) -> integer\n`
        : `engine ${name}\n    function compute(value: integer) -> integer\n        return value\n`;
    try {
      applyPreview((await applyEdit(target, { op: "appendDecl", text })).preview);
      newEngine = null;
      select({ kind: "engine", name });
      if (how !== "rust") return;
      await invoke("new_engine_source", { name });
      await invoke("build_engine", { name });
    } catch (err) {
      newEngineError = String(err);
      actionError = String(err);
    }
  }

  /** Whether the dialog for a new language is open, and what went wrong adding it. */
  let newLanguageOpen = $state(false);
  /** Clarity's own settings, open in their dialog. */
  let appSettingsOpen = $state(false);
  let newLanguageError = $state<string | null>(null);

  function startNewLanguage() {
    newLanguageError = null;
    newLanguageOpen = true;
  }

  /** Writes the translation file of `tag` and opens it. */
  async function addLanguage(tag: string) {
    try {
      applyPreview(await invoke<Preview>("add_language", { language: tag }));
      newLanguageOpen = false;
      select({ kind: "language", name: tag });
      tab = "project";
    } catch (err) {
      newLanguageError = String(err);
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

  /** The module a new theme goes into, while its dialog asks for the name. */
  let newTheme = $state<SpecFile | null>(null);
  let newThemeError = $state<string | null>(null);

  /** Asks for the name of a new theme, proposing the module that already declares themes, or the first one. */
  function startNewTheme() {
    const target = files.find((f) => f.themes.length > 0) ?? file ?? files[0];
    if (target) {
      newThemeError = null;
      newTheme = target;
    }
  }

  /** Adds a theme as a full copy of Clarity's own, and opens it. */
  async function addTheme(target: SpecFile, name: string, start: string) {
    newThemeError = null;
    try {
      const text = await invoke<string>("theme_starter", { name, start });
      applyPreview((await applyEdit(target, { op: "appendDecl", text })).preview);
      newTheme = null;
      select({ kind: "theme", name });
    } catch (err) {
      newThemeError = String(err);
    }
  }

  /** The project's settings with `change` made to the themes it offers, saved; nothing when it does not offer `name`. */
  async function changeOffered(name: string, change: (themes: string[]) => string[]) {
    if (!projectFile?.settings || !projectFile.settings.themes.includes(name)) return;
    await saveSettings({ ...projectFile.settings, themes: change(projectFile.settings.themes) });
  }

  /** Renames a theme where it is declared, and where the project offers it. */
  async function renameTheme(target: SpecFile, theme: { name: string; item: number }, newName: string) {
    actionError = null;
    try {
      const now = await invoke<CodeView>("decl_text", { path: target.path, item: theme.item });
      const text = now.text.replace(/^theme\s+\S+/, `theme ${newName}`);
      applyPreview((await applyEdit(target, { op: "replaceDecl", item: theme.item, text })).preview);
      await changeOffered(theme.name, (themes) => themes.map((t) => (t === theme.name ? newName : t)));
      select({ kind: "theme", name: newName });
    } catch (err) {
      actionError = String(err);
    }
  }

  async function deleteTheme(target: SpecFile, name: string, item: number) {
    const offered = projectFile?.settings?.themes.includes(name);
    const note = offered ? " The application stops offering it." : "";
    if (!(await ask(`Delete theme ${name}?${note}`, { title: "Clarity", kind: "warning" }))) return;
    actionError = null;
    try {
      await changeOffered(name, (themes) => themes.filter((t) => t !== name));
      applyPreview((await applyEdit(target, { op: "remove", item })).preview);
    } catch (err) {
      actionError = String(err);
    }
  }

  /** The menu a right click on the project tree opened, with what it was on. */
  let treeMenu = $state<{ x: number; y: number; item: Selected } | null>(null);

  /** Opens what the tree shows, as a click on it does. */
  function openItem(item: Selected) {
    const page = item.kind === "window" ? files.flatMap((f) => f.pages).find((p) => p.name.name === item.name) : undefined;
    if (page) selectPage(page);
    else select(item);
  }

  /** Deletes a window, entity, enum, theme or engine the tree shows, after asking. */
  async function deleteItem(item: Selected) {
    for (const f of files) {
      const theme = item.kind === "theme" ? f.themes.find((t) => t.name.name === item.name) : undefined;
      if (theme) return deleteTheme(f, item.name, theme.span.start);
      const engine = item.kind === "engine" ? f.engines.find((e) => e.name.name === item.name) : undefined;
      if (engine) return deleteEngine(f, item.name, engine.span.start);
      const decl =
        item.kind === "window"
          ? f.pages.find((p) => p.name.name === item.name)
          : item.kind === "entity"
            ? f.entities.find((e) => e.name.name === item.name)
            : item.kind === "enum"
              ? f.enums.find((e) => e.name.name === item.name)
              : undefined;
      if (!decl) continue;
      const noun = item.kind === "window" ? "window" : item.kind;
      if (!(await ask(`Delete ${noun} ${item.name}?`, { title: "Clarity", kind: "warning" }))) return;
      actionError = null;
      try {
        applyPreview((await applyEdit(f, { op: "remove", item: decl.span.start })).preview);
      } catch (err) {
        actionError = String(err);
      }
      return;
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

  // An app's menu item or button double-clicked in a window's designer opens
  // the app's designer at it, with its code.
  let handledAppCode: object | null = null;
  $effect(() => {
    const request = pageDesigner.appCode;
    if (!request || request === handledAppCode) return;
    handledAppCode = request;
    untrack(() => {
      selectApp();
      tick().then(() => pageDesigner.openPartCode(request.item));
    });
  });

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

  /** Opens the manual on the page about what you are doing: the agent's
   *  page while the focus is in its panel, else the current tab's. */
  function openManual() {
    const inAgent = (document.activeElement as HTMLElement | null)?.closest(".agent");
    invoke("open_manual", { topic: topicForTab(inAgent ? "agent" : tab) }).catch(() => {});
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.key === "F1") {
      e.preventDefault();
      openManual();
      return;
    }
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

  /** Applies a fix the Problems list offers to the file of the problem. */
  async function applyFix(path: string, edit: Edit) {
    const target = files.find((f) => f.path === path);
    if (!target) return;
    actionError = null;
    try {
      applyPreview((await applyEdit(target, edit)).preview);
    } catch (err) {
      actionError = String(err);
    }
  }

  /** A fix for the whole project, by its command (Make texts keys). */
  async function runFix(command: "make_texts_keys") {
    actionError = null;
    try {
      applyPreview(await invoke<Preview>(command));
    } catch (err) {
      actionError = String(err);
    }
  }

  function location(d: Diagnostic): string {
    return `${fileName(d.path)}:${d.span.line}:${d.span.col}`;
  }

  onMount(() => {
    // A turn of the agent ends: open what it made, show what it asked for, and offer Undo.
    agent.onEnded = async (ended) => {
      if (ended.opened) await openProject(ended.opened);
      else if (ended.changed) await checkProject();
      await refreshHistory();
      if (ended.shown) {
        stack = [{ kind: "window", name: ended.shown }];
        tab = "preview";
      }
    };
    const unlisten = listen<Preview>("preview-updated", (e) => applyPreview(e.payload));
    const unlistenProject = listen<ProjectFile>("project-updated", (e) => (projectFile = e.payload));
    // A build starts, from the toolbar or from the agent once the person allowed it.
    const unlistenStarted = listen("build-started", () => {
      buildLog = [];
      buildDone = null;
      showOutput = true;
      building = true;
    });
    const unlistenLine = listen<string>("build-line", (e) => {
      buildLog = [...buildLog.slice(-(BUILD_LOG_LINES - 1)), e.payload];
    });
    const unlistenBuilt = listen<EngineBuilt>("engine-built", (e) => recordBuilt(e.payload));
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
      unlistenStarted.then((stop) => stop());
      unlistenDone.then((stop) => stop());
      unlistenBuilt.then((stop) => stop());
    };
  });
</script>

<svelte:window {onkeydown} />

{#if preview}<ThemeStyles {preview} />{/if}

{#snippet languageGroup()}
  <!-- The language the canvas and Preview show the texts in. -->
  {#if preview}
    {@const written = preview.project.locale.language ?? "en"}
    <div class="group">
      <div class="buttons language-pick">
        <select
          aria-label="The language the designer shows"
          value={languageChoice.language === undefined ? "" : (languageChoice.language ?? written)}
          onchange={(e) => {
            const value = e.currentTarget.value;
            languageChoice.language = value === "" ? undefined : value === written ? null : value;
          }}
        >
          <option value="">System's language</option>
          <option value={written}>{languageName(written)} (as written)</option>
          {#each Object.keys(preview.project.translations).toSorted() as tag (tag)}<option value={tag}>{languageName(tag)}</option>{/each}
        </select>
      </div>
      <div class="group-name">Language</div>
    </div>
  {/if}
{/snippet}

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
      onnewtheme={full ? startNewTheme : null}
      onmenu={full ? (e, item) => (treeMenu = { x: e.clientX, y: e.clientY, item }) : null}
      onnewengine={full ? startNewEngine : null}
      onnewlanguage={full ? startNewLanguage : null}
      onnewwindow={full ? startNewWindow : null}
      onnewapp={full && preview.project.files[0] ? () => preview && addApp(preview.project.files[0]) : null}
    />
  {/if}
{/snippet}

<div class="app">
  <header class="ribbon">
    <div class="ribbon-tabs" role="tablist">
      <div class="brand"><img class="logo" src={logo} alt="" /> Clarity</div>
      {#each TABS as t (t.key)}
        <button role="tab" aria-selected={tab === t.key} class:active={tab === t.key} onclick={() => ((tab = t.key), (appSettingsOpen = false))}>{t.name}</button>
      {/each}
      {#if projectName}<span class="project-name">{projectName}</span>{/if}
      <button class="corner settings-button" class:active={appSettingsOpen} aria-pressed={appSettingsOpen} onclick={() => (appSettingsOpen = !appSettingsOpen)} title="Clarity's own settings: its display and what the agent may run">Clarity settings</button>
      <button class="corner manual-button" onclick={() => openManual()} title="Open the manual on this tab's page (F1)">Manual</button>
      <button class="corner agent-toggle" class:active={agent.open} aria-pressed={agent.open} onclick={() => agent.toggle()} title="Show or hide the agent">Agent</button>
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
          </div>
          <div class="group-name">Window</div>
        </div>
        {@render languageGroup()}
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
      {:else if tab === "vault"}
        <div class="group">
          <div class="buttons">
            <RibbonButton icon="key" label="Secrets" active={vault.view === "secrets"} onclick={() => (vault.view = "secrets")} />
            <RibbonButton icon="link" label="Connections" active={vault.view === "connections"} onclick={() => (vault.view = "connections")} />
          </div>
          <div class="group-name">Vault</div>
        </div>
        {#if vault.view === "secrets"}
          <div class="group">
            <div class="buttons">
              <RibbonButton icon="new" label="Add" disabled={!vault.unlocked} onclick={() => vault.startSecret()} />
              <RibbonButton icon="edit" label="Edit" disabled={!vault.unlocked || !vault.selected} onclick={() => vault.editSecret()} />
              <RibbonButton icon="delete" label="Delete" disabled={!vault.unlocked || !vault.selected} onclick={() => vault.deleteSecret()} />
              <RibbonButton icon="copy" label="Copy value" disabled={!vault.unlocked || !vault.selected} onclick={() => vault.copyValue()} />
              <RibbonButton icon="tag" label="Categories" disabled={!vault.unlocked} onclick={() => (vault.dialog = "categories")} />
            </div>
            <div class="group-name">Secrets</div>
          </div>
        {:else}
          <div class="group">
            <div class="buttons">
              <RibbonButton icon="new" label="New connection" onclick={() => vault.newConnection()} />
              <RibbonButton icon="delete" label="Delete" disabled={!vault.draft} onclick={() => vault.removeConnection()} />
            </div>
            <div class="group-name">Connections</div>
          </div>
        {/if}
        <div class="group">
          <div class="buttons">
            <RibbonButton icon="lock" label="Lock" disabled={!vault.unlocked} onclick={() => vault.lock()} />
            <RibbonButton icon="key" label="Change password" disabled={!vault.unlocked} onclick={() => (vault.dialog = "password")} />
            <RibbonButton icon="delete" label="Destroy vault" disabled={!vault.info?.exists} onclick={() => (vault.dialog = "destroy")} />
          </div>
          <div class="group-name">Master password</div>
        </div>
      {:else}
        <div class="group">
          <div class="buttons">
            <RibbonButton icon="window" label="Main window" disabled={!preview} onclick={() => preview && (stack = startWindow(preview))} />
            <RibbonButton icon="back" label="Back" disabled={stack.length < 2} onclick={() => (stack = stack.slice(0, -1))} />
          </div>
          <div class="group-name">Preview</div>
        </div>
        {@render languageGroup()}
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

  <!-- Clarity's settings take the place of the work below the ribbon; the work stays as it was. -->
  {#if appSettingsOpen}
    <AppSettingsView onclose={() => (appSettingsOpen = false)} />
  {/if}
  <div class="shell" class:hidden={appSettingsOpen} style:grid-template-columns="{layout.sizes.sidebar}px auto minmax(0, 1fr){agent.open ? ` auto ${layout.sizes.agent}px` : ''}">
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
              <dt>App folder</dt>
              <dd class="path"><button class="link" title="Open the project settings" onclick={selectSettings}>{projectFile?.settings?.out ?? "-"}</button></dd>
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
      {:else if tab === "vault"}
        <VaultSide />
      {:else if tab === "preview"}
        <section class="side-section fill">
          <h2>Run a window</h2>
          {@render tree(false)}
        </section>
      {/if}

      {#if preview && tab !== "vault"}
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
            {#each builtProblems as { engine, problem }, i (i)}
              <li class={problem.severity}>
                <button class="link" onclick={() => showEngineLine(engine, problem.line)}><code>{engine} {problem.file.split(/[\\/]/).pop()}:{problem.line}</code></button>
                <span>{problem.message}</span>
              </li>
            {/each}
            {#each preview.project.diagnostics as d, i (i)}
              {@const fix = fixOf(d.message)}
              <li class={d.severity}>
                <code>{location(d)}</code>
                <span>{d.message}</span>
                {#if fix}
                  {#if "edit" in fix}
                    <button class="fix" title="For every such problem in {fileName(d.path)}" onclick={() => applyFix(d.path, fix.edit)}>{fix.label}</button>
                  {:else}
                    <button class="fix" title="For every text in the project" onclick={() => runFix(fix.command)}>{fix.label}</button>
                  {/if}
                {/if}
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
        {#if tab === "vault"}
          <VaultView />
        {:else if !preview}
          <div class="placeholder">
            <p>Open a project, or create a new one, with the buttons on the Project tab.</p>
          </div>
        {:else if current?.kind === "settings" && projectFile && openFile}
          <ProjectSettings project={projectFile} file={openFile} revision={settingsRevision} onsave={saveSettings} {preview} onapplied={applyPreview} onopentheme={(name) => select({ kind: "theme", name })} />
        {:else if current?.kind === "language"}
          <TranslationsPanel
            {preview}
            language={current.name}
            onapplied={applyPreview}
            onopen={(language) => select({ kind: "language", name: language })}
            onnewlanguage={startNewLanguage}
          />
        {:else if entity && file}
          <EntityEditor
            {preview}
            {entity}
            stores={projectFile?.settings?.filestores.map((s) => s.name) ?? []}
            module={file.module?.name ?? ""}
            onedit={editData}
            ondelete={deletePicked}
            onrename={(newName) => renameData("entity", entity.name.name, newName)}
            onapplied={applyPreview}
          />
        {:else if enumDecl && file}
          <EnumEditor
            {preview}
            onapplied={applyPreview}
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
            onrename={(newName) => openTheme.file && renameTheme(openTheme.file, { name: openTheme.theme.name.name, item: openTheme.theme.span.start }, newName)}
            ondelete={() => openTheme.file && deleteTheme(openTheme.file, openTheme.theme.name.name, openTheme.theme.span.start)}
          />
        {:else if openEngine}
          <EngineEditor
            {preview}
            decl={openEngine.decl}
            file={openEngine.file}
            goTo={engineGoTo}
            onapply={async (edit) => applyPreview((await applyEdit(openEngine.file, edit)).preview)}
            ondelete={() => deleteEngine(openEngine.file, openEngine.decl.name.name, openEngine.decl.span.start)}
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
          <ShownWindow area={themeArea(themeOf(preview))} clientWidth={windowSize(page).width}>
            <div class="titlebar">
              <span>{page.title ? tr(preview, page.title) : page.name.name}</span>
              <code>{page.name.name}: {page.kind}</code>
            </div>
            <!-- The window's client, at the height it opens at. -->
            <div class="shown" style:height="{windowSize(page).height}px">
              <FrameView
                {preview}
                {page}
                frame={frameForPage(preview, page)}
                data={{ ...appData, ...pageData(preview, page) }}
                design={false}
                onnavigate={navigate}
              >
                <div class="client" style:padding="{CONTENT_PADDING}px">
                  <PageView {preview} {page} onnavigate={navigate} />
                </div>
              </FrameView>
            </div>
          </ShownWindow>
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
    </div>
    {#if agent.open}
      <Splitter axis="x" label="Width of the agent" onmove={(d) => layout.resize("agent", -d)} />
      <AgentPanel />
    {/if}
  </div>
</div>

<UnlockDialog />

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

{#if newEngine && preview}
  <NameDialog
    {preview}
    title="New engine"
    placeholder="Pricing"
    action="Add engine"
    file={newEngine}
    error={newEngineError}
    choice={{
      label: "Written in",
      picked: lastEngineLanguage(),
      options: [
        { value: "clarity", label: "Clarity: generated with the application on every stack" },
        { value: "rust", label: "Rust, built to WebAssembly" },
      ],
    }}
    onsubmit={addEngine}
    oncancel={() => (newEngine = null)}
  />
{/if}

{#if treeMenu}
  {@const item = treeMenu.item}
  <ContextMenu
    x={treeMenu.x}
    y={treeMenu.y}
    items={[
      { label: "Open", run: () => openItem(item) },
      { label: `Delete ${item.kind}`, danger: true, run: () => deleteItem(item) },
    ]}
    onclose={() => (treeMenu = null)}
  />
{/if}

{#if newTheme && preview}
  <NameDialog
    {preview}
    title="New theme"
    placeholder="Night"
    action="Add theme"
    file={newTheme}
    error={newThemeError}
    choice={{
      label: "Starts as",
      options: [
        { value: "system", label: "Light and dark, as the system is" },
        { value: "light", label: "Light" },
        { value: "dark", label: "Dark" },
        { value: "cream-green", label: "Cream and green" },
      ],
    }}
    onsubmit={addTheme}
    oncancel={() => (newTheme = null)}
  />
{/if}


{#if newLanguageOpen && preview}
  <NewLanguageDialog
    taken={[preview.project.locale.language ?? "en", ...Object.keys(preview.project.translations)]}
    error={newLanguageError}
    onsubmit={addLanguage}
    oncancel={() => (newLanguageOpen = false)}
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

  .shell.hidden {
    display: none;
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

  .ribbon-tabs .corner {
    align-self: center;
    padding: 3px 12px;
    border: 1px solid color-mix(in srgb, var(--titlebar-text) 35%, transparent);
    border-radius: var(--radius);
  }

  .ribbon-tabs .settings-button {
    margin: 0 0 0 auto;
  }

  .ribbon-tabs .project-name + .settings-button {
    margin-left: 12px;
  }

  .ribbon-tabs .manual-button {
    margin: 0 0 0 8px;
  }

  .ribbon-tabs .agent-toggle {
    margin: 0 0 0 8px;
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

  .language-pick {
    align-items: center;
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
    width: 24px;
    height: 24px;
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

  .diagnostics li .fix {
    align-self: flex-start;
    margin-top: 3px;
    padding: 1px 8px;
    font-size: 11px;
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

  .shown {
    display: flex;
    flex-direction: column;
  }

  .client {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    overflow: auto;
  }
</style>
