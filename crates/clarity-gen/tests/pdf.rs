//! `pdf of text stylesheet css theme true` on every stack (docs/ledger/reports-pdf.md,
//! step 4): each calls Clarity.Pdf in its own idiom and carries it only when
//! the code makes a PDF. The builds are whole builds, so they are ignored:
//! `cargo test -p clarity-gen --test pdf -- --ignored --exact <name>`, one at a
//! time. Go, C and C++ want Clarity.Pdf's library beside Clarity.UI's.

mod ide_build;

use clarity_project::Backend;

const MODULE: &str = "module studio

entity Note
    title: text, label @noteTitle
    body: text, label @noteBody

page Main: window
    title @mainTitle
    data note: Note
    data css: text
    button @printCaption name printButton at 1,1 span 2 on click: printClicked

code Main
    handler printClicked
        let plain = pdf of note.body
        let themed = pdf of \"# {note.title}\" stylesheet css theme true
        write plain to exports \"plain.pdf\"
        write themed to exports \"themed.pdf\"
";

const TRANSLATIONS: &str = "translation en\n    mainTitle \"Studio\"\n    printCaption \"Print\"\n    noteTitle \"Title\"\n    noteBody \"Body\"\n";

fn project(module: &str) -> clarity_spec::Project {
    let project = clarity_spec::load_converted([("studio/studio.clarity", module)]);
    let errors: Vec<String> = project.diagnostics.iter().filter(|d| d.severity == clarity_spec::Severity::Error).map(|d| d.message.clone()).collect();
    assert!(errors.is_empty(), "{errors:#?}");
    project
}

fn settings(stack: &str) -> clarity_project::Settings {
    clarity_project::parse(&format!("project Studio\n    stack {stack}\n    datasource main sqlite \"data/studio.db\"\n    filestore exports \"data/exports\"\n")).unwrap()
}

fn files(stack: &str) -> Vec<clarity_gen::OutFile> {
    clarity_gen::programs(&project(MODULE), &settings(stack)).unwrap_or_else(|e| panic!("{stack}: {e:#?}")).into_iter().flat_map(|p| p.files).collect()
}

fn holding<'a>(files: &'a [clarity_gen::OutFile], wanted: &str) -> &'a clarity_gen::OutFile {
    files.iter().find(|f| f.text.contains(wanted)).unwrap_or_else(|| panic!("no file holds {wanted:?}"))
}

#[test]
fn each_stack_calls_clarity_pdf_in_its_own_idiom() {
    let rust = files("rust desktop");
    assert!(holding(&rust, "clarity_pdf::made(").path.ends_with(".rs"));
    let manifest = rust.iter().find(|f| f.path == "Cargo.toml").unwrap();
    assert!(manifest.text.contains(&format!("clarity-pdf = \"{}\"", clarity_gen::rust_pdf_version())), "{}", manifest.text);
    let go = files("go desktop");
    assert!(go.iter().any(|f| f.path == "pdf.go" && f.text.contains("-lclarity_pdf")));
    assert!(holding(&go, "pdfOf(").path.ends_with(".go"));
    assert!(holding(&go, "pdfTheme(").text.contains("pdfTheme(true)") || holding(&go, "pdfOf(").text.contains("pdfTheme("));
    let csharp = files("csharp desktop");
    assert!(holding(&csharp, "Clarity.Pdf.PdfFunctions.Of(").path.ends_with(".cs"));
    assert!(holding(&csharp, "<PackageReference Include=\"Clarity.Pdf\"").path.ends_with(".csproj"));
    let c = files("c desktop");
    assert!(c.iter().any(|f| f.path.ends_with("pdf/pdf.c")));
    assert!(holding(&c, "clarity_pdf_of(").path.ends_with(".c"));
    assert!(holding(&c, "clarity_use_pdf(").path.ends_with("CMakeLists.txt"));
    let cpp = files("cpp desktop");
    assert!(cpp.iter().any(|f| f.path.ends_with("pdf/pdf.cpp")));
    assert!(holding(&cpp, "clarity::pdf_of(").path.ends_with(".cpp"));
    assert!(holding(&cpp, "clarity_use_pdf(").path.ends_with("CMakeLists.txt"));
}

#[test]
fn theme_true_gives_the_app_theme_in_its_light_colors() {
    let rust = files("rust desktop");
    let code = holding(&rust, "clarity_pdf::made(");
    // Clarity's own theme, as the project offers none: its light accent.
    assert!(code.text.contains("#1f5fbf"), "{}", code.text);
    assert!(!code.text.contains("\\\"dark\\\":true"), "{}", code.text);
}

