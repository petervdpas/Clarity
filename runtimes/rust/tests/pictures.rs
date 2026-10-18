//! Pictures in records (X2), against the cases every runtime shares
//! (conformance/pictures.json), and kept in SQLite or in a file store.

use clarity_runtime::picture::{self, Picture, PictureColumn};
use clarity_runtime::{Db, Dialect, Error, Id, Record, SqlValue, sql};
use serde_json::Value;

const CASES: &str = include_str!("../../../conformance/pictures.json");

fn cases() -> Value {
    serde_json::from_str(CASES).expect("pictures.json is JSON")
}

fn hex(text: &str) -> Vec<u8> {
    (0..text.len()).step_by(2).map(|i| u8::from_str_radix(&text[i..i + 2], 16).unwrap()).collect()
}

#[test]
fn a_picture_is_told_by_its_first_bytes() {
    let cases = cases();
    for kind in cases["kinds"].as_array().unwrap() {
        let found = picture::kind_of(&hex(kind["bytes"].as_str().unwrap())).expect("a picture");
        assert_eq!(found.extension, kind["extension"].as_str().unwrap());
        assert_eq!(found.media_type, kind["type"].as_str().unwrap());
    }
    for other in cases["notPictures"].as_array().unwrap() {
        assert!(picture::kind_of(&hex(other.as_str().unwrap())).is_none(), "{other}");
    }
}

#[test]
fn a_file_is_named_by_its_bytes() {
    for case in cases()["fileNames"].as_array().unwrap() {
        assert_eq!(picture::file_name(&hex(case["bytes"].as_str().unwrap())).unwrap(), case["name"].as_str().unwrap());
    }
    assert!(matches!(picture::file_name(b"%PDF"), Err(Error::Invalid(_))));
}

#[test]
fn a_picture_travels_to_the_renderer_as_every_runtime_sends_it() {
    for case in cases()["wire"].as_array().unwrap() {
        let value: Option<Picture> = match case["value"].as_str().unwrap() {
            "none" => None,
            "stored" => Some(Picture::Stored),
            _ => Some(Picture::New(hex(case["bytes"].as_str().unwrap()))),
        };
        assert_eq!(serde_json::to_value(&value).unwrap(), case["json"]);
        assert_eq!(serde_json::from_value::<Option<Picture>>(case["json"].clone()).unwrap(), value);
    }
}

#[test]
fn reading_records_holds_whether_there_is_a_picture() {
    let cases = cases();
    for (word, dialect) in [("sqlite", Dialect::Sqlite), ("postgres", Dialect::Postgres), ("mysql", Dialect::Mysql), ("mssql", Dialect::Mssql)] {
        assert_eq!(picture::presence(dialect, "cover"), cases["presence"][word].as_str().unwrap());
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
struct Album {
    id: Id,
    title: String,
    cover: Option<Picture>,
    photo: Option<Picture>,
}

impl Record for Album {
    const TABLE: &'static str = "album";
    const COLUMNS: &'static [&'static str] = &["title", "cover", "photo"];
    const PICTURES: &'static [PictureColumn] = &[PictureColumn { column: "cover", store: None }, PictureColumn { column: "photo", store: Some("photos") }];

    fn id(&self) -> Id {
        self.id
    }

    fn set_id(&mut self, id: Id) {
        self.id = id;
    }

    fn to_values(&self) -> Vec<SqlValue> {
        vec![sql::text(&self.title), sql::picture(&self.cover), sql::picture(&self.photo)]
    }

    fn from_values(id: Id, v: &[SqlValue]) -> clarity_runtime::Result<Self> {
        Ok(Album {
            id,
            title: sql::get_text("title", &v[0])?,
            cover: sql::get_picture("cover", &v[1])?,
            photo: sql::get_picture("photo", &v[2])?,
        })
    }
}

const PNG: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
const JPG: [u8; 4] = [0xFF, 0xD8, 0xFF, 0xE0];

fn albums(store: &std::path::Path) -> Db {
    let mut db = Db::memory().unwrap();
    db.migrate(&[&["CREATE TABLE album (id INTEGER PRIMARY KEY, title TEXT NOT NULL, cover BLOB, photo TEXT)"]]).unwrap();
    db.add_store("photos", &store.display().to_string()).unwrap();
    db
}

fn scratch(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("clarity-pictures-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

#[test]
fn a_picture_is_kept_in_the_database_and_fetched_on_its_own() {
    let store = scratch("db");
    let db = albums(&store);
    let mut album = Album { title: "Blue Train".into(), cover: Some(Picture::New(PNG.to_vec())), ..Album::default() };
    db.save(&mut album).unwrap();
    // Reading the record says there is a picture; the picture comes on its own.
    let read: Album = db.get(album.id).unwrap();
    assert_eq!(read.cover, Some(Picture::Stored));
    assert_eq!(read.photo, None);
    assert_eq!(db.picture::<Album>(album.id, "cover").unwrap(), Some(PNG.to_vec()));

    // Saving the record again keeps the picture it has.
    let mut again = read.clone();
    again.title = "Blue Train (mono)".into();
    db.save(&mut again).unwrap();
    assert_eq!(db.picture::<Album>(album.id, "cover").unwrap(), Some(PNG.to_vec()));

    // A new picture replaces it; none empties it.
    again.cover = Some(Picture::New(JPG.to_vec()));
    db.save(&mut again).unwrap();
    assert_eq!(db.picture::<Album>(album.id, "cover").unwrap(), Some(JPG.to_vec()));
    again.cover = None;
    db.save(&mut again).unwrap();
    assert_eq!(db.picture::<Album>(album.id, "cover").unwrap(), None);
    assert_eq!(db.get::<Album>(album.id).unwrap().cover, None);

    // Only a picture is kept.
    again.cover = Some(Picture::New(b"%PDF-1.7".to_vec()));
    assert!(matches!(db.save(&mut again), Err(Error::Invalid(_))));
}

#[test]
fn a_picture_in_a_file_store_is_a_file_named_by_its_bytes() {
    let store = scratch("store");
    let db = albums(&store);
    let mut album = Album { title: "Kind of Blue".into(), photo: Some(Picture::New(JPG.to_vec())), ..Album::default() };
    db.save(&mut album).unwrap();
    let name = picture::file_name(&JPG).unwrap();
    assert_eq!(std::fs::read(store.join(&name)).unwrap(), JPG);
    assert_eq!(db.get::<Album>(album.id).unwrap().photo, Some(Picture::Stored));
    assert_eq!(db.picture::<Album>(album.id, "photo").unwrap(), Some(JPG.to_vec()));

    // A path that leaves the store is refused.
    db.batch("", &format!("UPDATE album SET photo = '../secret.png' WHERE id = {}", album.id)).unwrap();
    assert!(matches!(db.picture::<Album>(album.id, "photo"), Err(Error::Invalid(_))));
    std::fs::remove_dir_all(&store).unwrap();
}

#[test]
fn a_store_comes_from_its_place_or_the_environment() {
    let mut db = Db::memory().unwrap();
    assert!(db.add_store("photos", "env:CLARITY_NO_SUCH_STORE").is_err());
    let store = scratch("env");
    db.add_store("photos", &store.display().to_string()).unwrap();
    assert!(store.is_dir(), "the folder is made");
    std::fs::remove_dir_all(&store).unwrap();
}

#[test]
fn a_picture_shows_as_a_data_url() {
    assert_eq!(picture::data_url(&PNG).unwrap(), "data:image/png;base64,iVBORw0KGgo=");
}
