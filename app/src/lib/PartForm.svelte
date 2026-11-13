<script lang="ts">
  // A custom part's declaration as a form: its library, the value it binds,
  // its properties, tells and hears, its size on the palette and its
  // picture. Each change writes one line of the declaration.

  import { invoke } from "@clarity/ui/host/backend.ts";
  import type { CodeView } from "@clarity/ui/generated/CodeView.ts";
  import type { PartDecl } from "@clarity/ui/generated/PartDecl.ts";
  import type { PartLibrary } from "@clarity/ui/generated/PartLibrary.ts";
  import type { SpecFile } from "@clarity/ui/generated/SpecFile.ts";
  import {
    PART_KINDS,
    messageLine,
    missingOf,
    paramsOf,
    partLibraryLine,
    propertyLine,
    sliceOf,
    startPartLibrary,
    typeChoices,
    type DeclLine,
    type PartKind,
  } from "./customDecl.ts";

  let {
    decl,
    file,
    onset,
    ongrows,
  }: {
    decl: PartDecl;
    file: SpecFile;
    /** Writes one line; throws with the reason when it is refused. */
    onset: (line: DeclLine) => Promise<void>;
    ongrows: (grows: boolean) => void;
  } = $props();

  const item = $derived(decl.span.start);
  const name = $derived(decl.name.name);
  let error = $state<string | null>(null);
  /** The declaration as written, for the values of properties as typed. */
  let declText = $state("");
  $effect(() => {
    const at = item;
    void file.version;
    invoke<CodeView>("decl_text", { path: file.path, item: at })
      .then((v) => (declText = v.text))
      .catch(() => (declText = ""));
  });

  async function set(line: DeclLine) {
    error = null;
    try {
      await onset(line);
    } catch (err) {
      error = String(err);
    }
  }

  /** The library being filled in, until it reads as a line. */
  let library = $state<PartLibrary | null>(null);
  $effect.pre(() => {
    library = decl.library ? { ...decl.library } : null;
  });

  function pickKind(kind: PartKind) {
    library = startPartLibrary(kind, name);
    saveLibrary();
  }

  function saveLibrary() {
    if (!library) return;
    const missing = missingOf(library);
    if (missing) {
      error = `Fill in ${missing} to write the library line.`;
      return;
    }
    set({ key: "library", line: partLibraryLine(library) });
  }

  const expr = (e: { span: { start: number; end: number } } | null) => (e && declText ? sliceOf(declText, item, e.span) : "");
  const lineOf = (span: { start: number; end: number }) => (declText ? sliceOf(declText, item, span) : "");
  const typeOf = (p: PartDecl["properties"][number]) => {
    const line = lineOf(p.span);
    const colon = line.indexOf(":");
    return colon >= 0 ? line.slice(colon + 1).split(",")[0].trim() : "";
  };

  function setProperty(p: PartDecl["properties"][number], change: Partial<{ name: string; type: string; min: string; max: string; default: string }>) {
    const values = { min: expr(p.min), max: expr(p.max), default: expr(p.default), ...change };
    const line = propertyLine(change.name ?? p.name.name, change.type ?? typeOf(p), values);
    set({ key: `property ${p.name.name}`, line: line.line });
  }

  let newProperty = $state({ name: "", type: "text" });
  let newMessage = $state({ word: "tells" as "tells" | "hears", name: "", params: "" });
  const types = $derived(typeChoices(file.enums.map((e) => e.name.name)));
</script>