#[test]
fn a_program_that_makes_no_pdf_carries_no_clarity_pdf() {
    let module = MODULE.replace("        let plain = pdf of note.body\n        let themed = pdf of \"# {note.title}\" stylesheet css theme true\n        write plain to exports \"plain.pdf\"\n        write themed to exports \"themed.pdf\"\n", "        note.title = \"none\"\n");
    for (stack, backend) in [("rust desktop", Backend::Rust), ("go desktop", Backend::Go), ("csharp desktop", Backend::Csharp), ("c desktop", Backend::C), ("cpp desktop", Backend::Cpp)] {
        let files: Vec<clarity_gen::OutFile> = clarity_gen::programs(&project(&module), &settings(stack)).unwrap().into_iter().flat_map(|p| p.files).collect();
        // The runtime's clarity.cmake defines clarity_use_pdf for any program; only a program that makes PDFs calls it.
        for file in files.iter().filter(|f| !f.path.ends_with("clarity.cmake")) {
            assert!(!file.text.contains("clarity_pdf") && !file.text.contains("clarity-pdf") && !file.text.contains("Clarity.Pdf"), "{backend:?}: {}", file.path);
        }
    }
}

#[test]
fn go_c_and_cpp_make_pdfs_only_in_a_desktop_program() {
    for stack in ["go web", "c cli", "cpp web"] {
        let Err(errors) = clarity_gen::programs(&project(MODULE), &settings(stack)) else { panic!("{stack} was generated") };
        assert!(errors.iter().any(|e| e.contains("makes PDFs")), "{stack}: {errors:#?}");
    }
    for stack in ["rust cli", "csharp web"] {
        assert!(clarity_gen::programs(&project(MODULE), &settings(stack)).is_ok(), "{stack}");
    }
}

fn builds_without_warnings(stack: &str) {
    let name = format!("pdf-{}", stack.split(' ').next().unwrap());
    let root = ide_build::repo().join("target/generated").join(&name);
    let _ = std::fs::remove_dir_all(&root);
    let dir = root.join("studio");
    std::fs::create_dir_all(dir.join("resources")).unwrap();
    std::fs::create_dir_all(dir.join("translations")).unwrap();
    std::fs::create_dir_all(dir.join("migrations/main")).unwrap();
    std::fs::write(dir.join("migrations/main/0001.clarity"), "migration \"Notes\"\n    add entity Note\n        title: text\n        body: text\n").unwrap();
    // The window goes to design/Main.cxaml, as Clarity keeps it.
    for (path, text) in clarity_spec::converted([("studio.clarity", MODULE)]) {
        let at = dir.join(path);
        std::fs::create_dir_all(at.parent().unwrap()).unwrap();
        std::fs::write(at, text).unwrap();
    }
    std::fs::write(dir.join("translations/en.clarity"), TRANSLATIONS).unwrap();
    let file = root.join("studio.clarity");
    std::fs::write(&file, format!("project Studio\n    title \"Studio\"\n    stack {stack}\n    output \"out\"\n    language \"en\"\n    datasource main sqlite \"data/studio.db\"\n    filestore exports \"data/exports\"\n")).unwrap();
    let settings = clarity_project::load(&file).settings.expect("the project file reads");
    let project = clarity_project::load_modules(&clarity_project::module_dir(&file)).unwrap();
    assert!(!project.has_errors(), "{:?}", project.diagnostics);
    ide_build::builds_without_warnings(&file, &project, &settings);
}

#[test]
#[ignore = "builds a PDF project on rust desktop; run with --ignored, one at a time"]
fn a_rust_pdf_project_builds_without_warnings() {
    builds_without_warnings("rust desktop");
}

#[test]
#[ignore = "builds a PDF project on csharp desktop; run with --ignored, one at a time"]
fn a_csharp_pdf_project_builds_without_warnings() {
    builds_without_warnings("csharp desktop");
}

#[test]
#[ignore = "builds a PDF project on go desktop; run with --ignored, one at a time"]
fn a_go_pdf_project_builds_without_warnings() {
    builds_without_warnings("go desktop");
}

#[test]
#[ignore = "builds a PDF project on c desktop; run with --ignored, one at a time"]
fn a_c_pdf_project_builds_without_warnings() {
    builds_without_warnings("c desktop");
}

#[test]
#[ignore = "builds a PDF project on cpp desktop; run with --ignored, one at a time"]
fn a_cpp_pdf_project_builds_without_warnings() {
    builds_without_warnings("cpp desktop");
}

const SHOP: &str = "module shop

entity Order
    number: text, label @orderNumber

page Orders: window
    title @ordersTitle
    data current: Order
    pdfReport name invoicePdf at 1,1 span 12 rows 10 report Invoice on gather: invoiceGather

code Orders
    handler invoiceGather
        invoicePdf.order = current
";

const INVOICE: &str = r#"<Report name="Invoice" paper="A5"><Data name="order" type="Order" /><ReportHeader height="20mm"><Field at="0,0" size="80,8" bind="order.number" /></ReportHeader></Report>"#;

