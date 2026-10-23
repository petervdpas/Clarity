//! The file field type of U1c: any file a record keeps, with its name; in
//! the database (its bytes, and its name in a column beside them) or, with
//! `store`, as a file of a file store. The `file` input chooses, opens,
//! saves a copy of and removes it. Code reads its name and extension,
//! compares it with `none`, empties it, and sets it to a file of a store.

use clarity_spec::keyed::load_keyed;
use clarity_spec::model::FieldType;
use clarity_spec::parser::parse;
use clarity_spec::widgets::{self, WidgetKind};
use clarity_spec::{Project, Severity};

const SHOP: &str = "module shop

entity Product
    name: text(80), required
    manual: file
    sheet: file, store documents

page ProductCard: window
    data product: Product
    file name manualBox at 1,1 span 6 bind product.manual on change: manualChanged
    file name sheetBox at 1,2 span 6 bind product.sheet
    textbox name manualText at 7,1 span 4
";

const CODE: &str = "
code ProductCard
    handler manualChanged
        if product.manual != none
            manualText.text = product.manual.name
            if product.manual.extension == \"exe\"
                product.manual = none
";

fn project(page: &str, code: &str) -> Project {
    let text = format!("{page}{code}");
    load_keyed([("shop.clarity", text.as_str())], &[], "en")
}

fn errors(page: &str, code: &str) -> Vec<String> {
    project(page, code).diagnostics.into_iter().filter(|d| d.severity == Severity::Error).map(|d| d.message).collect()
}

#[test]
fn a_file_field_and_its_input_read_and_write_back() {
    let (file, diagnostics) = parse(SHOP, "shop.clarity");
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let product = &file.entities[0];
    assert_eq!(product.field("manual").unwrap().ty, FieldType::File);
    assert_eq!(product.field("sheet").unwrap().modifiers.store.as_deref(), Some("documents"));
    let part = &file.pages[0].widgets[0];
    assert_eq!(part.props.kind, WidgetKind::File);
    assert_eq!(widgets::widget_line(&part.props), "file name manualBox at 1,1 span 6 bind product.manual on change: manualChanged");
    assert_eq!(widgets::widget_for(&FieldType::File), WidgetKind::File, "a dropped file field gets a file input");
}

#[test]
fn code_reads_a_files_name_and_empties_it() {
    let found = errors(SHOP, CODE);
    assert!(found.is_empty(), "{found:#?}");
    let set = errors(SHOP, &CODE.replace("manualText.text = product.manual.name", "product.manual.name = \"x.pdf\""));
    assert!(set.iter().any(|m| m.contains("a file's name is read, not set")), "{set:#?}");
    let size = errors(SHOP, &CODE.replace("product.manual.name", "product.manual.modified"));
    assert!(size.iter().any(|m| m.contains("a file of a record has name and extension")), "{size:#?}");
    let copied = errors(SHOP, &CODE.replace("product.manual = none", "product.manual = product.sheet"));
    assert!(copied.iter().any(|m| m.contains("a file cannot be copied from one record to another yet")), "{copied:#?}");
}

#[test]
fn the_rules_of_a_file_field() {
    let says = |page: String, wanted: &str| {
        let found = errors(&page, CODE);
        assert!(found.iter().any(|m| m.contains(wanted)), "{wanted}: {found:#?}");
    };
    says(SHOP.replace("    manual: file\n", "    manual: file, unique\n"), "a file cannot be unique ('manual')");
    says(SHOP.replace("    manual: file\n", "    manual: file, default \"a.pdf\"\n"), "a file has no default; it starts empty ('manual')");
    says(SHOP.replace("    name: text(80), required\n", "    name: text(80), required, store documents\n"), "only an image or a file field has a store ('name')");
    says(SHOP.replace("    file name manualBox at 1,1 span 6 bind product.manual", "    file name manualBox at 1,1 span 6 bind product.name"), "'name' is text; a file shows file");
    says(format!("{SHOP}\npage Browse: window\n    data products: list<Product>\n    table name grid at 1,1 span 12 rows 4 bind products columns name, manual\n"), "'manual' is a file; show it in a file part");
}

#[test]
fn a_file_part_has_clarity_s_own_words() {
    let p = project(SHOP, CODE);
    let en = &p.translations["en"];
    for (key, word) in clarity_spec::translations::FILE_KEYS {
        assert_eq!(en.get(key).map(String::as_str), Some(word), "{key}");
    }
}
