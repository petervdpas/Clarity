//! Icons (U1b): Clarity knows the Lucide icons by name; a button and a menu
//! item or toolbar button show one with `icon "save"`, and the `icon` part
//! shows one on its own. The checker knows every name, and the model the
//! renderer gets carries only the icons the project uses.

use clarity_spec::keyed::load_keyed;
use clarity_spec::parser::parse;
use clarity_spec::widgets::{self, WidgetKind};
use clarity_spec::{Preview, Severity, icons};

const SHOP: &str = "module shop

page Orders: window
    toolbar name mainToolbar
        button \"Refresh\" name refreshButton icon \"refresh-cw\" on click: refresh
    button \"Save\" name saveButton icon \"save\" at 1,1 span 2 on click: save
    icon \"triangle-alert\" name warnIcon at 3,1
";

const CODE: &str = "
code Orders
    handler refresh
        warnIcon.visible = false

    handler save
        warnIcon.visible = true
";

fn errors(page: &str) -> Vec<String> {
    let text = format!("{page}{CODE}");
    load_keyed([("shop.clarity", text.as_str())], &[], "en").diagnostics.into_iter().filter(|d| d.severity == Severity::Error).map(|d| d.message).collect()
}

#[test]
fn clarity_knows_the_lucide_icons_by_name() {
    assert!(icons::count() > 1800);
    let save = icons::path("save").expect("save is an icon");
    assert!(save.starts_with("M "), "{save}");
    assert!(icons::path("no-such-icon").is_none());
    assert_eq!(icons::suggestion("sav"), Some("save"));
    assert_eq!(icons::suggestion("triangle-alrt"), Some("triangle-alert"));
}

#[test]
fn a_button_has_an_icon_and_the_icon_part_shows_one() {
    let (file, diagnostics) = parse(SHOP, "shop.clarity");
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let save = &file.pages[0].widgets[0];
    assert_eq!(save.props.icon.as_deref(), Some("save"));
    assert_eq!(widgets::widget_line(&save.props), "button \"Save\" name saveButton icon \"save\" at 1,1 span 2 on click: save");
    let warn = &file.pages[0].widgets[1];
    assert_eq!(warn.props.kind, WidgetKind::Icon);
    assert_eq!(warn.props.icon.as_deref(), Some("triangle-alert"));
    assert_eq!(warn.props.caption, None, "the icon's name is no text people read");
    assert_eq!(widgets::widget_line(&warn.props), "icon \"triangle-alert\" name warnIcon at 3,1 span 1");
}

#[test]
fn the_checker_knows_every_icon_and_an_icons_name_is_no_key() {
    assert!(errors(SHOP).is_empty(), "{:#?}", errors(SHOP));
    for (from, to) in [("icon \"save\"", "icon \"sav\""), ("icon \"triangle-alert\" name", "icon \"sav\" name"), ("icon \"refresh-cw\"", "icon \"sav\"")] {
        let wrong = errors(&SHOP.replace(from, to));
        assert!(wrong.iter().any(|m| m == "unknown icon 'sav'; did you mean 'save'?"), "{wrong:#?}");
    }
    let text = format!("{SHOP}{CODE}");
    let project = load_keyed([("shop.clarity", text.as_str())], &[], "en");
    let en = &project.translations["en"];
    assert!(!en.values().any(|v| v == "triangle-alert"), "{en:?}");
    assert!(project.files[0].pages[0].widgets[1].props.icon.as_deref() == Some("triangle-alert"));
}

#[test]
fn the_renderer_gets_the_icons_the_project_uses() {
    let text = format!("{SHOP}{CODE}");
    let preview = Preview::new(load_keyed([("shop.clarity", text.as_str())], &[], "en"));
    let used: Vec<&str> = preview.icons.keys().map(String::as_str).collect();
    assert_eq!(used, vec!["refresh-cw", "save", "triangle-alert"]);
    assert_eq!(preview.icons["save"], icons::path("save").unwrap());
}
