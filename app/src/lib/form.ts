// The form a part sits in, so Enter in one of its inputs runs the form's
// `on submit` handler.

import { getContext, setContext } from "svelte";

const FORM = Symbol("clarity-form");

export interface FormContext {
  /** Runs the form's submit handler; false when none is wired. */
  submit(): boolean;
}

export function provideForm(form: FormContext) {
  setContext(FORM, form);
}

export function useForm(): FormContext | undefined {
  return getContext<FormContext | undefined>(FORM);
}
