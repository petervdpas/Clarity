<script lang="ts">
  // The markdown part: Markdown read into blocks the same way on every stack
  // (markdownBlocks.ts, conformance/markdown.json), shown as headings,
  // paragraphs, list items, code, tables, and math as its TeX (as code, until
  // Clarity draws formulas), wrapped, scrolling when longer than the
  // part. A link opens in the browser, through `onlink`.
  import { markdownBlocks, type Span } from "../screen/markdownBlocks";

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
      <a href={span.link} class:bold={span.bold} class:italic={span.italic} class:code={span.code || span.math} onclick={(e) => follow(e, span.link!)}>{span.text}</a>
    {:else}
      <span class:bold={span.bold} class:italic={span.italic} class:code={span.code || span.math}>{span.text}</span>
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
    {:else if block.kind === "quote"}
      <blockquote style:margin-left="{(block.depth - 1) * 1.2}em">{@render spans(block.spans)}</blockquote>
    {:else if block.kind === "rule"}
      <hr />
    {:else if block.kind === "table"}
      <table>
        <thead>
          <tr>
            {#each block.header as cell, c (c)}
              <th style:text-align={block.align[c] === "none" ? null : block.align[c]}>{@render spans(cell)}</th>
            {/each}
          </tr>
        </thead>
        <tbody>
          {#each block.rows as row, r (r)}
            <tr>
              {#each row as cell, c (c)}
                <td style:text-align={block.align[c] === "none" ? null : block.align[c]}>{@render spans(cell)}</td>
              {/each}
            </tr>
          {/each}
        </tbody>
      </table>
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

  p,
  blockquote {
    white-space: pre-line;
  }

  blockquote {
    margin: 0;
    padding-left: 0.9em;
    border-left: 3px solid var(--border-strong, #888);
  }

  hr {
    width: 25%;
    margin: 0.6em auto;
    border: none;
    border-top: 1px solid var(--text-muted, #999);
  }

  .heading {
    font-weight: 600;
  }

  /* A table as wide as its texts, a line under each row, the header's the strong one. */
  table {
    border-collapse: collapse;
    align-self: flex-start;
  }

  th,
  td {
    padding: 0.25em 0.5em;
    text-align: left;
    border-bottom: 1px solid var(--border, #ccc);
  }

  th {
    font-weight: 600;
    border-bottom-color: var(--border-strong, #888);
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
