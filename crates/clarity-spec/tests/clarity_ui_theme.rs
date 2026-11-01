//! Clarity.UI draws with Clarity's own theme until a project gives its own
//! (DESIGN 7.9): it carries that theme as Rust resolves it, for light and
//! dark mode, so the look stays defined in Clarity.
//! `CLARITY_WRITE_DEFAULTS=1 cargo test -p clarity-spec --test clarity_ui_theme` writes them again.

use std::path::Path;

#[test]
fn clarity_ui_carries_clarity_s_own_theme_as_resolved() {
    let themes = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../runtimes/csharp/Clarity.Ui/Themes");
    let base = clarity_spec::themes::built_in();
    for (name, dark) in [("clarity.light.json", false), ("clarity.dark.json", true)] {
        let text = serde_json::to_string_pretty(&clarity_spec::themes::resolve(&base, dark)).unwrap() + "\n";
        if std::env::var("CLARITY_WRITE_DEFAULTS").is_ok() {
            std::fs::write(themes.join(name), &text).unwrap();
        }
        let carried = std::fs::read_to_string(themes.join(name)).unwrap_or_default();
        assert!(carried == text, "{name} is behind Clarity's own theme; write it with CLARITY_WRITE_DEFAULTS=1");
    }
}