<datalist id="part-types-{name}">
  {#each types as t (t)}<option value={t}></option>{/each}
</datalist>

<div class="form">
  {#if error}<p class="error">{error}</p>{/if}

  <section>
    <h3>Library</h3>
    <div class="grid">
      <label for="part-kind">Drawn by</label>
      <select id="part-kind" value={library?.kind ?? ""} onchange={(e) => pickKind(e.currentTarget.value as PartKind)}>
        {#if !library}<option value="">choose</option>{/if}
        {#each PART_KINDS as k (k.kind)}<option value={k.kind}>{k.kind}: {k.label}</option>{/each}
      </select>
      {#if library}
        {#if library.kind === "native" || library.kind === "crate" || library.kind === "go"}
          <label for="part-folder">Folder</label>
          <input id="part-folder" bind:value={library.folder} onchange={saveLibrary} spellcheck="false" />
        {:else if library.kind === "nuget"}
          <label for="part-package">Package</label>
          <input id="part-package" bind:value={library.package} onchange={saveLibrary} placeholder="Markdig" spellcheck="false" />
          <label for="part-version">Version</label>
          <input id="part-version" bind:value={library.version} onchange={saveLibrary} placeholder="0.40.0" spellcheck="false" />
        {:else}
          <label for="part-file">Assembly</label>
          <input id="part-file" bind:value={library.file} onchange={saveLibrary} spellcheck="false" />
        {/if}
      {/if}
    </div>
  </section>

  <section>
    <h3>What the window sees</h3>
    <div class="grid">
      <label for="part-binds">Binds</label>
      <input
        id="part-binds"
        list="part-types-{name}"
        value={decl.binds ? lineOf(decl.binds.span) : ""}
        placeholder="nothing"
        title="The type of value it shows and edits, which bind gives it; empty for none"
        onchange={(e) => set({ key: "binds", line: e.currentTarget.value.trim() ? `binds ${e.currentTarget.value.trim()}` : null })}
      />
      <label for="part-span">Size on the palette</label>
      <span class="row">
        <input id="part-span" class="num" type="number" min="1" max="12" value={decl.size?.[0] ?? ""} placeholder="span"
          onchange={(e) => {
            const span = Number.parseInt(e.currentTarget.value, 10);
            set({ key: "size", line: span > 0 ? `size ${span} x ${decl.size?.[1] ?? 4}` : null });
          }} />
        x
        <input class="num" type="number" min="1" value={decl.size?.[1] ?? ""} placeholder="rows" aria-label="Rows"
          onchange={(e) => {
            const rows = Number.parseInt(e.currentTarget.value, 10);
            set({ key: "size", line: rows > 0 ? `size ${decl.size?.[0] ?? 6} x ${rows}` : null });
          }} />
      </span>
      <label for="part-preview">Picture</label>
      <input id="part-preview" value={decl.preview ?? ""} placeholder="a picture of the resources, such as arena.png" spellcheck="false"
        onchange={(e) => set({ key: "preview", line: e.currentTarget.value.trim() ? `preview ${JSON.stringify(e.currentTarget.value.trim())}` : null })} />
      <span>Layout</span>
      <label class="check" title="It takes the height the window has left, as a table or a canvas does; the declaration's grows line">
        <input type="checkbox" checked={decl.grows} onchange={(e) => ongrows(e.currentTarget.checked)} /> Grows with the window
      </label>
    </div>
  </section>

  <section>
    <h3>Properties</h3>
    <table>
      <thead><tr><th>Name</th><th>Type</th><th>Min</th><th>Max</th><th>Default</th><th></th></tr></thead>
      <tbody>
        {#each decl.properties as p (p.name.name)}
          <tr>
            <td><input value={p.name.name} spellcheck="false" onchange={(e) => setProperty(p, { name: e.currentTarget.value.trim() })} /></td>
            <td><input list="part-types-{name}" value={typeOf(p)} onchange={(e) => setProperty(p, { type: e.currentTarget.value })} /></td>
            <td><input class="num" value={expr(p.min)} onchange={(e) => setProperty(p, { min: e.currentTarget.value })} /></td>
            <td><input class="num" value={expr(p.max)} onchange={(e) => setProperty(p, { max: e.currentTarget.value })} /></td>
            <td><input value={expr(p.default)} onchange={(e) => setProperty(p, { default: e.currentTarget.value })} /></td>
            <td><button class="link" onclick={() => set({ key: `property ${p.name.name}`, line: null })}>Remove</button></td>
          </tr>
        {:else}
          <tr><td colspan="6" class="muted">No properties: values a window sets on the part and code reads and sets.</td></tr>
        {/each}
      </tbody>
    </table>
    <form
      class="add"
      onsubmit={(e) => {
        e.preventDefault();
        const n = newProperty.name.trim();
        if (n) set(propertyLine(n, newProperty.type || "text", {})).then(() => (newProperty.name = ""));
      }}
    >
      <input bind:value={newProperty.name} placeholder="speed" spellcheck="false" aria-label="New property's name" />
      <input bind:value={newProperty.type} list="part-types-{name}" aria-label="New property's type" />
      <button type="submit" disabled={!newProperty.name.trim()}>Add property</button>
    </form>
  </section>

  <section>
    <h3>Tells and hears</h3>
    <table>
      <thead><tr><th></th><th>Name</th><th>Values</th><th></th></tr></thead>
      <tbody>
        {#each [...decl.tells.map((m) => ({ word: "tells" as const, m })), ...decl.hears.map((m) => ({ word: "hears" as const, m }))] as { word, m } (word + m.name.name)}
          <tr>
            <td class="muted" title={word === "tells" ? "An event of the part, which a handler takes" : "Something code tells the part to do"}>{word}</td>
            <td><input value={m.name.name} spellcheck="false" onchange={(e) => set({ key: `${word} ${m.name.name}`, line: messageLine(word, e.currentTarget.value.trim(), paramsOf(lineOf(m.span))).line })} /></td>
            <td><input value={paramsOf(lineOf(m.span))} placeholder="points: integer" spellcheck="false" onchange={(e) => set(messageLine(word, m.name.name, e.currentTarget.value))} /></td>
            <td><button class="link" onclick={() => set({ key: `${word} ${m.name.name}`, line: null })}>Remove</button></td>
          </tr>
        {:else}
          <tr><td colspan="4" class="muted">No tells or hears: events of the part, and what code tells it to do.</td></tr>
        {/each}
      </tbody>
    </table>
    <form
      class="add"
      onsubmit={(e) => {
        e.preventDefault();
        const n = newMessage.name.trim();
        if (n) set(messageLine(newMessage.word, n, newMessage.params)).then(() => ((newMessage.name = ""), (newMessage.params = "")));
      }}
    >
      <select bind:value={newMessage.word} aria-label="Tell or hear">
        <option value="tells">tells</option>
        <option value="hears">hears</option>
      </select>
      <input bind:value={newMessage.name} placeholder="scored" spellcheck="false" aria-label="Name" />
      <input bind:value={newMessage.params} placeholder="points: integer" spellcheck="false" aria-label="Values" />
      <button type="submit" disabled={!newMessage.name.trim()}>Add</button>
    </form>
  </section>
</div>

<style>
  .form {
    display: flex;
    flex-direction: column;
    gap: 12px;
    font-size: 12px;
  }
  h3 {
    font-size: 0.85em;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    margin: 0 0 6px;
  }
  .grid {
    display: grid;
    grid-template-columns: max-content minmax(0, 420px);
    gap: 4px 10px;
    align-items: center;
  }
  .row {
    display: flex;
    gap: 6px;
    align-items: center;
  }
  .check {
    display: flex;
    gap: 6px;
    align-items: center;
  }
  table {
    border-collapse: collapse;
  }
  th {
    text-align: left;
    opacity: 0.7;
    font-weight: 600;
    padding: 2px 6px 2px 0;
  }
  td {
    padding: 2px 6px 2px 0;
  }
  .num {
    width: 64px;
  }
  .add {
    display: flex;
    gap: 6px;
    margin-top: 6px;
  }
  .link {
    border: none;
    background: none;
    padding: 0;
    color: var(--accent);
    cursor: pointer;
    font-size: 12px;
  }
  .muted {
    opacity: 0.7;
  }
  .error {
    color: var(--error);
    margin: 0;
  }
</style>
