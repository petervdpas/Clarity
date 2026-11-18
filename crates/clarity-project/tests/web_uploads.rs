//! A web app keeps a file a person picks for a file field in a file store on
//! the server, so a file part bound to one names its store there; a desktop
//! app picks on its own disk (docs/ledger/files-pictures.md, step 5).

use clarity_project::{parse, target_problems};

const MOVIES: &str = "module movies

entity Movie
    video: file

page Main: window
    data movie: Movie
    file name videoBox at 1,1 span 6 bind movie.video
";

fn problems(stack: &str, source: &str) -> Vec<String> {
    let settings = parse(&format!("project Movies\n    stack {stack}\n")).unwrap();
    let project = clarity_spec::load_converted([("movies.clarity", source)]);
    target_problems(&settings, &project, "movies.clarity").into_iter().map(|d| d.message).filter(|m| m.contains("file store")).collect()
}

#[test]
fn a_web_app_s_file_part_names_its_store() {
    assert_eq!(problems("go web", MOVIES), ["videoBox is a file part in a web app, which keeps a picked file in a file store on the server: name one, as in store uploads"]);
    assert!(problems("go desktop", MOVIES).is_empty(), "a desktop app picks on its disk");
    assert!(problems("go desktop web", &MOVIES.replace("bind movie.video", "bind movie.video store uploads")).is_empty());
}
