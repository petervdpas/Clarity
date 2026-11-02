//! What a program's model tells the screen (C# FieldFactsTests, the
//! settings' language and theme pickers): its windows, the facts of its
//! fields, its enums, the languages and themes it offers and the one the
//! person chose, kept for the next start.

use clarity_ui::choices;
use clarity_ui::facts::{format_written, without_places, ModelFacts};
use clarity_ui::AppFiles;

const MODEL: &str = r##"{
  "icons": { "list": "M 3 5 h 0.01 M 8 5 h 13" },
  "project": {
    "locale": { "language": "en", "currency": "EUR" },
    "translations": { "en": { "lightTheme": "Light", "darkTheme": "Dark", "red": "Red" }, "nl": { "lightTheme": "Licht", "darkTheme": "Donker", "red": "Rood" } },
    "keysEditable": true,
    "themes": ["Light", "Dark"],
    "files": [{
      "pages": [
        { "name": { "name": "Main" }, "kind": "window", "title": "@mainTitle", "size": null, "columns": 12,
          "data": [ { "name": { "name": "tasks" }, "of": { "kind": "list", "entity": { "name": "Task" } } },
                    { "name": { "name": "settings" }, "of": { "kind": "record", "entity": { "name": "Settings" } } } ] },
        { "name": { "name": "Pick" }, "kind": "dialog", "title": "Pick", "size": { "width": 320, "height": 200 }, "columns": 6, "data": [] }
      ],
      "apps": [],
      "enums": [ { "name": { "name": "Tone" }, "variants": [ { "name": { "name": "red" }, "label": "@red" }, { "name": { "name": "blue" }, "label": "Blue" } ] } ],
      "records": [],
      "themes": [
        { "name": { "name": "Light" }, "class": "theme-light", "label": "@lightTheme" },
        { "name": { "name": "Dark" }, "class": "theme-dark", "label": "@darkTheme" }
      ],
      "entities": [
        { "name": { "name": "Task" }, "fields": [
          { "name": { "name": "title" }, "type": { "kind": "text", "maxLength": 120 }, "label": "@taskTitleLabel", "modifiers": { "required": true, "min": null, "max": null, "format": null } },
          { "name": { "name": "cost" }, "type": { "kind": "decimal", "precision": 10, "scale": 2 }, "label": "Cost", "modifiers": { "required": false, "min": { "kind": "int", "value": 0 }, "max": { "kind": "int", "value": 99 }, "format": "percent" } },
          { "name": { "name": "tone" }, "type": { "kind": "enum", "name": { "name": "Tone" } }, "label": "Tone", "modifiers": {} },
          { "name": { "name": "parent" }, "type": { "kind": "ref", "entity": { "name": "Task" } }, "label": "Parent", "modifiers": {} },
          { "name": { "name": "color" }, "type": { "kind": "color" }, "label": "Color", "modifiers": { "store": null } }
        ] },
        { "name": { "name": "Settings" }, "fields": [
          { "name": { "name": "language" }, "type": { "kind": "language" }, "label": "@languagePicker", "modifiers": {} },
          { "name": { "name": "theme" }, "type": { "kind": "theme" }, "label": "@themePicker", "modifiers": {} }
        ] }
      ]
    }]
  }
}"##;

#[test]
fn the_model_says_which_windows_are_dialogs_and_their_size() {
    let _alone = super::one_at_a_time();
    let facts = ModelFacts::read(MODEL);
    assert!(facts.is_dialog("Pick"));
    assert!(!facts.is_dialog("Main"));
    assert_eq!(Some((320.0, 200.0)), facts.window("Pick").and_then(|w| w.size));
    assert_eq!(None, facts.window("Main").and_then(|w| w.size));
    assert!(facts.keys_editable());
    assert_eq!(Some("EUR".to_string()), facts.formats().currency);
}

