<script lang="ts">
  // The markdown part: Markdown read into blocks the same way on every stack
  // (markdownBlocks.ts, conformance/markdown.json), shown as headings,
  // paragraphs, list items and code, wrapped, scrolling when longer than the
  // part. A link opens in the browser, through `onlink`.
  import { markdownBlocks, type Span } from "../markdownBlocks";

  let { text, id, onlink }: { text: string; id?: string; onlink: (address: string) => void } = $props();

  const blocks = $derived(markdownBlocks(text));

  function follow(event: MouseEvent, address: string) {
    event.preventDefault();
    onlink(address);
  }
</script>

{#snippet spans(list: Span[])}
  {#each list as span, i (i)}
    {#if span.link}
      <a href={span.link} class:bold={span.bold} class:italic={span.italic} class:code={span.code} onclick={(e) => follow(e, span.link!)}>{span.text}</a>
    {:else}
      <span class:bold={span.bold} class:italic={span.italic} class:code={span.code}>{span.text}</span>
    {/if}
  {/each}
{/snippet}

<div class="markdown" {id}>
  {#each blocks as block, i (i)}
    {#if block.kind === "heading"}
      <svelte:element this={`h${Math.min(block.level, 6)}`} class="heading level{Math.min(block.level, 3)}">{@render spans(block.spans)}</svelte:element>
    {:else if block.kind === "paragraph"}
      <p>{@render spans(block.spans)}</p>
    {:else if block.kind === "item"}
      <div class="item" style:padding-left="{block.depth * 1.25}em">
        <span class="marker">{block.ordered ? `${block.number}.` : "•"}</span>
        <span>{@render spans(block.spans)}</span>
      </div>
    {:else}
      <pre>{block.text}</pre>
    {/if}
  {/each}
</div>

<style>
  .markdown {
    flex: 1;
    align-self: stretch;
    min-width: 0;
    height: 100%;
    overflow: auto;
    overflow-wrap: anywhere;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  p,
  pre,
  .heading {
    margin: 0;
  }

  .heading {
    font-weight: 600;
  }

  .level1 {
    font-size: 1.4em;
  }

  .level2 {
    font-size: 1.2em;
  }

  .level3 {
    font-size: 1.05em;
  }

  .item {
    display: flex;
    gap: 0.5em;
  }

  .marker {
    flex: none;
    min-width: 1em;
    text-align: right;
  }

  .bold {
    font-weight: 600;
  }

  .italic {
    font-style: italic;
  }

  .code,
  pre {
    font-family: var(--mono, monospace);
    font-size: 0.92em;
  }

  .code {
    background: color-mix(in srgb, currentColor 9%, transparent);
    border-radius: 3px;
    padding: 0 3px;
  }

  pre {
    padding: 6px 8px;
    white-space: pre-wrap;
    background: color-mix(in srgb, currentColor 6%, transparent);
    border-radius: 4px;
  }

  /* Underlined in the text's color, as on every stack. */
  a {
    color: inherit;
    text-decoration: underline;
    cursor: pointer;
  }
</style>
