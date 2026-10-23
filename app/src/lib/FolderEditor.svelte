<script lang="ts">
  // The folder of a custom part or a library, edited like a project in an
  // IDE. A toolbar on top builds it, writes Clarity's files, and saves or
  // reverts the tab shown. On the left, what it is and its files as a tree
  // (New file, New folder, Rename, Delete); on the right, the declaration and
  // each opened file in tabs that stay alive while hidden, so switching keeps
  // unsaved changes. Clarity's file opens to read and cannot be renamed or
  // deleted. A build's output goes to the Output panel.

  import type { Snippet } from "svelte";
  import { invoke } from "./backend";
  import CodeEditor from "./CodeEditor.svelte";
  import SourceEditor from "./SourceEditor.svelte";
  import { languageOfFile } from "./codeLanguages";
  import { dialogs } from "./dialogStore.svelte";
  import type { Diagnostic } from "./generated/Diagnostic";

  let {
    name,
    version,
    declarationLabel,
    declaration,
    summary,
    writeLabel,
    writeTitle,
    buildTitle,
    buildable,
  }: {
    /** The part or library whose folder this is. */
    name: string;
    /** Changes when the project changes, to list and load again. */
    version: string | number;
    /** The first tab's name: `part Arena`, `library Words`. */
    declarationLabel: string;
    /** The declaration, edited as Clarity text in the first tab. */
    declaration: {
      load: () => Promise<{ text: string; problems: Diagnostic[] }>;
      check: (text: string) => Promise<Diagnostic[]>;
      save: (text: string) => Promise<void>;
      reloadKey: string;
    };
    /** What shows above the files. */
    summary: Snippet;
    writeLabel: string;
    writeTitle: string;
    /** What Build does, or why it builds only with the program. */
    buildTitle: string;
    /** Whether it builds on its own, in its folder. */
    buildable: boolean;
  } = $props();

  type Entry = { path: string; folder: boolean; own: boolean };
  /** The declaration's tab, which has no file path. */
  const DECLARATION = "";

  let entries = $state<Entry[]>([]);
  let collapsed = $state<Set<string>>(new Set());
  let selected = $state<Entry | null>(null);
  let tabs = $state<Entry[]>([]);
  /** The tab shown: a file's path, or DECLARATION. */
  let current = $state(DECLARATION);
  let dirty = $state<Record<string, boolean>>({});
  let note = $state<string | null>(null);
  let declarationEditor = $state<SourceEditor | null>(null);
  let fileEditors = $state<Record<string, CodeEditor | null>>({});

  const shown = $derived(entries.filter((e) => !ancestors(e.path).some((a) => collapsed.has(a))));
  const currentEntry = $derived(tabs.find((t) => t.path === current) ?? null);
  const currentDirty = $derived(dirty[current] === true);

  function ancestors(path: string): string[] {
    const parts = path.split("/");
    return parts.slice(0, -1).map((_, i) => parts.slice(0, i + 1).join("/"));
  }

  function depth(path: string): number {
    return path.split("/").length - 1;
  }

  function base(path: string): string {
    return path.split("/").pop() ?? path;
  }

  async function list() {
    try {
      entries = await invoke<Entry[]>("part_files", { name });
      note = null;
    } catch (err) {
      entries = [];
      note = String(err);
    }
    const paths = new Set(entries.map((e) => e.path));
    tabs = tabs.filter((t) => paths.has(t.path));
    if (current !== DECLARATION && !paths.has(current)) current = DECLARATION;
    if (selected && !paths.has(selected.path)) selected = null;
  }

  $effect(() => {
    void name;
    void version;
    void list();
  });

  function choose(entry: Entry) {
    selected = entry;
    if (entry.folder) {
      const next = new Set(collapsed);
      if (next.has(entry.path)) next.delete(entry.path);
      else next.add(entry.path);
      collapsed = next;
      return;
    }
    if (!tabs.some((t) => t.path === entry.path)) tabs = [...tabs, entry];
    current = entry.path;
  }

  async function closeTab(path: string) {
    if (dirty[path] && !(await dialogs.confirm({ title: "Close", text: `${path} has unsaved changes. Close it without saving?`, action: "Close", danger: true }))) return;
    const at = tabs.findIndex((t) => t.path === path);
    tabs = tabs.filter((t) => t.path !== path);
    delete dirty[path];
    if (current === path) current = tabs[Math.max(0, at - 1)]?.path ?? DECLARATION;
  }

  async function save() {
    if (current === DECLARATION) await declarationEditor?.saveIfChanged();
    else await fileEditors[current]?.saveFile();
  }

  function revert() {
    if (current === DECLARATION) declarationEditor?.revertText();
    else void fileEditors[current]?.revertFile();
  }

  /** The folder new entries go in: the chosen folder, or the chosen file's. */
  function target(): string {
    if (!selected) return "";
    if (selected.folder) return selected.path + "/";
    const at = selected.path.lastIndexOf("/");
    return at < 0 ? "" : selected.path.slice(0, at + 1);
  }

  async function act(run: () => Promise<unknown>): Promise<boolean> {
    let ok = true;
    try {
      await run();
      note = null;
    } catch (err) {
      note = String(err);
      ok = false;
    }
    await list();
    return ok;
  }

  async function make(folder: boolean) {
    const into = target();
    const answer = await dialogs.prompt({
      title: folder ? "New folder" : "New file",
      text: into ? `In ${into}` : `In the folder of ${name}`,
      placeholder: folder ? "assets" : "src/sound.rs",
      action: "Make",
    });
    const path = answer?.trim().replace(/^\/+/, "");
    if (!path) return;
    const full = into + path;
    if (!(await act(() => invoke("new_part_file", { name, path: full, folder })))) return;
    const made = entries.find((e) => e.path === full);
    if (made && !folder) choose(made);
    else if (made) selected = made;
  }

  async function rename() {
    const entry = selected;
    if (!entry || entry.own) return;
    const answer = await dialogs.prompt({ title: "Rename", text: `New name or path for ${entry.path}`, value: entry.path, action: "Rename" });
    const to = answer?.trim().replace(/^\/+/, "");
    if (!to || to === entry.path) return;
    if (!(await act(() => invoke("rename_part_file", { name, from: entry.path, to })))) return;
    tabs = tabs.map((t) => (t.path === entry.path ? { ...t, path: to } : t));
    if (current === entry.path) current = to;
    selected = entries.find((e) => e.path === to) ?? null;
  }

  async function remove() {
    const entry = selected;
    if (!entry || entry.own) return;
    const what = entry.folder ? `the folder ${entry.path} and everything in it` : entry.path;
    if (!(await dialogs.confirm({ title: "Delete", text: `Delete ${what}? This cannot be undone.`, action: "Delete", danger: true }))) return;
    await act(() => invoke("delete_part_file", { name, path: entry.path }));
  }

  async function write() {
    const written = await invoke<string[]>("write_part_folders").catch((err) => {
      note = String(err);
      return null;
    });
    if (written) note = written.length > 0 ? `Wrote ${written.join(", ")}` : "The folder is up to date.";
    await list();
  }

  async function build() {
    // Unsaved tabs are saved first, as a build of the program does.
    await declarationEditor?.saveIfChanged();
    for (const t of tabs) if (dirty[t.path]) await fileEditors[t.path]?.saveFile();
    await invoke("build_part", { name }).catch((err) => (note = String(err)));
  }
