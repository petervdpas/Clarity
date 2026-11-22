//! A report's file read and written as the Reports tab's designer does
//! (app/src/lib/reportDesign.ts): every case of conformance/report-design.json.

use clarity_spec::report_design::{self, new_part, with_declaration};
use serde_json::Value;

fn cases() -> Value {
    serde_json::from_str(include_str!("../../../../conformance/report-design.json")).unwrap()
}

#[test]
fn a_report_is_written_back_as_the_designer_writes_it() {
    for case in cases()["reports"].as_array().unwrap() {
        let read = report_design::read(case["text"].as_str().unwrap()).unwrap_or_else(|e| panic!("{}: {e}", case["name"]));
        assert_eq!(report_design::write(&read), case["written"].as_str().unwrap(), "{}", case["name"]);
    }
}

#[test]
fn a_new_part_starts_as_the_designer_starts_it() {
    for case in cases()["parts"].as_array().unwrap() {
        let kind = case["kind"].as_str().unwrap();
        let at = (case["at"][0].as_f64().unwrap(), case["at"][1].as_f64().unwrap());
        let part = new_part(kind, at).unwrap_or_else(|| panic!("no new {kind}"));
        assert_eq!(part.size, (case["size"][0].as_f64().unwrap(), case["size"][1].as_f64().unwrap()), "{kind}");
        // serde_json gives an object's words sorted: compared as a set.
        let attrs: std::collections::BTreeMap<String, String> = case["attrs"].as_object().unwrap().iter().map(|(k, v)| (k.clone(), v.as_str().unwrap().to_string())).collect();
        assert_eq!(part.attrs.iter().cloned().collect::<std::collections::BTreeMap<_, _>>(), attrs, "{kind}");
    }
    assert!(new_part("Chart", (0.0, 0.0)).is_none(), "a kind a band has not");
}

#[test]
fn a_class_declaration_is_set_as_the_designer_sets_it() {
    for case in cases()["declarations"].as_array().unwrap() {
        let s = |k: &str| case[k].as_str().unwrap();
        assert_eq!(with_declaration(s("css"), s("selector"), s("property"), s("value")), s("result"), "{}", s("name"));
    }
}

#[test]
fn a_file_that_is_no_report_says_why() {
    assert!(report_design::read("<Window />").unwrap_err().contains("<Report>"));
    assert!(report_design::read("<Report><Footer height=\"5mm\" /></Report>").unwrap_err().contains("Footer"));
    assert!(report_design::read("<Report><Detail><Chart /></Detail></Report>").unwrap_err().contains("Chart"));
}

#[test]
fn the_classes_of_a_stylesheet_are_read_in_their_order() {
    let classes = report_design::classes("/* look */ body { font-size: 10pt; }\n.title { font-size: 18pt; font-weight: bold; }\n.cell, .title { color: #333; }\n");
    let names: Vec<&str> = classes.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(names, ["title", "cell"]);
    assert_eq!(report_design::word(&classes[0].1, "color"), Some("#333"));
    assert_eq!(report_design::word(&classes[0].1, "font-size"), Some("18pt"));
}

#[test]
fn the_designer_s_json_keeps_every_word_in_its_order() {
    let text = "<Report name=\"Invoice\" paper=\"A5 landscape\" stylesheet=\"invoice.css\" margin=\"10mm\">\n  <Data name=\"order\" type=\"Order\" />\n  <Detail height=\"7mm\" list=\"order.lines\">\n    <Field at=\"0,0\" size=\"80,6\" class=\"amount\" bind=\"line.product\" align=\"right\" />\n  </Detail>\n</Report>\n";
    let design = clarity_spec::report_design::read(text).unwrap();
    let json = clarity_spec::report_design::to_json(&design);
    // Words as an object in the order written, data as objects, places as pairs.
    assert!(json.starts_with(r#"{"attrs":{"name":"Invoice","paper":"A5 landscape","stylesheet":"invoice.css","margin":"10mm"},"data":[{"name":"order","type":"Order"}]"#), "{json}");
    assert!(json.contains(r#""attrs":{"class":"amount","bind":"line.product","align":"right"}"#), "{json}");
    assert!(json.contains(r#""at":[0.0,0.0],"size":[80.0,6.0]"#), "{json}");
    let back = clarity_spec::report_design::from_json(&json).unwrap();
    assert_eq!(back, design);
    assert_eq!(clarity_spec::report_design::write(&back), text);
    assert!(clarity_spec::report_design::from_json("{").is_err());
}

#[test]
fn a_stylesheet_s_rules_are_every_selector_s_and_its_classes_the_class_ones() {
    let css = "/* look */ body { font-size: 10pt }\n.title, .big { font-size: 18pt; color: #000 }\n.title { color: #333 }\n";
    let rules = clarity_spec::report_design::rules(css);
    let names: Vec<&str> = rules.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(names, ["body", ".title", ".big"]);
    assert_eq!(rules[1].1, [("font-size".to_string(), "18pt".to_string()), ("color".to_string(), "#333".to_string())]);
    let classes: Vec<String> = clarity_spec::report_design::classes(css).into_iter().map(|(n, _)| n).collect();
    assert_eq!(classes, ["title", "big"]);
}

#[test]
fn a_word_the_designer_left_empty_or_out_is_not_written() {
    let json = r#"{"attrs":{"name":"R"},"data":[],"bands":[{"kind":"ReportHeader","height":20.0,"attrs":{},"parts":[{"kind":"Text","at":[0.0,0.0],"size":[10.0,5.0],"attrs":{"key":"a","align":""}}]}]}"#;
    let text = clarity_spec::report_design::write(&clarity_spec::report_design::from_json(json).unwrap());
    assert!(!text.contains("align"), "{text}");
    assert!(text.contains(r#"<Text at="0,0" size="10,5" key="a" />"#), "{text}");
}
