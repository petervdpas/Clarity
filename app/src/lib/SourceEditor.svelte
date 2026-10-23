<script lang="ts" module>
  /** Clarity lines after which the next line is indented one level more. */
  export const OPENS_CLARITY_BLOCK = /^\s*(handler|if|else|for|function|action|engine|page|app|code|entity|enum|tabs|tab|panel|splitter|pane|menu|toolbar|ribbon|group|sidebar|footer|item)\b/;
  /** Lines of Rust (or another braced language) that open a block: ending in an open brace or bracket. */
  export const OPENS_BRACED_BLOCK = /[{([]\s*$/;
</script>

<script lang="ts">
  // Clarity text in CodeMirror, with line numbers, colors and problems from
  // the backend while it is typed, search (Ctrl+F), undo, and Save and Revert. What it edits (a window's design, its code
  // behind, a function) is up to the caller, through `load`, `check` and
  // `save`; the backend decides what is valid.

  import { onDestroy } from "svelte";
  import { Compartment, EditorState, RangeSetBuilder, StateEffect, StateField, type Extension } from "@codemirror/state";
  import { Decoration, EditorView, keymap, lineNumbers, highlightActiveLine, highlightActiveLineGutter, drawSelection, placeholder as placeholderText, type DecorationSet } from "@codemirror/view";
  import { defaultKeymap, history, historyKeymap, indentLess } from "@codemirror/commands";
  import { bracketMatching, foldGutter, indentUnit } from "@codemirror/language";
  import { highlightSelectionMatches, search, searchKeymap } from "@codemirror/search";
  import { lintGutter, setDiagnostics, type Diagnostic as Marked } from "@codemirror/lint";
  import { rust } from "@codemirror/lang-rust";
  import { invoke } from "./backend";
  import { ideLook } from "./codeMirrorLook";
  import type { Diagnostic } from "./generated/Diagnostic";
  import type { Piece } from "./generated/Piece";
  import type { PieceKind } from "./generated/PieceKind";

  let {
    load,
    check,
    save,
    reloadKey,
    navPattern = null,
    placeholder = "",
    opensBlock = OPENS_CLARITY_BLOCK,
    errorsBlockSave = true,
    language = "clarity",
    readonly = false,
    ondirty = () => {},
    ontext = () => {},
    onclose = () => {},
  }: {
    load: () => Promise<{ text: string; problems: Diagnostic[] }>;
    /** The problems the text would have; called while it is typed. */
    check: (text: string) => Promise<Diagnostic[]>;
    /** Writes the text; throws with the reason when it is refused. */
    save: (text: string) => Promise<void>;
    /** Loads again when this changes, unless there are unsaved changes. */
    reloadKey: string;
    /** Lines worth a jump button, with the name in the first group. */
    navPattern?: RegExp | null;
    placeholder?: string;
    /** Lines after which Enter indents one level more; Clarity's block lines unless given. */
    opensBlock?: RegExp;
    /** Whether errors keep the text from being saved: true for Clarity, which must be valid; false for a compiler's errors, which saving and building fix. */
    errorsBlockSave?: boolean;
    /** How the text is colored: Clarity as the lexer reads it, Rust, or not at all. */
    language?: "clarity" | "rust" | "plain";
    /** Shown to read, not to change: no Save, Revert or problems. */
    readonly?: boolean;
    ondirty?: (dirty: boolean) => void;
    /** The text as it is typed, saved or not, for a view that follows it. */
    ontext?: (text: string) => void;
    onclose?: () => void;
  } = $props();

  const INDENT = "    ";
  const CHECK_DELAY = 300;
  /** How long typing rests before the colors are asked again. */
  const PAINT_DELAY = 40;

  let saved = $state("");
  let text = $state("");
  let problems = $state<Diagnostic[]>([]);
  let error = $state<string | null>(null);
  let saving = $state(false);
  let host = $state<HTMLDivElement | null>(null);
  let view = $state.raw<EditorView | null>(null);

  const dirty = $derived(text !== saved);
  const errorCount = $derived(problems.filter((p) => p.severity === "error").length);
  const jumps = $derived(
    navPattern === null
      ? []
      : text
          .split("\n")
          .map((line, i) => ({ name: navPattern.exec(line)?.[1], line: i + 1 }))
          .filter((j): j is { name: string; line: number } => j.name !== undefined),
  );

  $effect(() => ondirty(dirty));
  $effect(() => ontext(text));

  async function reload() {
    try {
      const loaded = await load();
      saved = loaded.text;
      show(loaded.text);
      problems = loaded.problems;
      error = null;
    } catch (err) {
      error = String(err);
    }
  }

  let loadedFor: string | null = null;
  $effect(() => {
    const key = reloadKey;
    if (key !== loadedFor && (loadedFor === null || !dirty)) {
      loadedFor = key;
      reload();
    }
  });

  /** The class of each piece Clarity's lexer finds. */
  const PIECE_CLASSES: Record<PieceKind, string> = {
    keyword: "cm-clarity-keyword",
    type: "cm-clarity-type",
    name: "",
    literal: "cm-clarity-literal",
    number: "cm-clarity-number",
    text: "cm-clarity-text",
    slot: "cm-clarity-slot",
    key: "cm-clarity-key",
    comment: "cm-clarity-comment",
  };

  const painted = StateEffect.define<DecorationSet>();
  /** The colors of the text as last asked, moved along with what is typed since. */
  const paint = StateField.define<DecorationSet>({
    create: () => Decoration.none,
    update: (marks, tr) => {
      for (const e of tr.effects) if (e.is(painted)) return e.value;
      return marks.map(tr.changes);
    },
    provide: (field) => EditorView.decorations.from(field),
  });

  function marksOf(state: EditorState, pieces: Piece[]): DecorationSet {
    const builder = new RangeSetBuilder<Decoration>();
    const sorted = [...pieces].sort((a, b) => a.line - b.line || a.from - b.from);
    for (const p of sorted) {
      const name = PIECE_CLASSES[p.kind];
      if (!name || p.line < 1 || p.line > state.doc.lines) continue;
      const line = state.doc.line(p.line);
      const from = line.from + Math.min(p.from, line.length);
      const to = line.from + Math.min(p.to, line.length);
      if (to > from) builder.add(from, to, Decoration.mark({ class: name }));
    }
    return builder.finish();
  }

  // Color Clarity while it is typed, as the backend's lexer reads it.
  $effect(() => {
    const draft = text;
    if (language !== "clarity") return;
    const timer = setTimeout(async () => {
      try {
        const pieces = await invoke<Piece[]>("highlight", { text: draft });
        if (view && view.state.doc.toString() === draft) view.dispatch({ effects: painted.of(marksOf(view.state, pieces)) });
      } catch {
        // Without the backend the text stays plain.
      }
    }, PAINT_DELAY);
    return () => clearTimeout(timer);
  });

  // The problems, marked in the text and its gutter.
  $effect(() => {
    const marked = problems;
    if (!view) return;
    const doc = view.state.doc;
    const diagnostics: Marked[] = marked
      .filter((p) => p.span.line >= 1 && p.span.line <= doc.lines)
      .map((p) => {
        const line = doc.line(p.span.line);
        const from = Math.min(line.from + Math.max(p.span.col - 1, 0), line.to);
        return { from, to: Math.max(from, line.to), severity: p.severity === "error" ? "error" : "warning", message: p.message };
      });
    view.dispatch(setDiagnostics(view.state, diagnostics));
  });

  // Check the text while it is typed.
  $effect(() => {
    const draft = text;
    if (draft === saved) return;
    const timer = setTimeout(async () => {
      try {
        problems = await check(draft);
        error = null;
      } catch (err) {
        error = String(err);
      }
    }, CHECK_DELAY);
    return () => clearTimeout(timer);
  });

  async function store() {
    if (!dirty || saving) return;
    saving = true;
    try {
      await save(text);
      saved = text;
      error = null;
    } catch (err) {
      error = String(err);
    } finally {
      saving = false;
    }
  }

  /** Saves unsaved changes, as before a build; true when the text is saved afterwards. */
  export async function saveIfChanged(): Promise<boolean> {
    await store();
    return !dirty;
  }

  function revert() {
    show(saved);
    error = null;
    reload();
  }

  /** Puts the caret on `line` (1-based); `inside` moves to the line after it. */
  export function goToLine(line: number, inside = false) {
    if (!view) return;
    const doc = view.state.doc;
    const at = doc.line(Math.max(1, Math.min(inside ? line + 1 : line, doc.lines))).to;
    view.focus();
    view.dispatch({ selection: { anchor: at }, effects: EditorView.scrollIntoView(at, { y: "start", yMargin: 60 }) });
  }

  /** Jumps into the block whose jump button has `name`; false when there is none. */
  export function jumpTo(name: string): boolean {
    const target = jumps.find((j) => j.name === name);
    if (target) goToLine(target.line, true);
    return target !== undefined;
  }

  /** Puts `value` in the editor, as loading or reverting does. */
  function show(value: string) {
    text = value;
    if (view && view.state.doc.toString() !== value) view.dispatch({ changes: { from: 0, to: view.state.doc.length, insert: value } });
  }

  /** Enter keeps the indentation, one level more after a line that opens a block. */
  function newline(target: EditorView): boolean {
    const { state } = target;
    const range = state.selection.main;
    const line = state.doc.lineAt(range.from);
    const before = line.text.slice(0, range.from - line.from);
    const indent = /^ */.exec(before)?.[0] ?? "";
    const insert = "\n" + indent + (opensBlock.test(before) ? INDENT : "");
    target.dispatch(state.update(state.replaceSelection(insert), { scrollIntoView: true, userEvent: "input" }));
    return true;
  }

  /** Tab puts in one level, at the caret or before each chosen line. */
  function tab(target: EditorView): boolean {
    const { state } = target;
    if (state.selection.ranges.some((r) => !r.empty && state.doc.lineAt(r.from).number !== state.doc.lineAt(r.to).number)) {
      const changes = [];
      for (const r of state.selection.ranges) {
        for (let n = state.doc.lineAt(r.from).number; n <= state.doc.lineAt(r.to).number; n++) changes.push({ from: state.doc.line(n).from, insert: INDENT });
      }
      target.dispatch({ changes, userEvent: "input.indent" });
    } else {
      target.dispatch(state.update(state.replaceSelection(INDENT), { scrollIntoView: true, userEvent: "input" }));
    }
    return true;
  }

  function languageOf(of: typeof language): Extension {
    return of === "rust" ? rust() : [];
  }

  const languages = new Compartment();
  const editing = new Compartment();

  function editable(): Extension {
    return [EditorState.readOnly.of(readonly), EditorView.editable.of(!readonly), placeholderText(placeholder)];
  }

  $effect(() => {
    if (!host || view) return;
    view = new EditorView({
      parent: host,
      state: EditorState.create({
        doc: text,
        extensions: [
          lineNumbers(),
          foldGutter(),
          lintGutter(),
          highlightActiveLine(),
          highlightActiveLineGutter(),
          drawSelection(),
          history(),
          bracketMatching(),
          highlightSelectionMatches(),
          search({ top: true }),
          indentUnit.of(INDENT),
          EditorState.tabSize.of(4),
          keymap.of([
            { key: "Mod-s", preventDefault: true, run: () => (void store(), true) },
            { key: "Escape", run: () => (onclose(), false) },
            { key: "Enter", run: newline },
            { key: "Tab", run: tab, shift: indentLess },
            ...searchKeymap,
            ...historyKeymap,
            ...defaultKeymap,
          ]),
          languages.of(languageOf(language)),
          editing.of(editable()),
          paint,
          ideLook,
          EditorView.updateListener.of((update) => {
            if (update.docChanged) text = update.state.doc.toString();
          }),
        ],
      }),
    });
  });

  $effect(() => {
    view?.dispatch({ effects: [languages.reconfigure(languageOf(language)), editing.reconfigure(editable())] });
  });

  onDestroy(() => view?.destroy());
</script>

<div class="source">
  {#if !readonly}
  <div class="bar">
    {#if jumps.length > 0}
      <nav class="jumps">
        {#each jumps as j (j.line)}
          <button onclick={() => goToLine(j.line, true)}>{j.name}</button>
        {/each}
      </nav>
    {:else}
      <span></span>
    {/if}
    <div class="actions">
      {#if dirty}<span class="dirty">unsaved</span>{/if}
      <button disabled={!dirty} onclick={revert} title="Put back the saved text">Revert</button>
      <button
        class="primary"
        disabled={!dirty || saving || (errorsBlockSave && errorCount > 0)}
        onclick={store}
        title={errorsBlockSave && errorCount > 0 ? "Fix the errors first" : "Save (Ctrl+S)"}>{saving ? "Saving..." : "Save"}</button
      >
    </div>
  </div>
  {/if}

  <div class="editor" bind:this={host}></div>

  {#if !readonly}
  <footer>
    {#if error}<p class="refused" role="alert">{error}</p>{/if}
    <ul>
      {#each problems as p, i (i)}
        <li class={p.severity}>
          <button onclick={() => goToLine(p.span.line)}><code>{p.span.line}:{p.span.col}</code> {p.message}</button>
        </li>
      {:else}
        <li class="ok">No problems</li>
      {/each}
    </ul>
  </footer>
  {/if}
</div>

<style>
  .source {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 0;
  }

  .bar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 8px;
    padding: 6px 12px;
    border-bottom: 1px solid var(--border);
  }

  .jumps {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }

  .jumps button {
    font-family: var(--mono);
    font-size: 11px;
    padding: 1px 8px;
  }

  .actions {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-shrink: 0;
  }

  .dirty {
    font-size: 11px;
    color: var(--warning, #d9a400);
  }

  .editor {
    flex: 1;
    min-height: 0;
    overflow: hidden;
  }

  .editor :global(.cm-editor) {
    height: 100%;
  }

  footer {
    max-height: 30%;
    overflow: auto;
    border-top: 1px solid var(--border);
    font-size: 12px;
  }

  .refused {
    margin: 0;
    padding: 6px 12px;
    color: var(--error);
  }

  ul {
    margin: 0;
    padding: 4px 0;
    list-style: none;
  }

  li button {
    width: 100%;
    text-align: left;
    border: none;
    background: none;
    border-radius: 0;
    padding: 3px 12px;
    color: inherit;
    font: inherit;
  }

  li button:hover {
    background: var(--panel-alt);
  }

  li.error {
    color: var(--error);
  }

  li.warning {
    color: var(--warning, #d9a400);
  }

  li.ok {
    padding: 3px 12px;
    color: var(--text-muted);
  }

  li code {
    font-family: var(--mono);
    margin-right: 6px;
  }
</style>
