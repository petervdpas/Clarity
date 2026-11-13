<script lang="ts">
  // An external library (DESIGN 5.21): its package and the functions code
  // calls, as a form, and its adapter's files, opened and edited here. Its declaration is
  // Clarity, edited as text; the file of the adapter that types its functions
  // is Clarity's, written from the declaration, and shown to read.

  import { invoke } from "@clarity/ui/host/backend.ts";
  import FolderEditor from "./FolderEditor.svelte";
  import LibraryForm from "./LibraryForm.svelte";
  import type { CodeView } from "@clarity/ui/generated/CodeView.ts";
  import type { DeclTrial } from "@clarity/ui/generated/DeclTrial.ts";
  import type { Edit } from "@clarity/ui/generated/Edit.ts";
  import type { LibraryDecl } from "@clarity/ui/generated/LibraryDecl.ts";
  import type { SpecFile } from "@clarity/ui/generated/SpecFile.ts";

  let {
    decl,
    file,
    onapply,
  }: {
    decl: LibraryDecl;
    /** The module that declares it. */
    file: SpecFile;
    onapply: (edit: Edit) => Promise<void>;
  } = $props();

  const name = $derived(decl.name.name);
  const item = $derived(decl.span.start);
  const library = $derived.by(() => {
    const l = decl.source;
    if (!l) return "no package yet";
    if (l.kind === "nuget") return `the NuGet package ${l.package} ${l.version}`;
    if (l.kind === "dll") return l.file;
    if (l.kind === "crate") return `the crate ${l.name} ${l.version}`;
    if (l.kind === "go") return `the Go module ${l.module} ${l.version}`;
    return `${l.package}, as CMake finds it`;
  });

  const declaration = $derived({
    load: () => invoke<CodeView>("decl_text", { path: file.path, item }),
    check: async (text: string) => (await invoke<DeclTrial>("try_decl", { path: file.path, item, text })).problems,
    save: (text: string) => onapply({ op: "replaceDecl", item, text }),
    reloadKey: file.path + "\n" + item + "\n" + file.version,
  });
  /** C, C++ (CMake), Rust and Go build on their own; .NET and C libraries only with the program. */
  const buildable = $derived(['crate', 'go'].includes(decl.source?.kind ?? ""));
</script>

<div class="part">
  <header>
    <strong>{name}</strong>
    <span class="muted">library in module {file.module?.name ?? ""}, calling {library} from libraries/{name}</span>
  </header>

  <FolderEditor
    {name}
    version={file.version}
    declarationLabel="library {name}"
    {declaration}
    buildable={buildable}
    buildTitle={buildable ? "Builds it on its own in its folder, the output in the Output panel" : "It is built with the program: use Build in the toolbar"}
    writeLabel="Write its files" writeTitle="Writes the file that types the functions, and a starter adapter where the folder has none">
    {#snippet summary()}
      <LibraryForm {decl} {file} onset={({ key, line }) => onapply({ op: "setDeclLine", item, key, line })} />
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
</style>
