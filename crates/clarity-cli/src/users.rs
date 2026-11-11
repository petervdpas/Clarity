//! `clarity users`: the people of a project that signs people in
//! (docs/APP-CAPABILITIES-PLAN.md step 4), changed from the command line:
//! listed, added, given a password, a token or recovery codes, removed. The
//! secrets are kept as the programs keep them, through Clarity.Auth, with the
//! project's `minimum`. For the people of a SQLite database the project keeps;
//! a server database's people are changed in the program itself.

use std::path::Path;

use clarity_spec::signin::Signin;

/// What `clarity users` does.
pub enum Action {
    List,
    Add(String),
    Password(String),
    Token(String),
    Codes(String),
    Remove(String),
}

/// How many recovery codes `codes` makes.
const CODES: usize = 10;

/// Where the people are: the database, the table and its columns.
struct People {
    database: std::path::PathBuf,
    entity: String,
    table: String,
    key: String,
    /// The column people are found by (the first strategy that matches one).
    found_by: (String, String),
    signin: Signin,
    columns: std::collections::BTreeMap<String, String>,
}

impl People {
    /// The column of the secret field `strategy` names, and that field.
    fn secret_of(&self, strategy: &str) -> Result<(String, String), String> {
        let line = self.signin.strategy(strategy).ok_or_else(|| format!("the project's people do not sign in with {strategy}: add a '{strategy}' line under signin"))?;
        let info = clarity_spec::signin::strategy_info(strategy).ok_or_else(|| format!("there is no sign-in strategy '{strategy}'"))?;
        let field = info.fields.iter().zip(&line.fields).find(|(role, _)| role.role == "secret").map(|(_, f)| f.clone()).ok_or_else(|| format!("the {strategy} strategy keeps no secret"))?;
        Ok((self.columns[&field].clone(), field))
    }
}

fn people(file: &Path) -> Result<People, String> {
    let dir = clarity_project::module_dir(file);
    let settings = clarity_project::load(file).settings.ok_or_else(|| format!("{} cannot be read as a project file", file.display()))?;
    let signin = settings.signin.clone().ok_or_else(|| format!("{} signs nobody in: there is no signin block in it", file.display()))?;
    let project = clarity_project::load_modules(&dir).map_err(|e| format!("the project's modules cannot be read: {e}"))?;
    let entity = project.entity(&signin.entity).ok_or_else(|| format!("no module declares {}, the entity of people", signin.entity))?;
    let source_name = project.source_of(&entity.name.name).unwrap_or(project.default_source()).to_string();
    let source = settings.datasources.iter().find(|s| s.name == source_name).ok_or_else(|| format!("the project file has no data source {source_name}"))?;
    if source.kind != clarity_project::SourceKind::Sqlite {
        return Err(format!("the people live in {source_name}, a {} database: change them in the program itself", clarity_project::source_word(source.kind)));
    }
    let found_by = signin
        .strategies
        .iter()
        .find_map(|line| {
            let info = clarity_spec::signin::strategy_info(&line.name)?;
            info.fields.iter().zip(&line.fields).find(|(role, _)| role.role == "match").map(|(_, f)| f.clone())
        })
        .ok_or_else(|| "no strategy of the signin block finds people by a field (password, recovery or account)".to_string())?;
    let columns = entity.fields.iter().map(|f| (f.name.name.clone(), entity.column_of(&f.name.name))).collect();
    Ok(People {
        database: clarity_project::sqlite_path(&dir, source),
        entity: entity.name.name.clone(),
        table: entity.table_name(),
        key: entity.key_column().to_string(),
        found_by: (entity.column_of(&found_by), found_by),
        signin,
        columns,
    })
}

