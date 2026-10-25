<script lang="ts">
  // The main area of the Vault tab: making or unlocking Clarity's vault,
  // its secrets, and a connection's fields. Clarity's vault is Clarity's
  // own; an application keeps its own secrets, only when its design adds them.
  import Modal from "./Modal.svelte";
  import SecretField from "./SecretField.svelte";
  import { resizableColumns } from "./columnWidths";
  import { CategoryEditor } from "./vault";
  import { vault } from "./vaultStore.svelte";

  let password = $state("");
  let repeat = $state("");
  let current = $state("");
  let next = $state("");
  let nextRepeat = $state("");
  // The categories dialog: its list, the category picked in it, the name
  // being typed for Add or Rename, and why the last step was refused.
  let editor = $state<CategoryEditor | null>(null);
  let pickedCategory = $state<string | null>(null);
  let naming = $state<null | { mode: "add" | "rename"; name: string }>(null);
  let refusal = $state<string | null>(null);
  // The editor is a plain class, so each change counts here to redraw its list.
  let changes = $state(0);
  const categoryItems = $derived.by(() => {
    void changes;
    return editor ? [...editor.items] : [];
  });

  $effect(() => {
    if (vault.dialog === "categories") {
      editor = vault.categoryEditor();
      pickedCategory = null;
      naming = null;
      refusal = null;
    }
    if (vault.dialog !== null) current = next = nextRepeat = "";
  });

  /** Puts the caret in an input as soon as it shows, every time it shows. */
  function focused(node: HTMLInputElement) {
    queueMicrotask(() => {
      node.focus();
      node.select();
    });
  }

  function finishNaming() {
    if (!editor || !naming) return;
    refusal = naming.mode === "add" ? editor.add(naming.name) : pickedCategory ? editor.rename(pickedCategory, naming.name) : null;
    if (refusal === null) {
      pickedCategory = naming.name.trim() || pickedCategory;
      naming = null;
      changes++;
    }
  }

  /** Saves the list, with the name still being typed when there is one. */
  function saveCategories() {
    if (!editor) return;
    if (naming && naming.name.trim() !== "") {
      finishNaming();
      if (refusal !== null) return;
    }
    const r = editor.result();
    vault.saveCategories(r.names, r.renames);
  }

  function removeCategory() {
    if (!editor || !pickedCategory) return;
    refusal = editor.remove(pickedCategory);
    if (refusal === null) {
      pickedCategory = null;
      changes++;
    }
  }

  /** The categories a secret can have: the list, and its own when that is no longer in it. */
  const secretCategories = $derived.by(() => {
    const own = vault.editing?.was?.category;
    const list = [...vault.categories];
    if (own && !list.some((c) => c.toLowerCase() === own.toLowerCase())) list.unshift(own);
    return list;
  });
  const editedRow = $derived(vault.rows.find((r) => vault.editing?.was && r.category === vault.editing.was.category && r.key === vault.editing.was.key));

  async function create() {
    if (password !== repeat) return;
    if (await vault.create(password)) password = repeat = "";
  }

  async function unlock() {
    if (await vault.unlock(password)) password = "";
  }

  const time = (utc: string) => utc.replace("T", " ").slice(0, 16);
</script>

