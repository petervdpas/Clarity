//! A small project that calls every picture function (docs/IMAGE-PLAN.md),
//! built on each desktop stack as the IDE builds it, without a warning. Each
//! is a whole build, so they are ignored: run one at a time,
//! `cargo test -p clarity-gen --test picture_functions_build -- --ignored --exact <name>`.
//! Go, C and C++ want Clarity.Image's library beside Clarity.UI's:
//! `cargo build --release -p clarity-ui-ffi -p clarity-image-ffi`.

mod ide_build;

const MODULE: &str = "module studio

entity Book
    title: text, label @bookTitle
    cover: image, label @bookCover

page Main: window
    title @mainTitle
    data book: Book
    data wide: integer
    button @makeCaption name makeButton at 1,1 span 2 on click: makeClicked

code Main
    handler makeClicked
        let small = resize(book.cover, 200, 300)
        let part = crop(small, 10, 10, 100, 100)
        let shot = rotate(part, 90)
        shot = rotate(shot, 12.5)
        shot = flip(shot, true)
        shot = blend(book.cover, shot, 20, 30, 80)
        shot = adjust(shot, 30, -20, 10, 0)
        wide = pictureWidth(shot) + pictureHeight(book.cover)
        book.cover = shot
";

fn builds_without_warnings(stack: &str) {
    let name = format!("pictures-{}", stack.split(' ').next().unwrap());
    let root = ide_build::repo().join("target/generated").join(&name);
    let _ = std::fs::remove_dir_all(&root);
    let dir = root.join("studio");
    std::fs::create_dir_all(dir.join("resources")).unwrap();
    std::fs::create_dir_all(dir.join("translations")).unwrap();
    std::fs::create_dir_all(dir.join("migrations/main")).unwrap();
    std::fs::write(dir.join("migrations/main/0001.clarity"), "migration \"Books\"\n    add entity Book\n        title: text\n        cover: image\n").unwrap();
    std::fs::write(dir.join("studio.clarity"), MODULE).unwrap();
    std::fs::write(dir.join("translations/en.clarity"), "translation en\n    mainTitle \"Studio\"\n    makeCaption \"Make\"\n    bookTitle \"Title\"\n    bookCover \"Cover\"\n").unwrap();
    let file = root.join("studio.clarity");
    std::fs::write(&file, format!("project Studio\n    title \"Studio\"\n    stack {stack}\n    output \"out\"\n    language \"en\"\n    datasource main sqlite \"data/studio.db\"\n")).unwrap();
    let settings = clarity_project::load(&file).settings.expect("the project file reads");
    let project = clarity_project::load_modules(&clarity_project::module_dir(&file)).unwrap();
    assert!(!project.has_errors(), "{:?}", project.diagnostics);
    ide_build::builds_without_warnings(&file, &project, &settings);
}

#[test]
#[ignore = "builds a picture project on rust desktop; run with --ignored, one at a time"]
fn a_rust_picture_project_builds_without_warnings() {
    builds_without_warnings("rust desktop");
}

#[test]
#[ignore = "builds a picture project on csharp desktop; run with --ignored, one at a time"]
fn a_csharp_picture_project_builds_without_warnings() {
    builds_without_warnings("csharp desktop");
}

#[test]
#[ignore = "builds a picture project on go desktop; run with --ignored, one at a time"]
fn a_go_picture_project_builds_without_warnings() {
    builds_without_warnings("go desktop");
}

#[test]
#[ignore = "builds a picture project on c desktop; run with --ignored, one at a time"]
fn a_c_picture_project_builds_without_warnings() {
    builds_without_warnings("c desktop");
}

#[test]
#[ignore = "builds a picture project on cpp desktop; run with --ignored, one at a time"]
fn a_cpp_picture_project_builds_without_warnings() {
    builds_without_warnings("cpp desktop");
}
