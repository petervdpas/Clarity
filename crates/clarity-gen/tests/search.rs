//! Full-text search (U1h) in the database: the migration steps build each
//! database's own index of the fields marked `search`, keep it while fields
//! and tables are renamed or removed, and `a matches x` finds the records
//! holding every word of `x`, each as the start of a word. SQLite runs
//! always; the servers run when their scratch database is named, as in
//! tests/servers.rs.

use clarity_project::SourceKind;
use clarity_runtime::{Cond, Db, Dialect, Id, Query, Record, SqlValue, sql};

const MUSIC: &str = "module music

entity Recording
    title: text(80), required, search
    remarks: text, search
    tags: list<text>, search
";

const FIRST: &str = "migration \"Albums\"
    add entity Album
        title: text(80), search
        notes: text
        tags: list<text>
        year: integer
";

const SECOND: &str = "migration \"Search more\"
    add search Album.notes
    add search Album.tags
    rename field Album.notes to remarks
    remove field Album.year
    rename entity Album to Recording
";

fn steps(kind: SourceKind) -> Vec<Vec<String>> {
    let project = clarity_spec::load_with_migrations([("music.clarity", MUSIC)], [("0001.clarity", FIRST), ("0002.clarity", SECOND)]);
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    clarity_gen::migration_sql(&project, "main", kind).unwrap().into_iter().map(|(_, s)| s).collect()
}

#[derive(Debug, Clone, Default)]
struct Album {
    id: Id,
    title: String,
    notes: String,
    tags: String,
    year: i64,
}

impl Record for Album {
    const TABLE: &'static str = "album";
    const COLUMNS: &'static [&'static str] = &["title", "notes", "tags", "year"];
    fn id(&self) -> Id {
        self.id
    }
    fn set_id(&mut self, id: Id) {
        self.id = id;
    }
    fn to_values(&self) -> Vec<SqlValue> {
        vec![sql::text(&self.title), sql::text(&self.notes), sql::text(&self.tags), SqlValue::Integer(self.year)]
    }
    fn from_values(id: Id, v: &[SqlValue]) -> clarity_runtime::Result<Self> {
        Ok(Album {
            id,
            title: sql::get_text("title", &v[0])?,
            notes: sql::get_text("notes", &v[1])?,
            tags: sql::get_text("tags", &v[2])?,
            year: sql::get_integer("year", &v[3])?,
        })
    }
}

#[derive(Debug, Clone, Default)]
struct Recording {
    id: Id,
    title: String,
    remarks: String,
    tags: String,
}

impl Record for Recording {
    const TABLE: &'static str = "recording";
    const COLUMNS: &'static [&'static str] = &["title", "remarks", "tags"];
    fn id(&self) -> Id {
        self.id
    }
    fn set_id(&mut self, id: Id) {
        self.id = id;
    }
    fn to_values(&self) -> Vec<SqlValue> {
        vec![sql::text(&self.title), sql::text(&self.remarks), sql::text(&self.tags)]
    }
    fn from_values(id: Id, v: &[SqlValue]) -> clarity_runtime::Result<Self> {
        Ok(Recording { id, title: sql::get_text("title", &v[0])?, remarks: sql::get_text("remarks", &v[1])?, tags: sql::get_text("tags", &v[2])? })
    }
}

/// The titles of the recordings that match `text`, in title order.
fn found(db: &Db, text: &str) -> Vec<String> {
    let query = Query::new().filter(Cond::matches("recording", "id", &["title", "remarks", "tags"], text)).order("title", false);
    db.find::<Recording>(&query).unwrap().into_iter().map(|r| r.title).collect()
}

