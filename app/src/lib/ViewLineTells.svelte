<script lang="ts">
  // The tells of the view a content's view line shows (DESIGN 5.33), each
  // wired to a handler of the window that takes the tell's values, as a
  // custom part's tells are.

  import type { Edit } from "@clarity/ui/generated/Edit.ts";
  import type { Preview } from "@clarity/ui/generated/Preview.ts";
  import type { WidgetPage } from "@clarity/ui/generated/WidgetPage.ts";
  import { paramsText } from "@clarity/ui/screen/customParts.ts";
  import { pageDesigner } from "./pageDesignerStore.svelte";
  import { tellHandlerName } from "./viewTells";

  let {
    preview,
    line,
    owner,
    onedit,
  }: {
    preview: Preview;
    /** The view line of the content. */
    line: WidgetPage;
    /** The window whose handlers hear the tells. */
    owner: string;
    onedit: (edit: Edit) => void;
  } = $props();

  const view = $derived(line.view === null ? null : (preview.project.files.flatMap((f) => f.pages).find((p) => p.name.name === line.view && p.kind === "view") ?? null));

  /** Wires a handler, taking `params`, to a tell; `null` unwires it. */
  function wire(tell: string, handler: string | null, params: string) {
    const edit: Edit = { op: "setViewTell", target: line.span.start, tell, handler };
    if (handler === null) {
      onedit(edit);
      return;
    }
    onedit({ op: "batch", edits: [edit, { op: "addHandlerWith", owner, name: handler, params }] });
    pageDesigner.showCode(handler);
  }
</script>

{#if view && view.tells.length > 0}
  <fieldset class="tells">
    <legend>Tells of {view.name.name}</legend>
    {#each view.tells as tell (tell.name.name)}
      {@const handler = line.tells.find((t) => t.tell === tell.name.name)?.handler ?? ""}
      <div class="row">
        <span class="name">{tell.name.name}</span>
        <input
          value={handler}
          placeholder="handler"
          spellcheck="false"
          onchange={(e) => wire(tell.name.name, e.currentTarget.value.trim() || null, paramsText(tell.params))}
        />
        {#if handler}
          <span class="buttons">
            <button class="small" title="Show the handler in the code behind" onclick={() => pageDesigner.showCode(handler)}>Code</button>
            <button class="small" title="Unwire" onclick={() => wire(tell.name.name, null, "")}>&times;</button>
          </span>
        {:else}
          <button
            class="small"
            title="Wire a handler named after the view and the tell"
            onclick={() => view && wire(tell.name.name, tellHandlerName(view.name.name, tell.name.name), paramsText(tell.params))}>Wire</button
          >
        {/if}
      </div>
    {/each}
    <p class="hint">A handler wired to a tell takes the tell's values.</p>
  </fieldset>
{/if}

<style>
  .tells {
    border: 1px solid var(--border);
    border-radius: var(--radius);
    margin: 0;
    padding: 6px 10px 10px;
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 13px;
  }

  legend {
    font-size: 12px;
    color: var(--text-muted);
    padding: 0 4px;
  }

  .row {
    display: grid;
    grid-template-columns: 56px minmax(0, 1fr) auto;
    gap: 6px;
    align-items: center;
  }

  .name {
    font-family: var(--mono);
    font-size: 12px;
    color: var(--text-muted);
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .buttons {
    display: flex;
    gap: 4px;
  }

  .hint {
    margin: 0;
    font-size: 12px;
    color: var(--text-muted);
  }
</style>
