<script lang="ts">
  // A file of another language than Clarity (a part's C++, Rust, Go or C#,
  // its CMakeLists.txt or Cargo.toml), edited with CodeMirror: colored by its
  // language, with line numbers, folding, search (Ctrl+F), bracket matching
  // and undo. Ctrl+S saves; Revert loads the file again. It takes the IDE's
  // own colors, so it follows the light and dark theme.

  import { onDestroy } from "svelte";
  import { basicSetup } from "codemirror";
  import { Compartment, EditorState, type Extension } from "@codemirror/state";
  import { EditorView, keymap } from "@codemirror/view";
  import { StreamLanguage } from "@codemirror/language";
  import { cpp } from "@codemirror/lang-cpp";
  import { rust } from "@codemirror/lang-rust";
  import { go } from "@codemirror/lang-go";
  import { csharp } from "@codemirror/legacy-modes/mode/clike";
  import { cmake } from "@codemirror/legacy-modes/mode/cmake";
  import { toml } from "@codemirror/legacy-modes/mode/toml";
  import type { CodeLanguage } from "./codeLanguages";
  import { ideLook } from "./codeMirrorLook";

  let {
    load,
    save,
    reloadKey,
    language,
    readonly = false,
    bare = false,
    ondirty = () => {},
    onclose,
  }: {
    load: () => Promise<string>;
    /** Writes the text; throws with the reason when it is refused. */
    save: (text: string) => Promise<void>;
    /** Loads again when this changes, unless there are unsaved changes. */
    reloadKey: string;
    language: CodeLanguage;
    /** Shown to read, not to change. */
    readonly?: boolean;
    /** Without its own bar: the view around it saves and reverts. */
    bare?: boolean;
    ondirty?: (dirty: boolean) => void;
    onclose?: () => void;
  } = $props();

  let host = $state<HTMLDivElement | null>(null);
  let view: EditorView | null = null;
  let saved = $state("");
  let dirty = $state(false);
  let error = $state<string | null>(null);
  let saving = $state(false);
  const languages = new Compartment();
  const editing = new Compartment();

  function languageOf(of: CodeLanguage): Extension {
    switch (of) {
      case "cpp":
        return cpp();
      case "rust":
        return rust();
      case "go":
        return go();
      case "csharp":
        return StreamLanguage.define(csharp);
      case "cmake":
        return StreamLanguage.define(cmake);
      case "toml":
        return StreamLanguage.define(toml);
      default:
        return [];
    }
  }

  $effect(() => ondirty(dirty));

  /** Saves unsaved changes. */
  export function saveFile() {
    return store();
  }

  /** Loads the file again, dropping unsaved changes. */
  export function revertFile() {
    return revert();
  }

  async function store() {
    if (!view || readonly || saving) return;
    const text = view.state.doc.toString();
    saving = true;
    error = null;
    try {
      await save(text);
      saved = text;
      dirty = false;
    } catch (err) {
      error = String(err);
    } finally {
      saving = false;
    }
  }

  async function revert() {
    error = null;
    const text = await load().catch((err) => {
      error = String(err);
      return null;
    });
    if (text === null || !view) return;
    saved = text;
    view.dispatch({ changes: { from: 0, to: view.state.doc.length, insert: text } });
    dirty = false;
  }

  function make(text: string) {
    if (!host) return;
    view?.destroy();
    view = new EditorView({
      parent: host,
      state: EditorState.create({
        doc: text,
        extensions: [
          basicSetup,
          keymap.of([{ key: "Mod-s", preventDefault: true, run: () => (void store(), true) }]),
          languages.of(languageOf(language)),
          editing.of([EditorState.readOnly.of(readonly), EditorView.editable.of(!readonly)]),
          ideLook,
          EditorView.updateListener.of((update) => {
            if (update.docChanged) dirty = update.state.doc.toString() !== saved;
          }),
        ],
      }),
    });
  }

  // A new file, or the same one changed on disk while nothing is unsaved: load it.
  $effect(() => {
    void reloadKey;
    if (!host || dirty) return;
    load()
      .then((text) => {
        saved = text;
        dirty = false;
        make(text);
      })
      .catch((err) => (error = String(err)));
  });

  $effect(() => {
    view?.dispatch({ effects: [languages.reconfigure(languageOf(language)), editing.reconfigure([EditorState.readOnly.of(readonly), EditorView.editable.of(!readonly)])] });
  });

  onDestroy(() => view?.destroy());
</script>

<div class="code-editor" class:bare>
  {#if !bare}
  <div class="bar">
    <span class="state">
      {#if readonly}<span class="muted">read only</span>{:else if dirty}<span class="dirty">unsaved</span>{/if}
    </span>
    <span class="actions">
      {#if !readonly}
        <button disabled={!dirty} onclick={revert}>Revert</button>
        <button class="primary" disabled={!dirty || saving} onclick={store}>Save</button>
      {/if}
      {#if onclose}<button onclick={onclose} title="Close the file">&times;</button>{/if}
    </span>
  </div>
  {/if}
  {#if error}<p class="refused" role="alert">{error}</p>{/if}
  <div class="host" bind:this={host}></div>
</div>

<style>
  .code-editor {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 0;
    border: 1px solid var(--border);
  }
  .bare {
    border: none;
  }
  .bar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 8px;
    padding: 6px 12px;
    border-bottom: 1px solid var(--border);
  }
  .actions {
    display: flex;
    gap: 6px;
  }
  .dirty {
    font-size: 11px;
    color: var(--warning, #d9a400);
  }
  .muted {
    font-size: 11px;
    color: var(--text-muted);
  }
  .refused {
    margin: 0;
    padding: 6px 12px;
    color: var(--error);
  }
  .host {
    flex: 1;
    min-height: 0;
    overflow: hidden;
  }
  .host :global(.cm-editor) {
    height: 100%;
  }
</style>
