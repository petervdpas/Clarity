<script lang="ts">
  // A custom part (DESIGN 5.19): what it is declared as, where its library
  // is, and the library's files, opened and edited here. Its declaration is
  // Clarity, edited as text; the header in a native part's `clarity` folder
  // is Clarity's, written from the declaration, and shown to read.

  import { invoke } from "./backend";
  import SourceEditor from "./SourceEditor.svelte";
  import { paramsText } from "./customParts";
  import type { CodeView } from "./generated/CodeView";
  import type { DeclTrial } from "./generated/DeclTrial";
  import type { Edit } from "./generated/Edit";
  import type { PartDecl } from "./generated/PartDecl";
  import type { SpecFile } from "./generated/SpecFile";

  let {
    decl,
    file,
    onapply,
  }: {
    decl: PartDecl;
    /** The module that declares it. */
    file: SpecFile;
    onapply: (edit: Edit) => Promise<void>;
  } = $props();

  type PartFile = { path: string; own: boolean };

  const name = $derived(decl.name.name);
  const item = $derived(decl.span.start);
  const library = $derived.by(() => {
    const l = decl.library;
    if (!l) return "no library yet";
    if (l.kind === "native") return `C or C++ built from ${l.folder}`;
    if (l.kind === "crate") return `the Rust crate in ${l.folder}`;
    if (l.kind === "go") return `the Go package in ${l.folder}`;
    if (l.kind === "nuget") return `the NuGet package ${l.package} ${l.version}, through its adapter in parts/${name}`;
    return `${l.file}, through its adapter in parts/${name}`;
  });

  let files = $state<PartFile[]>([]);
  let open = $state<PartFile | null>(null);
  let note = $state<string | null>(null);

  async function list() {
    try {
      files = await invoke<PartFile[]>("part_files", { name });
      if (open && !files.some((f) => f.path === open?.path)) open = null;
    } catch (err) {
      files = [];
      note = String(err);
    }
  }

  $effect(() => {
    void name;
    void file.version;
    void list();
  });

  async function writeFolders() {
    const written = await invoke<string[]>("write_part_folders").catch((err) => {
      note = String(err);
      return null;
    });
    if (written) note = written.length > 0 ? `Wrote ${written.join(", ")}` : "The header and the library's files are up to date.";
    await list();
  }
</script>

<div class="part">
  <header>
    <strong>{name}</strong>
    <span class="muted">custom part in module {file.module?.name ?? ""}, drawn by {library}</span>
  </header>

  <div class="panes">
    <aside>
      <section>
        <h3>What the window sees</h3>
        <dl>
          <dt>Binds</dt>
          <dd>{decl.binds ? "a value" : "nothing"}</dd>
          <dt>Properties</dt>
          <dd>{decl.properties.map((p) => p.name.name).join(", ") || "none"}</dd>
          <dt>Tells</dt>
          <dd>{decl.tells.map((t) => `${t.name.name}(${paramsText(t.params)})`).join(", ") || "none"}</dd>
          <dt>Hears</dt>
          <dd>{decl.hears.map((h) => `${h.name.name}(${paramsText(h.params)})`).join(", ") || "none"}</dd>
        </dl>
      </section>

      <section>
        <h3>Its library</h3>
        <ul class="files">
          {#each files as f (f.path)}
            <li>
              <button class:active={open?.path === f.path} onclick={() => (open = f)}>
                <code>{f.path}</code>
                {#if f.own}<span class="muted">Clarity's</span>{/if}
              </button>
            </li>
          {:else}
            <li class="muted">No files yet.</li>
          {/each}
        </ul>
        <button onclick={writeFolders} title="Writes the header from the declaration, and a starter where the folder has none">Write header and starter</button>
        {#if note}<p class="muted">{note}</p>{/if}
      </section>
    </aside>

    <section class="code">
      {#if open}
        {@const shown = open}
        <div class="tab"><code>{shown.path}</code>{#if shown.own}<span class="muted"> written from the declaration</span>{/if}</div>
        <SourceEditor
          load={async () => ({ text: await invoke<string>("part_file_text", { name, path: shown.path }), problems: [] })}
          check={async () => []}
          save={(text) => invoke<void>("save_part_file", { name, path: shown.path, text })}
          reloadKey={name + "\n" + shown.path + "\n" + file.version}
          language="plain"
          readonly={shown.own}
          errorsBlockSave={false}
          opensBlock={/[{(]\s*$/}
          onclose={() => (open = null)}
        />
      {:else}
        <div class="tab"><code>part {name}</code></div>
        <SourceEditor
          load={() => invoke<CodeView>("decl_text", { path: file.path, item })}
          check={async (text) => (await invoke<DeclTrial>("try_decl", { path: file.path, item, text })).problems}
          save={(text) => onapply({ op: "replaceDecl", item, text })}
          reloadKey={file.path + "\n" + item + "\n" + file.version}
        />
      {/if}
    </section>
  </div>
</div>

<style>
  .part {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
    padding: 12px 16px;
    gap: 10px;
    box-sizing: border-box;
  }
  header {
    display: flex;
    gap: 10px;
    align-items: baseline;
  }
  .muted {
    opacity: 0.7;
  }
  .panes {
    display: flex;
    gap: 16px;
    flex: 1;
    min-height: 0;
  }
  aside {
    width: 300px;
    flex: none;
    overflow: auto;
  }
  h3 {
    font-size: 0.85em;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    margin: 8px 0;
  }
  dl {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 4px 10px;
    margin: 0;
  }
  dt {
    opacity: 0.7;
  }
  dd {
    margin: 0;
  }
  .files {
    list-style: none;
    padding: 0;
    margin: 0 0 8px;
  }
  .files button {
    width: 100%;
    text-align: left;
    display: flex;
    justify-content: space-between;
    gap: 8px;
  }
  .files button.active {
    font-weight: bold;
  }
  .code {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    min-height: 0;
  }
  .tab {
    padding: 4px 0;
  }
</style>
