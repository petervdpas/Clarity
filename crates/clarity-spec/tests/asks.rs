//! Asking the person (DESIGN 5.14): `ask ... then handler` in a handler.
//! The dialog shows after the handler has finished; the answer runs the
//! handler after `then`, with the values written there and the answer
//! last. What is asked decides the dialog: a question with yes and no, or a
//! value of a type (text, a number, a date, an enum, a record).

use clarity_spec::code::{AskForm, ExprKind, StmtKind, TypeRefKind};
use clarity_spec::keyed::load_keyed;
use clarity_spec::{Project, Severity};

const MUSIC: &str = "module music

enum Format
    Vinyl
    Cd

entity Friend
    name: text(80), required

entity Album
    title: text(120), required
    format: Format
    copies: integer

page Albums: window
    data album: Album
    data friends: list<Friend>
    button \"Delete\" name deleteButton at 1,1 span 2 on click: deleteClicked
    button \"Lend\" name lendButton at 3,1 span 2 on click: lendClicked
";

fn project(code: &str) -> Project {
    let text = format!("{MUSIC}\ncode Albums\n{code}");
    load_keyed([("music.clarity", text.as_str())], &[], "en")
}

fn errors(code: &str) -> Vec<String> {
    let p = project(code);
    p.diagnostics.into_iter().filter(|d| d.severity == Severity::Error).map(|d| d.message).collect()
}

fn warnings(code: &str) -> Vec<String> {
    let p = project(code);
    p.diagnostics.into_iter().filter(|d| d.severity == Severity::Warning).map(|d| d.message).collect()
}

const GOOD: &str = "    handler deleteClicked
        ask \"Delete {album.title}?\" yes \"Delete\" no \"Keep\" then deleteConfirmed

    handler deleteConfirmed
        delete album

    handler lendClicked
        ask \"Lend it to\" for Friend from friends then lend(album)

    handler lend(what: Album, friend: Friend)
        what.copies = what.copies - 1
        save what

    handler copiesClicked
        ask \"How many copies?\" for integer default album.copies then setCopies

    handler setCopies(copies: integer)
        album.copies = copies

    handler formatClicked
        ask \"Which format?\" for Format then setFormat

    handler setFormat(format: Format)
        album.format = format
";

#[test]
fn ask_reads_a_question_its_form_and_the_handler_the_answer_runs() {
    let p = project(GOOD);
    let errors: Vec<&str> = p.diagnostics.iter().filter(|d| d.severity == Severity::Error).map(|d| d.message.as_str()).collect();
    assert!(errors.is_empty(), "{errors:#?}");
    let code = &p.files[0].code[0];
    let asks: Vec<&StmtKind> = code.handlers.iter().flat_map(|h| &h.body).map(|s| &s.kind).filter(|k| matches!(k, StmtKind::Ask { .. })).collect();
    assert_eq!(asks.len(), 4);
    let StmtKind::Ask { question, form, then, args } = asks[0] else { unreachable!() };
    // Texts people read are keys, the question's slot filled by name.
    assert!(matches!(&question.kind, ExprKind::Key { args, .. } if args.len() == 1), "{question:?}");
    let AskForm::Confirm { yes: Some(yes), no: Some(_) } = form.as_ref() else { panic!("{form:?}") };
    assert!(matches!(yes.kind, ExprKind::Key { .. }), "{yes:?}");
    assert_eq!((then.name.as_str(), args.len()), ("deleteConfirmed", 0));
    let StmtKind::Ask { form, then, args, .. } = asks[1] else { unreachable!() };
    let AskForm::Value { ty, from, default } = form.as_ref() else { panic!("{form:?}") };
    assert_eq!(ty.kind, TypeRefKind::Named { name: "Friend".into() });
    assert!(from.is_some() && default.is_none());
    assert_eq!((then.name.as_str(), args.len()), ("lend", 1));
    let StmtKind::Ask { form, .. } = asks[2] else { unreachable!() };
    let AskForm::Value { ty, default, .. } = form.as_ref() else { unreachable!() };
    assert_eq!((ty.kind.clone(), default.is_some()), (TypeRefKind::Integer, true));
}

