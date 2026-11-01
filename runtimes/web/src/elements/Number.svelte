<script lang="ts">
  // The `number` part in a generated window (DESIGN 7.8).
  import Element from "./Element.svelte";
  let given = $props();
</script>

<Element kind="number" {...given} />
