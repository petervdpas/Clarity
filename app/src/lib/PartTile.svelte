<script lang="ts">
  // One tile in a designer's Parts panel: a small drawing of the part and
  // its name. Press and drag it onto the canvas.

  let {
    kind,
    name,
    detail = null,
    title = "Drag onto the canvas",
    onpointerdown,
    ondblclick,
  }: {
    /** A widget kind, or `group` / `row` for form layout parts. */
    kind: string;
    name: string;
    detail?: string | null;
    title?: string;
    onpointerdown: (e: PointerEvent) => void;
    ondblclick?: () => void;
  } = $props();
</script>

<button class="tile" {title} {onpointerdown} {ondblclick}>
  <svg viewBox="0 0 48 28" aria-hidden="true">
    {#if kind === "label"}
      <text x="24" y="18" text-anchor="middle" class="ink">Label</text>
    {:else if kind === "help"}
      <rect x="4" y="5" width="40" height="10" rx="2" />
      <text x="5" y="23" class="ink tiny">Help for the input</text>
    {:else if kind === "heading"}
      <text x="24" y="19" text-anchor="middle" class="ink bold">Title</text>
    {:else if kind === "text"}
      <text x="5" y="17.5" class="ink small">Ada Lovelace</text>
      <line x1="4" y1="21" x2="44" y2="21" class="dashed" />
    {:else if kind === "markdown"}
      <text x="4" y="10" class="ink bold small"># Rules</text>
      <circle cx="6" cy="16" r="1" class="fill" />
      <line x1="9" y1="16" x2="36" y2="16" />
      <circle cx="6" cy="22" r="1" class="fill" />
      <line x1="9" y1="22" x2="30" y2="22" />
    {:else if kind === "memo"}
      <rect x="4" y="3" width="40" height="22" rx="2" />
      <line x1="8" y1="9" x2="38" y2="9" />
      <line x1="8" y1="14" x2="34" y2="14" />
      <line x1="8" y1="19" x2="24" y2="19" />
    {:else if kind === "password"}
      <rect x="4" y="7" width="40" height="14" rx="2" />
      <text x="8" y="18.5" class="ink small">&#8226;&#8226;&#8226;&#8226;&#8226;</text>
    {:else if kind === "switch"}
      <rect x="14" y="8" width="20" height="12" rx="6" class="fill" />
      <circle cx="28" cy="14" r="4" />
    {:else if kind === "radio"}
      <circle cx="10" cy="14" r="4" class="fill" />
      <circle cx="10" cy="14" r="1.5" />
      <text x="16" y="17" class="ink tiny">A</text>
      <circle cx="28" cy="14" r="4" />
      <text x="34" y="17" class="ink tiny">B</text>
    {:else if kind === "form"}
      <rect x="4" y="3" width="40" height="22" rx="2" class="dashed" />
      <line x1="8" y1="9" x2="16" y2="9" />
      <rect x="18" y="6" width="22" height="6" rx="1" />
      <line x1="8" y1="18" x2="16" y2="18" />
      <rect x="18" y="15" width="22" height="6" rx="1" />
    {:else if kind === "textbox"}
      <rect x="4" y="7" width="40" height="14" rx="2" />
      <text x="8" y="17.5" class="ink small">abc</text>
    {:else if kind === "number"}
      <rect x="10" y="7" width="22" height="14" rx="2" />
      <text x="21" y="17.5" text-anchor="middle" class="ink small">8</text>
      <rect x="33" y="7" width="6" height="6.5" rx="1" />
      <rect x="33" y="14.5" width="6" height="6.5" rx="1" />
    {:else if kind === "checkbox"}
      <rect x="17" y="7" width="14" height="14" rx="2" class="fill" />
      <path d="M20 14 l3 3 l6 -6" class="tick" />
    {:else if kind === "dropdown"}
      <rect x="4" y="7" width="40" height="14" rx="2" />
      <text x="8" y="17.5" class="ink small">Sine</text>
      <path d="M34 12 l3 3 l3 -3" class="line" />
    {:else if kind === "date"}
      <rect x="6" y="7" width="36" height="14" rx="2" />
      <text x="24" y="17.5" text-anchor="middle" class="ink small">01/01</text>
    {:else if kind === "button"}
      <rect x="10" y="7" width="28" height="14" rx="3" class="fill" />
      <text x="24" y="17.5" text-anchor="middle" class="ink small">OK</text>
    {:else if kind === "panel"}
      <rect x="4" y="3" width="40" height="22" rx="2" />
      <line x1="4" y1="9" x2="44" y2="9" />
    {:else if kind === "tabs"}
      <rect x="4" y="3" width="12" height="6" rx="1" class="fill" />
      <rect x="17" y="3" width="12" height="6" rx="1" />
      <rect x="4" y="9" width="40" height="16" rx="1" />
    {:else if kind === "accordion"}
      <rect x="4" y="3" width="40" height="5" rx="1" class="fill" />
      <rect x="4" y="9" width="40" height="10" rx="1" />
      <rect x="4" y="20" width="40" height="5" rx="1" class="fill" />
    {:else if kind === "scroll"}
      <rect x="4" y="3" width="40" height="22" rx="2" />
      <line x1="40" y1="6" x2="40" y2="14" />
      <line x1="9" y1="9" x2="32" y2="9" />
      <line x1="9" y1="15" x2="32" y2="15" />
      <line x1="9" y1="21" x2="26" y2="21" />
    {:else if kind === "toggle"}
      <rect x="10" y="7" width="28" height="14" rx="3" class="fill" />
      <path d="M17 14 l4 4 l9 -9" />
    {:else if kind === "buttons"}
      <rect x="6" y="7" width="12" height="14" rx="3" />
      <rect x="18" y="7" width="12" height="14" class="fill" />
      <rect x="30" y="7" width="12" height="14" rx="3" />
    {:else if kind === "separator"}
      <line x1="4" y1="14" x2="44" y2="14" />
    {:else if kind === "link"}
      <text x="24" y="17.5" text-anchor="middle" class="ink small">link</text>
      <line x1="12" y1="20" x2="36" y2="20" />
    {:else if kind === "badge"}
      <rect x="12" y="8" width="24" height="12" rx="6" class="fill" />
      <text x="24" y="17" text-anchor="middle" class="ink small">3</text>
    {:else if kind === "color"}
      <rect x="6" y="7" width="12" height="14" rx="2" class="fill" />
      <rect x="21" y="7" width="21" height="14" rx="2" />
      <text x="31.5" y="17.5" text-anchor="middle" class="ink small">#</text>
    {:else if kind === "file"}
      <path d="M8 5 h10 l5 5 v13 h-15 z" />
      <path d="M18 5 v5 h5" />
      <rect x="27" y="10" width="15" height="8" rx="2" class="fill" />
    {:else if kind === "slider"}
      <line x1="6" y1="14" x2="42" y2="14" />
      <rect x="6" y="12.5" width="16" height="3" rx="1.5" class="fill" />
      <circle cx="22" cy="14" r="4" class="fill" />
    {:else if kind === "spinbox"}
      <rect x="6" y="7" width="36" height="14" rx="2" />
      <text x="17" y="18" text-anchor="middle" class="ink small">12</text>
      <path d="M33 12 l3 -3 l3 3 M33 16 l3 3 l3 -3" />
    {:else if kind === "progress"}
      <rect x="4" y="10" width="40" height="8" rx="4" />
      <rect x="4" y="10" width="24" height="8" rx="4" class="fill" />
    {:else if kind === "busy"}
      <path d="M24 6 A8 8 0 1 1 16 14" />
    {:else if kind === "icon"}
      <circle cx="24" cy="14" r="8" />
      <path d="M24 9 L25.5 12.5 L29 13 L26.5 15.5 L27 19 L24 17.2 L21 19 L21.5 15.5 L19 13 L22.5 12.5 Z" class="fill" />
    {:else if kind === "popover"}
      <rect x="4" y="3" width="14" height="6" rx="1" class="fill" />
      <path d="M10 9 L13 12 L16 9" />
      <rect x="6" y="12" width="34" height="13" rx="2" />
    {:else if kind === "steps"}
      <circle cx="10" cy="6" r="3" class="fill" />
      <line x1="13" y1="6" x2="21" y2="6" />
      <circle cx="24" cy="6" r="3" />
      <line x1="27" y1="6" x2="35" y2="6" />
      <circle cx="38" cy="6" r="3" />
      <rect x="4" y="11" width="40" height="9" rx="1" />
      <rect x="32" y="21.5" width="12" height="4" rx="1" class="fill" />
    {:else if kind === "splitter"}
      <rect x="4" y="3" width="40" height="22" rx="2" />
      <line x1="22" y1="3" x2="22" y2="25" />
      <line x1="25" y1="3" x2="25" y2="25" />
    {:else if kind === "table"}
      <rect x="4" y="3" width="40" height="22" rx="1" />
      <line x1="4" y1="9" x2="44" y2="9" />
      <line x1="4" y1="15" x2="44" y2="15" />
      <line x1="4" y1="20" x2="44" y2="20" />
      <line x1="18" y1="3" x2="18" y2="25" />
      <line x1="31" y1="3" x2="31" y2="25" />
    {:else if kind === "list"}
      <rect x="8" y="3" width="32" height="22" rx="1" />
      <line x1="12" y1="9" x2="34" y2="9" />
      <line x1="12" y1="14" x2="30" y2="14" />
      <line x1="12" y1="19" x2="32" y2="19" />
    {:else if kind === "image"}
      <rect x="8" y="3" width="32" height="22" rx="1" />
      <path d="M10 23 l9 -10 l6 6 l4 -4 l9 8 z" class="fill" />
      <circle cx="32" cy="9" r="2.5" class="fill" />
    {:else if kind === "canvas"}
      <rect x="6" y="3" width="36" height="22" rx="1" class="dashed" />
      <path d="M11 20 C16 6, 22 24, 28 12 S36 8, 38 16" class="line" />
    {:else if kind === "group"}
      <rect x="4" y="6" width="40" height="19" rx="2" />
      <text x="7" y="5.5" class="ink tiny">GROUP</text>
      <rect x="10" y="12" width="28" height="8" rx="1" />
    {:else if kind === "row"}
      <rect x="3" y="8" width="12" height="12" rx="1" />
      <rect x="18" y="8" width="12" height="12" rx="1" />
      <rect x="33" y="8" width="12" height="12" rx="1" />
    {:else if kind === "menu"}
      <rect x="3" y="3" width="42" height="22" rx="1" />
      <rect x="3" y="3" width="42" height="6" class="fill" />
      <text x="6" y="8" class="ink tiny">File  Edit  Help</text>
      <rect x="5" y="9" width="16" height="11" rx="1" />
      <line x1="8" y1="13" x2="18" y2="13" />
      <line x1="8" y1="17" x2="16" y2="17" />
    {:else if kind === "toolbar"}
      <rect x="3" y="3" width="42" height="22" rx="1" />
      <rect x="5" y="5" width="7" height="6" rx="1" class="fill" />
      <rect x="14" y="5" width="7" height="6" rx="1" class="fill" />
      <line x1="24" y1="5" x2="24" y2="11" />
      <rect x="27" y="5" width="7" height="6" rx="1" class="fill" />
      <line x1="3" y1="13" x2="45" y2="13" />
    {:else if kind === "ribbon"}
      <rect x="3" y="3" width="42" height="22" rx="1" />
      <rect x="5" y="4" width="9" height="4" rx="1" class="fill" />
      <line x1="3" y1="8" x2="45" y2="8" />
      <rect x="6" y="10" width="8" height="9" rx="1" class="fill" />
      <rect x="16" y="10" width="5" height="4" rx="1" />
      <rect x="16" y="15" width="5" height="4" rx="1" />
      <line x1="24" y1="10" x2="24" y2="21" />
      <rect x="27" y="10" width="8" height="9" rx="1" class="fill" />
    {:else if kind === "sidebar"}
      <rect x="3" y="3" width="42" height="22" rx="1" />
      <rect x="3" y="3" width="12" height="22" class="fill" />
      <line x1="6" y1="8" x2="12" y2="8" />
      <line x1="6" y1="12" x2="12" y2="12" />
      <line x1="6" y1="16" x2="12" y2="16" />
    {:else if kind === "footer"}
      <rect x="3" y="3" width="42" height="22" rx="1" />
      <rect x="3" y="19" width="42" height="6" class="fill" />
      <text x="5" y="23.5" class="ink tiny">Ready</text>
    {:else}
      <rect x="4" y="3" width="40" height="22" rx="2" class="dashed" />
    {/if}
  </svg>
  <span class="name">{name}</span>
  {#if detail}<span class="detail">{detail}</span>{/if}
</button>

<style>
  .tile {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
    padding: 6px 4px 4px;
    min-width: 0;
    cursor: grab;
    touch-action: none;
    background: var(--panel-alt);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }

  .tile:hover {
    border-color: var(--accent);
  }

  svg {
    width: 48px;
    height: 28px;
    fill: none;
    stroke: var(--text-muted);
    stroke-width: 1.2;
  }

  .fill {
    fill: var(--border-strong);
  }

  .dashed {
    stroke-dasharray: 3 2;
  }

  .line,
  .tick {
    stroke: var(--text);
    stroke-width: 1.5;
  }

  .ink {
    fill: var(--text);
    stroke: none;
    font-size: 10px;
    font-family: inherit;
  }

  .ink.small {
    font-size: 8px;
  }

  .ink.tiny {
    font-size: 5px;
    fill: var(--text-muted);
  }

  .ink.bold {
    font-size: 12px;
    font-weight: 700;
  }

  .name {
    font-size: 11px;
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .detail {
    font-size: 10px;
    font-family: var(--mono);
    color: var(--text-muted);
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
