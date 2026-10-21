<script lang="ts">
  // An engine, edited the way an IDE edits a unit: its inspector on the left
  // (functions, services, state, limits) writes the declaration in its
  // module, and its code opens in a tab on the right, built from here. A
  // function added in the inspector gets its stub in the code; a double click
  // on a function goes to it. The state line says in plain words whether the
  // engine is ready, needs building, or what is wrong, with the button that
  // fixes it. An engine written in Clarity is its code: the tab holds the
  // whole engine, and the inspector sets its state and services above it.

  import { untrack } from "svelte";
  import { invoke } from "./backend";
  import SourceEditor, { OPENS_BRACED_BLOCK } from "./SourceEditor.svelte";
  import { asDiagnostic, engineBuilds } from "./engineBuilds.svelte";
  import { engineText, formOf, freeName, paramsOf, paramsText, projectEngines, SERVICES, withEngineLine, type EngineForm, type EngineLine } from "./engines";
  import type { CodeView } from "./generated/CodeView";
  import type { DeclTrial } from "./generated/DeclTrial";
  import type { Edit } from "./generated/Edit";
  import type { EngineDecl } from "./generated/EngineDecl";
  import type { EngineInfo } from "./generated/EngineInfo";
  import type { Preview } from "./generated/Preview";
  import type { SpecFile } from "./generated/SpecFile";

  let {
    preview,
    decl,
    file,
    goTo = null,
    onapply,
    ondelete,
  }: {
    preview: Preview;
    decl: EngineDecl;
    /** The module that declares it. */
    file: SpecFile;
    /** A line of the code to show, as from the Problems list; `n` changes for each jump. */
    goTo?: { line: number; n: number } | null;
    onapply: (edit: Edit) => Promise<void>;
    ondelete: () => void;
  } = $props();

  const name = $derived(decl.name.name);
  /** Written in Clarity: no file to build, the declaration is its code. */
  const written = $derived(decl.body !== null);
  const item = $derived(decl.span.start);
  const form = $derived(formOf(decl));
  const declProblems = $derived(projectEngines(preview.project).find((e) => e.engine.name.name === name)?.problems ?? []);
  const built = $derived(engineBuilds.byEngine[name] ?? null);
  const buildErrors = $derived(built?.problems.filter((p) => p.severity === "error").length ?? 0);

  /** What the inspector offers as types: Clarity's, and the project's entities and enums. */
  const types = $derived([
    "text",
    "integer",
    "decimal",
    "money",
    "bool",
    "date",
    "datetime",
    "duration",
    "bytes",
    ...preview.project.files.flatMap((f) => f.enums.map((e) => e.name.name)),
    ...preview.project.files.flatMap((f) => f.entities.flatMap((e) => [e.name.name, `list<${e.name.name}>`])),
  ]);

  /** The code fills the editor, the declaration set aside. */
  let filled = $state(false);
  let info = $state<EngineInfo | null>(null);
  let error = $state<string | null>(null);
  let busy = $state(false);
  let asText = $state(false);
  /** Changes when the code must load again: after stubs were added or a build said something. */
  let codeRevision = $state(0);
  let code: SourceEditor | undefined = $state();

  async function refresh() {
    if (written) return;
    try {
      info = await invoke<EngineInfo>("engine_info", { name });
    } catch (err) {
      error = String(err);
    }
  }

  // Again when the declaration, its module or a build changes.
  $effect(() => {
    void [name, file.version, engineBuilds.revision, preview];
    refresh();
  });
  $effect(() => {
    void engineBuilds.revision;
    untrack(() => codeRevision++);
  });
  $effect(() => {
    if (goTo && code) code.goToLine(goTo.line);
  });

  async function act(work: () => Promise<void>) {
    if (busy) return;
    busy = true;
    error = null;
    try {
      await work();
    } catch (err) {
      error = String(err);
    } finally {
      busy = false;
    }
  }

  /** Writes the form as the declaration, in place of the one there. */
  function write(next: EngineForm, then?: () => Promise<void>) {
    act(async () => {
      await onapply({ op: "replaceDecl", item, text: engineText(next) });
      if (then) await then();
    });
  }

  function change(edit: (f: EngineForm) => void, then?: () => Promise<void>) {
    const next: EngineForm = structuredClone($state.snapshot(form));
    edit(next);
    write(next, then);
  }

  /** Stubs in the code for functions it does not write yet, after saving what is typed there. */
  async function addStubs() {
    if (!info?.source) return;
    if (code && !(await code.saveIfChanged())) return;
    info = await invoke<EngineInfo>("add_engine_stubs", { name });
    codeRevision++;
  }

  function addFunction() {
    const fname = freeName(
      form.functions.map((f) => f.name),
      "compute",
    );
    change((f) => f.functions.push({ name: fname, params: [{ name: "value", type: "integer" }], returns: "integer" }), addStubs);
  }

  function build() {
    act(async () => {
      if (code && !(await code.saveIfChanged())) return;
      await invoke("build_engine", { name });
    });
  }

  function makeSource() {
    act(async () => {
      info = await invoke<EngineInfo>("new_engine_source", { name });
      await invoke("build_engine", { name });
    });
  }

  /** Sets the `uses` or `state` line of an engine written in Clarity, after saving what is typed in its code. */
  function setLine(kind: EngineLine, line: string | null) {
    act(async () => {
      if (code && !(await code.saveIfChanged())) return;
      const now = await invoke<CodeView>("decl_text", { path: file.path, item });
      await onapply({ op: "replaceDecl", item, text: withEngineLine(now.text, kind, line) });
    });
  }

  /** The state of the engine in one line, and how bad it is. */
  const status = $derived.by((): { text: string; tone: "ok" | "warn" | "error" | "plain" } => {
    if (written) {
      if (declProblems.length > 0) return { text: declProblems[0], tone: "error" };
      return { text: "Ready: written in Clarity, generated with the application on every stack", tone: "ok" };
    }
    if (!info) return { text: "Looking at the engine...", tone: "plain" };
    // A file that is not there yet is what making its code and building it fixes.
    if (declProblems.length > 0 && (info.built || info.source)) return { text: declProblems[0], tone: "error" };
    if (built && !built.ok) return { text: `It did not build: ${buildErrors} ${buildErrors === 1 ? "error" : "errors"} in its code`, tone: "error" };
    if (!info.source) return { text: info.built ? `${info.file} is built outside Clarity` : `There is no ${info.file} yet`, tone: info.built ? "plain" : "warn" };
    if (info.missingTools.length > 0) return { text: `To build it here, install ${info.missingTools.join(", and ")}`, tone: "warn" };
    if (info.missingFunctions.length > 0) return { text: `The code does not write ${info.missingFunctions.join(", ")} yet`, tone: "warn" };
    if (info.needsBuild) return { text: "The code changed since it was built; Build, or Run, builds it", tone: "warn" };
    return { text: "Ready: the file is built from the code as it is", tone: "ok" };
  });
