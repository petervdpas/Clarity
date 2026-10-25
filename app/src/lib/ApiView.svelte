<script lang="ts">
  // A web API of the project (DESIGN 5.23), opened from the project tree:
  // where it is, how it is signed, and what the application uses of its
  // OpenAPI description. It is a data source of the project file, as in the
  // project settings, shown where a developer looks for it.

  import ApiPanel from "./ApiPanel.svelte";
  import type { AuthKind } from "./generated/AuthKind";
  import type { DataSource } from "./generated/DataSource";
  import type { ProjectFile } from "./generated/ProjectFile";
  import type { Settings } from "./generated/Settings";

  let {
    project,
    name,
    onsave,
  }: {
    project: ProjectFile;
    name: string;
    onsave: (settings: Settings) => void;
  } = $props();

  const at = $derived(project.settings?.datasources.findIndex((s) => s.name === name) ?? -1);
  const source = $derived(at >= 0 ? project.settings?.datasources[at] : undefined);

  function change(changed: Partial<DataSource>) {
    const settings = project.settings;
    if (!settings || at < 0) return;
    onsave({ ...settings, datasources: settings.datasources.map((s, i) => (i === at ? { ...s, ...changed } : s)) });
  }

  function setAuth(kind: AuthKind | "") {
    if (!source) return;
    change({ auth: kind === "" ? null : { kind, secret: source.auth?.secret ?? `env:${source.name.toUpperCase()}_TOKEN` } });
  }
</script>

<div class="view">
  {#if source}
    <header>
      <h1>Web API {source.name}</h1>
      <span class="muted">a data source of the project file</span>
    </header>
    <section class="fields">
      <label class="form-label" for="api-address">Address</label>
      <input id="api-address" class="mono" value={source.url} placeholder="https://api.example.com" spellcheck="false" onchange={(e) => change({ url: e.currentTarget.value.trim() })} />
      <p class="form-help">Where the service is; <code>http get "/orders" from {source.name}</code> calls a path of it.</p>
      <label class="form-label" for="api-auth">Key</label>
      <div class="auth">
        <select id="api-auth" value={source.auth?.kind ?? ""} onchange={(e) => setAuth(e.currentTarget.value as AuthKind | "")}>
          <option value="">None</option>
          {#each project.catalog.auths as a (a.id)}<option value={a.id}>{a.label}</option>{/each}
        </select>
        {#if source.auth}
          <input
            class="mono"
            value={source.auth.secret}
            title="Where the key is kept: env:NAME for an environment variable"
            spellcheck="false"
            onchange={(e) => source.auth && change({ auth: { ...source.auth, secret: e.currentTarget.value.trim() } })}
          />
        {/if}
      </div>
      <p class="form-help">How each call is signed. The key itself never goes in the project: name where it is kept, as <code>env:SHOP_TOKEN</code>.</p>
    </section>
    <ApiPanel {source} onsave={change} />
  {:else}
    <p class="muted">The project file has no web API {name}.</p>
  {/if}
</div>

<style>
  .view {
    display: flex;
    flex-direction: column;
    gap: 14px;
    padding: 16px 24px;
    overflow: auto;
    height: 100%;
    box-sizing: border-box;
  }
  header {
    display: flex;
    align-items: baseline;
    gap: 12px;
  }
  h1 {
    margin: 0;
    font-size: 18px;
  }
  .muted {
    color: var(--text-muted);
    font-size: 12px;
  }
  .fields {
    display: grid;
    grid-template-columns: 120px minmax(0, 1fr);
    gap: 6px 12px;
    align-items: center;
  }
  .form-help {
    grid-column: 2;
    margin: 0 0 6px;
    color: var(--text-muted);
    font-size: 12px;
  }
  .auth {
    display: flex;
    gap: 6px;
  }
  .auth input {
    flex: 1;
  }
</style>
