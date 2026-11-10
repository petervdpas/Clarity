<script lang="ts">
  // Clarity's own file browser (docs/PROJECT-BROWSER-PLAN.md), shown by
  // `browse()` in place of the system's file and folder windows. Browsing is
  // the way: click into a folder, Up to go back; the address bar shows where
  // you are, and a path typed there jumps to it. Clarity's projects are
  // marked as the backend tells them: a project file with its name and stack,
  // the folder of its modules beside it, a folder that holds one. Each use
  // starts where it was left last time.

  import { invoke } from "@clarity/ui/host/backend.ts";
  import type { Listing } from "@clarity/ui/generated/Listing.ts";
  import type { Entry } from "@clarity/ui/generated/Entry.ts";
  import type { ProjectInfo } from "@clarity/ui/generated/ProjectInfo.ts";
  import type { BrowsePlaces } from "@clarity/ui/generated/BrowsePlaces.ts";
  import type { BrowseShow } from "@clarity/ui/generated/BrowseShow.ts";
  import logo from "../../src-tauri/icons/icon.svg";
  import Modal from "./Modal.svelte";
  import { fileBrowser, type BrowseRequest } from "./fileBrowserStore.svelte";
  import { chosen } from "./fileBrowser";

  const request = $derived(fileBrowser.current);

  let listing = $state<Listing | null>(null);
  let places = $state<BrowsePlaces | null>(null);
  let recent = $state<ProjectInfo[]>([]);
  let address = $state("");
  let selected = $state<Entry | null>(null);
  let name = $state("");
  let hidden = $state(false);
  let error = $state<string | null>(null);

  const BUTTONS: Record<BrowseRequest["pick"], string> = { project: "Open", folder: "Choose", file: "Choose", save: "Save" };

  function showOf(r: BrowseRequest): BrowseShow {
    if (r.pick === "folder") return { kind: "folders" };
    if ((r.pick === "file" || r.pick === "save") && r.kinds && r.kinds.length > 0) return { kind: "files", kinds: r.kinds };
    return { kind: "everything" };
  }

  // A new request: where its use starts, and for opening a project the projects opened last.
  $effect(() => {
    const r = request;
    if (!r) return;
    selected = null;
    error = null;
    name = r.name ?? "";
    void (async () => {
      try {
        places = await invoke<BrowsePlaces>("browse_places", { purpose: r.purpose, asked: r.start ?? null });
        recent = r.pick === "project" ? await invoke<ProjectInfo[]>("recent_projects") : [];
        await go(places.start);
      } catch (err) {
        error = String(err);
      }
    })();
  });

  async function go(folder: string) {
    const r = request;
    if (!r || !folder.trim()) return;
    const next = await invoke<Listing>("browse_list", { folder: folder.trim(), show: showOf(r), hidden });
    listing = next;
    address = next.folder;
    selected = null;
  }

  // Opening a project: each project as a card (its file and the folder of its modules as one, or a folder
  // that holds exactly one), and the other folders and files as rows below.
  const asCards = $derived(request?.pick === "project");
  const cards = $derived(asCards ? (listing?.entries ?? []).filter((e) => e.project || (e.folder && e.projects.length === 1)) : []);
  const rows = $derived(
    asCards ? (listing?.entries ?? []).filter((e) => !e.project && !(e.folder && e.projects.length === 1) && !(e.folder && e.modulesOf)) : (listing?.entries ?? []),
  );

  function cardProject(entry: Entry): ProjectInfo | null {
    return entry.project ?? (entry.projects.length === 1 ? entry.projects[0] : null);
  }

  const answer = $derived(request && listing ? chosen(request.pick, selected, listing.folder, name, request.kinds ?? []) : null);

  async function finish(path: string | null) {
    const r = request;
    if (!r || !path) return;
    if (listing && !listing.problem) {
      await invoke("browse_remember", { purpose: r.purpose, folder: listing.folder }).catch(() => {});
    }
    fileBrowser.answer(path);
  }

  function open(entry: Entry) {
    if (!request) return;
    if (entry.folder) {
      void go(entry.path);
      return;
    }
    selected = entry;
    void finish(chosen(request.pick, entry, listing?.folder ?? "", name, request.kinds ?? []));
  }

  function newProjectHere() {
    const r = request;
    const folder = listing?.folder;
    if (!r?.newProject || !folder) return;
    fileBrowser.answer(null);
    r.newProject(folder);
  }

  function detail(entry: Entry): string {
    if (entry.project) return entry.project.problem && !entry.project.stack ? entry.project.problem : entry.project.stack;
    if (entry.modulesOf) return `the modules of ${entry.modulesOf}`;
    if (entry.projects.length === 1) return `holds the project ${entry.projects[0].title}`;
    if (entry.projects.length > 1) return `holds ${entry.projects.length} projects`;
    if (!entry.folder) return size(entry.size);
    return "";
  }

  function size(bytes: number): string {
    if (bytes >= 1_048_576) return `${(bytes / 1_048_576).toFixed(1)} MB`;
    if (bytes >= 1024) return `${Math.round(bytes / 1024)} KB`;
    return `${bytes} bytes`;
  }

  function lastPart(path: string): string {
    return path.split(/[\\/]/).filter(Boolean).at(-1) ?? path;
  }
