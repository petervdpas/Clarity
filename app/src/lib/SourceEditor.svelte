<script lang="ts" module>
  /** Clarity lines after which the next line is indented one level more. */
  export const OPENS_CLARITY_BLOCK = /^\s*(handler|if|else|for|function|action|engine|page|app|code|entity|enum|tabs|tab|panel|splitter|pane|menu|toolbar|ribbon|group|sidebar|footer|item)\b/;
  /** Lines of Rust (or another braced language) that open a block: ending in an open brace or bracket. */
  export const OPENS_BRACED_BLOCK = /[{([]\s*$/;
</script>

<script lang="ts">
  // Clarity text with line numbers, problems from the backend while it is
  // typed, and Save and Revert. What it edits (a window's design, its code
  // behind, a function) is up to the caller, through `load`, `check` and
  // `save`; the backend decides what is valid.

  import { invoke } from "./backend";
  import type { Diagnostic } from "./generated/Diagnostic";
  import type { Piece } from "./generated/Piece";
  import { linesOf, rustPieces, type Segment } from "./highlight";

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
    ondirty = () => {},
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
    ondirty?: (dirty: boolean) => void;
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
  let editor: HTMLTextAreaElement | undefined = $state();
  let gutter: HTMLElement | undefined = $state();
  let paint: HTMLElement | undefined = $state();
  /** The colored runs of the lines last colored, by their text. */
  let colored = $state.raw(new Map<string, Segment[]>());
  const painted = $derived(text.split("\n").map((line) => colored.get(line) ?? [{ text: line, kind: null }]));

  const dirty = $derived(text !== saved);
  const lineCount = $derived(text.split("\n").length);
  const errorLines = $derived(new Set(problems.filter((p) => p.severity === "error").map((p) => p.span.line)));
  const warningLines = $derived(new Set(problems.filter((p) => p.severity === "warning").map((p) => p.span.line)));
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

  async function reload() {
    try {
      const view = await load();
      saved = view.text;
      text = view.text;
      problems = view.problems;
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

  // Color the text while it is typed; a line typed since shows plain until then.
  $effect(() => {
    const draft = text;
    if (language === "plain") return;
    if (language === "rust") {
      colored = linesOf(draft, rustPieces(draft));
      return;
    }
    const timer = setTimeout(async () => {
      try {
        colored = linesOf(draft, await invoke<Piece[]>("highlight", { text: draft }));
      } catch {
        // Without the backend the text stays plain.
      }
    }, PAINT_DELAY);
    return () => clearTimeout(timer);
  });

  function scrolled() {
    if (!editor) return;
    if (gutter) gutter.scrollTop = editor.scrollTop;
    if (paint) {
      paint.scrollTop = editor.scrollTop;
      paint.scrollLeft = editor.scrollLeft;
    }
  }

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
    text = saved;
    error = null;
    reload();
  }

  /** Puts the caret on `line` (1-based); `inside` moves to the line after it. */
  export function goToLine(line: number, inside = false) {
    if (!editor) return;
    const lines = text.split("\n");
    const index = Math.min(inside ? line : line - 1, lines.length - 1);
    const offset = lines.slice(0, index).reduce((sum, l) => sum + l.length + 1, 0) + (lines[index]?.length ?? 0);
    editor.focus();
    editor.setSelectionRange(offset, offset);
    const lineHeight = parseFloat(getComputedStyle(editor).lineHeight) || 18;
    editor.scrollTop = Math.max(0, (index - 3) * lineHeight);
  }

  /** Jumps into the block whose jump button has `name`; false when there is none. */
  export function jumpTo(name: string): boolean {
    const target = jumps.find((j) => j.name === name);
    if (target) goToLine(target.line, true);
    return target !== undefined;
  }

  /** Tab indents, Enter keeps the indentation (one level more after a block line), Ctrl+S saves. */
  function onkeydown(e: KeyboardEvent) {
    if (!editor) return;
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "s") {
      e.preventDefault();
      store();
      return;
    }
    if (e.key === "Escape") {
      onclose();
      return;
    }
    const start = editor.selectionStart;
    const end = editor.selectionEnd;
    if (e.key === "Tab" && !e.shiftKey) {
      e.preventDefault();
      insert(start, end, INDENT);
    } else if (e.key === "Enter") {
      e.preventDefault();
      const lineStart = text.lastIndexOf("\n", start - 1) + 1;
      const line = text.slice(lineStart, start);
      const indent = /^ */.exec(line)?.[0] ?? "";
      insert(start, end, "\n" + indent + (opensBlock.test(line) ? INDENT : ""));
    }
  }

  function insert(start: number, end: number, value: string) {
    text = text.slice(0, start) + value + text.slice(end);
    const caret = start + value.length;
    requestAnimationFrame(() => editor?.setSelectionRange(caret, caret));
  }
</script>

<div class="source">
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

  <div class="editor">
    <div class="gutter" bind:this={gutter} aria-hidden="true">
      {#each { length: lineCount } as _, i (i)}
        <div class:error={errorLines.has(i + 1)} class:warning={!errorLines.has(i + 1) && warningLines.has(i + 1)}>{i + 1}</div>
      {/each}
    </div>
    <div class="surface">
      <pre class="paint" bind:this={paint} aria-hidden="true">{#each painted as line, i (i)}{#each line as run, j (j)}{#if run.kind}<span
                class={run.kind}>{run.text}</span
              >{:else}{run.text}{/if}{/each}{"\n"}{/each}{" "}</pre>
      <textarea
        class:colored={language !== "plain"}
        bind:this={editor}
        bind:value={text}
        {onkeydown}
        onscroll={scrolled}
        spellcheck="false"
        wrap="off"
        {placeholder}
      ></textarea>
    </div>
  </div>

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
    display: flex;
    min-height: 0;
    font-family: var(--mono);
    font-size: 13px;
    line-height: 19px;
  }

  .gutter {
    overflow: hidden;
    padding: 8px 8px 8px 12px;
    text-align: right;
    color: var(--text-muted);
    background: var(--panel-alt);
    user-select: none;
    min-width: 36px;
  }

  .gutter .error {
    color: var(--error);
    font-weight: 700;
  }

  .gutter .warning {
    color: var(--warning, #d9a400);
  }

  /* The text area on top takes the typing; the colored copy under it shows. */
  .surface {
    flex: 1;
    position: relative;
    min-width: 0;
    background: var(--bg, var(--panel));
  }

  textarea,
  .paint {
    position: absolute;
    inset: 0;
    margin: 0;
    padding: 8px 12px;
    font: inherit;
    line-height: inherit;
    tab-size: 4;
    white-space: pre;
    border: none;
  }

  textarea {
    width: 100%;
    height: 100%;
    resize: none;
    border-radius: 0;
    background: transparent;
    color: var(--text);
    outline: none;
  }

  textarea.colored {
    color: transparent;
    caret-color: var(--text);
  }

  textarea::placeholder {
    color: var(--text-muted);
  }

  .paint {
    overflow: hidden;
    color: var(--text);
    pointer-events: none;
  }

  .paint .keyword {
    color: var(--code-keyword);
  }

  .paint .type {
    color: var(--code-type);
  }

  .paint .literal,
  .paint .number {
    color: var(--code-literal);
  }

  .paint .text {
    color: var(--code-text);
  }

  .paint .slot {
    color: var(--code-slot);
  }

  .paint .key {
    color: var(--code-key);
  }

  .paint .comment {
    color: var(--code-comment);
    font-style: italic;
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