/// Runs `action` on the project `file`; `password` gives a typed password.
/// What is shown once (a token, codes) goes to `out`; what happened to `said`.
pub fn run(file: &Path, action: Action, password: &mut dyn FnMut() -> Option<String>, out: &mut dyn std::io::Write, said: &mut dyn std::io::Write) -> Result<(), String> {
    let people = people(file)?;
    if !people.database.is_file() {
        return Err(format!("{} is not there yet: clarity data migrate makes it", people.database.display()));
    }
    let db = rusqlite::Connection::open(&people.database).map_err(|e| format!("{} cannot be opened: {e}", people.database.display()))?;
    let (by_column, by_field) = &people.found_by;
    let id_of = |name: &str| -> Result<i64, String> {
        db.query_row(&format!("SELECT \"{}\" FROM \"{}\" WHERE \"{by_column}\" = ?1", people.key, people.table), [name], |r| r.get(0))
            .map_err(|_| format!("there is no {} with {by_field} {name}", people.entity))
    };
    let set = |id: i64, column: &str, value: &str| -> Result<(), String> {
        db.execute(&format!("UPDATE \"{}\" SET \"{column}\" = ?1 WHERE \"{}\" = ?2", people.table, people.key), rusqlite::params![value, id]).map(|_| ()).map_err(|e| e.to_string())
    };
    let new_password = |password: &mut dyn FnMut() -> Option<String>| -> Result<String, String> {
        let (_, field) = people.secret_of("password")?;
        let typed = password().ok_or("no password was typed")?;
        if let Some(minimum) = people.signin.minimum_of(&field)
            && !clarity_auth::secret_fits(&typed, minimum)
        {
            return Err(format!("the password needs at least {minimum} characters"));
        }
        clarity_auth::hash(people.signin.hash_of(&field), &typed).map_err(|e| e.text)
    };
    let write = |w: &mut dyn std::io::Write, text: &str| writeln!(w, "{text}").map_err(|e| e.to_string());
    match action {
        Action::List => {
            let mut query = db.prepare(&format!("SELECT \"{}\", \"{by_column}\" FROM \"{}\" ORDER BY \"{}\"", people.key, people.table, people.key)).map_err(|e| e.to_string())?;
            let rows = query.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, Option<String>>(1)?))).map_err(|e| e.to_string())?;
            for row in rows {
                let (id, name) = row.map_err(|e| e.to_string())?;
                write(out, &format!("{id}\t{}", name.unwrap_or_default()))?;
            }
        }
        Action::Add(name) => {
            if id_of(&name).is_ok() {
                return Err(format!("there is a {} with {by_field} {name} already", people.entity));
            }
            // With a password strategy the person gets one at once; the other fields take their defaults.
            let hashed = if people.signin.strategy("password").is_some() { Some(new_password(password)?) } else { None };
            db.execute(&format!("INSERT INTO \"{}\" (\"{by_column}\") VALUES (?1)", people.table), [&name]).map_err(|e| format!("{name} cannot be added: {e}"))?;
            let id = id_of(&name)?;
            if let Some(hashed) = hashed {
                set(id, &people.secret_of("password")?.0, &hashed)?;
            }
            write(said, &format!("Added {name} to {}.", people.entity))?;
        }
        Action::Password(name) => {
            let id = id_of(&name)?;
            let hashed = new_password(password)?;
            set(id, &people.secret_of("password")?.0, &hashed)?;
            write(said, &format!("{name} has a new password."))?;
        }
        Action::Token(name) => {
            let id = id_of(&name)?;
            let (column, field) = people.secret_of("token")?;
            let token = clarity_auth::new_token().map_err(|e| e.text)?;
            set(id, &column, &clarity_auth::hash(people.signin.hash_of(&field), &token).map_err(|e| e.text)?)?;
            write(out, &token)?;
            write(said, &format!("{name} has a new token, shown once above."))?;
        }
        Action::Codes(name) => {
            let id = id_of(&name)?;
            let (column, field) = people.secret_of("recovery")?;
            let codes = clarity_auth::new_recovery_codes(CODES).map_err(|e| e.text)?;
            set(id, &column, &clarity_auth::hash(people.signin.hash_of(&field), &codes).map_err(|e| e.text)?)?;
            write(out, &codes)?;
            write(said, &format!("{name} has {CODES} new recovery codes, shown once above; each signs in once."))?;
        }
        Action::Remove(name) => {
            let id = id_of(&name)?;
            db.execute(&format!("DELETE FROM \"{}\" WHERE \"{}\" = ?1", people.table, people.key), [id]).map_err(|e| format!("{name} cannot be removed: {e}"))?;
            write(said, &format!("Removed {name}."))?;
        }
    }
    Ok(())
}
