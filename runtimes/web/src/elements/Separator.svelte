<script lang="ts">
  // The `separator` part in a generated window (DESIGN 7.8).
  import Element from "./Element.svelte";
  let given = $props();
</script>

<Element kind="separator" {...given} />