</script>

<div class="engine-editor">
  {#if written}
  <header>
    <div class="title">
      <strong>{name}</strong>
      <span class="muted">engine in module {file.module?.name ?? ""}, written in Clarity</span>
    </div>
    <div class="buttons">
      <button class="danger" onclick={ondelete}>Delete engine</button>
    </div>
  </header>

  <div class="status {status.tone}" role="status"><span>{status.text}</span></div>
  {#if error}<p class="refused" role="alert">{error}</p>{/if}

  <div class="panes" class:filled>
    <aside class="inspector" class:hidden={filled}>
      <section>
        <h3>Functions</h3>
        <p class="muted">What code calls as <code>{name}.function(...)</code>. Double-click one to go to its code.</p>
        <ul class="names">
          {#each decl.body?.functions ?? [] as f (f.span.start)}
            <li ondblclick={() => code?.jumpTo(f.name.name)} title="Double-click to go to its code"><code>{f.name.name}</code></li>
          {/each}
        </ul>
      </section>

      <section>
        <h3>What it keeps</h3>
        <p class="muted">Values kept from call to call, each with what it starts as: <code>keep moves: integer = 0</code> in the code.</p>
        <ul class="names">
          {#each decl.body?.keeps ?? [] as k (k.span.start)}
            <li><code>{k.name.name}</code></li>
          {:else}
            <li class="muted">Nothing: every call starts from its values alone.</li>
          {/each}
        </ul>
      </section>

      <section>
        <h3>Services</h3>
        <label class="check">
          <input type="checkbox" checked={form.uses.includes("random")} disabled={busy} onchange={(e) => setLine("uses", e.currentTarget.checked ? "uses random" : null)} />
          random: <code>random(n)</code> gives a whole number from 1 to n
        </label>
      </section>

      <section>
        <h3>State</h3>
        {#each [["none", "None: every call starts fresh", null], ["app", "Per app: one set of values for the whole application", "state per app"], ["window", "Per window: a set of values for each open window", "state per window"]] as [value, label, line] (value)}
          <label class="check">
            <input type="radio" name="engine-state" checked={form.state === value} disabled={busy} onchange={() => setLine("state", line)} />
            {label}
          </label>
        {/each}
      </section>
    </aside>

    <section class="code">
      <div class="tab">
        <code>engine {name}</code>
        <button class="fill" title={filled ? "Give the inspector back" : "Let the code fill the editor"} onclick={() => (filled = !filled)}
          >{filled ? "\u2921 Inspector" : "\u2922 Fill"}</button
        >
      </div>
      <SourceEditor
        bind:this={code}
        load={() => invoke<CodeView>("decl_text", { path: file.path, item })}
        check={async (text) => (await invoke<DeclTrial>("try_decl", { path: file.path, item, text })).problems}
        save={(text) => onapply({ op: "replaceDecl", item, text })}
        reloadKey={file.path + "\n" + item + "\n" + file.version}
        navPattern={/^\s*function\s+([A-Za-z0-9_]+)/}
      />
    </section>
  </div>
  {:else}
  <header>
    <div class="title">
      <strong>{name}</strong>
      <span class="muted">engine in module {file.module?.name ?? ""}, {info?.source ? `code in engines/${info.file.replace(/\.wasm$/, "")}` : `file ${decl.file}`}</span>
    </div>
    <div class="buttons">
      <button class:active={asText} onclick={() => (asText = !asText)} title="The declaration as Clarity text">Text</button>
      {#if info?.source}
        <button class="primary" disabled={busy || (info.missingTools.length > 0)} onclick={build} title="Saves the code and builds {decl.file}">Build</button>
      {/if}
      <button class="danger" onclick={ondelete}>Delete engine</button>
    </div>
  </header>

  <div class="status {status.tone}" role="status">
    <span>{status.text}</span>
    {#if info && !info.source}
      <button disabled={busy} onclick={makeSource} title="Writes a Rust crate with a stub for each function, and builds it">Write its code in Rust</button>
    {:else if info?.source && info.missingFunctions.length > 0}
      <button disabled={busy} onclick={() => act(addStubs)}>Write the missing functions</button>
    {:else if info?.source && info.needsBuild && info.missingTools.length === 0 && !(built && !built.ok)}
      <button disabled={busy} onclick={build}>Build</button>
    {/if}
  </div>
  {#if error}<p class="refused" role="alert">{error}</p>{/if}

  <div class="panes" class:filled>
    <aside class="inspector" class:hidden={filled}>
      {#if asText}
        <SourceEditor
          load={() => invoke<CodeView>("decl_text", { path: file.path, item })}
          check={async (text) => (await invoke<DeclTrial>("try_decl", { path: file.path, item, text })).problems}
          save={(text) => onapply({ op: "replaceDecl", item, text })}
          reloadKey={file.path + "\n" + item + "\n" + file.version}
          onclose={() => (asText = false)}
        />
      {:else}
        <section>
          <h3>Functions</h3>
          <p class="muted">What code calls as <code>{name}.function(...)</code>. Double-click one to go to its code.</p>
          <table>
            <thead><tr><th>Name</th><th>Values</th><th>Gives</th><th></th></tr></thead>
            <tbody>
              {#each form.functions as f, i (i + f.name)}
                <tr ondblclick={() => code?.jumpTo(f.name)} title="Double-click to go to its code">
                  <td><input class="mono" value={f.name} aria-label="Name" onchange={(e) => change((n) => (n.functions[i].name = e.currentTarget.value.trim()), addStubs)} /></td>
                  <td><input class="mono" value={paramsText(f.params)} aria-label="Values" placeholder="none" onchange={(e) => change((n) => (n.functions[i].params = paramsOf(e.currentTarget.value)))} /></td>
                  <td><input class="mono" list="engine-types" value={f.returns} aria-label="Gives" onchange={(e) => change((n) => (n.functions[i].returns = e.currentTarget.value.trim()))} /></td>
                  <td class="remove"><button title="Remove the function from the declaration; its code stays" onclick={() => change((n) => n.functions.splice(i, 1))}>×</button></td>
                </tr>
              {/each}
            </tbody>
          </table>
          <datalist id="engine-types">
            {#each types as t (t)}<option value={t}></option>{/each}
          </datalist>
          <button class="add" disabled={busy} onclick={addFunction}>+ Add function</button>
        </section>

        <section>
          <h3>Services</h3>
          {#each SERVICES as s (s)}
            <label class="check">
              <input type="checkbox" checked={form.uses.includes(s)} onchange={(e) => change((n) => (n.uses = e.currentTarget.checked ? [...n.uses, s] : n.uses.filter((u) => u !== s)))} />
              {s}
            </label>
          {/each}
        </section>

        <section>
          <h3>Texts it gives</h3>
          <textarea
            rows="4"
            value={form.says.join("\n")}
            placeholder={"You win\nA draw"}
            title="One text a line: what the engine gives people to read, translated like the project's own texts"
            onchange={(e) =>
              change(
                (n) =>
                  (n.says = e.currentTarget.value
                    .split("\n")
                    .map((t) => t.trim())
                    .filter((t) => t !== "")),
              )}
          ></textarea>
        </section>

        <section>
          <h3>State</h3>
          {#each [["none", "None: every call starts fresh"], ["app", "Per app: one instance for the whole application"], ["window", "Per window: an instance for each open window"]] as [value, label] (value)}
            <label class="check">
              <input type="radio" name="engine-state" checked={form.state === value} onchange={() => change((n) => (n.state = value as EngineForm["state"]))} />
              {label}
            </label>
          {/each}
          <label class="check">
            <input type="checkbox" checked={form.opens} onchange={(e) => change((n) => (n.opens = e.currentTarget.checked))} />
            On open: runs when an instance is made
          </label>
        </section>

        <section>
          <h3>Limits</h3>
          <div class="limits">
            <label>
              Seconds a call may run
              <input type="number" min="1" max="3600" placeholder="5" value={form.seconds ?? ""} onchange={(e) => change((n) => (n.seconds = e.currentTarget.value === "" ? null : Math.floor(Number(e.currentTarget.value))))} />
            </label>
            <label>
              Memory in MB
              <input type="number" min="1" max="4096" placeholder="256" value={form.megabytes ?? ""} onchange={(e) => change((n) => (n.megabytes = e.currentTarget.value === "" ? null : Math.floor(Number(e.currentTarget.value))))} />
            </label>
          </div>
        </section>
      {/if}
    </aside>

    <section class="code">
      {#if info?.source}
        {@const main = info.source.main}
        <div class="tab">
          <code>{main.split(/[\\/]/).slice(-3).join("/")}</code>
          <button class="fill" title={filled ? "Give the declaration back" : "Let the code fill the editor"} onclick={() => (filled = !filled)}
            >{filled ? "\u2921 Declaration" : "\u2922 Fill"}</button
          >
        </div>
        <SourceEditor
          bind:this={code}
          load={async () => ({
            text: await invoke<string>("engine_source_text", { name }),
            problems: (engineBuilds.byEngine[name]?.problems ?? []).filter((p) => p.file === main).map(asDiagnostic),
          })}
          check={async () => (engineBuilds.byEngine[name]?.problems ?? []).filter((p) => p.file === main).map(asDiagnostic)}
          save={async (text) => {
            info = await invoke<EngineInfo>("save_engine_source", { name, text });
          }}
          reloadKey={name + "\n" + codeRevision}
          navPattern={/function!\(\s*([A-Za-z0-9_]+)/}
          opensBlock={OPENS_BRACED_BLOCK}
          errorsBlockSave={false}
        />
      {:else}
        <div class="no-code">
          <p>
            Clarity has no code for this engine: <code>{decl.file}</code> is used as it is. Write its code in Rust to edit and build it here, or
            build it with your own tools into the <code>engines</code> folder.
          </p>
        </div>
      {/if}
    </section>
  </div>
  {/if}
</div>

<style>
  textarea {
    width: 100%;
    box-sizing: border-box;
    font: inherit;
    resize: vertical;
  }

  .engine-editor {
    display: flex;
    flex-direction: column;
    gap: 10px;
    height: 100%;
    min-height: 0;
  }

  header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
  }

  .title strong {
    font-size: 16px;
    margin-right: 8px;
  }

  .buttons {
    display: flex;
    gap: 6px;
  }

  .buttons .active {
    border-color: var(--accent);
  }

  .muted {
    color: var(--text-muted);
    font-size: 12px;
  }

  .status {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 6px 10px;
    border-radius: var(--radius);
    border: 1px solid var(--border-strong);
    background: var(--panel);
    font-size: 13px;
  }

  .status.ok {
    border-color: color-mix(in srgb, var(--ok, #2e8b57) 60%, transparent);
  }

  .status.warn {
    border-color: color-mix(in srgb, var(--warning, #c08a00) 70%, transparent);
  }

  .status.error {
    border-color: color-mix(in srgb, var(--error, #c0392b) 70%, transparent);
  }

  .refused {
    color: var(--error, #c0392b);
    margin: 0;
  }

  .panes {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: minmax(300px, 380px) minmax(0, 1fr);
    gap: 10px;
  }

  /* The code fills the editor, the declaration set aside. */
  .panes.filled {
    grid-template-columns: minmax(0, 1fr);
  }

  .inspector.hidden {
    display: none;
  }

  .names {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .names li {
    padding: 2px 4px;
    border-radius: var(--radius);
    cursor: default;
  }

  .names li:hover {
    background: var(--panel-alt);
  }

  .tab .fill {
    margin-left: auto;
  }

  .inspector {
    display: flex;
    flex-direction: column;
    gap: 14px;
    overflow: auto;
    min-height: 0;
    padding: 10px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    background: var(--panel);
  }

  .inspector h3 {
    margin: 0 0 4px;
    font-size: 11px;
    letter-spacing: 0.05em;
    text-transform: uppercase;
    color: var(--text-muted);
  }

  .inspector p {
    margin: 0 0 6px;
  }

  table {
    width: 100%;
    border-collapse: collapse;
  }

  th {
    text-align: left;
    font-size: 11px;
    font-weight: normal;
    color: var(--text-muted);
    padding: 2px 4px;
  }

  td {
    padding: 2px;
  }

  td input {
    width: 100%;
    min-width: 0;
  }

  tr:hover td {
    background: var(--panel-alt);
  }

  .remove button {
    padding: 0 6px;
  }

  .mono {
    font-family: var(--mono);
    font-size: 12px;
  }

  .add {
    margin-top: 6px;
  }

  .check {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 13px;
    padding: 2px 0;
  }

  .limits {
    display: grid;
    gap: 6px;
  }

  .limits label {
    display: grid;
    grid-template-columns: 1fr 90px;
    align-items: center;
    font-size: 13px;
  }

  .code {
    display: flex;
    flex-direction: column;
    min-height: 0;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    background: var(--panel);
    overflow: hidden;
  }

  .tab {
    display: flex;
    align-items: center;
    padding: 4px 12px;
    font-size: 12px;
    border-bottom: 1px solid var(--border);
    background: var(--panel-alt);
  }

  .no-code {
    padding: 16px;
    color: var(--text-muted);
  }
</style>
