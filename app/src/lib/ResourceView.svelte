<script lang="ts">
  // A resource of the project, opened from Resources under i18n in the
  // project tree: a Markdown file to write, with how it reads beside it, a
  // picture to look at, or a PDF, drawing or other file the application
  // carries. Replace file puts a picked file of its type in its place. It
  // says which keys point a language at it.

  import { invoke } from "@tauri-apps/api/core";
  import { open as openDialog } from "@tauri-apps/plugin-dialog";
  import type { Preview } from "./generated/Preview";
  import MarkdownView from "./ui/MarkdownView.svelte";
  import { openLink } from "./backend";
  import { loadPicturesWith, pictureUrl } from "./pictures.svelte";
  import { resourceKind, resourceUses } from "./resources";
  import { resourceKinds } from "./resourceKinds.svelte";

  let { preview, name, onapplied }: { preview: Preview; name: string; onapplied: (preview: Preview) => void } = $props();

  const kind = $derived(resourceKind(name, resourceKinds.list));
  const typeName = $derived(resourceKinds.list.find((k) => k.kind === kind)?.name ?? "File");
  /** The extension a replacing file must have: the resource's own. */
  const extension = $derived(name.includes(".") ? (name.split(".").at(-1) ?? "") : "");
  const saved = $derived(preview.project.resources?.markdown[name] ?? "");
  const uses = $derived(resourceUses(preview.project.resource_keys, name));

  /** The text being written; it starts as the file is, and again when another file opens. */
  let text = $state("");
  let shownName = $state("");
  $effect.pre(() => {
    if (name !== shownName) {
      shownName = name;
      text = saved;
    }
  });
  const changed = $derived(text !== saved);
  let error = $state<string | null>(null);

  async function save() {
    try {
      error = null;
      onapplied(await invoke<Preview>("save_markdown", { name, text }));
    } catch (err) {
      error = String(err);
    }
  }

  /** Puts a picked file of the resource's type in its place; the name stays. */
  async function replace() {
    const picked = await openDialog({ multiple: false, filters: extension ? [{ name: typeName, extensions: [extension] }] : [] });
    if (typeof picked !== "string") return;
    try {
      error = null;
      const next = await invoke<Preview>("replace_resource_file", { name, from: picked });
      if (kind === "markdown") text = next.project.resources?.markdown[name] ?? "";
      // The picture shows anew, not as it was loaded.
      if (kind === "picture") loadPicturesWith((picture) => invoke<string>("project_picture", { name: picture }));
      onapplied(next);
    } catch (err) {
      error = String(err);
    }
  }

  function keydown(e: KeyboardEvent) {
    if ((e.ctrlKey || e.metaKey) && e.key === "s") {
      e.preventDefault();
      if (changed) save();
    }
  }
</script>

<section class="resource">
  <header>
    <h2>{name}</h2>
    <span class="kind">{typeName} in resources/</span>
    <button onclick={replace} title="Put a picked .{extension || "file"} in its place; its name stays">Replace file...</button>
    {#if kind === "markdown"}
      <button class="primary" onclick={save} disabled={!changed} title="Save the file (Ctrl+S)">Save</button>
    {/if}
  </header>
  <p class="uses">
    {#if uses.length > 0}
      Pointed at by {uses.join(", ")}.
    {:else}
      No key points at it; a part can still show it by its Source.
    {/if}
  </p>
  {#if error}<p class="error" role="alert">{error}</p>{/if}

  {#if kind === "markdown"}
    <div class="sides">
      <textarea bind:value={text} onkeydown={keydown} spellcheck="true" aria-label="Markdown of {name}"></textarea>
      <div class="shown"><MarkdownView text={text} onlink={openLink} /></div>
    </div>
  {:else if kind === "picture"}
    {@const url = pictureUrl(name)}
    <div class="picture">
      {#if url}<img src={url} alt={name} />{:else}<p class="uses">Loading the picture...</p>{/if}
    </div>
  {:else}
    <p class="uses">The application carries this {typeName === "Other file" ? "file" : typeName} as it is{kind === "drawing" ? "; an image does not show a drawing yet" : ""}.</p>
  {/if}
</section>

<style>
  .resource {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 12px 16px;
    height: 100%;
    min-height: 0;
    box-sizing: border-box;
  }

  header {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  h2 {
    margin: 0;
    font-size: 15px;
  }

  .kind {
    flex: 1;
    color: var(--text-muted);
    font-size: 12px;
  }

  .uses,
  .error {
    margin: 0;
    font-size: 12px;
    color: var(--text-muted);
  }

  .error {
    color: var(--error);
  }

  .sides {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
  }

  textarea {
    resize: none;
    font-family: var(--mono);
    font-size: 13px;
    padding: 8px;
    min-height: 0;
  }

  .shown {
    min-height: 0;
    display: flex;
    padding: 8px 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--panel);
  }

  .picture {
    flex: 1;
    min-height: 0;
    display: grid;
    place-items: center;
  }

  .picture img {
    max-width: 100%;
    max-height: 100%;
    object-fit: contain;
  }
</style>
