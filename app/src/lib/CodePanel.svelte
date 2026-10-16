<script lang="ts">
  // A window (or the app) as Clarity text, in a panel that slides out over
  // the designer. Design shows the window's own declaration: while it is
  // typed the designer follows the text, and Save writes it. Code shows the
  // code behind. Both are checked by the backend while they are typed, and
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
  import { pageDesigner, type PanelView } from "./pageDesigner.svelte";

  let {
    file,
    owner,
    item,
    onapplied,
    onclose,
  }: {
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
    const result = await applyEdit(file, { op: "setCode", owner, text });
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
    <code class="owner">{owner}</code>
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
      load={() => invoke<CodeView>("code_text", { path: file.path, owner })}
      check={(text) => invoke<Diagnostic[]>("check_code", { path: file.path, owner, text })}
      save={saveCode}
      {reloadKey}
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

  .owner {
    font-family: var(--mono);
    color: var(--text-muted);
    flex: 1;
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
