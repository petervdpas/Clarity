<script lang="ts">
  // A custom part (DESIGN 5.19): what it is declared as, and its folder as a
  // file tree with tabs. Its declaration is Clarity, edited as text in the
  // first tab; the file Clarity writes from it is shown to read.

  import { invoke } from "@clarity/ui/host/backend.ts";
  import FolderEditor from "./FolderEditor.svelte";
  import { paramsText } from "@clarity/ui/screen/customParts.ts";
  import type { CodeView } from "@clarity/ui/generated/CodeView.ts";
  import type { DeclTrial } from "@clarity/ui/generated/DeclTrial.ts";
  import type { Edit } from "@clarity/ui/generated/Edit.ts";
  import type { PartDecl } from "@clarity/ui/generated/PartDecl.ts";
  import type { SpecFile } from "@clarity/ui/generated/SpecFile.ts";

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

  const declaration = $derived({
    load: () => invoke<CodeView>("decl_text", { path: file.path, item }),
    check: async (text: string) => (await invoke<DeclTrial>("try_decl", { path: file.path, item, text })).problems,
    save: (text: string) => onapply({ op: "replaceDecl", item, text }),
    reloadKey: file.path + "\n" + item + "\n" + file.version,
  });
  /** C, C++ (CMake), Rust and Go build on their own; .NET and C libraries only with the program. */
  const buildable = $derived(['native', 'crate', 'go'].includes(decl.library?.kind ?? ""));
</script>

<div class="part">
  <header>
    <strong>{name}</strong>
    <span class="muted">custom part in module {file.module?.name ?? ""}, drawn by {library}</span>
  </header>

  <FolderEditor
    {name}
    version={file.version}
    declarationLabel="part {name}"
    {declaration}
    buildable={buildable}
    buildTitle={buildable ? "Builds it on its own in its folder, the output in the Output panel" : "It is built with the program: use Build in the toolbar"}
    writeLabel="Write interface and starter" writeTitle="Writes the file that follows the declaration, and a starter where the folder has none">
    {#snippet summary()}
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
          <dt>Layout</dt>
          <dd>
            <label title="It takes the height the window has left, as a table or a canvas does; the declaration's grows line"
              ><input type="checkbox" checked={decl.grows} onchange={(e) => onapply({ op: "setPartGrows", item, grows: e.currentTarget.checked })} /> Grows with the window</label
            >
          </dd>
        </dl>
      </section>
    {/snippet}
  </FolderEditor>
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
</style>
