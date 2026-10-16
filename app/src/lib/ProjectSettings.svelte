<script lang="ts">
  // The project settings from clarity.toml: name, stack and data sources.
  // Every change is saved at once; the Rust backend checks it and refuses
  // settings with problems, with the reason.

  import StackFields from "./StackFields.svelte";
  import type { AuthKind } from "./generated/AuthKind";
  import type { DataSource } from "./generated/DataSource";
  import type { ProjectFile } from "./generated/ProjectFile";
  import type { Settings } from "./generated/Settings";
  import type { SourceKind } from "./generated/SourceKind";

  let {
    project,
    folder,
    onsave,
    oncreate,
    revision,
  }: {
    project: ProjectFile;
    folder: string;
    onsave: (settings: Settings) => void;
    oncreate: () => void;
    /** Changes when a save was refused, so the inputs show the file again. */
    revision: number;
  } = $props();

  const catalog = $derived(project.catalog);

  function sourceInfo(kind: SourceKind) {
    return catalog.sources.find((s) => s.id === kind);
  }

  function save(change: Partial<Settings>) {
    if (project.settings) onsave({ ...project.settings, ...change });
  }

  function saveSource(i: number, change: Partial<DataSource>) {
    if (!project.settings) return;
    const datasources = project.settings.datasources.map((s, j) => (j === i ? { ...s, ...change } : s));
    save({ datasources });
  }

  /** A new kind starts from its example location, since the old one rarely fits. */
  function setKind(i: number, kind: SourceKind) {
    saveSource(i, { kind, url: sourceInfo(kind)?.urlExample ?? "", auth: null });
  }

  function setAuth(i: number, source: DataSource, kind: AuthKind | "") {
    const auth = kind === "" ? null : { kind, secret: source.auth?.secret ?? "keychain:" + source.name };
    saveSource(i, { auth });
  }

  function move(i: number, delta: number) {
    if (!project.settings) return;
    const datasources = [...project.settings.datasources];
    const [source] = datasources.splice(i, 1);
    datasources.splice(i + delta, 0, source);
    save({ datasources });
  }

  function remove(i: number) {
    if (project.settings) save({ datasources: project.settings.datasources.filter((_, j) => j !== i) });
  }

  let newName = $state("");
  let newKind = $state<SourceKind>("sqlite");

  function addSource(e: SubmitEvent) {
    e.preventDefault();
    const name = newName.trim();
    if (!name || !project.settings) return;
    const source: DataSource = { name, kind: newKind, url: sourceInfo(newKind)?.urlExample ?? "", auth: null };
    save({ datasources: [...project.settings.datasources, source] });
    newName = "";
  }
</script>