fn searches(db: &mut Db, dialect: Dialect, kind: SourceKind) {
    for table in ["recording", "album", "clarity_schema"] {
        db.batch("main", &format!("DROP TABLE IF EXISTS {}", dialect.quote(table))).unwrap();
    }
    if dialect == Dialect::Sqlite {
        for index in ["recording__search", "album__search"] {
            db.batch("main", &format!("DROP TABLE IF EXISTS {}", dialect.quote(index))).unwrap();
        }
    }
    let steps = steps(kind);
    let steps: Vec<Vec<&str>> = steps.iter().map(|s| s.iter().map(String::as_str).collect()).collect();
    let steps: Vec<&[&str]> = steps.iter().map(Vec::as_slice).collect();
    assert_eq!(db.migrate(&steps[..1]).unwrap(), 1);
    for (title, notes, tags) in [
        ("Kind of Blue", "Miles Davis, recorded in 1959", r#"["jazz","modal"]"#),
        ("Blue Train", "John Coltrane on Blue Note", r#"["jazz","hard bop"]"#),
        ("The Joshua Tree", "U2 in the desert", r#"["rock"]"#),
        ("Café Tacvba", "Re", r#"["rock en español"]"#),
    ] {
        db.save(&mut Album { title: title.into(), notes: notes.into(), tags: tags.into(), year: 1959, ..Album::default() }).unwrap();
    }
    assert_eq!(db.migrate(&steps).unwrap(), 1);

    // Every word, each as the start of a word, in any searched field.
    assert_eq!(found(db, "blue"), ["Blue Train", "Kind of Blue"]);
    assert_eq!(found(db, "BLUE  note"), ["Blue Train"]);
    assert_eq!(found(db, "jaz mod"), ["Kind of Blue"]);
    assert_eq!(found(db, "hard-bop"), ["Blue Train"]);
    assert_eq!(found(db, "u2"), ["The Joshua Tree"]);
    // Common words count too (MySQL leaves its stopwords out otherwise).
    assert_eq!(found(db, "the desert"), ["The Joshua Tree"]);
    assert_eq!(found(db, "español"), ["Café Tacvba"]);
    assert_eq!(found(db, "lue"), Vec::<String>::new());
    assert_eq!(found(db, "blue rock"), Vec::<String>::new());
    assert_eq!(found(db, " - ").len(), 4);

    // The index follows new, changed and removed records.
    let mut new = Recording { title: "A Love Supreme".into(), remarks: "Coltrane again".into(), tags: r#"["spiritual"]"#.into(), ..Recording::default() };
    db.save(&mut new).unwrap();
    assert_eq!(found(db, "coltrane"), ["A Love Supreme", "Blue Train"]);
    new.remarks = "Impulse!".into();
    db.save(&mut new).unwrap();
    assert_eq!(found(db, "coltrane"), ["Blue Train"]);
    assert_eq!(found(db, "impul"), ["A Love Supreme"]);
    db.delete(&new).unwrap();
    assert_eq!(found(db, "impul"), Vec::<String>::new());
}

#[test]
fn sqlite_finds_by_its_full_text_index() {
    let path = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("search-{}.db", std::process::id()));
    let _ = std::fs::remove_file(&path);
    let mut db = Db::connect(&[("main", Dialect::Sqlite, path.to_str().unwrap())]).unwrap();
    searches(&mut db, Dialect::Sqlite, SourceKind::Sqlite);
    drop(db);
    std::fs::remove_file(&path).unwrap();
}

fn on_server(variable: &str, dialect: Dialect, kind: SourceKind) {
    let Ok(at) = std::env::var(variable) else {
        eprintln!("{variable} is not set; skipped");
        return;
    };
    let mut db = Db::connect(&[("main", dialect, &at)]).unwrap();
    searches(&mut db, dialect, kind);
}

#[test]
fn postgres_finds_by_its_full_text_index() {
    on_server("CLARITY_TEST_POSTGRES", Dialect::Postgres, SourceKind::Postgres);
}

#[test]
fn mysql_finds_by_its_full_text_index() {
    on_server("CLARITY_TEST_MYSQL", Dialect::Mysql, SourceKind::Mysql);
}

#[test]
fn mssql_finds_by_its_words() {
    on_server("CLARITY_TEST_MSSQL", Dialect::Mssql, SourceKind::Mssql);
}

#[test]
fn each_database_gets_its_own_index() {
    let sqlite = steps(SourceKind::Sqlite).concat().join("\n");
    assert!(
        sqlite.contains(
            "CREATE VIRTUAL TABLE \"album__search\" USING fts5(\"title\", content='album', content_rowid='id', tokenize='unicode61 remove_diacritics 0');"
        ),
        "{sqlite}"
    );
    assert!(sqlite.contains("INSERT INTO \"recording__search\"(\"recording__search\") VALUES ('rebuild');"), "{sqlite}");
    let postgres = steps(SourceKind::Postgres).concat().join("\n");
    assert!(
        postgres.contains(
            "CREATE INDEX \"recording__search\" ON \"recording\" USING GIN (to_tsvector('simple', coalesce(\"title\", '') || ' ' || coalesce(\"remarks\", '') || ' ' || coalesce(\"tags\", '')));"
        ),
        "{postgres}"
    );
    let mysql = steps(SourceKind::Mysql).concat().join("\n");
    assert!(mysql.contains("CREATE FULLTEXT INDEX `recording__search` ON `recording` (`title`, `remarks`, `tags`);"), "{mysql}");
    let mssql = steps(SourceKind::Mssql).concat().join("\n");
    assert!(!mssql.contains("search"), "{mssql}");
}

/// The build tests' fixture, with search on an album's title and tags.
const FIXTURE: &str = include_str!("fixtures/asks.clarity");

fn generated(backend: clarity_project::Backend, ui: clarity_project::Ui) -> String {
    let project = clarity_spec::keyed::load_keyed([("music.clarity", FIXTURE)], &[], "en");
    let errors: Vec<&str> = project.diagnostics.iter().filter(|d| d.severity == clarity_spec::Severity::Error).map(|d| d.message.as_str()).collect();
    assert!(errors.is_empty(), "{errors:#?}");
    let mut settings = clarity_project::starter("Music", "music");
    settings.stack.backend = backend;
    settings.stack.ui = ui;
    let files = clarity_gen::generate(&project, &settings).unwrap_or_else(|e| panic!("{e:#?}"));
    files.iter().map(|f| format!("// {}\n{}\n", f.path, f.text)).collect()
}

#[test]
fn every_stack_finds_by_words() {
    use clarity_project::{Backend, Ui};
    for (backend, ui, wanted) in [
        (Backend::Rust, Ui::Web, r#"Cond::matches("album", "id", &["title", "tags"], &"#),
        (Backend::Go, Ui::Web, r#"clarity.Matches("album", "id", []string{"title", "tags"}, "#),
        (Backend::C, Ui::Web, r#"clarity_matches("album", "id", 2, (const char*[]){"title", "tags"}, "#),
        (Backend::Cpp, Ui::Web, r#"clarity::Cond::matches("album", "id", {"title", "tags"}, "#),
        (Backend::Csharp, Ui::Avalonia, r#"Cond.Matches("album", "id", ["title", "tags"], "#),
    ] {
        let text = generated(backend, ui);
        assert!(text.contains(wanted), "{backend:?}: missing {wanted}");
    }
}
