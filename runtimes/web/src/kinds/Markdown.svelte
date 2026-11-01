<script lang="ts">
  import type { PartContext } from "./context.ts";
  import MarkdownView from "../parts/MarkdownView.svelte";
  import { keyFile, markdownOf, tr } from "../screen/model.ts";
  import { openLink } from "../host/backend.ts";
  let { ctx }: { ctx: PartContext } = $props();
  const p = $derived(ctx.p);
  const preview = $derived(ctx.preview);
  const value = $derived(ctx.value);
  const keyed = $derived(p.caption ? keyFile(preview, p.caption) : undefined);
</script>

<!-- A file of the resources; a key's file in the language shown (a generated application has it as the key's text); or what it binds. -->
<MarkdownView
  id={ctx.id}
  text={p.source ? markdownOf(preview, p.source) : keyed ? markdownOf(preview, keyed) : p.caption ? tr(preview, p.caption) : typeof value === "string" ? value : ""}
  onlink={openLink}
/>
