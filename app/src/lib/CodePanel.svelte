<script lang="ts">
  // A window (or the app) as Clarity text, in a panel that slides out over
  // the designer. Design shows the window's own declaration: while it is
  // typed the designer follows the text, and Save writes it. Code shows the
  // code behind: of the part picked in the designer (its handlers and the
  // handlers they call), of the window's own events when nothing is picked,
  // or all of it. Both are checked by the backend while they are typed, and
  // a save is one designer edit, so Undo takes it back.

  import { invoke } from "@tauri-apps/api/core";
  import { onDestroy } from "svelte";
  import SourceEditor from "./SourceEditor.svelte";
  import Splitter from "./Splitter.svelte";
  import { layout } from "./layout.svelte";
  import { applyEdit } from "./applyEdit";
  import type { CodeView } from "./generated/CodeView";
  import type { DeclTrial } from "./generated/DeclTrial";
  import type { Diagnostic } from "./generated/Diagnostic";
  import type { Preview } from "./generated/Preview";
  import type { SpecFile } from "./generated/SpecFile";
  import { pageDesigner, type PanelView } from "./pageDesignerStore.svelte";
  import { commandAction, focusOf, handlerName, mergeFocus } from "./codeParts";
  import { findFrameElement } from "./frame";
  import { locateWidget } from "./pages";
  import type { Page } from "./generated/Page";

  let {
    preview,
    page,
    file,
    owner,
    item,
    onapplied,
    onclose,
  }: {
    preview: Preview;
    /** The window (or the app as a window) whose parts are picked. */
    page: Page;
    file: SpecFile;
    /** The window or app shown. */
    owner: string;
    /** Where its declaration starts in the file. */
    item: number;
    onapplied: (preview: Preview) => void;
    onclose: () => void;
  } = $props();

  const VIEWS: { key: PanelView; name: string }[] = [
    { key: "design", name: "Design" },
    { key: "code", name: "Code" },
  ];

  let codeEditor: SourceEditor | undefined = $state();

  /** Whether the code shows all of the window's handlers, not only the picked part's. */
  let whole = $state(false);

  /** Whose code shows: the picked part's handlers, or the window's own events'. */
  const scope = $derived.by((): { label: string; roots: string[] } | null => {
    const selected = pageDesigner.selected;
    const element = selected === null ? null : findFrameElement(page.frame, selected);
    if (element?.kind === "command") {
      const c = element.command.props;
      const caption = c.caption ?? c.name ?? "the command";
      const does = commandAction(c);
      if (c.handler) return { label: `"${caption}": on click`, roots: [c.handler] };
      return { label: `"${caption}" ${does ?? "runs no code yet; double-click it to write its handler"}`, roots: [] };
    }
    const picked = selected === null ? null : locateWidget(preview, page, selected);
    if (picked) {
      const p = picked.widget.props;
      const events = p.handlers.map((h) => h.event);
      return {
        label: (p.name ?? p.kind) + (events.length > 0 ? ": on " + events.join(", on ") : ": no events wired"),
        roots: p.handlers.map((h) => handlerName(h.handler)),
      };
    }
    const own = page.handlers ?? [];
    return own.length > 0 ? { label: `${owner}: on ${own.map((h) => h.event).join(", on ")}`, roots: own.map((h) => handlerName(h.handler)) } : null;
  });
  const focused = $derived(!whole && scope !== null && scope.roots.length > 0);

  // What was loaded, to merge a save into: the window's whole code and the handlers shown of it.
  let loaded: { code: string; names: string[] } = { code: "", names: [] };

  async function loadCode(): Promise<CodeView> {
    const view = await invoke<CodeView>("code_text", { path: file.path, owner });
    if (!focused || !scope) {
      loaded = { code: view.text, names: [] };
      return view;
    }
    const focus = focusOf(view.text, scope.roots);
    // A part whose handlers are not written yet starts with them empty.
    const unwritten = scope.roots.filter((r) => !focus.names.includes(r));
    const text = [focus.text, ...unwritten.map((r) => `handler ${r}\n`)].filter((t) => t !== "").join("\n");
    loaded = { code: view.text, names: [...focus.names, ...unwritten] };
    return { ...view, text, problems: placed(view.problems, focus.firstLine, text) };
  }

  /** The problems of the window's code that fall in the shown text, at its lines. */
  function placed(problems: Diagnostic[], firstLine: number, text: string): Diagnostic[] {
    const lines = text.split("\n").length;
    return problems
      .filter((p) => p.span.line >= firstLine && p.span.line < firstLine + lines)
      .map((p) => ({ ...p, span: { ...p.span, line: p.span.line - firstLine + 1 } }));
  }

  async function checkCode(text: string): Promise<Diagnostic[]> {
    if (loaded.names.length === 0) return invoke<Diagnostic[]>("check_code", { path: file.path, owner, text });
    const merged = mergeFocus(loaded.code, loaded.names, text);
    return placed(await invoke<Diagnostic[]>("check_code", { path: file.path, owner, text: merged.text }), merged.firstLine, text);
  }

  // Show a requested handler once, as soon as its code is there.
  let shown: object | null = null;
  $effect(() => {
    const request = pageDesigner.code;
    const focus = request.focus;
    if (request === shown || !focus || !codeEditor) return;
    if (codeEditor.jumpTo(focus)) shown = request;
  });

  // The designer shows the saved window again when the panel goes.
  onDestroy(() => (pageDesigner.draft = null));

  async function loadDesign() {
    return invoke<CodeView>("decl_text", { path: file.path, item });
  }

  async function checkDesign(text: string): Promise<Diagnostic[]> {
    try {
      const trial = await invoke<DeclTrial>("try_decl", { path: file.path, item, text });
      pageDesigner.draft = trial.preview;
      return trial.problems;
    } catch (err) {
      // Text that is not one declaration has no preview to show.
      pageDesigner.draft = null;
      throw err;
    }
  }

  async function saveDesign(text: string) {
    const result = await applyEdit(file, { op: "replaceDecl", item, text });
    pageDesigner.draft = null;
    onapplied(result.preview);
  }

  async function saveCode(text: string) {
    const whole = loaded.names.length === 0 ? text : mergeFocus(loaded.code, loaded.names, text).text;
    const result = await applyEdit(file, { op: "setCode", owner, text: whole });
    onapplied(result.preview);
  }

  const reloadKey = $derived(file.path + "\n" + owner + "\n" + item + "\n" + file.version);