fn shop() -> clarity_spec::Project {
    let mut project = clarity_spec::load_converted([("shop/shop.clarity", SHOP)]);
    project.reports = vec![clarity_spec::reports::ReportFile { name: "Invoice".into(), path: "reports/Invoice.report".into(), text: INVOICE.into(), stylesheet: None }];
    let mut said = clarity_spec::reports::check(&project);
    said.extend(project.diagnostics.iter().filter(|d| d.severity == clarity_spec::Severity::Error).cloned());
    assert!(said.is_empty(), "{said:#?}");
    project
}

#[test]
fn a_pdf_report_ships_its_report_and_gets_its_data_on_every_stack() {
    for (stack, sets) in [("rust desktop", "ui.set_part_value(\"invoicePdf\", \"order\""), ("go desktop", "\"invoicePdf\", \"order\""), ("csharp desktop", ".SetValue(\"order\""), ("c desktop", "\"invoicePdf\", \"order\""), ("cpp desktop", "\"invoicePdf\", \"order\"")] {
        let files: Vec<clarity_gen::OutFile> = clarity_gen::programs(&shop(), &settings(stack)).unwrap_or_else(|e| panic!("{stack}: {e:#?}")).into_iter().flat_map(|p| p.files).collect();
        let report = files.iter().find(|f| f.path.ends_with("resources/reports/Invoice.report")).unwrap_or_else(|| panic!("{stack} ships no report: {:?}", files.iter().map(|f| &f.path).collect::<Vec<_>>()));
        assert_eq!(report.text, INVOICE);
        assert!(files.iter().any(|f| f.text.contains(sets)), "{stack} sets no invoicePdf.order");
    }
}

#[test]
fn a_pdf_report_that_names_no_report_or_data_it_lacks_is_said() {
    let mut project = clarity_spec::load_converted([("shop/shop.clarity", SHOP.replace("invoicePdf.order = current", "invoicePdf.ordr = current").as_str())]);
    project.reports = vec![clarity_spec::reports::ReportFile { name: "Invoice".into(), path: "reports/Invoice.report".into(), text: INVOICE.into(), stylesheet: None }];
    let said: Vec<String> = clarity_spec::reports::check(&project).into_iter().map(|d| d.message).collect();
    assert!(said.iter().any(|m| m.contains("takes no data 'ordr'")), "{said:#?}");
    project.reports.clear();
    let said: Vec<String> = clarity_spec::reports::check(&project).into_iter().map(|d| d.message).collect();
    assert!(said.iter().any(|m| m.contains("there is no report 'Invoice'")), "{said:#?}");
}

const SHOP_PRINTS: &str = "module shop

entity Order
    number: text, label @orderNumber

page Orders: window
    title @ordersTitle
    data current: Order
    button @printCaption name printButton at 1,1 span 2 on click: printClicked

code Orders
    handler printClicked
        let file = pdf of Invoice with order = current
        write file to exports \"invoice.pdf\"
";

fn printing() -> clarity_spec::Project {
    let mut project = clarity_spec::load_converted([("shop/shop.clarity", SHOP_PRINTS)]);
    project.reports = vec![clarity_spec::reports::ReportFile { name: "Invoice".into(), path: "reports/Invoice.report".into(), text: INVOICE.into(), stylesheet: None }];
    let mut said = clarity_spec::reports::check(&project);
    said.extend(project.diagnostics.iter().filter(|d| d.severity == clarity_spec::Severity::Error).cloned());
    assert!(said.is_empty(), "{said:#?}");
    project
}

#[test]
fn code_makes_a_pdf_of_a_report_through_clarity_ui_on_every_stack() {
    for (stack, call) in [
        ("rust desktop", "clarity_runtime::native_ui::report_pdf(\"Invoice\""),
        ("go desktop", "reportPdf(\"Invoice\", map[string]any{\"order\":"),
        ("csharp desktop", "Clarity.Ui.Reports.Pdf(\"Invoice\""),
        ("c desktop", "clarity_desktop_report_pdf(\"Invoice\""),
        ("cpp desktop", "clarity::ui::report_pdf(\"Invoice\""),
    ] {
        let files: Vec<clarity_gen::OutFile> = clarity_gen::programs(&printing(), &settings(stack)).unwrap_or_else(|e| panic!("{stack}: {e:#?}")).into_iter().flat_map(|p| p.files).collect();
        assert!(files.iter().any(|f| f.text.contains(call)), "{stack} makes no report PDF with {call}");
    }
}

