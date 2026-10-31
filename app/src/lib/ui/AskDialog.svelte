<script lang="ts">
  // A question a handler asked (DESIGN 5.14), in Clarity's own dialog: the
  // question, the input its form needs (text, a number, a day, a moment, or
  // a choice of an enum's values or of records), and its buttons. The
  // answer, or cancelling, goes back to the runtime, which runs the handler
  // the answer goes to.
  import type { Preview } from "../generated/Preview";
  import type { JsonValue } from "../generated/serde_json/JsonValue";
  import { answerable, numberFormat, type Ask } from "../asks";
  import { numberEditing } from "../format";
  import { findEnum, formatsOf, ownWord, shownText, tr } from "../model";
  import ChoiceInput, { type Choice } from "./ChoiceInput.svelte";
  import DateInput from "./DateInput.svelte";
  import DateTimeInput from "./DateTimeInput.svelte";
  import Dialog from "./Dialog.svelte";
  import NumberInput from "./NumberInput.svelte";
  import TextInput from "./TextInput.svelte";

  let {
    model,
    ask,
    options,
    onanswer,
    oncancel,
  }: {
    model: Preview;
    ask: Ask;
    /** The records of an entity a reference can point at, as the backend gives them. */
    options: (entity: string) => Promise<{ id: number; text: string }[]>;
    /** The person answered: null for a yes, otherwise the value. */
    onanswer: (value: JsonValue | null) => void;
    oncancel: () => void;
  } = $props();

  const form = $derived(ask.form);
  const question = $derived(shownText(model, ask.question));
  const asked = $derived(form.kind === "value" ? form : null);
  const number = $derived(asked ? numberFormat(asked.type) : null);

  let value = $state<JsonValue | null>(null);
  let choices = $state<Choice[] | null>(null);
  $effect.pre(() => {
    value = asked?.default ?? null;
  });

  // An enum's values with their labels, or the records to choose from.
  $effect(() => {
    if (!asked || (asked.type !== "enum" && asked.type !== "record")) {
      choices = null;
      return;
    }
    const of = asked.of ?? "";
    const only = asked.only;
    const pick = (found: Choice[]) => {
      choices = only ? found.filter((c) => only.includes(c.value)) : found;
      if (value === null && choices.length > 0) value = choices[0].value;
    };
    if (asked.type === "enum") {
      pick((findEnum(model, of)?.variants ?? []).map((v) => ({ value: v.name.name, label: tr(model, v.label) })));
    } else {
      options(of)
        .then((found) => pick(found.map((o) => ({ value: o.id, label: o.text }))))
        .catch(() => pick([]));
    }
  });

  const ok = $derived(form.kind === "confirm" && form.yes ? shownText(model, form.yes) : ownWord(model, "OK"));
  const cancel = $derived(form.kind === "confirm" && form.no ? shownText(model, form.no) : ownWord(model, "Cancel"));
  const ready = $derived(answerable(form, value));

  let formElement = $state<HTMLFormElement | null>(null);

  /** Enter in the answer's box answers, as OK does: the box has taken the value first. */
  function answerNow() {
    formElement?.requestSubmit();
    return true;
  }

  /** A question is answered once, however Enter and OK came together. */
  let answered = false;

  function submit(e: SubmitEvent) {
    e.preventDefault();
    if (answered || !ready) return;
    answered = true;
    onanswer(form.kind === "confirm" ? null : value);
  }

  function focused(node: HTMLElement) {
    queueMicrotask(() => (node.querySelector("input, select, button.ok") as HTMLElement | null)?.focus());
  }
</script>

<Dialog label={question} onclose={oncancel}>
  <form bind:this={formElement} onsubmit={submit} use:focused>
    <p class="question">{question}</p>
    {#if asked}
      {#if choices}
        <ChoiceInput label={question} {value} {choices} onchange={(next) => (value = next)} />
      {:else if number}
        {@const editing = numberEditing(number, formatsOf(model))}
        <NumberInput label={question} {value} text={editing.text} read={editing.read} onchange={(next) => (value = next)} onsubmit={answerNow} />
      {:else if asked.type === "date"}
        <DateInput label={question} {value} onchange={(next) => (value = next)} />
      {:else if asked.type === "datetime"}
        <DateTimeInput label={question} {value} onchange={(next) => (value = next)} />
      {:else}
        <TextInput label={question} value={value ?? ""} onchange={(next) => (value = next)} onsubmit={answerNow} />
      {/if}
    {/if}
    <div class="buttons">
      <button type="button" onclick={oncancel}>{cancel}</button>
      <button type="submit" class="ok" disabled={!ready}>{ok}</button>
    </div>
  </form>
</Dialog>

<style>
  form {
    display: flex;
    flex-direction: column;
    gap: calc(var(--padding) * 2);
  }

  .question {
    margin: 0;
    line-height: 1.5;
  }

  .buttons {
    display: flex;
    justify-content: flex-end;
    gap: var(--padding);
  }
</style>