#[test]
fn texts_asked_are_keys_and_answering_handlers_count_as_wired() {
    let p = project(GOOD);
    let keys: Vec<&String> = p.translations.get("en").map(|t| t.keys().collect()).unwrap_or_default();
    assert!(keys.len() >= 5, "the question, yes and no, and the other questions became keys: {keys:?}");
    let not_wired: Vec<String> = warnings(GOOD).into_iter().filter(|w| w.contains("not wired")).collect();
    assert!(not_wired.iter().all(|w| !w.contains("deleteConfirmed") && !w.contains("lend") && !w.contains("setCopies")), "{not_wired:#?}");
}

#[test]
fn the_answer_must_fit_the_last_parameter_and_the_values_the_others() {
    let wrong_type = errors(
        "    handler lendClicked
        ask \"How many?\" for integer then lend
    handler lend(name: text)
        album.title = name
",
    );
    assert!(wrong_type.iter().any(|m| m.contains("the answer is integer") && m.contains("lend")), "{wrong_type:#?}");
    let too_few = errors(
        "    handler lendClicked
        ask \"Lend it to\" for Friend then lend
    handler lend(what: Album, friend: Friend)
        save what
",
    );
    assert!(too_few.iter().any(|m| m.contains("lend") && m.contains("2 parameters")), "{too_few:#?}");
    let confirm = errors(
        "    handler deleteClicked
        ask \"Sure?\" then gone
    handler gone(yes: bool)
        delete album
",
    );
    assert!(confirm.iter().any(|m| m.contains("gone") && m.contains("takes no answer")), "{confirm:#?}");
    let missing = errors("    handler deleteClicked\n        ask \"Sure?\" then nowhere\n");
    assert!(missing.iter().any(|m| m.contains("no handler 'nowhere'")), "{missing:#?}");
}

#[test]
fn what_can_be_asked_for_and_with_what() {
    let list = errors("    handler deleteClicked\n        ask \"Which?\" for list<Friend> then pick\n    handler pick(f: list<Friend>)\n        reload friends\n");
    assert!(list.iter().any(|m| m.contains("ask for text, a number, a date, a duration, an enum or an entity")), "{list:#?}");
    let default = errors("    handler deleteClicked\n        ask \"How many?\" for integer default album.title then setCopies\n    handler setCopies(n: integer)\n        album.copies = n\n");
    assert!(default.iter().any(|m| m.contains("default")), "{default:#?}");
    let from = errors("    handler deleteClicked\n        ask \"Who?\" for Friend from album.title then pick\n    handler pick(f: Friend)\n        save f\n");
    assert!(from.iter().any(|m| m.contains("from") && m.contains("list<Friend>")), "{from:#?}");
}

#[test]
fn only_a_handler_asks_and_it_cannot_close_its_window_too() {
    let action = load_keyed(
        [(
            "music.clarity",
            format!("{MUSIC}\naction tidy()\n    ask \"Sure?\" then done\n").as_str(),
        )],
        &[],
        "en",
    );
    assert!(action.diagnostics.iter().any(|d| d.message.contains("'ask' belongs in a handler")), "{:#?}", action.diagnostics);
    let closes = errors("    handler deleteClicked\n        ask \"Sure?\" then gone\n        close\n    handler gone\n        delete album\n");
    assert!(closes.iter().any(|m| m.contains("asks and closes")), "{closes:#?}");
}

#[test]
fn a_question_written_as_text_asks_for_a_key_as_a_message_does() {
    let text = format!("{MUSIC}\ncode Albums\n    handler deleteClicked\n        ask \"Delete it?\" then gone\n    handler gone\n        delete album\n");
    let checked = |text: &str| {
        let mut p = clarity_spec::load_sources([("music.clarity", text)]);
        clarity_spec::translations::attach(&mut p, std::iter::empty());
        p
    };
    let p = checked(&text);
    let message = checked(&text.replace("ask \"Delete it?\" then gone", "message \"Delete it?\""));
    let about = |p: &Project| p.diagnostics.iter().filter(|d| d.message.contains("key")).map(|d| d.message.clone()).collect::<Vec<_>>();
    assert!(!about(&message).is_empty(), "a message written as text asks for a key");
    assert_eq!(about(&p).len(), about(&message).len(), "{:#?}", about(&p));
}
