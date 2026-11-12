<script lang="ts">
  // Who is using the program (docs/APP-CAPABILITIES-PLAN.md step 4): the
  // project file's `signin` block in Settings. The entity of people, the
  // targets people sign in on, and the strategies of Clarity.Auth's catalog,
  // each with the fields it names; the backend checks what is saved.

  import type { Entity } from "@clarity/ui/generated/Entity.ts";
  import type { ProviderSettings } from "@clarity/ui/generated/ProviderSettings.ts";
  import type { Signin } from "@clarity/ui/generated/Signin.ts";
  import type { StrategyChoice } from "@clarity/ui/generated/StrategyChoice.ts";

  let {
    signin,
    strategies,
    entities,
    targets,
    onchange,
  }: {
    signin: Signin | null;
    strategies: StrategyChoice[];
    /** The project's stored entities, one of which holds the people. */
    entities: Entity[];
    /** The targets of the stack, people sign in on some of them. */
    targets: string[];
    onchange: (signin: Signin | null) => void;
  } = $props();

  const DEFAULT_SESSION_MINUTES = 480;
  const DEFAULT_TRIES = 5;
  const DEFAULT_TRIES_MINUTES = 15;

  const people = $derived(entities.find((e) => e.name.name === signin?.entity));

  /** The base type of a field as written: `text(80)` is text. */
  function kindOf(typeText: string): string {
    return typeText.split(/[(\s]/)[0];
  }

  /** The fields of the entity of people a role may name. */
  function fieldsFor(kinds: string[]): string[] {
    return (people?.fields ?? []).filter((f) => kinds.includes(kindOf(f.typeText))).map((f) => f.name.name);
  }

  function change(part: Partial<Signin>) {
    if (signin) onchange({ ...signin, ...part });
  }

  /** Turns sign-in on with the first stored entity and the password strategy, or off. */
  function setOn(on: boolean) {
    if (!on) return onchange(null);
    const entity = entities[0]?.name.name ?? "Person";
    const where = targets.includes("web") ? ["web"] : targets.filter((t) => t !== "cli").slice(0, 1);
    onchange({ entity, targets: where, strategies: [], sessionMinutes: DEFAULT_SESSION_MINUTES, tries: DEFAULT_TRIES, triesMinutes: DEFAULT_TRIES_MINUTES, rememberAsks: false, shows: [], languagePicker: false });
  }

  function setTarget(target: string, on: boolean) {
    if (!signin) return;
    change({ targets: on ? [...signin.targets, target] : signin.targets.filter((t) => t !== target) });
  }

  function used(name: string) {
    return signin?.strategies.find((s) => s.name === name);
  }

  function setStrategy(choice: StrategyChoice, on: boolean) {
    if (!signin) return;
    const strategies = on
      ? [...signin.strategies, { name: choice.name, fields: choice.roles.map((r) => fieldsFor(r.kinds)[0] ?? "") }]
      : signin.strategies.filter((s) => s.name !== choice.name);
    change({ strategies, shows: on ? signin.shows : signin.shows.filter((n) => n !== choice.name) });
  }

  function setField(name: string, at: number, field: string) {
    if (!signin) return;
    change({ strategies: signin.strategies.map((s) => (s.name === name ? { ...s, fields: s.fields.map((f, i) => (i === at ? field : f)) } : s)) });
  }

  /** Sets the fewest characters of a strategy's secret; empty takes the rule away. */
  function setMinimum(name: string, text: string) {
    if (!signin) return;
    const minimum = text.trim() === "" ? undefined : wholeNumber(text, 12);
    change({ strategies: signin.strategies.map((s) => (s.name === name ? { ...s, minimum } : s)) });
  }

  /** Sets one of a provider's settings (`client`, `secret`, `tenant`, `issuer`, `register`); empty text takes it away. */
  function setProvider(name: string, setting: string, value: string | boolean) {
    if (!signin) return;
    change({
      strategies: signin.strategies.map((s) => {
        if (s.name !== name) return s;
        const provider: ProviderSettings = { client: "", register: false, ...s.provider };
        if (setting === "register") provider.register = value === true;
        else if (setting === "client") provider.client = String(value).trim();
        else if (setting === "secret" || setting === "tenant" || setting === "issuer") provider[setting] = String(value).trim() || undefined;
        return { ...s, provider };
      }),
    });
  }

  /** What each provider setting is called, and what it holds. */
  const PROVIDER_FIELDS: Record<string, { label: string; placeholder: string; help: string }> = {
    client: { label: "Client id", placeholder: "the app's id at the provider", help: "The id of the app registered at the provider" },
    secret: { label: "Secret setting", placeholder: "setting:githubSecret", help: "The secret setting that holds the app's secret, never the secret itself; give it its value in the program's settings" },
    tenant: { label: "Tenant", placeholder: "common", help: "The company's id or domain, organizations or consumers; common for any Microsoft account" },
    issuer: { label: "Issuer", placeholder: "https://accounts.google.com", help: "The provider's address, by which its sign-in pages are found" },
  };

  /** The block's strategies in the order the sign-in part shows them: the shown ones first. */
  function shownOrder(): string[] {
    if (!signin) return [];
    const names = signin.strategies.map((s) => s.name);
    return [...signin.shows.filter((n) => names.includes(n)), ...names.filter((n) => !signin!.shows.includes(n))];
  }

  /** Shows or hides a strategy in the sign-in part; all shown is written as no `shows` line. */
  function setShown(name: string, on: boolean) {
    if (!signin) return;
    const current = signin.shows.length === 0 ? signin.strategies.map((s) => s.name) : signin.shows;
    const shows = on ? [...current.filter((n) => n !== name), name] : current.filter((n) => n !== name);
    const all = shows.length === signin.strategies.length && signin.strategies.every((s, i) => s.name === shows[i]);
    change({ shows: all ? [] : shows });
  }

  /** Moves a strategy one place earlier in the sign-in part. */
  function moveShown(name: string, by: number) {
    if (!signin) return;
    const order = signin.shows.length === 0 ? signin.strategies.map((s) => s.name) : [...signin.shows];
    const at = order.indexOf(name);
    if (at < 0 || at + by < 0 || at + by >= order.length) return;
    [order[at], order[at + by]] = [order[at + by], order[at]];
    change({ shows: order });
  }

  /** Whether a strategy works on any of the block's targets. */
  function applies(choice: StrategyChoice): boolean {
    return !!signin && signin.targets.some((t) => choice.targets.includes(t));
  }

  function wholeNumber(text: string, fallback: number): number {
    const n = Number(text);
    return Number.isInteger(n) && n > 0 ? n : fallback;
  }
</script>

<div class="form-grid">
  <span class="form-label">Sign-in</span>
  <label><input type="checkbox" checked={!!signin} onchange={(e) => setOn(e.currentTarget.checked)} /> People sign in</label>
  <p class="form-help">On, the program knows who is using it: code reads <code>me</code>, and windows, commands and actions say who may use them with <code>allow when</code>.</p>
  {#if signin}
    <label class="form-label" for="signin-entity">People</label>
    <select id="signin-entity" value={signin.entity} onchange={(e) => change({ entity: e.currentTarget.value })}>
      {#each entities as entity (entity.name.name)}
        <option value={entity.name.name}>{entity.name.name}</option>
      {/each}
    </select>
    <p class="form-help">The entity whose records are the people who sign in.</p>
    <span class="form-label">On</span>
    <div class="row">
      {#each targets as target (target)}
        <label><input type="checkbox" checked={signin.targets.includes(target)} onchange={(e) => setTarget(target, e.currentTarget.checked)} /> {target}</label>
      {/each}
    </div>
    <p class="form-help">Where people sign in. A served program always asks first; on the desktop and the command line only when ticked.</p>
    <span class="form-label">Strategies</span>
    <div class="strategies">
      {#each strategies as choice (choice.name)}
        {@const line = used(choice.name)}
        <div class="strategy" class:off={!applies(choice)}>
          <label title={choice.help}><input type="checkbox" checked={!!line} disabled={!line && !applies(choice)} onchange={(e) => setStrategy(choice, e.currentTarget.checked)} /> {choice.label}</label>
          {#if line}
            {#each choice.roles as role, i (role.role)}
              <label class="field" title={role.hashed ? "The field needs the hashed modifier" : ""}>
                {role.label}
                <select value={line.fields[i] ?? ""} onchange={(e) => setField(choice.name, i, e.currentTarget.value)}>
                  {#each fieldsFor(role.kinds) as field (field)}
                    <option value={field}>{field}</option>
                  {/each}
                </select>
              </label>
            {/each}
            {#each choice.settings.filter((s) => s !== "register") as setting (setting)}
              {@const field = PROVIDER_FIELDS[setting]}
              <label class="field" title={field.help}>
                {field.label}
                <input class="mono" value={setting === "client" ? (line.provider?.client ?? "") : ((line.provider?.[setting as "secret" | "tenant" | "issuer"]) ?? "")} placeholder={field.placeholder} onchange={(e) => setProvider(choice.name, setting, e.currentTarget.value)} />
              </label>
            {/each}
            {#if choice.settings.includes("register")}
              <label class="field" title="A first sign-in of someone without a record makes their record; off, only people already known sign in">
                <input type="checkbox" checked={line.provider?.register ?? false} onchange={(e) => setProvider(choice.name, "register", e.currentTarget.checked)} /> New people may register
              </label>
            {/if}
            {#if choice.minimum}
              <label class="field" title="The fewest characters a password may have; empty for no rule">
                At least
                <input class="mono short" type="number" min="1" value={line.minimum ?? ""} placeholder="any" onchange={(e) => setMinimum(choice.name, e.currentTarget.value)} />
                characters
              </label>
            {/if}
          {/if}
          <span class="help">{choice.help}; works on {choice.targets.join(", ")}.</span>
        </div>
      {/each}
    </div>
    <p class="form-help">How people prove who they are, several at once. Each names the fields of the entity it matches.</p>
    {#if signin.strategies.length > 1}
      <span class="form-label">Sign-in part</span>
      <div class="shows">
        {#each shownOrder() as name, i (name)}
          <div class="shown">
            <label><input type="checkbox" checked={signin.shows.length === 0 || signin.shows.includes(name)} onchange={(e) => setShown(name, e.currentTarget.checked)} /> {strategies.find((c) => c.name === name)?.label ?? name}</label>
            <button type="button" class="icon-button" title="Earlier" aria-label="Earlier" disabled={i === 0} onclick={() => moveShown(name, -1)}>&uarr;</button>
          </div>
        {/each}
      </div>
      <p class="form-help">What the sign-in part offers, in this order; the first is the default. All ticked shows every strategy that works where the program runs.</p>
    {/if}
    <label class="form-label" for="signin-session">Session</label>
    <input
      id="signin-session"
      class="mono"
      type="number"
      min="1"
      value={signin.sessionMinutes}
      onchange={(e) => change({ sessionMinutes: wholeNumber(e.currentTarget.value, signin?.sessionMinutes ?? DEFAULT_SESSION_MINUTES) })}
    />
    <p class="form-help">How long a sign-in lasts, in minutes. 480 is eight hours.</p>
    <span class="form-label">Tries</span>
    <div class="row">
      <input class="mono short" type="number" min="1" value={signin.tries} onchange={(e) => change({ tries: wholeNumber(e.currentTarget.value, signin?.tries ?? DEFAULT_TRIES) })} />
      per
      <input class="mono short" type="number" min="1" value={signin.triesMinutes} onchange={(e) => change({ triesMinutes: wholeNumber(e.currentTarget.value, signin?.triesMinutes ?? DEFAULT_TRIES_MINUTES) })} />
      minutes
    </div>
    <p class="form-help">Failed tries per account and per address before signing in waits.</p>
    {#if signin.targets.includes("desktop")}
      <label class="form-label" for="signin-remember">Remember</label>
      <input
        id="signin-remember"
        class="mono"
        type="number"
        min="1"
        value={signin.rememberDays ?? ""}
        placeholder="none"
        onchange={(e) => change({ rememberDays: e.currentTarget.value.trim() === "" ? undefined : wholeNumber(e.currentTarget.value, signin?.rememberDays ?? 30) })}
      />
      <p class="form-help">How many days the desktop program remembers who signed in. Empty asks at every start.</p>
      {#if signin.rememberDays}
        <span class="form-label">Asked</span>
        <label><input type="checkbox" checked={signin.rememberAsks} onchange={(e) => change({ rememberAsks: e.currentTarget.checked })} /> The sign-in part shows a "keep me signed in" box</label>
        <p class="form-help">On, a person is remembered only when they tick the box; off, everyone who signs in is remembered.</p>
      {/if}
    {/if}
    <span class="form-label">Language</span>
    <label><input type="checkbox" checked={signin.languagePicker} onchange={(e) => change({ languagePicker: e.currentTarget.checked })} /> The sign-in part lets people pick their language</label>
    <p class="form-help">A dropdown of the program's languages above the fields; off, the sign-in shows in the language the program starts in.</p>
  {/if}
</div>

<style>
  .row {
    display: flex;
    gap: 12px;
    align-items: center;
  }

  .strategies {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .strategy {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 12px;
    align-items: center;
  }

  .strategy.off {
    opacity: 0.6;
  }

  .strategy .field {
    display: flex;
    gap: 6px;
    align-items: center;
    font-size: 12px;
  }

  .shows {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .shown {
    display: flex;
    gap: 8px;
    align-items: center;
  }

  .strategy .help {
    flex-basis: 100%;
    font-size: 12px;
    color: var(--text-muted);
  }

  input.short {
    width: 5em;
  }
</style>
