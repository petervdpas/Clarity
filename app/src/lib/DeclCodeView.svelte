<script lang="ts">
  // One declaration's code, as written in its module, in an editor: a
  // command (`command review set(...)`), a function or an action. Saving
  // writes the declaration back.

  import { invoke } from "@tauri-apps/api/core";
  import SourceEditor from "./SourceEditor.svelte";
  import type { CodeView } from "@clarity/ui/generated/CodeView.ts";
  import type { DeclTrial } from "@clarity/ui/generated/DeclTrial.ts";
  import type { SpecFile } from "@clarity/ui/generated/SpecFile.ts";

  let {
    title,
    what,
    item,
    file,
    onsave,
  }: {
    /** Its name as code calls it: `review set`, `linksOf`. */
    title: string;
    /** What it is: command, function, action. */
    what: string;
    /** Where its declaration starts in the file. */
    item: number;
    /** The module that declares it. */
    file: SpecFile;
    /** Writes the declaration back; throws with the reason when it is refused. */
    onsave: (item: number, text: string) => Promise<void>;
  } = $props();

</script>

<div class="decl-code">
  {#key item}
    <SourceEditor
      heading={title}
      headingNote="{what} in module {file.module?.name ?? ''}"
      load={() => invoke<CodeView>("decl_text", { path: file.path, item })}
      check={async (text) => (await invoke<DeclTrial>("try_decl", { path: file.path, item, text })).problems}
      save={(text) => onsave(item, text)}
      reloadKey={file.path + "\n" + item + "\n" + file.version}
    />
  {/key}
</div>

<style>
  /* The editor fills the main area: its name and what it is sit in its own bar, beside Revert and Save. */
  .decl-code {
    display: flex;
    flex: 1;
    min-height: 0;
    overflow: hidden;
  }
</style>
