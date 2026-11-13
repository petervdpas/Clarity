<script lang="ts">
  // A library's declaration as a form: its package, and the functions code
  // calls with their values and result. Each change writes one line of the
  // declaration.

  import { invoke } from "@clarity/ui/host/backend.ts";
  import type { CodeView } from "@clarity/ui/generated/CodeView.ts";
  import type { LibraryDecl } from "@clarity/ui/generated/LibraryDecl.ts";
  import type { LibrarySource } from "@clarity/ui/generated/LibrarySource.ts";
  import type { SpecFile } from "@clarity/ui/generated/SpecFile.ts";
  import {
    LIBRARY_KINDS,
    functionLine,
    librarySourceLine,
    missingOf,
    paramsOf,
    sliceOf,
    startLibrarySource,
    typeChoices,
    type DeclLine,
    type LibraryKind,
  } from "./customDecl.ts";

  let {
    decl,
    file,
    onset,
  }: {
    decl: LibraryDecl;
    file: SpecFile;
    /** Writes one line; throws with the reason when it is refused. */
    onset: (line: DeclLine) => Promise<void>;
  } = $props();

  const item = $derived(decl.span.start);
  const name = $derived(decl.name.name);
  let error = $state<string | null>(null);
  /** The declaration as written, for the values and results as typed. */
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

  /** The package being filled in, until it reads as a line. */
  let source = $state<LibrarySource | null>(null);
  $effect.pre(() => {
    source = decl.source ? { ...decl.source } : null;
  });

  function pickKind(kind: LibraryKind) {
    source = startLibrarySource(kind, name);
    saveSource();
  }

  function saveSource() {
    if (!source) return;
    const missing = missingOf(source);
    if (missing) {
      error = `Fill in ${missing} to write the package line.`;
      return;
    }
    set({ key: "source", line: librarySourceLine(source) });
  }

  const lineOf = (span: { start: number; end: number }) => (declText ? sliceOf(declText, item, span) : "");
  const returnsOf = (f: LibraryDecl["functions"][number]) => (f.returns ? lineOf(f.returns.span) : "");
  let newFunction = $state({ name: "", params: "", returns: "" });
  const types = $derived(typeChoices([]));
</script>

<datalist id="library-types-{name}">
  {#each types as t (t)}<option value={t}></option>{/each}
</datalist>

<div class="form">
  {#if error}<p class="error">{error}</p>{/if}

  <section>
    <h3>Package</h3>
    <div class="grid">
      <label for="library-kind">From</label>
      <select id="library-kind" value={source?.kind ?? ""} onchange={(e) => pickKind(e.currentTarget.value as LibraryKind)}>
        {#if !source}<option value="">choose</option>{/if}
        {#each LIBRARY_KINDS as k (k.kind)}<option value={k.kind}>{k.kind}: {k.label}</option>{/each}
      </select>
      {#if source}
        {#if source.kind === "nuget"}
          <label for="library-package">Package</label>
          <input id="library-package" bind:value={source.package} onchange={saveSource} placeholder="Humanizer.Core" spellcheck="false" />
          <label for="library-version">Version</label>
          <input id="library-version" bind:value={source.version} onchange={saveSource} placeholder="2.14.1" spellcheck="false" />
        {:else if source.kind === "crate"}
          <label for="library-crate">Crate</label>
          <input id="library-crate" bind:value={source.name} onchange={saveSource} placeholder="humanize" spellcheck="false" />
          <label for="library-version">Version</label>
          <input id="library-version" bind:value={source.version} onchange={saveSource} placeholder="0.1" spellcheck="false" />
        {:else if source.kind === "go"}
          <label for="library-module">Module</label>
          <input id="library-module" bind:value={source.module} onchange={saveSource} placeholder="github.com/dustin/go-humanize" spellcheck="false" />
          <label for="library-version">Version</label>
          <input id="library-version" bind:value={source.version} onchange={saveSource} placeholder="v1.0.1" spellcheck="false" />
        {:else if source.kind === "dll"}
          <label for="library-file">Assembly</label>
          <input id="library-file" bind:value={source.file} onchange={saveSource} spellcheck="false" />
        {:else}
          <label for="library-cmake">CMake package</label>
          <input id="library-cmake" bind:value={source.package} onchange={saveSource} placeholder="SFML" spellcheck="false" />
        {/if}
      {/if}
    </div>
  </section>

  <section>
    <h3>What code calls</h3>
    <table>
      <thead><tr><th>Function</th><th>Values</th><th>Result</th><th></th></tr></thead>
      <tbody>
        {#each decl.functions as f (f.name.name)}
          {@const params = paramsOf(lineOf(f.span))}
          <tr>
            <td><input value={f.name.name} spellcheck="false" onchange={(e) => set({ key: `function ${f.name.name}`, line: functionLine(e.currentTarget.value.trim(), params, returnsOf(f)).line })} /></td>
            <td><input value={params} placeholder="n: integer" spellcheck="false" onchange={(e) => set(functionLine(f.name.name, e.currentTarget.value, returnsOf(f)))} /></td>
            <td><input list="library-types-{name}" value={returnsOf(f)} placeholder="nothing" onchange={(e) => set(functionLine(f.name.name, params, e.currentTarget.value))} /></td>
            <td><button class="link" onclick={() => set({ key: `function ${f.name.name}`, line: null })}>Remove</button></td>
          </tr>
        {:else}
          <tr><td colspan="4" class="muted">No functions yet: add what code calls as {name}.name(values).</td></tr>
        {/each}
      </tbody>
    </table>
    <form
      class="add"
      onsubmit={(e) => {
        e.preventDefault();
        const n = newFunction.name.trim();
        if (n) set(functionLine(n, newFunction.params, newFunction.returns)).then(() => (newFunction = { name: "", params: "", returns: "" }));
      }}
    >
      <input bind:value={newFunction.name} placeholder="spell" spellcheck="false" aria-label="New function's name" />
      <input bind:value={newFunction.params} placeholder="n: integer" spellcheck="false" aria-label="Its values" />
      <input bind:value={newFunction.returns} list="library-types-{name}" placeholder="text" aria-label="Its result" />
      <button type="submit" disabled={!newFunction.name.trim()}>Add function</button>
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
