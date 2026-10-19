<script lang="ts">
  // Clarity's manual, in a window of its own beside Clarity's: the topics on
  // the left, the page on the right. Links go to other pages, to the
  // language reference and to headings; Back goes back. It remembers the
  // page you were on. F1 and the Manual button in Clarity open it on the page
  // about what you are doing.
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { onMount, tick } from "svelte";
  import Splitter from "./lib/Splitter.svelte";
  import { layout } from "./lib/layout.svelte";
  import { linkTarget } from "./lib/manual";
  import { markdown } from "./lib/markdown";
  import type { ManualEntry } from "./lib/generated/ManualEntry";

  const REMEMBERED = "clarity.manual.topic";

  function remembered(): string | null {
    try {
      return localStorage.getItem(REMEMBERED);
    } catch {
      return null;
    }
  }

  let topics = $state<ManualEntry[]>([]);
  let current = $state(new URLSearchParams(location.search).get("topic") || remembered() || "start.overview");
  let text = $state("");
  let problem = $state<string | null>(null);
  let filter = $state("");
  let back = $state<string[]>([]);
  let article = $state<HTMLElement | null>(null);

  const shown = $derived(
    topics.filter((t) => {
      const words = filter.trim().toLowerCase();
      return words === "" || `${t.title} ${t.summary}`.toLowerCase().includes(words);
    }),
  );
  const title = $derived(current === "reference" ? "The language reference" : (topics.find((t) => t.id === current)?.title ?? current));

  async function show(id: string, anchor: string | null = null, remember = true) {
    if (remember && id !== current) back = [...back, current];
    current = id;
    try {
      localStorage.setItem(REMEMBERED, id);
    } catch {
      // Remembering the page is a convenience only.
    }
    try {
      text = await invoke<string>("manual_page", { id });
      problem = null;
    } catch (err) {
      problem = String(err);
      text = "";
    }
    await tick();
    const target = anchor ? article?.querySelector(`#${CSS.escape(anchor)}`) : null;
    if (target) target.scrollIntoView();
    else article?.scrollTo(0, 0);
  }

  function goBack() {
    const previous = back.at(-1);
    if (!previous) return;
    back = back.slice(0, -1);
    show(previous, null, false);
  }

  /** A link on the page: another page, the reference, or a heading here. */
  function follow(e: MouseEvent) {
    const link = (e.target as HTMLElement).closest<HTMLAnchorElement>("a[data-link]");
    if (!link) return;
    e.preventDefault();
    const to = linkTarget(link.dataset.link ?? "");
    if (!to) return;
    if (to.topic && to.topic !== current) show(to.topic, to.anchor);
    else if (to.anchor) article?.querySelector(`#${CSS.escape(to.anchor)}`)?.scrollIntoView();
  }

  function keydown(e: KeyboardEvent) {
    if (e.altKey && e.key === "ArrowLeft") {
      e.preventDefault();
      goBack();
    }
  }

  onMount(() => {
    invoke<ManualEntry[]>("manual_topics").then((list) => (topics = list));
    show(current, null, false);
    // Clarity asks for another page while this window is open.
    const stop = listen<string>("manual-show", (e) => show(e.payload));
    return () => {
      stop.then((unlisten) => unlisten());
    };
  });
</script>

<svelte:window onkeydown={keydown} />

<div class="manual">
  <nav style:width="{layout.sizes.manualTopics}px">
    <input class="filter" type="search" placeholder="Find a topic" aria-label="Find a topic" bind:value={filter} />
    <ul>
      {#each shown as topic (topic.id)}
        <li>
          <button class:current={topic.id === current} title={topic.summary} onclick={() => show(topic.id)}>{topic.title}</button>
        </li>
      {/each}
      <li>
        <button class:current={current === "reference"} title="The exact syntax of every construct" onclick={() => show("reference")}>The language reference</button>
      </li>
    </ul>
  </nav>
  <Splitter axis="x" label="Width of the topics" onmove={(d) => layout.resize("manualTopics", d)} />
  <main>
    <header>
      <button onclick={goBack} disabled={back.length === 0} title="Back (Alt+Left)">Back</button>
      <span class="where">{title}</span>
    </header>
    <!-- The page is Markdown made HTML by markdown.ts, which escapes the text first. -->
    <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions -->
    <article bind:this={article} onclick={follow}>
      {#if problem}
        <p class="problem">{problem}</p>
      {:else}
        {@html markdown(text, { page: true })}
      {/if}
    </article>
  </main>
</div>

<style>
  .manual {
    display: flex;
    height: 100vh;
    background: var(--bg);
    color: var(--text);
  }

  nav {
    flex: none;
    display: flex;
    flex-direction: column;
    min-width: 0;
    background: var(--panel-alt);
    border-right: 1px solid var(--border);
  }

  .filter {
    margin: 10px;
  }

  ul {
    list-style: none;
    margin: 0;
    padding: 0 6px 10px;
    overflow: auto;
  }

  nav button {
    display: block;
    width: 100%;
    text-align: left;
    padding: 5px 8px;
    border: none;
    border-radius: var(--radius);
    background: none;
    color: inherit;
    font: inherit;
    cursor: pointer;
  }

  nav button:hover {
    background: var(--accent-soft);
  }

  nav button.current {
    background: var(--selection);
    font-weight: 600;
  }

  main {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  header {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 16px;
    border-bottom: 1px solid var(--border);
    background: var(--panel);
  }

  .where {
    color: var(--text-muted);
  }

  article {
    flex: 1;
    overflow: auto;
    padding: 8px 32px 32px;
    max-width: 820px;
    line-height: 1.55;
  }

  article :global(h1) {
    font-size: 1.6em;
    margin: 0.6em 0 0.4em;
  }

  article :global(h2) {
    font-size: 1.25em;
    margin: 1.4em 0 0.4em;
  }

  article :global(h3) {
    font-size: 1.05em;
    margin: 1.2em 0 0.3em;
  }

  article :global(code) {
    font-family: var(--mono);
    font-size: 0.92em;
    background: var(--panel-alt);
    padding: 1px 4px;
    border-radius: 3px;
  }

  article :global(pre) {
    background: var(--panel-alt);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 10px 12px;
    overflow: auto;
  }

  article :global(pre code) {
    background: none;
    padding: 0;
  }

  article :global(a) {
    color: var(--accent);
  }

  article :global(table) {
    border-collapse: collapse;
    margin: 0.6em 0;
  }

  article :global(th),
  article :global(td) {
    border: 1px solid var(--border);
    padding: 4px 10px;
    text-align: left;
    vertical-align: top;
  }

  article :global(th) {
    background: var(--panel-alt);
  }

  .problem {
    color: var(--error);
  }
</style>