<div class="editor">
  <header>
    <h1>Project settings</h1>
    <span class="muted mono path" title={folder + "/clarity.toml"}>{folder}/clarity.toml</span>
  </header>

  {#if project.problems.length > 0}
    <ul class="problems">
      {#each project.problems as p, i (i)}<li>{p}</li>{/each}
    </ul>
  {/if}

  {#if !project.settings}
    {#if project.version === ""}
      <p>This project has no project file yet. It says what the project is transpiled into and where its data lives.</p>
      <div><button class="primary" onclick={oncreate}>Create clarity.toml</button></div>
    {:else}
      <p>The project file cannot be read. Fix it in a text editor; this view follows the file as it changes.</p>
    {/if}
  {:else}
    {@const settings = project.settings}
    {#key revision}
      <section>
        <h2>Project</h2>
        <label class="field">
          <span>Name</span>
          <input value={settings.name} onchange={(e) => save({ name: e.currentTarget.value.trim() })} />
        </label>
      </section>

      <section>
        <h2>Stack</h2>
        <p class="muted">What the project is transpiled into.</p>
        <StackFields {catalog} stack={settings.stack} onchange={(stack) => save({ stack })} />
      </section>

      <section>
        <h2>Data sources</h2>
        <p class="muted">Where the data lives. Entities use the first source unless they name another. Secrets are never stored here: use env:NAME or keychain:NAME.</p>
        {#if settings.datasources.length > 0}
          <div class="table-wrap">
            <table>
              <thead>
                <tr>
                  <th>Name</th>
                  <th>Kind</th>
                  <th>Location</th>
                  <th>Auth</th>
                  <th></th>
                </tr>
              </thead>
              <tbody>
                {#each settings.datasources as source, i (i)}
                  {@const info = sourceInfo(source.kind)}
                  <tr>
                    <td><input class="mono" value={source.name} onchange={(e) => saveSource(i, { name: e.currentTarget.value.trim() })} /></td>
                    <td>
                      <select value={source.kind} onchange={(e) => setKind(i, e.currentTarget.value as SourceKind)}>
                        {#each catalog.sources as s (s.id)}<option value={s.id}>{s.label}</option>{/each}
                      </select>
                    </td>
                    <td>
                      <input
                        class="mono"
                        value={source.url}
                        title={info?.urlLabel}
                        placeholder={info?.urlExample}
                        onchange={(e) => saveSource(i, { url: e.currentTarget.value.trim() })}
                      />
                    </td>
                    <td>
                      {#if info?.hasAuth}
                        <div class="auth">
                          <select value={source.auth?.kind ?? ""} onchange={(e) => setAuth(i, source, e.currentTarget.value as AuthKind | "")}>
                            <option value="">None</option>
                            {#each catalog.auths as a (a.id)}<option value={a.id}>{a.label}</option>{/each}
                          </select>
                          {#if source.auth}
                            <input
                              class="mono"
                              value={source.auth.secret}
                              title="Where the secret is kept"
                              onchange={(e) => source.auth && saveSource(i, { auth: { ...source.auth, secret: e.currentTarget.value.trim() } })}
                            />
                          {/if}
                        </div>
                      {:else}
                        <span class="muted">-</span>
                      {/if}
                    </td>
                    <td class="actions">
                      <button title="Move up" disabled={i === 0} onclick={() => move(i, -1)}>&uarr;</button>
                      <button title="Move down" disabled={i === settings.datasources.length - 1} onclick={() => move(i, 1)}>&darr;</button>
                      <button class="danger" title="Remove the data source" onclick={() => remove(i)}>&times;</button>
                    </td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
        {/if}
        <form class="add" onsubmit={addSource}>
          <input class="mono" bind:value={newName} placeholder="new_source" spellcheck="false" />
          <select bind:value={newKind}>
            {#each catalog.sources as s (s.id)}<option value={s.id}>{s.label}</option>{/each}
          </select>
          <button type="submit" disabled={!newName.trim()}>Add data source</button>
        </form>
      </section>
    {/key}
  {/if}
</div>

<style>
  .editor {
    display: flex;
    flex-direction: column;
    gap: 18px;
    max-width: 1000px;
  }

  header {
    display: flex;
    align-items: baseline;
    gap: 12px;
  }

  h1 {
    margin: 0;
    font-size: 18px;
    white-space: nowrap;
  }

  h2 {
    margin: 0 0 6px;
    font-size: 13px;
  }

  section {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  p {
    margin: 0;
  }

  .muted {
    color: var(--text-muted);
    font-size: 12px;
  }

  .path {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
  }

  .mono {
    font-family: var(--mono);
    font-size: 12px;
  }

  .problems {
    margin: 0;
    padding: 8px 12px 8px 28px;
    border: 1px solid var(--error);
    border-radius: var(--radius);
    color: var(--error);
    font-size: 12px;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 3px;
    font-size: 12px;
    color: var(--text-muted);
    max-width: 320px;
  }

  section :global(.stack) {
    max-width: 320px;
  }

  .table-wrap {
    overflow: auto;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    background: var(--panel);
  }

  table {
    width: 100%;
    border-collapse: collapse;
  }

  th {
    text-align: left;
    font-size: 12px;
    font-weight: 600;
    color: var(--text-muted);
    padding: 6px 8px;
    background: var(--panel-alt);
    border-bottom: 1px solid var(--border-strong);
  }

  td {
    padding: 3px 4px;
    border-bottom: 1px solid var(--border);
  }

  td input {
    width: 100%;
    min-width: 0;
  }

  .auth {
    display: flex;
    gap: 4px;
  }

  .actions {
    white-space: nowrap;
  }

  .actions button {
    padding: 1px 7px;
  }

  .danger {
    color: var(--error);
  }

  .add {
    display: flex;
    gap: 8px;
  }
</style>
