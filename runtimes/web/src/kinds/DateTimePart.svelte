<script lang="ts">
  import type { PartContext } from "./context.ts";
  import DateTimeInput from "../parts/DateTimeInput.svelte";

  let { ctx }: { ctx: PartContext } = $props();
</script>

<!-- A moment: a day and a 24-hour time, as on every stack. -->
<DateTimeInput
  id={ctx.id}
  value={ctx.value}
  disabled={ctx.disabled}
  readonly={ctx.readonly}
  invalid={ctx.error !== null}
  describedBy={ctx.describedBy}
  onkeydown={(e) => {
    if (e.key === "Enter" && ctx.submit()) e.preventDefault();
  }}
  onchange={(next) => {
    ctx.setValue(next);
    ctx.fire("change");
  }}
/>
