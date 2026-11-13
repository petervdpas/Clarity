<script lang="ts">
  // A view's tells (DESIGN 5.33): what it tells the window it shows in,
  // `saved(name: text)`, each declared on the view and changed or taken away
  // here; the view's code says `tell saved(name)`.

  import type { Edit } from "@clarity/ui/generated/Edit.ts";
  import type { Page } from "@clarity/ui/generated/Page.ts";
  import { tellName, tellText } from "./viewTells";

  let { page, onedit }: { page: Page; onedit: (edit: Edit) => void } = $props();

  let message = $state("");
  const name = $derived(tellName(message));

  /** Declares the tell `name` as `text`, in place of one of that name; `null` takes it away. */
  function setTell(old: string, text: string | null) {
    onedit({ op: "setPageTell", page: page.span.start, name: old, message: text });
  }

  function add(e: SubmitEvent) {
    e.preventDefault();
    if (!name) return;
    setTell(name, message.trim());
    message = "";
  }
</script>

{#if page.kind === "view"}
  <fieldset class="tells">
    <legend>Tells</legend>
    {#each page.tells as tell (tell.name.name)}
      <div class="row">
        <input
          class="mono"
          value={tellText(tell)}
          spellcheck="false"
          title="What it tells and the values it gives, as code tells it"
          onchange={(e) => {
            const text = e.currentTarget.value.trim();
            if (text && text !== tellText(tell)) setTell(tell.name.name, text);
          }}
        />
        <button class="small" title="Take the tell away" onclick={() => setTell(tell.name.name, null)}>&times;</button>
      </div>
    {/each}
    <form class="row" onsubmit={add}>
      <input class="mono" bind:value={message} placeholder="saved(name: text)" spellcheck="false" />
      <button class="small" type="submit" disabled={!name}>Add</button>
    </form>
    <p class="hint">The view's code says <code>tell {page.tells[0]?.name.name ?? "saved"}(...)</code>; the window wires it on the content's view line.</p>
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
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 6px;
    align-items: center;
  }

  .mono {
    font-family: var(--mono);
    font-size: 12px;
  }

  .hint {
    margin: 0;
    font-size: 12px;
    color: var(--text-muted);
  }
</style>