<section class="vault">
  {#if vault.error}
    <p class="error" role="alert">{vault.error} <button onclick={() => (vault.error = null)}>Dismiss</button></p>
  {/if}

  {#if vault.view === "connections" && vault.draft}
    {@const draft = vault.draft}
    <h2>{#if vault.connection}Connection <code>{vault.connection}</code>{:else}New connection{/if}</h2>
    <p class="intro">
      A connection is a named set of fields. Plain fields hold their text; a field from the vault points at a secret by its
      category and key, so the password never sits in a file. Clarity reads a connection by its name; each field by the name a
      data source or a web API asks for, such as host, user or password.
    </p>
    <div class="form-grid">
      <label class="form-label" for="connection-name">Name</label>
      <input id="connection-name" class="mono" bind:value={draft.name} spellcheck="false" />
    </div>
    <table class="fields" use:resizableColumns={"clarity.columns.connectionFields"}>
      <thead><tr><th>Field</th><th>From</th><th>Value</th><th></th></tr></thead>
      <tbody>
        {#each draft.fields as field, i (i)}
          <tr>
            <td><input class="mono" bind:value={field[0]} placeholder="host" spellcheck="false" aria-label="Field name" /></td>
            <td>
              <select value={field[1].mode} onchange={(e) => vault.setMode(i, e.currentTarget.value as "plain" | "vault")} aria-label="Where the value is">
                <option value="plain">Plain text</option>
                <option value="vault">Vault</option>
              </select>
            </td>
            <td>
              {#if field[1].mode === "plain"}
                <input bind:value={field[1].value} spellcheck="false" aria-label="Value" />
              {:else}
                {@const secret = field[1]}
                <div class="pair">
                  <input list="vault-categories" bind:value={secret.category} placeholder="Category" spellcheck="false" aria-label="Category" />
                  <input list="vault-keys-{i}" bind:value={secret.key} placeholder="Key" spellcheck="false" aria-label="Key" />
                  <datalist id="vault-keys-{i}">
                    {#each vault.keysOf(secret.category) as key (key)}<option value={key}></option>{/each}
                  </datalist>
                </div>
              {/if}
            </td>
            <td><button title="Remove the field" onclick={() => vault.removeField(i)}>&times;</button></td>
          </tr>
        {/each}
      </tbody>
    </table>
    <datalist id="vault-categories">
      {#each vault.categories as category (category)}<option value={category}></option>{/each}
    </datalist>
    {#if vault.problems.length > 0}
      <ul class="problems">
        {#each vault.problems as problem (problem)}<li>{problem}</li>{/each}
      </ul>
    {/if}
    <div class="actions">
      <button onclick={() => vault.addField()}>Add field</button>
      <span class="spacer"></span>
      <button disabled={!vault.changed} onclick={() => vault.pickConnection(vault.connection)}>Undo changes</button>
      <button class="primary" disabled={!vault.changed || vault.problems.length > 0} onclick={() => vault.saveConnection()}>Save</button>
    </div>
    {#if !vault.unlocked}
      <p class="hint">Unlock the vault to choose secrets from it; a field can name one either way.</p>
    {/if}
  {:else if vault.view === "connections"}
    <h2>Connections</h2>
    <p class="intro">
      {vault.connections.length === 0 ? "There are no connections yet." : "Pick a connection in the sidebar."} New connection in the
      ribbon adds one. They are kept in <code>connections.json</code> beside the vault.
    </p>
  {:else if !vault.info}
    <p class="hint">Reading the vault...</p>
  {:else if !vault.info.exists}
    <h2>Clarity's vault</h2>
    <p class="intro">
      The vault keeps the secrets Clarity uses while you design, such as the password of a database and the key of a web API,
      encrypted behind one master password that only you know. It is Clarity's own, in <code>{vault.info.folder}</code>; an application you design has its own, only when its design adds one.
    </p>
    <div class="form-grid">
      <label class="form-label" for="new-password">Master password</label>
      <SecretField id="new-password" bind:value={password} autocomplete="new-password" />
      <label class="form-label" for="repeat-password">Again</label>
      <SecretField id="repeat-password" bind:value={repeat} autocomplete="new-password" />
      {#if repeat && password !== repeat}<p class="form-help">The two passwords differ.</p>{/if}
      <span></span>
      <div class="form-inline">
        <button class="primary" disabled={!password || password !== repeat || vault.busy} onclick={create}>{vault.busy ? "Making..." : "Make the vault"}</button>
      </div>
      <p class="form-help">Write the password down somewhere safe: without it, nobody can open the vault, Clarity included.</p>
    </div>
  {:else if !vault.info.unlocked}
    <h2>Clarity's vault is locked</h2>
    <p class="intro">It locks itself after 15 minutes without use. <code>{vault.info.folder}</code></p>
    <form class="form-grid" onsubmit={(e) => (e.preventDefault(), unlock())}>
      <label class="form-label" for="unlock-password">Master password</label>
      <SecretField id="unlock-password" bind:value={password} autocomplete="current-password" />
      <span></span>
      <div class="form-inline">
        <button class="primary" type="submit" disabled={!password || vault.busy}>{vault.busy ? "Unlocking..." : "Unlock"}</button>
      </div>
    </form>
  {:else if vault.editing}
    {@const draft = vault.editing}
    <h2>{draft.was ? "Change the secret" : "New secret"}</h2>
    <form class="form-grid" onsubmit={(e) => (e.preventDefault(), vault.saveSecret())}>
      <label class="form-label" for="secret-category">Category</label>
      {#if secretCategories.length === 0}
        <p class="refusal">No categories defined. Add one with Categories in the ribbon first.</p>
      {:else}
        <select id="secret-category" bind:value={draft.category}>
          {#if !draft.category}<option value="" disabled>Pick a category</option>{/if}
          {#each secretCategories as category (category)}<option value={category}>{category}</option>{/each}
        </select>
      {/if}
      <label class="form-label" for="secret-key">Key</label>
      <input id="secret-key" bind:value={draft.key} spellcheck="false" />
      <label class="form-label" for="secret-value">Value</label>
      <SecretField id="secret-value" bind:value={draft.value} />
      <label class="form-label" for="secret-description">Description</label>
      <textarea id="secret-description" rows="4" bind:value={draft.description}></textarea>
      <p class="form-help">Not secret: the description shows in the list, to tell secrets apart.</p>
      {#if editedRow}
        <span></span>
        <p class="form-help">Created {time(editedRow.created)} UTC · Updated {time(editedRow.updated)} UTC</p>
      {/if}
      <span></span>
      <div class="form-inline">
        <button type="button" onclick={() => (vault.editing = null)}>Cancel</button>
        <button class="primary" type="submit" disabled={!draft.category.trim() || !draft.key.trim()}>Save</button>
      </div>
    </form>
  {:else}
    <input class="filter" placeholder="Filter secrets..." bind:value={vault.filter} aria-label="Filter secrets" />
    <table class="secrets" use:resizableColumns={"clarity.columns.vaultSecrets"}>
      <thead><tr><th>Category</th><th>Key</th><th>Description</th><th>Updated (UTC)</th></tr></thead>
      <tbody>
        {#each vault.shown as row (row.category + "\u0000" + row.key)}
          {@const picked = vault.selected?.category === row.category && vault.selected?.key === row.key}
          <tr
            class:picked
            onclick={() => (vault.selected = { category: row.category, key: row.key })}
            ondblclick={() => vault.editSecret()}
          >
            <td>{row.category}</td>
            <td>{row.key}</td>
            <td class="description">{row.description}</td>
            <td class="time">{time(row.updated)}</td>
          </tr>
        {:else}
          <tr><td colspan="4" class="hint">{vault.rows.length === 0 ? "No secrets yet. Add one with Add in the ribbon." : "No secret matches."}</td></tr>
        {/each}
      </tbody>
    </table>
    {#if vault.copied}<p class="hint">Copied the value of {vault.copied} to the clipboard.</p>{/if}
  {/if}

  {#if vault.dialog === "categories" && editor}
    <Modal title="Categories" width={640} onclose={() => (vault.dialog = null)}>
      {#if vault.error}<p class="refusal" role="alert">{vault.error}</p>{/if}
      <div class="category-editor">
        <ul class="category-list" role="listbox" aria-label="Categories">
          {#each categoryItems as category (category)}
            <li>
              <button class:active={pickedCategory === category} role="option" aria-selected={pickedCategory === category} onclick={() => ((pickedCategory = category), (naming = null), (refusal = null))}>
                {category} <span>{editor.usage(category) || ""}</span>
              </button>
            </li>
          {:else}
            <li class="hint">No categories yet.</li>
          {/each}
        </ul>
        <div class="category-buttons">
          <button onclick={() => ((naming = { mode: "add", name: "" }), (refusal = null))}>Add</button>
          <button disabled={!pickedCategory} onclick={() => pickedCategory && ((naming = { mode: "rename", name: pickedCategory }), (refusal = null))}>Rename</button>
          <button disabled={!pickedCategory} onclick={removeCategory}>Remove</button>
        </div>
      </div>
      {#if naming}
        {@const usedBy = naming.mode === "rename" && pickedCategory ? editor.usage(pickedCategory) : 0}
        <form class="naming" onsubmit={(e) => (e.preventDefault(), finishNaming())}>
          <label for="category-name">{naming.mode === "add" ? "New category" : `Rename '${pickedCategory}'`}</label>
          <input id="category-name" bind:value={naming.name} spellcheck="false" use:focused />
          <button type="submit">OK</button>
          <button type="button" onclick={() => ((naming = null), (refusal = null))}>Cancel</button>
        </form>
        {#if usedBy > 0}<p class="hint">{usedBy} secret(s) use this category; they move to the new name when you save.</p>{/if}
      {/if}
      {#if refusal}<p class="refusal" role="alert">{refusal}</p>{/if}
      <div class="actions">
        <span class="spacer"></span>
        <button onclick={() => (vault.dialog = null)}>Cancel</button>
        <button class="primary" onclick={saveCategories}>Save</button>
      </div>
    </Modal>
  {:else if vault.dialog === "password"}
    <Modal title="Change the master password" width={440} onclose={() => (vault.dialog = null)}>
      {#if vault.error}<p class="refusal" role="alert">{vault.error}</p>{/if}
      <div class="form-grid">
        <label class="form-label" for="current-password">Current</label>
        <SecretField id="current-password" bind:value={current} autocomplete="current-password" />
        <label class="form-label" for="next-password">New</label>
        <SecretField id="next-password" bind:value={next} autocomplete="new-password" />
        <label class="form-label" for="next-repeat">Again</label>
        <SecretField id="next-repeat" bind:value={nextRepeat} autocomplete="new-password" />
      </div>
      <div class="actions">
        <button onclick={() => (vault.dialog = null)}>Cancel</button>
        <button class="primary" disabled={!current || !next || next !== nextRepeat || vault.busy} onclick={() => vault.changePassword(current, next)}>
          {vault.busy ? "Changing..." : "Change"}
        </button>
      </div>
    </Modal>
  {:else if vault.dialog === "destroy"}
    <Modal title="Destroy the vault" width={440} tone="danger" onclose={() => (vault.dialog = null)}>
      {#if vault.error}<p class="refusal" role="alert">{vault.error}</p>{/if}
      <p>Every secret in it is gone for good, for Clarity and for any tool that shares this vault. Connections stay, but their vault fields find nothing.</p>
      <div class="form-grid">
        <label class="form-label" for="destroy-password">Master password</label>
        <SecretField id="destroy-password" bind:value={current} autocomplete="current-password" />
      </div>
      <div class="actions">
        <button onclick={() => (vault.dialog = null)}>Cancel</button>
        <button class="destroy" disabled={!current || vault.busy} onclick={() => vault.destroy(current)}>Destroy the vault</button>
      </div>
    </Modal>
  {/if}
</section>

<style>
  .vault {
    padding: 20px 24px;
    display: flex;
    flex-direction: column;
    gap: 12px;
    min-height: 0;
    overflow: auto;
  }

  h2 {
    margin: 0;
    font-size: 18px;
  }

  .intro,
  .hint {
    margin: 0;
    font-size: 13px;
    color: var(--text-muted);
    max-width: 760px;
  }

  code,
  .mono {
    font-family: var(--mono);
  }

  .error {
    margin: 0;
    padding: 8px 12px;
    border-radius: 8px;
    background: var(--error-soft);
    color: var(--error);
    display: flex;
    gap: 12px;
    align-items: center;
  }

  .filter {
    width: 100%;
    box-sizing: border-box;
  }

  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 13px;
  }

  th {
    text-align: left;
    font-weight: 500;
    color: var(--text-muted);
    padding: 6px 8px;
    border-bottom: 1px solid var(--border-strong);
  }

  td {
    padding: 6px 8px;
    border-bottom: 1px solid var(--border);
  }

  .secrets tbody tr {
    cursor: default;
  }

  .secrets tbody tr:hover {
    background: var(--panel-alt);
  }

  .secrets tr.picked {
    background: color-mix(in srgb, var(--accent) 22%, transparent);
  }

  #secret-description {
    width: 100%;
    box-sizing: border-box;
    resize: vertical;
  }

  /* The label of a box of several lines sits at its top. */
  label[for="secret-description"] {
    align-self: start;
    padding-top: 6px;
  }

  .description {
    white-space: pre-line;
  }

  .time {
    white-space: nowrap;
    color: var(--text-muted);
  }

  .fields td {
    padding: 4px;
    border: none;
  }

  .fields input,
  .fields select {
    width: 100%;
    box-sizing: border-box;
  }

  .fields td:first-child {
    width: 22%;
  }

  .fields td:nth-child(2) {
    width: 130px;
  }

  .fields td:last-child {
    width: 36px;
  }

  .pair {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 6px;
  }

  .problems {
    margin: 0;
    padding-left: 18px;
    color: var(--warning);
    font-size: 13px;
  }

  .actions {
    display: flex;
    gap: 8px;
    align-items: center;
  }

  .spacer {
    flex: 1;
  }

  /* A dialog sits over the screen, as a window of its own would. */


  .category-editor {
    display: flex;
    gap: 10px;
    align-items: flex-start;
  }

  .category-list {
    list-style: none;
    margin: 0;
    padding: 4px;
    flex: 1;
    min-height: 160px;
    max-height: 280px;
    overflow: auto;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
  }

  .category-list button {
    width: 100%;
    display: flex;
    justify-content: space-between;
    border: none;
    background: transparent;
    text-align: left;
    padding: 4px 8px;
  }

  .category-list button.active {
    background: color-mix(in srgb, var(--accent) 25%, transparent);
  }

  .category-list span {
    color: var(--text-muted);
    font-size: 12px;
  }

  .category-buttons {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .naming {
    display: flex;
    gap: 8px;
    align-items: center;
  }

  .naming input {
    flex: 1;
  }

  .refusal {
    margin: 0;
    color: var(--error);
    font-size: 13px;
  }

  .destroy {
    background: var(--error);
    border-color: var(--error);
    color: #fff;
  }
</style>
