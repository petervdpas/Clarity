//! Which databases of a project Clarity backs up (docs/BACKUP-PLAN.md): the
//! SQLite ones it owns. An external source is someone else's, and a server
//! database is its administrator's until Clarity has a backup of its own
//! for those.

use clarity_project::database_to_back_up;

const PROJECT: &str = "project Shop\n    stack rust desktop\n    datasource main sqlite \"data/shop.db\"\n    datasource erp sqlite \"legacy/erp.db\" external\n    datasource orders postgres \"env:ORDERS_DB\"\n";

fn project(name: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!("clarity-backups-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("shop")).unwrap();
    std::fs::write(root.join("shop.clarity"), PROJECT).unwrap();
    root.join("shop")
}

#[test]
fn the_owned_sqlite_databases_are_backed_up_and_the_others_say_why_not() {
    let dir = project("which");
    let (source, path) = database_to_back_up(&dir, None).unwrap();
    assert_eq!(source, "main", "the first owned source by default");
    assert!(path.ends_with("shop/data/shop.db"), "{}", path.display());
    assert_eq!(database_to_back_up(&dir, Some("main")).unwrap().1, path);
    let external = database_to_back_up(&dir, Some("erp")).unwrap_err();
    assert!(external.contains("external"), "{external}");
    let server = database_to_back_up(&dir, Some("orders")).unwrap_err();
    assert!(server.contains("administrator"), "{server}");
    let none = database_to_back_up(&dir, Some("nothing")).unwrap_err();
    assert!(none.contains("no data source"), "{none}");
    let _ = std::fs::remove_dir_all(dir.parent().unwrap());
}
