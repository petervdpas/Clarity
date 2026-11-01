<script lang="ts">
  import Icon from "../parts/Icon.svelte";
  import { matchesShortcut } from "../screen/shortcut.ts";
  import type { PartContext } from "./context.ts";

  let { ctx }: { ctx: PartContext } = $props();
  const p = $derived(ctx.p);
  const hidden = $derived(ctx.partState?.visible === false);

  /** Whether a key press lands in a field the person types in. */
  function typingIn(target: EventTarget | null): boolean {
    if (target instanceof HTMLTextAreaElement) return true;
    if (target instanceof HTMLElement && target.isContentEditable) return true;
    return target instanceof HTMLInputElement && !["checkbox", "radio", "button", "submit", "reset", "range", "color", "file"].includes(target.type);
  }

  /** The keys a button answers: the person's where the project lets them set keys (DESIGN 5.28). */
  function buttonKeys(): string[] {
    if (ctx.live && !ctx.design) return ctx.live.keysOf(p.name, p.shortcut, false);
    return p.shortcut ? [p.shortcut] : [];
  }

  // A button's shortcut, and Enter and Escape for the default and cancel
  // buttons, work anywhere in the running window. Heard on the document,
  // before the frame's keys on the window: the innermost goes first (DESIGN 5.28).
  function windowKey(e: KeyboardEvent) {
    if (ctx.design || !ctx.live || p.kind !== "button" || hidden || ctx.disabled || e.defaultPrevented) return;
    const plain = !e.ctrlKey && !e.altKey && !e.metaKey && !e.shiftKey;
    const inTextArea = e.target instanceof HTMLTextAreaElement;
    const onButton = e.target instanceof HTMLButtonElement;
    const hit =
      buttonKeys().some((k) => matchesShortcut(e, k, typingIn(e.target))) ||
      (p.default && plain && e.key === "Enter" && !inTextArea && !onButton) ||
      (p.cancel && plain && e.key === "Escape");
    if (!hit) return;
    e.preventDefault();
    click();
  }

  function click() {
    ctx.click();
  }

  /** A split button's arrow: its menu, below the button. */
  function openSplit(e: MouseEvent) {
    if (ctx.design || !ctx.widget.menu) return;
    const at = (e.currentTarget as HTMLElement).parentElement!.getBoundingClientRect();
    ctx.openMenu(at.left, at.bottom + 2);
  }
</script>

<svelte:document onkeydown={windowKey} />

{#if p.split}
  <!-- A split button: its click, and an arrow that opens its menu below it. -->
  <span class="split">
    <button id={ctx.id} class="push {ctx.partClass ?? ''}" class:default={p.default} disabled={ctx.disabled} tabindex={p.tabOrder ?? undefined} aria-keyshortcuts={buttonKeys().join(" ") || undefined} onclick={click}
      >{#if p.icon}<Icon paths={ctx.preview.icons} name={p.icon} />{/if}{ctx.caption ?? ""}</button
    ><button type="button" class="push arrow {ctx.partClass ?? ''}" disabled={ctx.disabled} aria-label="More" aria-haspopup="menu" onclick={openSplit}
      ><Icon paths={{ down: "M 6 9 l 6 6 l 6 -6" }} name="down" /></button
    >
  </span>
{:else}
  <button id={ctx.id} class="push {ctx.partClass ?? ''}" class:default={p.default} disabled={ctx.disabled} tabindex={p.tabOrder ?? undefined} aria-keyshortcuts={buttonKeys().join(" ") || undefined} onclick={click}
    >{#if p.icon}<Icon paths={ctx.preview.icons} name={p.icon} />{/if}{ctx.caption ?? ""}</button
  >
{/if}

<style>
  .push.default {
    border-color: var(--accent);
  }

  /* An icon stands beside the caption. */
  .push {
    width: 100%;
    height: 100%;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
  }

  .split {
    display: flex;
    width: 100%;
    height: 100%;
  }

  .split > .push:first-child {
    flex: 1;
    border-top-right-radius: 0;
    border-bottom-right-radius: 0;
  }

  .split > .arrow {
    width: auto;
    flex: none;
    padding: 0 6px;
    border-left-width: 0;
    border-top-left-radius: 0;
    border-bottom-left-radius: 0;
  }
</style>
