<script lang="ts">
  // The `password` part in a generated window (DESIGN 7.8).
  import Element from "./Element.svelte";
  let given = $props();
</script>

<Element kind="password" {...given} />