#[test]
fn a_fields_facts_are_found_by_its_path_with_or_without_the_place_of_its_row() {
    let _alone = super::one_at_a_time();
    let facts = ModelFacts::read(MODEL);
    for path in ["tasks.cost", "tasks[3].cost"] {
        let cost = facts.field("Main", path).expect(path);
        assert_eq!("Cost", cost.label);
        assert_eq!(Some("decimal 2 percent".to_string()), cost.format);
        assert_eq!((Some("0".to_string()), Some("99".to_string())), (cost.min, cost.max));
    }
    let title = facts.field("Main", "tasks[0].title").unwrap();
    assert!(title.required);
    assert_eq!("@taskTitleLabel", title.label);
    // A tree's records name their parent through a reference to their own entity.
    assert_eq!(Some("parent".to_string()), title.parent);
    assert_eq!(Some("Task".to_string()), facts.field("Main", "tasks.parent").unwrap().refers);
    assert!(facts.field("Main", "tasks.color").unwrap().color);
    assert_eq!(None, facts.field("Main", "tasks.nothing"));
    assert_eq!(None, facts.field("Other", "tasks.cost"));
}

#[test]
fn a_values_type_is_its_enum_its_entity_or_its_kind() {
    let _alone = super::one_at_a_time();
    let facts = ModelFacts::read(MODEL);
    assert_eq!(Some("Tone".to_string()), facts.type_of("Main", "tasks[1].tone"));
    assert_eq!(Some("Task".to_string()), facts.type_of("Main", "tasks"));
    assert_eq!(Some("decimal".to_string()), facts.type_of("Main", "tasks.cost"));
    assert_eq!(Some("language".to_string()), facts.type_of("Main", "settings.language"));
    let tones = facts.enum_choices("Tone");
    assert_eq!(vec!["red", "blue"], tones.iter().map(|c| c.value.as_str().unwrap()).collect::<Vec<_>>());
    assert_eq!("Blue", tones[1].label);
    assert_eq!("tasks.done", without_places("tasks[12].done"));
    assert_eq!(Some("date long".to_string()), format_written("date", None, Some("long")));
    assert_eq!(None, format_written("text", None, None));
}

static FILES: AppFiles = AppFiles {
    windows: &[],
    apps: &[],
    themes: &[("light.light", "{}"), ("light.dark", "{}"), ("dark.light", "{}"), ("dark.dark", "{}")],
    model: MODEL,
    resources: &[],
    title: "Facts",
    width: 0.0,
    height: 0.0,
};

#[test]
fn the_person_chooses_a_language_and_a_theme_kept_for_the_next_start() {
    let _alone = super::one_at_a_time();
    let folder = std::env::temp_dir().join(format!("clarity-ui-choices-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);
    clarity_runtime::i18n::load(MODEL);
    choices::start(&FILES, Some(folder.clone()));
    let languages = choices::languages();
    assert_eq!(vec!["en", "nl"], languages.iter().map(|l| l.name.as_str()).collect::<Vec<_>>());
    assert_eq!(vec!["English", "Nederlands"], languages.iter().map(|l| l.label.as_str()).collect::<Vec<_>>());
    assert!(languages[0].current);
    let themes = choices::themes();
    assert_eq!(vec![("Light", "Light", true), ("Dark", "Dark", false)], themes.iter().map(|t| (t.name.as_str(), t.label.as_str(), t.current)).collect::<Vec<_>>());

    let before = choices::changes();
    choices::choose_language("nl");
    assert_eq!("nl", choices::language());
    assert_eq!(Some("Rood".to_string()), clarity_runtime::i18n::value("red"));
    // A theme's label shows in the language shown.
    assert_eq!("Donker", choices::themes()[1].label);
    choices::choose_theme("Dark");
    assert_eq!(Some("Dark".to_string()), choices::theme());
    assert!(choices::changes() >= before + 2);
    // One the program does not offer is left.
    choices::choose_theme("Neon");
    choices::choose_language("xx");
    assert_eq!((Some("Dark".to_string()), "nl".to_string()), (choices::theme(), choices::language()));

    // The next start shows what was chosen.
    choices::start(&FILES, Some(folder.clone()));
    assert_eq!((Some("Dark".to_string()), "nl".to_string()), (choices::theme(), choices::language()));
    choices::choose_language("en");
    assert_eq!(None, clarity_runtime::i18n::language());
    let _ = std::fs::remove_dir_all(&folder);
}

#[test]
fn the_icons_the_program_carries_answer_by_their_name() {
    let _alone = super::one_at_a_time();
    // The ribbon's large buttons and icon parts draw the path the model carries.
    let facts = ModelFacts::read(MODEL);
    assert_eq!(Some("M 3 5 h 0.01 M 8 5 h 13"), facts.icon("list"));
    assert_eq!(None, facts.icon("languages"));
}