</script>

<div class="panel" role="dialog" aria-label="{owner} as Clarity text" style:width="min({layout.sizes.codePanel}px, 90%)">
  <div class="edge">
    <Splitter axis="x" label="Width of the text panel" onmove={(d) => layout.resize("codePanel", -d)} />
  </div>
  <header>
    <div class="tabs" role="tablist">
      {#each VIEWS as v (v.key)}
        <button
          role="tab"
          aria-selected={pageDesigner.code.view === v.key}
          class:active={pageDesigner.code.view === v.key}
          onclick={() => (pageDesigner.code = { ...pageDesigner.code, view: v.key, focus: null })}>{v.name}</button
        >
      {/each}
    </div>
    {#if pageDesigner.code.view === "code"}
      <span class="scope">
        {#if scope}
          <span class="what">{whole ? `All of ${owner}` : scope.label}</span>
          <label><input type="checkbox" bind:checked={whole} /> Whole window</label>
        {:else}
          <span class="what">All of {owner}</span>
        {/if}
      </span>
    {/if}
    <button class="close" onclick={onclose} title="Close (Escape)">&times;</button>
  </header>

  <!-- Both stay mounted, so switching keeps unsaved text. -->
  <div class="view" class:hidden={pageDesigner.code.view !== "design"}>
    <SourceEditor
      load={loadDesign}
      check={checkDesign}
      save={saveDesign}
      {reloadKey}
      ondirty={(dirty) => !dirty && (pageDesigner.draft = null)}
      {onclose}
    />
    {#if pageDesigner.draft}
      <p class="following">The designer shows this text. Save it to keep it, or Revert.</p>
    {/if}
  </div>
  <div class="view" class:hidden={pageDesigner.code.view !== "code"}>
    <SourceEditor
      bind:this={codeEditor}
      load={loadCode}
      check={checkCode}
      save={saveCode}
      reloadKey={reloadKey + "\n" + (focused && scope ? scope.roots.join(",") : "all")}
      navPattern={/^handler\s+([A-Za-z_]\w*)/}
      placeholder={'handler save_clicked\n    message "Hello"'}
      {onclose}
    />
  </div>
</div>

<style>
  .panel {
    position: absolute;
    top: 0;
    right: 0;
    bottom: 0;

    display: flex;
    flex-direction: column;
    background: var(--panel);
    border-left: 1px solid var(--border-strong);
    box-shadow: -12px 0 32px rgb(0 0 0 / 0.35);
    pointer-events: auto;
    animation: slide-in 140ms ease-out;
  }

  .edge {
    position: absolute;
    left: 0;
    top: 0;
    bottom: 0;
    display: flex;
  }

  @keyframes slide-in {
    from {
      transform: translateX(40px);
      opacity: 0;
    }
  }

  header {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 6px 12px 0;
    border-bottom: 1px solid var(--border);
  }

  .tabs {
    display: flex;
    gap: 2px;
  }

  .tabs button {
    border: none;
    border-radius: var(--radius) var(--radius) 0 0;
    background: none;
    padding: 6px 14px;
    color: var(--text-muted);
  }

  .tabs button.active {
    background: var(--panel-alt);
    color: var(--text);
    font-weight: 600;
  }

  .scope {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 12px;
    padding-left: 12px;
    font-size: 12px;
    color: var(--text-muted);
  }

  .scope .what {
    color: var(--text);
  }

  .scope label {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .close {
    margin-bottom: 4px;
  }

  .view {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 0;
  }

  .view.hidden {
    display: none;
  }

  .following {
    margin: 0;
    padding: 6px 12px;
    font-size: 12px;
    color: var(--warning, #d9a400);
    border-top: 1px solid var(--border);
  }
</style>