</script>

<div class="folder-editor">
  <div class="toolbar" role="toolbar">
    <button onclick={build} disabled={!buildable} title={buildTitle}>Build</button>
    <button onclick={write} title={writeTitle}>{writeLabel}</button>
    <span class="spacer"></span>
    {#if note}<span class="note" title={note}>{note}</span>{/if}
    <button onclick={revert} disabled={!currentDirty} title="Put back the saved text of this tab">Revert</button>
    <button class="primary" onclick={save} disabled={!currentDirty} title="Save this tab (Ctrl+S)">Save</button>
  </div>

  <div class="panes">
    <aside>
      {@render summary()}

      <div class="explorer">
        <div class="explorer-head">
          <span class="title">Files</span>
          <span class="tools">
            <button onclick={() => make(false)} title="New file, in the chosen folder">+ File</button>
            <button onclick={() => make(true)} title="New folder, in the chosen folder">+ Folder</button>
            <button onclick={rename} disabled={!selected || selected.own} title="Rename or move the chosen file or folder">Rename</button>
            <button onclick={remove} disabled={!selected || selected.own} title="Delete the chosen file or folder">Delete</button>
          </span>
        </div>
        <ul class="tree" role="tree">
          {#each shown as e (e.path)}
            <li role="treeitem" aria-selected={selected?.path === e.path} aria-expanded={e.folder ? !collapsed.has(e.path) : undefined}>
              <button class:chosen={selected?.path === e.path} class:open={current === e.path} style:padding-left="{8 + depth(e.path) * 14}px" onclick={() => choose(e)} title={e.path}>
                <span class="twist">{e.folder ? (collapsed.has(e.path) ? "▸" : "▾") : ""}</span>
                <code class:folder={e.folder}>{base(e.path)}{e.folder ? "/" : ""}</code>
                {#if dirty[e.path]}<span class="dot" title="unsaved">●</span>{/if}
                {#if e.own}<span class="muted">Clarity's</span>{/if}
              </button>
            </li>
          {:else}
            <li class="muted empty">No files yet.</li>
          {/each}
        </ul>
      </div>
    </aside>

    <section class="code">
      <nav class="tabs">
        <span class="tab" class:current={current === DECLARATION}>
          <button class="tab-name" onclick={() => (current = DECLARATION)}><code>{declarationLabel}</code></button>
          {#if dirty[DECLARATION]}<span class="dot" title="unsaved">●</span>{/if}
        </span>
        {#each tabs as t (t.path)}
          <span class="tab" class:current={current === t.path}>
            <button class="tab-name" onclick={() => (current = t.path)} title={t.path}><code>{base(t.path)}</code></button>
            <button class="tab-close" onclick={() => closeTab(t.path)} title="Close {t.path}">{dirty[t.path] ? "●" : "×"}</button>
          </span>
        {/each}
      </nav>
      <div class="pages">
        <div class="page" class:hidden={current !== DECLARATION}>
          <SourceEditor
            bind:this={declarationEditor}
            bare
            load={declaration.load}
            check={declaration.check}
            save={declaration.save}
            reloadKey={declaration.reloadKey}
            ondirty={(d) => (dirty[DECLARATION] = d)}
          />
        </div>
        {#each tabs as t (t.path)}
          <div class="page" class:hidden={current !== t.path}>
            <CodeEditor
              bind:this={fileEditors[t.path]}
              bare
              load={() => invoke<string>("part_file_text", { name, path: t.path })}
              save={(text) => invoke<void>("save_part_file", { name, path: t.path, text })}
              reloadKey={name + "\n" + t.path + "\n" + version}
              language={languageOfFile(t.path)}
              readonly={t.own}
              ondirty={(d) => (dirty[t.path] = d)}
            />
          </div>
        {/each}
      </div>
      <footer class="status">
        {#if currentEntry}
          <code>{currentEntry.path}</code>
          {#if currentEntry.own}<span>read only: Clarity writes it from the declaration</span>{/if}
        {:else}
          <code>{declarationLabel}</code><span>Clarity, checked while you type</span>
        {/if}
        {#if currentDirty}<span class="unsaved">unsaved</span>{/if}
      </footer>
    </section>
  </div>
</div>

<style>
  .folder-editor {
    display: flex;
    flex-direction: column;
    gap: 10px;
    flex: 1;
    min-height: 0;
  }
  .toolbar {
    display: flex;
    align-items: center;
    gap: 6px;
    padding-bottom: 8px;
    border-bottom: 1px solid var(--border);
  }
  .spacer {
    flex: 1;
  }
  .note {
    font-size: 12px;
    opacity: 0.75;
    max-width: 50%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .panes {
    display: flex;
    gap: 16px;
    flex: 1;
    min-height: 0;
  }
  aside {
    width: 300px;
    flex: none;
    display: flex;
    flex-direction: column;
    gap: 10px;
    min-height: 0;
  }
  .explorer {
    flex: 1;
    min-height: 120px;
    display: flex;
    flex-direction: column;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg);
    overflow: hidden;
  }
  .explorer-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 6px;
    padding: 6px 8px;
    background: var(--panel-alt);
    border-bottom: 1px solid var(--border);
  }
  .title {
    font-size: 0.8em;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  .tools {
    display: flex;
    gap: 4px;
  }
  .tools button {
    font-size: 11px;
    padding: 2px 6px;
  }
  .tree {
    flex: 1;
    overflow: auto;
    list-style: none;
    padding: 4px 0;
    margin: 0;
  }
  .tree button {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 4px;
    border: none;
    border-radius: 0;
    background: none;
    color: inherit;
    text-align: left;
    padding-top: 2px;
    padding-bottom: 2px;
    padding-right: 8px;
  }
  .tree button:hover {
    background: var(--panel-alt);
  }
  .tree button.chosen {
    background: color-mix(in srgb, var(--code-keyword) 20%, transparent);
  }
  .tree button.open code {
    font-weight: bold;
  }
  .twist {
    width: 10px;
    flex: none;
    opacity: 0.7;
  }
  .tree code {
    flex: 1;
  }
  .folder {
    opacity: 0.85;
  }
  .muted {
    opacity: 0.7;
    font-size: 11px;
  }
  .empty {
    padding: 4px 8px;
  }
  .dot {
    color: var(--warning, #d9a400);
    font-size: 10px;
  }
  .code {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    min-height: 0;
    border: 1px solid var(--border);
    border-radius: 6px;
    overflow: hidden;
  }
  .tabs {
    display: flex;
    overflow-x: auto;
    background: var(--panel-alt);
    border-bottom: 1px solid var(--border);
    flex: none;
  }
  .tab {
    display: flex;
    align-items: center;
    gap: 2px;
    padding-right: 6px;
    border-right: 1px solid var(--border);
    opacity: 0.75;
  }
  .tab.current {
    background: var(--bg);
    opacity: 1;
    box-shadow: inset 0 2px 0 var(--code-keyword);
  }
  .tab-name {
    padding: 6px 4px 6px 12px;
    border: none;
    border-radius: 0;
    background: none;
    color: inherit;
  }
  .tab-close {
    padding: 0 4px;
    border: none;
    background: none;
    color: inherit;
    opacity: 0.7;
    font-size: 12px;
  }
  .tab-close:hover {
    opacity: 1;
  }
  .pages {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
  .page {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
  .page.hidden {
    display: none;
  }
  .status {
    display: flex;
    gap: 12px;
    align-items: center;
    padding: 3px 10px;
    font-size: 11px;
    background: var(--panel-alt);
    border-top: 1px solid var(--border);
    opacity: 0.85;
    flex: none;
  }
  .unsaved {
    color: var(--warning, #d9a400);
  }
</style>