</script>

{#if request}
  <Modal title={request.title} width={760} onclose={() => fileBrowser.answer(null)}>
    <div class="browser">
      <div class="bar">
        <button type="button" onclick={() => listing?.up && go(listing.up)} disabled={!listing?.up} title="Up to the folder above">Up</button>
        <button type="button" onclick={() => places?.home && go(places.home)} disabled={!places?.home} title="Home">Home</button>
        <input
          class="address"
          aria-label="Folder"
          bind:value={address}
          spellcheck="false"
          onkeydown={(e) => e.key === "Enter" && (e.preventDefault(), go(address))}
          title="Where you are; type or paste a path and press Enter to go there"
        />
      </div>
      {#if listing?.problem}<p class="error">{listing.problem}</p>{/if}

      <div class="body">
        <nav class="places" aria-label="Places">
          {#if places?.home}<button type="button" onclick={() => places?.home && go(places.home)}>Home</button>{/if}
          {#if places?.projects}<button type="button" onclick={() => places?.projects && go(places.projects)} title={places.projects}>Projects folder</button>{/if}
          {#if places?.apps}<button type="button" onclick={() => places?.apps && go(places.apps)} title={places.apps}>Apps folder</button>{/if}
          {#each places?.roots ?? [] as root (root)}<button type="button" onclick={() => go(root)}>{root}</button>{/each}
          {#if request.pick === "project" && recent.length > 0}
            <p class="heading">Recent projects</p>
            {#each recent as r (r.file)}
              <button type="button" class="recent" onclick={() => finish(r.file)} title={r.file}>
                <img src={logo} alt="" />
                <span><strong>{r.title}</strong><span class="muted">{lastPart(r.file)}</span></span>
              </button>
            {/each}
          {/if}
        </nav>

        <div class="content">
        {#if cards.length > 0}
          <ul class="cards" role="listbox" aria-label="Clarity projects here">
            {#each cards as entry (entry.path)}
              {@const p = cardProject(entry)}
              {#if p}
                <li
                  role="option"
                  aria-selected={selected?.path === entry.path}
                  class="card"
                  class:selected={selected?.path === entry.path}
                  onclick={() => (selected = entry)}
                  ondblclick={() => finish(p.file)}
                  onkeydown={(e) => e.key === "Enter" && finish(p.file)}
                  tabindex="0"
                  title={p.file}
                >
                  <div class="card-head">
                    <img class="icon" src={logo} alt="" />
                    <strong>{p.title}</strong>
                  </div>
                  <span class="stack" class:problem={p.problem && !p.stack}>{p.stack || p.problem}</span>
                  {#if p.readme}<p class="readme">{p.readme}</p>{/if}
                  <!-- A folder that holds the project names itself, so two projects of one name tell apart. -->
                  <span class="file muted">{entry.folder ? `${entry.name}/${lastPart(p.file)}` : lastPart(p.file)}</span>
                </li>
              {/if}
            {/each}
          </ul>
        {/if}
        <ul class="list" role="listbox" aria-label="What the folder holds">
          {#each rows as entry (entry.path)}
            <li
              role="option"
              aria-selected={selected?.path === entry.path}
              class:selected={selected?.path === entry.path}
              class:project={entry.project || entry.projects.length > 0}
              onclick={() => (selected = entry)}
              ondblclick={() => open(entry)}
              onkeydown={(e) => e.key === "Enter" && open(entry)}
              tabindex="0"
            >
              {#if entry.project || entry.projects.length > 0}
                <img class="icon" src={logo} alt="" />
              {:else}
                <span class="icon glyph" aria-hidden="true">{entry.folder ? "▸" : "·"}</span>
              {/if}
              <span class="name">{entry.project ? entry.project.title : entry.name}</span>
              {#if entry.project}<span class="file muted">{entry.name}</span>{/if}
              <span class="detail muted" class:problem={entry.project?.problem && !entry.project.stack}>{detail(entry)}</span>
            </li>
          {:else}
            {#if listing && !listing.problem && cards.length === 0}<li class="empty muted">Nothing to show here.</li>{/if}
          {/each}
        </ul>
        </div>
      </div>

      {#if request.pick === "save"}
        <div class="save">
          <label for="browse-name">Name</label>
          <input id="browse-name" bind:value={name} spellcheck="false" onkeydown={(e) => e.key === "Enter" && answer && finish(answer)} />
        </div>
      {/if}
      {#if error}<p class="error">{error}</p>{/if}

      <div class="buttons">
        <label class="check"><input type="checkbox" bind:checked={hidden} onchange={() => listing && go(listing.folder)} /> Show hidden</label>
        <span class="spacer"></span>
        {#if request.pick === "project" && request.newProject}
          <button type="button" onclick={newProjectHere} disabled={!listing || !!listing.problem}>New project here</button>
        {/if}
        <button type="button" onclick={() => fileBrowser.answer(null)}>Cancel</button>
        <button type="button" class="primary" disabled={!answer} onclick={() => finish(answer)}>{BUTTONS[request.pick]}</button>
      </div>
    </div>
  </Modal>
{/if}

<style>
  .browser {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .bar,
  .buttons,
  .save {
    display: flex;
    gap: 6px;
    align-items: center;
  }
  .address {
    flex: 1;
    min-width: 0;
    font-family: var(--mono);
  }
  .body {
    display: grid;
    grid-template-columns: 180px minmax(0, 1fr);
    gap: 8px;
    height: 380px;
  }
  .places {
    display: flex;
    flex-direction: column;
    gap: 2px;
    overflow: auto;
    border-right: 1px solid var(--border);
    padding-right: 6px;
  }
  .places button {
    text-align: left;
    background: transparent;
    border-color: transparent;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .places button:hover {
    background: var(--panel-alt);
  }
  .heading {
    margin: 8px 0 2px;
    font-size: 11px;
    color: var(--text-muted);
    text-transform: uppercase;
  }
  .recent {
    display: flex;
    gap: 6px;
    align-items: center;
  }
  .recent img {
    width: 16px;
    height: 16px;
    flex: none;
  }
  .recent span {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .content {
    display: flex;
    flex-direction: column;
    gap: 8px;
    overflow: auto;
    min-height: 0;
  }
  .cards {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
    gap: 8px;
  }
  .card {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 8px 10px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--panel);
    cursor: default;
    min-width: 0;
  }
  .card:hover {
    border-color: var(--border-strong);
  }
  .card.selected {
    border-color: var(--accent);
    box-shadow: 0 0 0 1px var(--accent);
  }
  .card-head {
    display: flex;
    gap: 6px;
    align-items: center;
    min-width: 0;
  }
  .card-head strong {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .stack {
    font-size: 12px;
    color: var(--text-muted);
  }
  .stack.problem {
    color: var(--error);
  }
  .readme {
    margin: 0;
    font-size: 12px;
    display: -webkit-box;
    -webkit-line-clamp: 3;
    line-clamp: 3;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  .list {
    list-style: none;
    margin: 0;
    padding: 0;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--panel);
  }
  .list li {
    display: flex;
    gap: 8px;
    align-items: baseline;
    padding: 3px 8px;
    cursor: default;
    min-width: 0;
  }
  .list li:hover {
    background: var(--panel-alt);
  }
  .list li.selected {
    background: var(--accent);
    color: var(--accent-text, #fff);
  }
  .list li.selected .muted {
    color: inherit;
    opacity: 0.85;
  }
  .list li.project .name {
    font-weight: 600;
  }
  .icon {
    width: 16px;
    height: 16px;
    flex: none;
    align-self: center;
  }
  .glyph {
    text-align: center;
    color: var(--text-muted);
  }
  .name {
    white-space: nowrap;
  }
  .file {
    font-family: var(--mono);
    font-size: 12px;
  }
  .detail {
    margin-left: auto;
    font-size: 12px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .detail.problem {
    color: var(--error);
  }
  .empty {
    padding: 8px;
  }
  .save input {
    flex: 1;
  }
  .muted {
    color: var(--text-muted);
  }
  .error {
    margin: 0;
    color: var(--error);
    font-size: 12px;
  }
  .check {
    display: flex;
    gap: 4px;
    align-items: center;
    font-size: 12px;
  }
  .spacer {
    flex: 1;
  }
</style>