#[test]
fn a_pdf_of_a_report_is_made_only_in_a_desktop_program() {
    for stack in ["rust web", "csharp web", "go cli"] {
        let Err(errors) = clarity_gen::programs(&printing(), &settings(stack)) else { panic!("{stack} was generated") };
        assert!(errors.iter().any(|e| e.contains("PDFs of reports")), "{stack}: {errors:#?}");
    }
}

const PRINTS: &str = "module shop

entity Order
    number: text, label @orderNumber

page Orders: window
    title @ordersTitle
    data current: Order
    button @printCaption name printButton at 1,1 span 2 on click: printClicked
    pdfReport name invoicePdf at 1,2 span 12 rows 10 report Invoice on gather: invoiceGather

code Orders
    handler invoiceGather
        invoicePdf.order = current

    handler printClicked
        let file = pdf of Invoice with order = current
        write file to exports \"invoice.pdf\"
";

const PRINTS_TRANSLATIONS: &str = "translation en\n    orderNumber \"Number\"\n    ordersTitle \"Orders\"\n    printCaption \"Print\"\n";

/// A program with a pdfReport part and code that makes the report's PDF, built whole on `stack`.
fn printing_builds_without_warnings(stack: &str) {
    let name = format!("report-pdf-{}", stack.split(' ').next().unwrap());
    let root = ide_build::repo().join("target/generated").join(&name);
    let _ = std::fs::remove_dir_all(&root);
    let dir = root.join("shop");
    for folder in ["translations", "migrations/main", "reports"] {
        std::fs::create_dir_all(dir.join(folder)).unwrap();
    }
    std::fs::write(dir.join("migrations/main/0001.clarity"), "migration \"Orders\"\n    add entity Order\n        number: text\n").unwrap();
    std::fs::write(dir.join("reports/Invoice.report"), INVOICE).unwrap();
    for (path, text) in clarity_spec::converted([("shop.clarity", PRINTS)]) {
        let at = dir.join(path);
        std::fs::create_dir_all(at.parent().unwrap()).unwrap();
        std::fs::write(at, text).unwrap();
    }
    std::fs::write(dir.join("translations/en.clarity"), PRINTS_TRANSLATIONS).unwrap();
    let file = root.join("shop.clarity");
    std::fs::write(&file, format!("project Shop\n    title \"Shop\"\n    stack {stack}\n    output \"out\"\n    language \"en\"\n    datasource main sqlite \"data/shop.db\"\n    filestore exports \"data/exports\"\n")).unwrap();
    let settings = clarity_project::load(&file).settings.expect("the project file reads");
    let project = clarity_project::load_modules(&clarity_project::module_dir(&file)).unwrap();
    assert!(!project.has_errors(), "{:?}", project.diagnostics);
    assert_eq!(project.reports.len(), 1, "the report is read");
    ide_build::builds_without_warnings(&file, &project, &settings);
}

#[test]
#[ignore = "builds a report-printing project on rust desktop; run with --ignored, one at a time"]
fn a_rust_report_printing_project_builds_without_warnings() {
    printing_builds_without_warnings("rust desktop");
}

#[test]
#[ignore = "builds a report-printing project on go desktop; run with --ignored, one at a time"]
fn a_go_report_printing_project_builds_without_warnings() {
    printing_builds_without_warnings("go desktop");
}

#[test]
#[ignore = "builds a report-printing project on csharp desktop; run with --ignored, one at a time"]
fn a_csharp_report_printing_project_builds_without_warnings() {
    printing_builds_without_warnings("csharp desktop");
}

#[test]
#[ignore = "builds a report-printing project on c desktop; run with --ignored, one at a time"]
fn a_c_report_printing_project_builds_without_warnings() {
    printing_builds_without_warnings("c desktop");
}

#[test]
#[ignore = "builds a report-printing project on cpp desktop; run with --ignored, one at a time"]
fn a_cpp_report_printing_project_builds_without_warnings() {
    printing_builds_without_warnings("cpp desktop");
}

#[test]
fn a_web_program_names_its_reports_among_the_resources_its_browser_reads() {
    let files: Vec<clarity_gen::OutFile> = clarity_gen::programs(&shop(), &settings("go web")).unwrap_or_else(|e| panic!("{e:#?}")).into_iter().flat_map(|p| p.files).collect();
    let json = files.iter().find(|f| f.path == clarity_gen::web_browser::FILES_JSON).expect("files.json");
    let read: serde_json::Value = serde_json::from_str(&json.text).unwrap();
    let names: Vec<&str> = read["resources"].as_array().unwrap().iter().filter_map(|v| v.as_str()).collect();
    assert!(names.contains(&"reports/Invoice.report"), "{names:?}");
    assert!(names.contains(&"reports/Invoice.formats.json"), "{names:?}");
    for name in names {
        assert!(files.iter().any(|f| f.path == format!("{}/{name}", clarity_gen::web_browser::RESOURCES)), "{name} is named but not shipped");
    }
}
