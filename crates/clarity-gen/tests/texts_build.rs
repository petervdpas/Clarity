//! A small project that uses every text function (conformance/texts.json),
//! built on each desktop stack as the IDE builds it, without a warning. Each is a whole build, so they are
//! ignored: run one at a time,
//! `cargo test -p clarity-gen --test texts_build -- --ignored --exact <name>`.
//! Go, C and C++ want Clarity.Audio's library beside Clarity.UI's:
//! `cargo build --release -p clarity-ui-ffi -p clarity-audio-ffi`.

mod ide_build;

const MODULE: &str = "module player

page Main: window
    title @mainTitle
    data text: text
    data count: integer
    data starts: bool
    data shown: text
    textbox name textBox at 1,1 span 6 bind text
    button @playCaption name countButton at 7,1 span 2 on click: countClicked
    textbox name shownBox at 1,2 span 6 bind shown readonly

code Main
    handler countClicked
        let all = words(text)
        count = count(all)
        let parts = split(text, \",\")
        shown = join(parts, \"; \")
        shown = trim(lower(upper(shown)))
        shown = replace(shown, \"a\", \"b\")
        starts = startsWith(shown, \"b\") or endsWith(shown, \".\")
        shown = slice(shown, 2, 3)
        count = indexOf(shown, \"b\") + count(words(\"one two\"))
";

fn builds_without_warnings(stack: &str) {
    let name = format!("texts-{}", stack.split(' ').next().unwrap());
    let root = ide_build::repo().join("target/generated").join(&name);
    let _ = std::fs::remove_dir_all(&root);
    let dir = root.join("player");
    std::fs::create_dir_all(dir.join("resources")).unwrap();
    std::fs::create_dir_all(dir.join("translations")).unwrap();
    std::fs::write(dir.join("player.clarity"), MODULE).unwrap();
    std::fs::write(dir.join("translations/en.clarity"), "translation en\n    mainTitle \"Player\"\n    playCaption \"Play\"\n    pauseCaption \"Pause\"\n    stopCaption \"Stop\"\n").unwrap();
    let file = root.join("player.clarity");
    std::fs::write(&file, format!("project Player\n    title \"Player\"\n    stack {stack}\n    output \"out\"\n    language \"en\"\n")).unwrap();
    let settings = clarity_project::load(&file).settings.expect("the project file reads");
    let project = clarity_project::load_modules(&clarity_project::module_dir(&file)).unwrap();
    assert!(!project.has_errors(), "{:?}", project.diagnostics);
    ide_build::builds_without_warnings(&file, &project, &settings);
}

#[test]
#[ignore = "builds a text project on rust desktop; run with --ignored, one at a time"]
fn a_rust_text_project_builds_without_warnings() {
    builds_without_warnings("rust desktop");
}

#[test]
#[ignore = "builds a text project on csharp desktop; run with --ignored, one at a time"]
fn a_csharp_text_project_builds_without_warnings() {
    builds_without_warnings("csharp desktop");
}

#[test]
#[ignore = "builds a text project on go desktop; run with --ignored, one at a time"]
fn a_go_text_project_builds_without_warnings() {
    builds_without_warnings("go desktop");
}

#[test]
#[ignore = "builds a text project on c desktop; run with --ignored, one at a time"]
fn a_c_text_project_builds_without_warnings() {
    builds_without_warnings("c desktop");
}

#[test]
#[ignore = "builds a text project on cpp desktop; run with --ignored, one at a time"]
fn a_cpp_text_project_builds_without_warnings() {
    builds_without_warnings("cpp desktop");
}
