<script lang="ts">
  // A toast the project declares (`toast saved`): its text (the key and its
  // words in the default language), how long it shows, its level, its icon
  // and its corner, as a form; each change writes one line of the
  // declaration. A sample shows it as the screen does. Code shows it with
  // `toast saved`, filling the slots its text has.

  import IconField from "./IconField.svelte";
  import Icon from "@clarity/ui/parts/Icon.svelte";
  import type { Edit } from "@clarity/ui/generated/Edit.ts";
  import type { Preview } from "@clarity/ui/generated/Preview.ts";
  import type { SpecFile } from "@clarity/ui/generated/SpecFile.ts";
  import type { ToastDecl } from "@clarity/ui/generated/ToastDecl.ts";
  import { TOAST_CORNERS, TOAST_LEVELS, TOAST_LIFESPAN, cornerLine, iconLine, levelColor, levelLine, lifespanLine, textLine, type ToastLine } from "./toastLines";

  let {
    preview,
    decl,
    file,
    onapply,
    ontext,
    ondelete,
  }: {
    preview: Preview;
    decl: ToastDecl;
    /** The module that declares it. */
    file: SpecFile;
    onapply: (edit: Edit) => Promise<void>;
    /** Sets the words of `key` in the default language. */
    ontext: (key: string, value: string) => Promise<void>;
    ondelete: () => void;
  } = $props();

  const item = $derived(decl.span.start);
  const name = $derived(decl.name.name);
  const key = $derived(decl.text?.name ?? "");
  const language = $derived(preview.project.locale.language ?? "en");
  const words = $derived(key ? (preview.project.translations[language]?.[key] ?? "") : "");
  const lifespan = $derived(decl.lifespan ?? TOAST_LIFESPAN.default);
  let error = $state<string | null>(null);

  async function set(line: ToastLine | string) {
    error = null;
    if (typeof line === "string") {
      error = line;
      return;
    }
    try {
      await onapply({ op: "setDeclLine", item, key: line.key, line: line.line });
    } catch (err) {
      error = String(err);
    }
  }

  async function setWords(value: string) {
    error = null;
    if (!key) {
      error = "Give the toast a text key first.";
      return;
    }
    try {
      await ontext(key, value);
    } catch (err) {
      error = String(err);
    }
  }
</script>

<div class="toast-view">
  <header>
    <strong>{name}</strong>
    <span class="muted">toast in module {file.module?.name ?? ""}; code shows it with <code>toast {name}</code></span>
    <button class="delete" onclick={ondelete} title="Take the toast away">Delete</button>
  </header>
  {#if error}<p class="refused" role="alert">{error}</p>{/if}

  <div class="form">
    <label for="toast-key">Text key</label>
    <input id="toast-key" value={key} placeholder="as {name}Toast" onchange={(e) => set(textLine(e.currentTarget.value))} />

    <label for="toast-words">Text ({language})</label>
    <input id="toast-words" value={words} placeholder="as Saved {'{'}name{'}'}" onchange={(e) => setWords(e.currentTarget.value)} />
    <p class="hint">Slots in braces, like <code>{"{name}"}</code>, are filled by code: <code>toast {name}(name = customer.name)</code>.</p>

    <label for="toast-lifespan">Lifespan</label>
    <span class="with-unit">
      <input id="toast-lifespan" type="number" min={TOAST_LIFESPAN.least} max={TOAST_LIFESPAN.most} step="100" value={lifespan} onchange={(e) => set(lifespanLine(Number(e.currentTarget.value)))} />
      <span class="muted">milliseconds</span>
    </span>

    <label for="toast-level">Level</label>
    <select id="toast-level" value={decl.level} onchange={(e) => set(levelLine(e.currentTarget.value as ToastDecl["level"]))}>
      {#each TOAST_LEVELS as level (level.value)}<option value={level.value}>{level.label}</option>{/each}
    </select>

    <span class="label">Icon</span>
    <IconField {preview} icon={decl.icon} onwrite={(icon) => set(iconLine(icon))} />

    <span class="label">Corner</span>
    <div class="corners" role="radiogroup" aria-label="The corner of the window it shows in">
      {#each TOAST_CORNERS as corner (corner.value)}
        <button role="radio" aria-checked={decl.corner === corner.value} class:chosen={decl.corner === corner.value} onclick={() => set(cornerLine(corner.value))}>{corner.label}</button>
      {/each}
    </div>
  </div>

  <div class="sample-window" aria-label="How the toast shows">
    <div class="sample {decl.corner}" style:--edge={levelColor(decl.level)}>
      {#if decl.icon}<Icon paths={preview.icons} name={decl.icon} />{/if}
      <span>{words || key || name}</span>
      <span class="x">x</span>
    </div>
    <span class="sample-note muted">goes after {lifespan / 1000} s</span>
  </div>
</div>

<style>
  .toast-view {
    display: flex;
    flex-direction: column;
    gap: 14px;
    padding: 16px;
    overflow: auto;
  }
  header {
    display: flex;
    align-items: baseline;
    gap: 10px;
  }
  header .delete {
    margin-left: auto;
  }
  .muted {
    color: var(--text-muted);
    font-size: 12px;
  }
  .refused {
    color: var(--error);
    margin: 0;
  }
  .form {
    display: grid;
    grid-template-columns: max-content minmax(0, 420px);
    gap: 8px 12px;
    align-items: center;
  }
  .form .hint {
    grid-column: 2;
    margin: -4px 0 0;
    font-size: 12px;
    color: var(--text-muted);
  }
  .with-unit {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .with-unit input {
    width: 110px;
  }
  .corners {
    display: grid;
    grid-template-columns: repeat(2, max-content);
    gap: 4px;
  }
  .corners .chosen {
    border-color: var(--accent);
    font-weight: 600;
  }
  .sample-window {
    position: relative;
    height: 150px;
    max-width: 560px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg);
  }
  .sample {
    position: absolute;
    display: flex;
    align-items: center;
    gap: 8px;
    max-width: 260px;
    padding: 8px 10px 8px 14px;
    border: 1px solid var(--border);
    border-left: 4px solid var(--edge);
    border-radius: 6px;
    background: var(--panel);
    font-size: 13px;
  }
  .sample :global(svg) {
    color: var(--edge);
  }
  .sample .x {
    margin-left: 8px;
    color: var(--text-muted);
  }
  .sample.topRight {
    top: 10px;
    right: 10px;
  }
  .sample.topLeft {
    top: 10px;
    left: 10px;
  }
  .sample.bottomRight {
    bottom: 10px;
    right: 10px;
  }
  .sample.bottomLeft {
    bottom: 10px;
    left: 10px;
  }
  .sample-note {
    position: absolute;
    left: 50%;
    top: 50%;
    transform: translate(-50%, -50%);
  }
</style>
