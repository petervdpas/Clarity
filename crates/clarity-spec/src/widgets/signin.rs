//! The catalog's sign-in parts (docs/ledger/signin.md): the sign-in part and the
//! parts of a sign-in view.

use super::*;

pub(super) const ENTRIES: &[Entry] = &[
    // Signs in with the project's signin block, drawn from its strategies; on the sign-in window or the app's splash.
    Entry {
        kind: WidgetKind::Signin,
        keyword: "signin",
        name: "Sign-in",
        category: Category::BasicInput,
        size: (8, 6),
        caption: None,
        bind: BindTarget::Nothing,
        field_types: &[],
        input: false,
        container: None,
        clickable: false,
        events: &[],
        // Only its place and look: the project's signin block says what it offers.
        properties: &[],
    },
    // The parts of a sign-in view (docs/ledger/signin.md):
    // each keeps its place and look; what it signs in with is the project's signin block.
    signin_part(WidgetKind::SigninField, "signinField", "Field", (6, 2), &[WidgetProperty::Input]),
    signin_part(WidgetKind::SigninButton, "signinButton", "Sign-in button", (6, 1), &[WidgetProperty::Strategy]),
    signin_part(WidgetKind::SigninRemember, "signinRemember", "Keep me signed in", (6, 1), &[]),
    signin_part(WidgetKind::SigninLanguage, "signinLanguage", "Language", (6, 2), &[]),
    signin_part(WidgetKind::SigninOther, "signinOther", "Other way", (6, 1), &[WidgetProperty::Shows]),
    signin_part(WidgetKind::SigninRefusal, "signinRefusal", "Refusal", (6, 1), &[]),
];
