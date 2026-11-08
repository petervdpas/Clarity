//! A small project that reads and sets a memo's caret, reads its selected
//! text, inserts at the caret and takes rows dropped on it (Writer gap 21),
//! built on each desktop stack as the IDE builds it, without a warning. Each
//! is a whole build, so they are ignored: run one at a time,
//! `cargo test -p clarity-gen --test typing_build -- --ignored --exact <name>`.

mod ide_build;

const MODULE: &str = "module studio

entity Fragment
    heading: text, label @fragmentHeading
    text: text, label @fragmentText

entity Book
    title: text, label @bookTitle
    text: text, label @bookText

page Main: window
    title @mainTitle
    data book: Book
    data fragments: list<Fragment>
    data at: integer
    data picked: text
    list name fragmentList at 1,1 span 3 rows 6 bind fragments show heading
    memo name manuscriptBox at 4,1 span 6 rows 6 bind book.text markdown toolbox WritingTools on drop from fragmentList: fragmentDropped
    button @makeCaption name makeButton at 10,1 span 2 on click: quoteClicked

page WritingTools: toolbox
    tool name quoteTool icon \"quote\" at 1,1 on click: quoteClicked

code Main
    handler fragmentDropped
        at = manuscriptBox.caret
        manuscriptBox.insert(fragmentList.selected.text)

    handler quoteClicked
        picked = manuscriptBox.selectedText
        manuscriptBox.insert(\"> \" + picked)
        manuscriptBox.caret = at + 2
";

fn builds_without_warnings(stack: &str) {
    let name = format!("typing-{}", stack.split(' ').next().unwrap());
    let root = ide_build::repo().join("target/generated").join(&name);
    let _ = std::fs::remove_dir_all(&root);
    let dir = root.join("studio");
    std::fs::create_dir_all(dir.join("resources")).unwrap();
    std::fs::create_dir_all(dir.join("translations")).unwrap();
    std::fs::create_dir_all(dir.join("migrations/main")).unwrap();
    std::fs::write(dir.join("migrations/main/0001.clarity"), "migration \"Books\"\n    add entity Book\n        title: text\n        text: text\n    add entity Fragment\n        heading: text\n        text: text\n").unwrap();
    std::fs::write(dir.join("studio.clarity"), MODULE).unwrap();
    std::fs::write(dir.join("translations/en.clarity"), "translation en\n    mainTitle \"Studio\"\n    makeCaption \"Quote\"\n    bookTitle \"Title\"\n    bookText \"Text\"\n    fragmentHeading \"Heading\"\n    fragmentText \"Text\"\n    markdownWrite \"Write\"\n    markdownPreview \"Preview\"\n").unwrap();
    let file = root.join("studio.clarity");
    std::fs::write(&file, format!("project Studio\n    title \"Studio\"\n    stack {stack}\n    output \"out\"\n    language \"en\"\n    datasource main sqlite \"data/studio.db\"\n")).unwrap();
    let settings = clarity_project::load(&file).settings.expect("the project file reads");
    let project = clarity_project::load_modules(&clarity_project::module_dir(&file)).unwrap();
    assert!(!project.has_errors(), "{:?}", project.diagnostics);
    ide_build::builds_without_warnings(&file, &project, &settings);
}

#[test]
#[ignore = "builds a typing project on rust desktop; run with --ignored, one at a time"]
fn a_rust_typing_project_builds_without_warnings() {
    builds_without_warnings("rust desktop");
}

#[test]
#[ignore = "builds a typing project on csharp desktop; run with --ignored, one at a time"]
fn a_csharp_typing_project_builds_without_warnings() {
    builds_without_warnings("csharp desktop");
}

#[test]
#[ignore = "builds a typing project on go desktop; run with --ignored, one at a time"]
fn a_go_typing_project_builds_without_warnings() {
    builds_without_warnings("go desktop");
}

#[test]
#[ignore = "builds a typing project on c desktop; run with --ignored, one at a time"]
fn a_c_typing_project_builds_without_warnings() {
    builds_without_warnings("c desktop");
}

#[test]
#[ignore = "builds a typing project on cpp desktop; run with --ignored, one at a time"]
fn a_cpp_typing_project_builds_without_warnings() {
    builds_without_warnings("cpp desktop");
}
