<script lang="ts">
  // New custom part and New library: a name, the module it goes into, and
  // its package in the stack's own kind (DESIGN 5.19, 5.21). It hands back
  // the declaration to add; its properties, tells and functions are set in
  // its view afterwards.

  import Modal from "./Modal.svelte";
  import type { Backend } from "@clarity/ui/generated/Backend.ts";
  import type { Preview } from "@clarity/ui/generated/Preview.ts";
  import type { SpecFile } from "@clarity/ui/generated/SpecFile.ts";
  import {
    LIBRARY_KINDS,
    PART_KINDS,
    isDeclName,
    kindsFor,
    missingOf,
    newLibraryText,
    newPartText,
    startLibrarySource,
    startPartLibrary,
    type LibraryKind,
    type PartKind,
  } from "./customDecl.ts";

  let {
    preview,
    what,
    file,
    backend = null,
    error,
    onsubmit,
    oncancel,
  }: {
    preview: Preview;
    /** A custom part or a library. */
    what: "part" | "library";
    /** The module proposed first. */
    file: SpecFile;
    /** The project's stack: only its kinds are offered; every kind when unknown. */
    backend?: Backend | null;
    error: string | null;
    /** Adds the declaration `text` to `file`. */
    onsubmit: (file: SpecFile, name: string, text: string) => void;
    oncancel: () => void;
  } = $props();

  const kinds = $derived(what === "part" ? kindsFor(PART_KINDS, backend) : kindsFor(LIBRARY_KINDS, backend));
  let name = $state("");
  let path = $state("");
  let kind = $state("");
  /** The package's values as typed: folder, package, version, file, crate, module. */
  let values = $state<Record<string, string>>({});
  $effect.pre(() => {
    path = file.path;
  });
  $effect.pre(() => {
    if (!kinds.some((k) => k.kind === kind)) kind = kinds[0]?.kind ?? "";
  });

  /** The package as the declaration writes it, from the kind's start and what was typed. */
  const source = $derived.by(() => {
    if (!kind) return null;
    const start = what === "part" ? startPartLibrary(kind as PartKind, name || "Name") : startLibrarySource(kind as LibraryKind, name || "Name");
    const typed = Object.fromEntries(Object.entries(values).filter(([k, v]) => k in start && v.trim() !== ""));
    return { ...start, ...typed } as typeof start;
  });
  const fields = $derived(source ? Object.keys(source).filter((k) => k !== "kind") : []);
  const missing = $derived(source ? missingOf(source) : "the kind");
  const taken = $derived(
    preview.project.files.some((f) => f.parts.some((p) => p.name.name === name.trim()) || f.libraries.some((l) => l.name.name === name.trim())),
  );
  const nameProblem = $derived(!name.trim() ? null : !isDeclName(name.trim()) ? "A name starts with a capital, then letters and digits, as in Arena." : taken ? "The project has a part or library of that name." : null);
  const ready = $derived(name.trim() !== "" && !nameProblem && !missing);

  const FIELD_LABELS: Record<string, string> = {
    folder: "Folder",
    package: "Package",
    version: "Version",
    file: "Assembly",
    name: "Crate",
    module: "Module",
  };

  function submit(e: SubmitEvent) {
    e.preventDefault();
    const target = preview.project.files.find((f) => f.path === path);
    if (!target || !ready || !source) return;
    const n = name.trim();
    const text = what === "part" ? newPartText(n, source as never) : newLibraryText(n, source as never);
    onsubmit(target, n, text);
  }
</script>

<Modal title={what === "part" ? "New custom part" : "New library"} width={440} onclose={oncancel}>
  <form class="dialog" onsubmit={submit}>
    <label>
      <span>Name</span>
      <!-- svelte-ignore a11y_autofocus -->
      <input bind:value={name} placeholder={what === "part" ? "Arena" : "Words"} spellcheck="false" autofocus />
    </label>
    {#if nameProblem}<p class="error">{nameProblem}</p>{/if}
    <label>
      <span>In module</span>
      <select bind:value={path}>
        {#each preview.project.files as f (f.path)}
          <option value={f.path}>{f.module?.name ?? "(no module)"}</option>
        {/each}
      </select>
    </label>
    <label>
      <span>{what === "part" ? "Drawn by" : "From"}</span>
      <select bind:value={kind}>
        {#each kinds as k (k.kind)}<option value={k.kind}>{k.kind}: {k.label}</option>{/each}
      </select>
    </label>
    {#each fields as field (kind + field)}
      <label>
        <span>{FIELD_LABELS[field] ?? field}</span>
        <input
          value={values[field] ?? ""}
          placeholder={(source as Record<string, string> | null)?.[field] || ""}
          spellcheck="false"
          oninput={(e) => (values = { ...values, [field]: e.currentTarget.value })}
        />
      </label>
    {/each}
    <p class="note">
      {what === "part"
        ? "Its properties, tells and hears are set in its view; Build writes its interface file and a starter in its folder."
        : "Its functions are set in its view; Build writes the file that types them and a starter adapter in libraries/."}
    </p>
    {#if error}<p class="error">{error}</p>{/if}
    <div class="buttons">
      <button type="button" onclick={oncancel}>Cancel</button>
      <button type="submit" class="primary" disabled={!ready} title={missing && name.trim() ? `Fill in ${missing}` : ""}>
        {what === "part" ? "Add part" : "Add library"}
      </button>
    </div>
  </form>
</Modal>

<style>
  .dialog {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  label {
    display: flex;
    flex-direction: column;
    gap: 3px;
    font-size: 12px;
    color: var(--text-muted);
  }

  .note {
    margin: 0;
    font-size: 12px;
    color: var(--text-muted);
  }

  .error {
    margin: 0;
    color: var(--error);
    font-size: 12px;
  }

  .buttons {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
</style>
