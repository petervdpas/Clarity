<script lang="ts">
  // A password or secret in Clarity's own forms (the vault): typed into
  // directly, with the eye and the copy button every secret input has.
  import SecretButtons from "./ui/SecretButtons.svelte";

  let {
    id,
    value = $bindable(""),
    autocomplete = "off",
  }: {
    id: string;
    value?: string;
    autocomplete?: "off" | "new-password" | "current-password";
  } = $props();

  let revealed = $state(false);
</script>

<span class="secret-field">
  <input {id} type={revealed ? "text" : "password"} bind:value {autocomplete} spellcheck="false" />
  <SecretButtons bind:revealed text={() => value} />
</span>

<style>
  .secret-field {
    display: flex;
    align-items: stretch;
    gap: 2px;
    min-width: 0;
  }
  input {
    flex: 1;
    min-width: 0;
  }
</style>
