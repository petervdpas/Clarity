<script lang="ts">
  // An icon of the IDE, by its word (ideIcons.ts), drawn by the one Icon part.
  import Icon from "@clarity/ui/parts/Icon.svelte";
  import lucide from "../../../crates/clarity-spec/icons/lucide.json";
  import { idePaths, type IdeIconName } from "./ideIcons";

  const paths = idePaths(lucide as Record<string, string>);
  let { name, size = 16 }: { name: IdeIconName; size?: number } = $props();
</script>

<Icon {paths} {name} {size} />
