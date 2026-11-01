<script lang="ts">
  import { goBack, goNext, shownStep, stepButtons, type StepButton, type StepPlace } from "../screen/steps.ts";
  import { wiredCall } from "../screen/calls.ts";
  import { ownWord, tr } from "../screen/model.ts";
  import type { PartContext } from "./context.ts";

  let { ctx }: { ctx: PartContext } = $props();
  const p = $derived(ctx.p);

  // A wizard: the step its part's state is on, or in the designer the step picked.
  let stepHere = $state(1);
  const stepPlace: StepPlace = {
    get: () => (ctx.live && p.name ? ctx.live.parts[p.name]?.step : stepHere),
    set: (step) => {
      if (ctx.live && p.name) ctx.live.setStep(p.name, step);
      else stepHere = step;
    },
  };
  const shownStepIndex = $derived(ctx.design ? ctx.activeTab : shownStep(ctx.live && p.name ? ctx.partState?.step : stepHere, ctx.widget.pages.length) - 1);
  const buttonsOfStep = $derived(stepButtons(shownStepIndex + 1, ctx.widget.pages.length));

  function pressStep(button: StepButton) {
    if (ctx.design) return;
    const count = ctx.widget.pages.length;
    const live = ctx.live;
    // In a running application the runtime decides the step (DESIGN 1.7); a preview moves on its own.
    if (live?.step && p.name) {
      void live.step(p.name, button, count, wiredCall(p.handlers, "next"), wiredCall(p.handlers, "finish"));
      return;
    }
    if (button === "back") goBack(stepPlace, count);
    else if (button === "next") void goNext(stepPlace, count, wiredCall(p.handlers, "next") !== null ? () => ctx.fire("next") : undefined);
    else void ctx.fire("finish");
  }
</script>

<div class="steps">
  <div class="step-titles">
    {#each ctx.widget.pages as step, index (step.span.start)}
      {#if ctx.design}
        <button
          class="step-title"
          class:current={index === shownStepIndex}
          class:selected={ctx.isSelected(step.span.start)}
          data-tab-switch="{ctx.offset}:{index}"
          onpointerdown={(e) => ctx.pressTab(e, index, step.span.start)}>{step.title ? tr(ctx.preview, step.title) : step.title}</button
        >
      {:else}
        <span class="step-title" class:current={index === shownStepIndex} class:done={index < shownStepIndex} aria-current={index === shownStepIndex ? "step" : undefined}
          >{step.title ? tr(ctx.preview, step.title) : step.title}</span
        >
      {/if}
    {/each}
  </div>
  {#if ctx.widget.pages[shownStepIndex]}
    {@const step = ctx.widget.pages[shownStepIndex]}
    <div class="step-page">
      {@render ctx.grid({ widgets: step.children, columns: p.width, container: step.span.start })}
    </div>
  {/if}
  <div class="step-buttons">
    {#each buttonsOfStep as button (button)}
      <button type="button" disabled={ctx.disabled} onclick={() => pressStep(button)}
        >{ownWord(ctx.preview, button === "back" ? "Back" : button === "next" ? "Next" : "Finish")}</button
      >
    {/each}
  </div>
</div>

<style>
  .steps {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: calc(var(--padding) * 2);
    min-width: 0;
  }

  .step-titles {
    display: flex;
    gap: calc(var(--padding) * 3);
    border-bottom: var(--border-width) solid var(--border-strong);
    padding-bottom: var(--padding);
  }

  .step-title {
    color: var(--text-muted);
    background: none;
    border: none;
    padding: 0;
  }

  .step-title.done {
    color: var(--text);
  }

  .step-title.current {
    color: var(--accent);
    font-weight: 600;
  }

  .step-title.selected {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }

  .step-page {
    flex: 1;
    overflow: hidden;
  }

  .step-buttons {
    display: flex;
    justify-content: flex-end;
    gap: var(--padding);
  }
</style>
