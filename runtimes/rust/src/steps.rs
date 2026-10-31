//! A wizard, the steps part (DESIGN 5.15), decided here, not in the screen
//! (DESIGN 1.7): the step it shows, the buttons it has, and what Next does.
//! Next runs the handler of `on next` on the step shown, then moves one step
//! on, unless the handler refused (the step stays) or set the step itself
//! (that step wins). Back goes one step back and runs nothing; Finish runs
//! `on finish`. Every runtime walks alike (conformance/steps.json).

use crate::ui::{Reply, WindowState};
use crate::{Db, Result};
use serde::{Deserialize, Serialize};

/// A button of a wizard.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StepButton {
    Back,
    Next,
    Finish,
}

/// The step shown when the part's step is `step` of `count`: the nearest
/// there is, the first without one.
pub fn shown(step: Option<i64>, count: i64) -> i64 {
    step.unwrap_or(1).clamp(1, count.max(1))
}

/// The buttons of step `shown` of `count`: Back from the second on, Next
/// before the last, Finish on the last.
pub fn buttons(shown: i64, count: i64) -> Vec<StepButton> {
    let mut buttons = vec![];
    if shown > 1 {
        buttons.push(StepButton::Back);
    }
    buttons.push(if shown < count { StepButton::Next } else { StepButton::Finish });
    buttons
}

fn step_of(state: &WindowState, part: &str) -> Option<i64> {
    state.parts.get(part).and_then(|p| p.step)
}

fn set_step(state: &mut WindowState, part: &str, step: i64) {
    state.parts.entry(part.to_string()).or_default().step = Some(step);
}

/// `step_event` (DESIGN 1.7): `button` pressed on the wizard `part` of
/// `count` steps, whose `on next` and `on finish` handlers are `next` and
/// `finish`. `run` runs a handler as `dispatch_event` does, as one database
/// step; a refusal or a failure is its error, and the step stays. Back, and
/// a button without a handler, answer the window with its new step (`draw`).
#[allow(clippy::too_many_arguments)]
pub fn event(
    db: &Db,
    mut state: WindowState,
    part: &str,
    button: StepButton,
    count: i64,
    next: Option<&str>,
    finish: Option<&str>,
    run: impl FnOnce(WindowState, &str) -> Result<Reply>,
    draw: impl FnOnce(WindowState) -> Result<Reply>,
) -> Result<Reply> {
    let before = shown(step_of(&state, part), count);
    set_step(&mut state, part, before);
    let handler = |h: Option<&str>| h.filter(|h| !h.is_empty()).map(str::to_string);
    match button {
        StepButton::Back => {
            if before > 1 {
                set_step(&mut state, part, before - 1);
            }
            draw(state)
        }
        StepButton::Next => match handler(next) {
            Some(next) => {
                let mut reply = crate::stepped(db, || run(state, &next))?;
                if shown(step_of(&reply.state, part), count) == before && before < count {
                    set_step(&mut reply.state, part, before + 1);
                }
                Ok(reply)
            }
            None => {
                if before < count {
                    set_step(&mut state, part, before + 1);
                }
                draw(state)
            }
        },
        StepButton::Finish => match handler(finish) {
            Some(finish) => crate::stepped(db, || run(state, &finish)),
            None => draw(state),
        },
    }
}
