//! Which databases of a project Clarity backs up, and where (docs/ledger/backups.md):
//! the SQLite ones it owns, each in a folder of its data source's name inside
//! the folder the project's `backups` line names. A project without the line
//! takes no backups. An external source is someone else's, and a server
//! database is its administrator's until Clarity has a backup of its own
//! for those. The IDE, the agents and the clarity command all come here.

use clarity_project::backups;

const SOURCES: &str = "    datasource main sqlite \"data/shop.db\"\n    datasource erp sqlite \"legacy/erp.db\" external\n    datasource orders postgres \"env:ORDERS_DB\"\n";

fn project(name: &str, backups_line: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!("clarity-backups-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("shop/data")).unwrap();
    std::fs::write(root.join("shop.clarity"), format!("project Shop\n    stack rust desktop\n{backups_line}{SOURCES}")).unwrap();
    root.join("shop")
}

#[test]
fn the_owned_sqlite_databases_are_backed_up_in_the_projects_folder_and_the_others_say_why_not() {
    let dir = project("which", "    backups \"data/backups\"\n");
    let found = backups::backed_up(&dir, None).unwrap();
    assert_eq!(found.source, "main", "the first owned source by default");
    assert!(found.database.ends_with("shop/data/shop.db"), "{}", found.database.display());
    assert_eq!(found.folder, std::path::absolute(dir.join("data/backups/main")).unwrap(), "a folder of the source's name in the project's backups folder");
    assert_eq!(backups::backed_up(&dir, Some("main")).unwrap(), found);
    let external = backups::backed_up(&dir, Some("erp")).unwrap_err();
    assert!(external.contains("external"), "{external}");
    let server = backups::backed_up(&dir, Some("orders")).unwrap_err();
    assert!(server.contains("administrator"), "{server}");
    let none = backups::backed_up(&dir, Some("nothing")).unwrap_err();
    assert!(none.contains("no data source"), "{none}");
    let _ = std::fs::remove_dir_all(dir.parent().unwrap());
}

#[test]
fn a_project_without_a_backups_folder_takes_no_backups_and_says_where_to_set_it() {
    let dir = project("off", "");
    // An empty file is an SQLite database without tables.
    std::fs::write(dir.join("data/shop.db"), "").unwrap();
    let said = backups::backed_up(&dir, None).unwrap_err();
    assert!(said.contains("backups \"data/backups\"") && said.contains("Project settings"), "{said}");
    assert_eq!(backups::take(&dir, None).unwrap_err(), said);
    assert_eq!(backups::list(&dir, None).unwrap_err(), said);
    assert_eq!(backups::restore(&dir, None, "shop-20261105T000000Z-manual.db").unwrap_err(), said);
    assert_eq!(backups::delete(&dir, None, "shop-20261105T000000Z-manual.db").unwrap_err(), said);
    assert!(!dir.join("data/backups").exists(), "nothing was written");
    // A server database still says why Clarity does not back it up.
    assert!(backups::backed_up(&dir, Some("orders")).unwrap_err().contains("administrator"));
    let _ = std::fs::remove_dir_all(dir.parent().unwrap());
}

#[test]
fn a_backup_is_taken_listed_restored_and_deleted_by_its_file_name() {
    let dir = project("all", "    backups \"data/backups\"\n");
    std::fs::write(dir.join("data/shop.db"), "").unwrap();
    let (found, taken) = backups::take(&dir, Some("main")).unwrap();
    assert_eq!(taken.path.parent().unwrap(), found.folder);
    let (_, listed) = backups::list(&dir, None).unwrap();
    assert_eq!(listed, vec![taken.clone()]);
    let path = taken.path.display().to_string();
    let refused = backups::restore(&dir, None, &path).unwrap_err();
    assert!(refused.contains("not a path"), "{refused}");
    assert!(backups::restore(&dir, None, "../shop.db").is_err(), "a file name, never a path");
    let (_, before) = backups::restore(&dir, None, &taken.file).unwrap();
    assert_eq!(before.why, "before-restore", "what it replaced is kept");
    backups::delete(&dir, None, &taken.file).unwrap();
    let (_, left) = backups::list(&dir, None).unwrap();
    assert_eq!(left.iter().map(|b| b.why.as_str()).collect::<Vec<_>>(), ["before-restore"]);
    let _ = std::fs::remove_dir_all(dir.parent().unwrap());
}
