//! A command-line program (DESIGN 5.20): the `cli` target and the `cli`
//! block and commands of the modules go together.

use clarity_project::{parse, target_problems};

fn problems(stack: &str, module: &str) -> Vec<String> {
    let settings = parse(&format!("project Shop\n    stack {stack}\n")).unwrap();
    let project = clarity_spec::load_sources([("shop/shop.clarity", module)]);
    target_problems(&settings, &project, "shop.clarity").into_iter().map(|d| d.message).collect()
}

const PROGRAM: &str = "module shop\n\ncli Shop\n    help \"The shop\"\n\ncommand backup\n    help \"Copies it\"\n    print \"done\"\n";

#[test]
fn the_cli_target_and_block_go_together() {
    assert!(problems("go cli", PROGRAM).is_empty());
    assert!(problems("go web desktop cli", PROGRAM).is_empty());
    assert!(problems("go web desktop", "module shop\n").is_empty());
    assert_eq!(
        problems("go cli", "module shop\n"),
        ["the stack has the cli target, so a module needs the program's block: cli Shop"]
    );
    assert_eq!(
        problems("go web desktop", PROGRAM),
        [
            "cli Shop is the command-line program, which needs the cli target: stack go web desktop cli",
            "command backup is for the command-line program, which needs the cli target: stack go web desktop cli",
        ]
    );
    assert_eq!(
        problems("go cli", &PROGRAM.replace("cli Shop", "cli Store")),
        ["the cli block is named after the project: cli Shop"]
    );
}
