<script lang="ts">
  // A moment (a datetime field): a day picked from the calendar and a time
  // typed as hh:mm on a 24-hour clock, as on Avalonia. The time is taken when
  // it is left or Enter is pressed; a time that is no time is marked and the
  // moment stays.
  import { joinMoment, readTime, splitMoment } from "../moment";
  import { typedDay } from "./typedDay";

  let {
    value,
    id = undefined,
    label = undefined,
    disabled = false,
    readonly = false,
    invalid = false,
    describedBy = undefined,
    onchange,
    onkeydown = undefined,
  }: {
    value: unknown;
    id?: string;
    label?: string;
    disabled?: boolean;
    readonly?: boolean;
    invalid?: boolean;
    describedBy?: string;
    onchange: (next: string | null) => void;
    onkeydown?: (e: KeyboardEvent) => void;
  } = $props();

  const shown = $derived(splitMoment(value));
  /** What was typed in the time while it is no time. */
  let badTime = $state<string | null>(null);
  $effect(() => {
    void value;
    badTime = null;
  });

  function pickDay(day: string) {
    const next = joinMoment(day, shown.time);
    if (next !== (value ?? null)) onchange(next);
  }

  function typeTime(typed: string) {
    const time = readTime(typed);
    if (time === null) {
      badTime = typed;
      return;
    }
    badTime = null;
    if (shown.day === "") return;
    const next = joinMoment(shown.day, time);
    if (next !== value) onchange(next);
  }
</script>

<span class="moment">
  <input
    {id}
    type="date"
    aria-label={label}
    value={shown.day}
    {disabled}
    {readonly}
    aria-describedby={describedBy}
    aria-invalid={invalid}
    use:typedDay={pickDay}
    {onkeydown}
  />
  <input
    class="time"
    class:bad={badTime !== null}
    aria-label={label ? `${label}, time` : "Time"}
    aria-invalid={badTime !== null}
    placeholder="hh:mm"
    inputmode="numeric"
    value={badTime ?? shown.time}
    {disabled}
    {readonly}
    onchange={(e) => typeTime(e.currentTarget.value)}
    onkeydown={(e) => {
      if (e.key === "Enter") typeTime(e.currentTarget.value);
      onkeydown?.(e);
    }}
  />
</span>

<style>
  .moment {
    display: flex;
    gap: 4px;
    width: 100%;
    min-width: 0;
  }

  .moment input[type="date"] {
    flex: 1;
    min-width: 0;
  }

  .time {
    width: 64px;
    flex: none;
    text-align: center;
  }

  .time.bad {
    border-color: var(--error);
  }
</style>
