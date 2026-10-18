<script lang="ts">
  // The project settings from the project file: name, stack, output and data sources.
  // Every change is saved at once; the Rust backend checks it and refuses
  // settings with problems, with the reason.

  import { invoke } from "@tauri-apps/api/core";
  import { open as openDialog, save as saveDialog } from "@tauri-apps/plugin-dialog";
  import ImportPanel from "./ImportPanel.svelte";
  import MigrationPanel from "./MigrationPanel.svelte";
  import TranslationsPanel from "./TranslationsPanel.svelte";
  import { themeChoice } from "./scheme.svelte";
  import StackFields from "./StackFields.svelte";
  import type { Preview } from "./generated/Preview";
  import type { AuthKind } from "./generated/AuthKind";
  import type { DataSource } from "./generated/DataSource";
  import type { FileStore } from "./generated/FileStore";
  import type { ProjectFile } from "./generated/ProjectFile";
  import type { Settings } from "./generated/Settings";
  import type { SourceKind } from "./generated/SourceKind";

  let {
    project,
    file,
    onsave,
    revision,
    preview,
    onapplied,
    onopentheme,
  }: {
    project: ProjectFile;
    /** The project file. */
    file: string;
    onsave: (settings: Settings) => void;
    /** Changes when a save was refused, so the inputs show the file again. */
    revision: number;
    /** The loaded project, for its migrations. */
    preview: Preview;
    onapplied: (preview: Preview) => void;
    /** Opens a theme in its own view, where it is written. */
    onopentheme: (name: string) => void;
  } = $props();

  const catalog = $derived(project.catalog);
  /** The SQL data sources Clarity owns: each has its own migration steps. */
  const databases = $derived(project.settings?.datasources.filter((s) => s.kind !== "rest" && !s.external) ?? []);
  /** The themes the project's modules declare. */
  const themes = $derived(preview.project.files.flatMap((f) => f.themes.map((t) => t.name.name)));

  /** Offers theme `name` in the application, or stops offering it. */
  function setOffered(name: string, on: boolean) {
    if (!project.settings) return;
    const current = project.settings.themes;
    save({ themes: on ? [...current, name] : current.filter((t) => t !== name) });
  }

  /** Makes theme `name` the one the application starts with. */
  function setFirst(name: string) {
    if (project.settings) save({ themes: [name, ...project.settings.themes.filter((t) => t !== name)] });
  }
  const fileName = $derived(file.split(/[\\/]/).at(-1) ?? file);

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

  /** A picked path as the project file stores it: relative inside the project's folder. */
  async function stored(picked: string): Promise<string> {
    return invoke<string>("project_path", { picked });
  }

  async function browseOutput() {
    const picked = await openDialog({
      directory: true,
      title: "Output folder: where Run and Build put the application",
      defaultPath: await invoke<string>("project_folder"),
    });
    if (typeof picked === "string") save({ out: await stored(picked) });
  }

  /** A SQLite file may not exist yet, so this asks where it goes, like a save. */
  async function browseDatabase(i: number) {
    const picked = await saveDialog({
      title: "Database file",
      defaultPath: (await invoke<string>("project_folder")) + "/data/app.db",
      filters: [{ name: "SQLite database", extensions: ["db", "sqlite", "sqlite3"] }],
    });
    if (typeof picked === "string") saveSource(i, { url: await stored(picked) });
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

  /** The external data source being imported from, if any. */
  let importing = $state<string | null>(null);
  /** The external data source waiting for the go-ahead to be owned. */
  let owning = $state<string | null>(null);
  let ownError = $state<string | null>(null);

  async function takeOwnership() {
    try {
      onapplied(await invoke<Preview>("take_ownership", { source: owning }));
      owning = null;
      ownError = null;
    } catch (err) {
      ownError = String(err);
    }
  }

  function saveStore(i: number, change: Partial<FileStore>) {
    if (!project.settings) return;
    save({ filestores: project.settings.filestores.map((s, j) => (j === i ? { ...s, ...change } : s)) });
  }

  async function browseStore(i: number) {
    const picked = await openDialog({
      directory: true,
      title: "File store: the folder that keeps the pictures",
      defaultPath: await invoke<string>("project_folder"),
    });
    if (typeof picked === "string") saveStore(i, { location: await stored(picked) });
  }

  let newStore = $state("");

  function addStore(e: SubmitEvent) {
    e.preventDefault();
    const name = newStore.trim();
    if (!name || !project.settings) return;
    save({ filestores: [...project.settings.filestores, { name, location: name }] });
    newStore = "";
  }

  let newName = $state("");
  let newKind = $state<SourceKind>("sqlite");

  function addSource(e: SubmitEvent) {
    e.preventDefault();
    const name = newName.trim();
    if (!name || !project.settings) return;
    const source: DataSource = { name, kind: newKind, url: sourceInfo(newKind)?.urlExample ?? "", auth: null, external: false, readonly: false };
    save({ datasources: [...project.settings.datasources, source] });
    newName = "";
  }
</script>

<div class="editor">
  <header>
    <h1>Project settings</h1>
    <span class="file" title={file}>{fileName}</span>
  </header>

  {#if project.problems.length > 0}
    <ul class="problems">
      {#each project.problems as p, i (i)}<li>{p}</li>{/each}
    </ul>
  {/if}

  {#if !project.settings}
    <p>The project file cannot be read. Fix it in a text editor; this view follows the file as it changes.</p>
  {:else}
    {@const settings = project.settings}
    {#key revision}
      <section>
        <h2>Project</h2>
        <div class="form-grid">
          <label class="form-label" for="project-name">Name</label>
          <input
            id="project-name"
            class="mono"
            value={settings.name}
            spellcheck="false"
            onchange={(e) => save({ name: e.currentTarget.value.trim() })}
          />
          <p class="form-help">A Clarity name, like MyShop; also the program's name.</p>
          <label class="form-label" for="project-title">Title</label>
          <input
            id="project-title"
            value={settings.title ?? ""}
            placeholder={project.title}
            onchange={(e) => save({ title: e.currentTarget.value.trim() || null })}
          />
          <p class="form-help">Shown in the window title. Empty follows the name.</p>
        </div>
      </section>

      <section>
        <h2>Stack</h2>
        <p class="intro">What the project is transpiled into.</p>
        <div class="form-grid">
          <StackFields {catalog} stack={settings.stack} onchange={(stack) => save({ stack })} />
        </div>
      </section>

      <section>
        <h2>Language, formats and theme</h2>
        <p class="intro">Dates, numbers and money show the way the person using the application writes them, on every stack.</p>
        <div class="form-grid">
          <label class="form-label" for="project-language">Language</label>
          <input
            id="project-language"
            class="mono"
            value={settings.language ?? ""}
            placeholder="en"
            spellcheck="false"
            onchange={(e) => save({ language: e.currentTarget.value.trim() || null })}
          />
          <p class="form-help">The language the project's texts are written in, like en or nl-NL. Formats use it where the system names no locale.</p>
          <label class="form-label" for="project-currency">Currency</label>
          <input
            id="project-currency"
            class="mono"
            value={settings.currency ?? ""}
            placeholder="EUR"
            maxlength="3"
            spellcheck="false"
            onchange={(e) => save({ currency: e.currentTarget.value.trim().toUpperCase() || null })}
          />
          <p class="form-help">The currency of money fields, as its code: EUR, USD, GBP. Empty shows amounts without a currency sign.</p>
          <span class="form-label">Themes</span>
          <div class="themes">
            {#each themes as theme (theme)}
              {@const offered = settings.themes.includes(theme)}
              <div class="theme-row">
                <label><input type="checkbox" checked={offered} onchange={(e) => setOffered(theme, e.currentTarget.checked)} /> {theme}</label>
                {#if offered}
                  <label class="muted">
                    <input type="radio" name="first-theme" checked={settings.themes[0] === theme} onchange={() => setFirst(theme)} /> starts with it
                  </label>
                {:else}
                  <span></span>
                {/if}
                <button class="edit-theme" title="Open the theme, to change how it looks" onclick={() => onopentheme(theme)}>Edit</button>
              </div>
            {:else}
              <span class="muted">No module declares a theme yet; New theme under Themes in the project tree starts one.</span>
            {/each}
            {#if settings.themes.length > 1}
              <label>
                Show the designer in
                <select value={themeChoice.theme ?? ""} onchange={(e) => (themeChoice.theme = e.currentTarget.value || undefined)}>
                  <option value="">the one it starts with</option>
                  {#each settings.themes as theme (theme)}<option value={theme}>{theme}</option>{/each}
                </select>
              </label>
            {/if}
          </div>
          <p class="form-help">
            The themes the application offers; people switch between them with a click target <code>theme</code>. Without any, it looks as
            Clarity does.
          </p>
        </div>
      </section>

      <section>
        <h2>Output</h2>
        <div class="form-grid">
          <label class="form-label" for="output-folder">Output folder</label>
          <div class="form-inline">
            <input
              id="output-folder"
              class="mono"
              value={settings.out}
              spellcheck="false"
              onchange={(e) => save({ out: e.currentTarget.value.trim() })}
            />
            <button onclick={browseOutput} title="Choose the output folder">Browse...</button>
          </div>
          <p class="form-help">
            Where Run and Build put the application, inside the project's folder or elsewhere. Clarity replaces its
            contents, so it only writes into an empty folder or one it made.
          </p>
        </div>
      </section>

      <section>
        <h2>Data sources</h2>
        <p class="intro">Where the data lives; entities use the first one unless they name another. Secrets stay out of the file: write <code>env:NAME</code> or <code>keychain:NAME</code>.</p>
        {#if settings.datasources.length > 0}
          <div class="table-wrap">
            <table>
              <thead>
                <tr>
                  <th>Name</th>
                  <th>Kind</th>
                  <th>Location</th>
                  <th>Auth</th>
                  <th>Access</th>
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
                      <div class="with-browse">
                        <input
                          class="mono"
                          value={source.url}
                          title={info?.urlLabel}
                          placeholder={info?.urlExample}
                          onchange={(e) => saveSource(i, { url: e.currentTarget.value.trim() })}
                        />
                        {#if source.kind === "sqlite"}
                          <button onclick={() => browseDatabase(i)} title="Choose the database file">Browse...</button>
                        {/if}
                      </div>
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
                    <td>
                      {#if source.kind === "rest"}
                        <span class="muted">external</span>
                      {:else}
                        <div class="flags">
                          <label title="A database Clarity does not own: it never changes its tables">
                            <input type="checkbox" checked={source.external} onchange={(e) => saveSource(i, { external: e.currentTarget.checked })} />
                            External
                          </label>
                          <label title="Records are read, never saved or deleted">
                            <input type="checkbox" checked={source.readonly} onchange={(e) => saveSource(i, { readonly: e.currentTarget.checked })} />
                            Read-only
                          </label>
                        </div>
                      {/if}
                    </td>
                    <td class="actions">
                      {#if source.external && source.kind === "sqlite"}
                        <button title="Import tables of this database as entities" onclick={() => (importing = source.name)}>Import...</button>
                        <button title="Let Clarity change this database's tables from now on" onclick={() => (owning = source.name)}>Own...</button>
                      {/if}
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
        {#if owning}
          <div class="confirm">
            <p>
              Take ownership of <strong>{owning}</strong>? Its tables, as the dictionary maps them, become the first migration step, and from
              then on Clarity changes them through migration steps. The step itself changes nothing in the database.
            </p>
            {#if ownError}<p class="problem">{ownError}</p>{/if}
            <div class="buttons">
              <button onclick={takeOwnership}>Take ownership</button>
              <button onclick={() => ((owning = null), (ownError = null))}>Cancel</button>
            </div>
          </div>
        {/if}
        {#if importing}
          <ImportPanel source={importing} {onapplied} onclose={() => (importing = null)} />
        {/if}
      </section>

      <section>
        <h2>File stores</h2>
        <p class="intro">
          Folders that keep pictures as files, for image fields with <code>store</code>, such as a folder of photos. A folder relative to the
          application, or <code>env:NAME</code> when it differs per machine.
        </p>
        {#if settings.filestores.length > 0}
          <div class="table-wrap">
            <table>
              <thead>
                <tr>
                  <th>Name</th>
                  <th>Folder</th>
                  <th></th>
                </tr>
              </thead>
              <tbody>
                {#each settings.filestores as store, i (i)}
                  <tr>
                    <td><input class="mono" value={store.name} onchange={(e) => saveStore(i, { name: e.currentTarget.value.trim() })} /></td>
                    <td>
                      <div class="with-browse">
                        <input class="mono" value={store.location} placeholder="pictures" onchange={(e) => saveStore(i, { location: e.currentTarget.value.trim() })} />
                        <button onclick={() => browseStore(i)} title="Choose the folder">Browse...</button>
                      </div>
                    </td>
                    <td class="actions">
                      <button class="danger" title="Remove the file store" onclick={() => save({ filestores: settings.filestores.filter((_, j) => j !== i) })}>&times;</button>
                    </td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
        {/if}
        <form class="add" onsubmit={addStore}>
          <input class="mono" bind:value={newStore} placeholder="photos" spellcheck="false" />
          <button type="submit" disabled={!newStore.trim()}>Add file store</button>
        </form>
      </section>

      {#each databases as db (db.name)}
        <MigrationPanel source={db.name} named={databases.length > 1} {preview} {onapplied} />
      {/each}

      <TranslationsPanel {preview} {onapplied} />
    {/key}
  {/if}
</div>

<style>
  .confirm {
    border: 1px solid var(--warning, #d9a400);
    border-radius: var(--radius);
    padding: 10px 12px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .confirm .buttons {
    display: flex;
    gap: 8px;
  }

  .problem {
    color: var(--error);
  }

  .flags {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .flags label {
    display: flex;
    gap: 6px;
    align-items: center;
    white-space: nowrap;
    font-size: 12px;
  }

  td .flags input {
    width: auto;
    height: auto;
  }

  .editor {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  header {
    display: flex;
    align-items: baseline;
    gap: 12px;
    min-width: 0;
  }

  h1 {
    margin: 0;
    font-size: 18px;
    white-space: nowrap;
  }

  .file {
    font-family: var(--mono);
    font-size: 12px;
    color: var(--text-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* Each group of settings is a card, like the module view. */
  /* Sections of child components, such as the migrations, look the same. */
  .editor :global(section) {
    display: flex;
    flex-direction: column;
    gap: 10px;
    background: var(--panel);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 12px 16px 16px;
  }

  .editor :global(h2) {
    margin: 0;
    font-size: 12px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-muted);
  }

  .editor :global(p) {
    margin: 0;
  }

  .editor :global(.intro) {
    font-size: 12px;
    color: var(--text-muted);
  }

  .editor :global(.intro code) {
    font-family: var(--mono);
  }

  .muted {
    color: var(--text-muted);
    font-size: 12px;
  }

  .mono {
    font-family: var(--mono);
    font-size: 12px;
  }

  .themes {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .theme-row {
    display: grid;
    grid-template-columns: 140px 130px auto;
    gap: 16px;
    align-items: center;
  }

  .edit-theme {
    justify-self: start;
    border: none;
    background: none;
    padding: 0;
    font-size: 12px;
    color: var(--accent);
    cursor: pointer;
  }

  .theme-row label {
    display: flex;
    gap: 6px;
    align-items: center;
  }

  .problems {
    margin: 0;
    padding: 8px 12px 8px 28px;
    border: 1px solid var(--error);
    border-radius: var(--radius);
    color: var(--error);
    font-size: 12px;
  }

  .with-browse {
    display: flex;
    gap: 6px;
  }

  .with-browse input {
    flex: 1;
    min-width: 0;
  }

  .with-browse button {
    flex: none;
    white-space: nowrap;
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

  /* One height for every field, as in the form grid. */
  td input,
  td select,
  .add input,
  .add select {
    height: 30px;
    box-sizing: border-box;
  }

  td select,
  .add select {
    padding-top: 0;
    padding-bottom: 0;
  }

  td select {
    width: 100%;
  }

  tr:last-child td {
    border-bottom: none;
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
