<script lang="ts">
  // The `popover` part in a generated window (DESIGN 5.16, 7.8): it opens next to the part it is `for`.
  import Element from "./Element.svelte";
  let given = $props();
</script>

<Element kind="popover" {...given} />
