//! The README.md this stack writes for a project (P3): what it needs, how to
//! build and run it without Clarity (clarity_gen_core::readme and its own sections).

use clarity_project::{Backend, Settings, Ui, starter};
use clarity_spec::load_converted;
use clarity_gen_go::program as generate;

const SHOP: &str = "module shop

entity Customer
    name: text(80)

page Card: window
    title \"Customer card\"
    data customer: Customer
";

fn readme(settings: &Settings) -> String {
    let project = load_converted([("shop.clarity", SHOP)]);
    let files = generate(&project, settings).unwrap_or_else(|e| panic!("{e:#?}"));
    files.into_iter().find(|f| f.path == "README.md").expect("a README.md").text
}

fn has(text: &str, expected: &str) {
    assert!(text.contains(expected), "expected\n{expected}\nin\n{text}");
}

#[test]
fn a_go_project_says_what_it_needs_and_how_to_build_it() {
    let mut settings = starter("Shop", "shop");
    settings.stack.backend = Backend::Go;
    settings.stack.ui = Ui::Clarity;
    let text = readme(&settings);
    has(&text, "Go with Clarity.UI");
    has(&text, "- Go, from https://go.dev/dl\n");
    assert!(!text.contains("WebKitGTK") && !text.contains("production"), "{text}");
    // Clarity.UI's library comes from the folder CLARITY_UI names, through cgo.
    has(&text, "    export CGO_CFLAGS=\"-I$CLARITY_UI/include\"\n");
    has(&text, "Clarity's build puts the libraries beside the program, and you ship them along with it");
    // cgo downloads nothing: the README says where the release's archive is.
    has(&text, &format!("`clarity-ui-{}-<system>.tar.gz`, from the release https://github.com/petervdpas/Clarity/releases/tag/runtimes/rust-ui-ffi/v{}", clarity_gen_core::clarity_ui::version(), clarity_gen_core::clarity_ui::version()));
    has(&text, "    go build -trimpath -ldflags=\"-s -w\" -o bin/shop .\n");
    has(&text, "    go run .\n");
    // The runtime comes as a Go module, or from Clarity's repository while working on it.
    has(&text, &format!("Clarity's runtime comes as the Go module github.com/petervdpas/Clarity/runtimes/go ({})", clarity_gen_go::runtime_version()));
    has(&text, "    go work init .\n    go work edit -replace github.com/petervdpas/Clarity/runtimes/go=<Clarity>/runtimes/go\n");
    // The web program serves Clarity.UI's browser files, and needs no native library.
    settings.stack.targets = vec![clarity_project::Target::Web];
    let web = readme(&settings);
    has(&web, "Go with Clarity.UI");
    has(&web, "Clarity.UI's browser files (`clarity-ui.js`, `clarity-ui.wasm`, `clarity-ui-worker.js`, `clarity-ui-page.js`)");
    has(&web, &format!("the archive `clarity-ui-{}-browser.tar.gz`", clarity_gen_core::clarity_ui::version()));
    assert!(!web.contains("CGO_CFLAGS") && !web.contains("--serve"), "{web}");
    settings.stack.targets = vec![clarity_project::Target::Desktop, clarity_project::Target::Web];
    has(&readme(&settings), "Started with `--serve`, the program serves instead of opening its window.");
}
