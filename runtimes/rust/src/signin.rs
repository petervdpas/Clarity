//! Who is using the program (docs/APP-CAPABILITIES-PLAN.md, step 4). The
//! generated program names its entity of people and its strategies once,
//! after its databases open (`Db::add_signin`); `Db::signin` then finds the
//! person by what Clarity.Auth says, checks them and makes them the current
//! person. On the desktop that is one person for the program; served, each
//! call is the person of its session cookie (`served`), so `me` is read per
//! call (`Db::me_id`). Refusals are Clarity's own words, by key
//! (conformance/signin.json).
//!
//! The first account: when the table of people has no rows and the strategy
//! can make one (its catalog entry says `firstAccount`), the record is
//! inserted with only the fields the strategy names (the match field, the
//! secret as its hash); every other column takes its default from the
//! table, so a required field without a default makes the insert fail.

use std::cell::RefCell;
use std::sync::Mutex;

use serde_json::{Map, Value, json};

use crate::ui::Text;
use crate::{Db, Error, Id, Record, Result, SqlValue, sql};

/// The name of the session cookie a served program sets.
pub const COOKIE: &str = "clarity_session";

/// The name of the remembered desktop sign-in among the program's secrets.
pub const REMEMBERED: &str = "clarityRemembered";

/// One strategy the project uses: its name in Clarity.Auth's catalog and
/// the columns of the table of people it names, in the catalog's order of
/// roles (`password email passwordHash` is `["email", "password_hash"]`).
#[derive(Debug, Clone, PartialEq)]
pub struct SigninStrategy {
    pub name: String,
    pub columns: Vec<String>,
    /// A provider's settings for Clarity.Auth (`client`, `secret` as
    /// `setting:<name>`, `tenant`, `issuer`, `register`, `claim`); empty for
    /// a strategy that is no provider.
    pub options: Vec<(String, String)>,
}

/// The project's `signin` block as the program uses it: where its people
/// are kept, how they sign in, how long a sign-in lasts and how many
/// failed tries are allowed.
#[derive(Debug, Clone, PartialEq)]
pub struct SigninSource {
    /// The data source of the entity of people; empty for the first.
    pub source: String,
    pub table: String,
    /// Its key column.
    pub key: String,
    /// The window of the kind `signin`.
    pub page: String,
    /// Where people sign in: `desktop`, `web`, `cli`.
    pub targets: Vec<String>,
    pub strategies: Vec<SigninStrategy>,
    /// How long a served session lasts.
    pub session_seconds: u64,
    /// Failed tries per key allowed within `tries_seconds`.
    pub tries: u32,
    pub tries_seconds: u64,
    /// The program's name, under which its secrets are kept (the remembered sign-in).
    pub program: String,
    /// How long the desktop program keeps a sign-in between runs (`remember N
    /// days`), in the system's protected store; none without the line.
    pub remember_seconds: Option<u64>,
    /// Whether the sign-in part asks whether to remember (`remember N days
    /// when asked`): then only a sign-in whose input `remember` is "true"
    /// is remembered; else every one is.
    pub remember_asks: bool,
}

impl SigninSource {
    /// The people in `table` (key `id`, the first data source), signing in
    /// on `page`, on the desktop; 8 hours a session, 5 tries per 15 minutes.
    pub fn new(table: &str, page: &str) -> SigninSource {
        SigninSource { source: String::new(), table: table.into(), key: "id".into(), page: page.into(), targets: vec!["desktop".into()], strategies: Vec::new(), session_seconds: 8 * 3600, tries: 5, tries_seconds: 15 * 60, program: String::new(), remember_seconds: None, remember_asks: false }
    }

    /// Adds the strategy `name` with its columns by role.
    pub fn strategy(mut self, name: &str, columns: &[&str]) -> SigninSource {
        self.strategies.push(SigninStrategy { name: name.into(), columns: columns.iter().map(|c| c.to_string()).collect(), options: Vec::new() });
        self
    }

    fn on(&self, target: &str) -> bool {
        self.targets.iter().any(|t| t == target)
    }
}

/// What the database handle knows of who is using the program.
#[derive(Default)]
pub(crate) struct People {
    source: Option<SigninSource>,
    /// The person signed in on the desktop or the command line.
    me: std::cell::Cell<Option<Id>>,
    /// The command line signs in as `cli` (`Db::signin_cli`); else `desktop`.
    cli: std::cell::Cell<bool>,
    /// The computer's user was tried at the start (once: after `signout` the window shows).
    started: std::cell::Cell<bool>,
}

/// The served call this thread answers: its person, the session and the
/// address it came with, and what signing in or out changed of its session.
struct Call {
    me: Option<Id>,
    token: Option<String>,
    address: String,
    opened: Option<(String, u64)>,
    closed: bool,
}

std::thread_local! {
    static CALL: RefCell<Option<Call>> = const { RefCell::new(None) };
}

/// The sign-in window of a served program whose people sign in on the web:
/// a call without a session reaches only it (`served`).
static WEB_PAGE: Mutex<Option<String>> = Mutex::new(None);

/// Clarity's words when signing in did not work.
pub fn refused() -> Text {
    Text::keyed("signinRefused", "Signing in did not work. Check what you typed and try again.")
}

/// Clarity's words when too many tries failed.
pub fn too_many_tries() -> Text {
    Text::keyed("signinTooManyTries", "Too many tries. Wait a few minutes and try again.")
}

/// Clarity's words when `allow when` does not hold.
pub fn not_allowed() -> Text {
    Text::keyed("notAllowed", "You are not allowed to do this.")
}

/// Clarity's words when `readable when` or `writable when` does not hold.
pub fn not_yours() -> Text {
    Text::keyed("notYours", "This record is not yours.")
}

/// `allow when <condition>` on an action, command or window: refused with
/// `notAllowed` when it does not hold.
pub fn allow(holds: bool) -> Result<()> {
    if holds { Ok(()) } else { Err(Error::Refused(not_allowed())) }
}

/// `readable when` / `writable when` of an entity, checked on a find or a
/// save: refused with `notYours` when it does not hold.
pub fn yours(holds: bool) -> Result<()> {
    if holds { Ok(()) } else { Err(Error::Refused(not_yours())) }
}

/// Clarity's words when a secret is shorter than its field's `minimum`.
pub fn secret_too_short(minimum: u32) -> Text {
    Text::keyed("secretTooShort", "This needs at least {minimum} characters.").with("minimum", crate::ui::Slot::integer(minimum as i64, None))
}

/// Text assigned to a `hashed` field: what is stored, by the field's
/// method (`argon2id`; `sha256` for a token field, `sha256-lines` for a
/// recovery field). With the field's `minimum`, a shorter text is refused
/// with `secretTooShort` before anything is hashed.
pub fn hashed(method: &str, text: &str, minimum: Option<u32>) -> Result<String> {
    if let Some(minimum) = minimum
        && !clarity_auth::secret_fits(text, minimum)
    {
        return Err(Error::Refused(secret_too_short(minimum)));
    }
    clarity_auth::hash(method, text).map_err(|p| Error::Value(p.text))
}

/// `newRecoveryCodes(count)` in code: one-time codes, one per line, to show
/// the person once and assign to a recovery field.
pub fn new_recovery_codes(count: i64) -> Result<String> {
    clarity_auth::new_recovery_codes(count.max(0) as usize).map_err(|p| Error::Value(p.text))
}

/// `newToken()` in code.
pub fn new_token() -> Result<String> {
    clarity_auth::new_token().map_err(|p| Error::Value(p.text))
}

fn served_now() -> bool {
    CALL.with(|c| c.borrow().is_some())
}

/// What a command line gives to sign in with: `--user name` (the password
/// then typed), `--token` (or the environment's CLARITY_TOKEN).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CliSignin {
    pub user: Option<String>,
    pub token: Option<String>,
}

impl CliSignin {
    /// `--user name` and `--token value` among `args`, the token else from CLARITY_TOKEN.
    pub fn from_args(args: &[String]) -> CliSignin {
        let after = |flag: &str| args.iter().position(|a| a == flag).and_then(|at| args.get(at + 1)).cloned();
        let token = after("--token").or_else(|| std::env::var("CLARITY_TOKEN").ok()).filter(|t| !t.is_empty());
        CliSignin { user: after("--user"), token }
    }
}

impl Db {
    /// Names the people of the program and how they sign in, after the
    /// databases opened: nobody is signed in yet.
    pub fn add_signin(&mut self, source: SigninSource) {
        if source.on("web") {
            *WEB_PAGE.lock().unwrap_or_else(|e| e.into_inner()) = Some(source.page.clone());
        }
        self.people = People { source: Some(source), ..Default::default() };
    }

    fn people(&self) -> Result<&SigninSource> {
        self.people.source.as_ref().ok_or_else(|| Error::Storage("the program names no people who sign in".into()))
    }

    /// The record signed in for this call: served, the one of the call's
    /// session; on the desktop, the one of the program. None for nobody.
    pub fn me_id(&self) -> Option<Id> {
        CALL.with(|c| match c.borrow().as_ref() {
            Some(call) => call.me,
            None => self.people.me.get(),
        })
    }

    /// `me` in code: the record signed in now; refused with `notAllowed` when nobody is.
    pub fn me<R: Record>(&self) -> Result<R> {
        match self.me_id() {
            Some(id) => self.get::<R>(id),
            None => Err(Error::Refused(not_allowed())),
        }
    }

    /// The sign-in window the program starts with, as `Started.signin` says
    /// it: none when it names no people, when its people do not sign in
    /// where it runs (`web` served, else `desktop`), or when someone is
    /// signed in. On the desktop, once at the start (after `signout` the
    /// sign-in window shows): a sign-in remembered (`remember N days`) that
    /// still holds signs its person in, else the `account` strategy signs
    /// the computer's user in by itself when they have a record (no try counted).
    pub fn signin_first(&self) -> Option<String> {
        let people = self.people.source.as_ref()?;
        let target = self.target();
        if !people.on(target) || self.me_id().is_some() {
            return None;
        }
        let first = !self.people.started.replace(true);
        if target == "desktop" && first {
            if let Some(id) = self.remembered(people) {
                self.people.me.set(Some(id));
                return None;
            }
            if people.strategies.iter().any(|s| s.name == "account") && self.attempt("account", &Map::new(), false).is_ok() {
                return None;
            }
        }
        Some(people.page.clone())
    }

    /// The person of the remembered sign-in, when it still holds: the entry
    /// names a record whose stored secret of some strategy it was signed
    /// with, and it has not expired. None for anything else (no entry, no
    /// store, the record gone, the secret changed).
    fn remembered(&self, people: &SigninSource) -> Option<Id> {
        people.remember_seconds?;
        let entry = remembered_entry(people)?;
        let record = clarity_auth::remember_record(&entry);
        if record <= 0 {
            return None;
        }
        let now = clarity_auth::now();
        secret_columns(people).iter().find_map(|column| {
            let stored = self.stored_secret(people, column, record)?;
            (clarity_auth::remember_check(&entry, &stored, now) == record).then_some(record)
        })
    }

    /// After a sign-in on the desktop with `remember N days`: an entry signed
    /// with the person's stored secret of the strategy used, kept in the
    /// system's protected store. A strategy without a secret (`account`)
    /// remembers nothing; a store that cannot keep it leaves the sign-in as it is.
    fn remember(&self, people: &SigninSource, secret: Option<&str>, id: Id) {
        let (Some(seconds), Some(column)) = (people.remember_seconds, secret) else { return };
        // Read after signing in: what signing in changed (a recovery code used up) is what holds.
        let Some(stored) = self.stored_secret(people, column, id) else { return };
        if let Ok(entry) = clarity_auth::remember_make(id, seconds, &stored, clarity_auth::now()) {
            keep_remembered(people, &entry);
        }
    }

    /// The stored secret in `column` of the people's record `id`; none when
    /// the record is gone, the column empty or the database cannot be read.
    fn stored_secret(&self, people: &SigninSource, column: &str, id: Id) -> Option<String> {
        let mut conn = self.conn(&people.source).ok()?;
        let d = conn.dialect();
        let query = format!("SELECT {} FROM {} WHERE {} = {}", d.quote(column), d.quote(&people.table), d.quote(&people.key), d.mark(1));
        match conn.rows(&query, &[SqlValue::Integer(id)]).ok()?.into_iter().next()?.into_iter().next()? {
            SqlValue::Text(text) if !text.is_empty() => Some(text),
            _ => None,
        }
    }

    /// Signs a person in with `strategy` and its `inputs` (`name`,
    /// `password`, `token`, as the catalog names them): the record signed
    /// in, or Clarity's refusal (`signinRefused`, `signinTooManyTries`).
    /// Failed tries count per account and, served, per address.
    pub fn signin(&self, strategy: &str, inputs: &Map<String, Value>) -> Result<Id> {
        self.attempt(strategy, inputs, true)
    }

    fn target(&self) -> &'static str {
        if served_now() {
            "web"
        } else if self.people.cli.get() {
            "cli"
        } else {
            "desktop"
        }
    }

    /// Signs the command line's person in before a command runs, when the
    /// program's people sign in on `cli`: `--token` for `token`, `--user`
    /// with the password `password` asks for (typed without echo,
    /// `cli::read_password`) for `password`, else the computer's user by
    /// `account` when they have a record. None when the command line needs
    /// no sign-in; else the record signed in, or Clarity's refusal.
    pub fn signin_cli(&self, given: &CliSignin, password: &mut dyn FnMut() -> Option<String>) -> Result<Option<Id>> {
        let Some(people) = self.people.source.as_ref().filter(|p| p.on("cli")) else { return Ok(None) };
        self.people.cli.set(true);
        let uses = |name: &str| people.strategies.iter().any(|s| s.name == name);
        let mut inputs = Map::new();
        if let Some(token) = given.token.as_ref().filter(|_| uses("token")) {
            inputs.insert("token".into(), Value::from(token.as_str()));
            return self.signin("token", &inputs).map(Some);
        }
        if let Some(user) = given.user.as_ref().filter(|_| uses("password")) {
            let Some(typed) = password() else { return Err(Error::Refused(refused())) };
            inputs.insert("name".into(), Value::from(user.as_str()));
            inputs.insert("password".into(), Value::from(typed));
            return self.signin("password", &inputs).map(Some);
        }
        if uses("account") {
            return self.signin("account", &inputs).map(Some);
        }
        Err(Error::Refused(refused()))
    }

    /// One attempt, decided by Clarity.Auth (`attempt`): this looks the
    /// person up and stores what it is told. `counted` attempts count failed tries.
    fn attempt(&self, strategy: &str, inputs: &Map<String, Value>, counted: bool) -> Result<Id> {
        use clarity_auth::attempt::{Begun, Decided};
        let people = self.people()?;
        let target = self.target();
        let block = block_of(people);
        let address = CALL.with(|c| c.borrow().as_ref().map(|c| c.address.clone()));
        let now = clarity_auth::now();
        let refused = |key: &str| Error::Refused(if key == clarity_auth::attempt::TOO_MANY_TRIES { too_many_tries() } else { refused() });
        let (column, secret, equals, keys) = match clarity_auth::attempt::signin_begin(&block, strategy, inputs, target, address.as_deref(), now) {
            Begun::Lookup { column, secret, equals, keys } => (column, secret, equals, keys),
            Begun::Refused(key) => return Err(refused(key)),
        };
        let keys = if counted { keys } else { Vec::new() };
        let found = self.person_by(people, &column, secret.as_deref(), &equals)?;
        let no_people = found.is_none() && self.no_people(people)?;
        let id = match clarity_auth::attempt::signin_decide(&block, strategy, inputs, &keys, found.as_ref().map(|(_, s)| s.as_str()), no_people, now) {
            Decided::SignedIn { store } => {
                let id = found.map(|(id, _)| id).unwrap_or_default();
                self.store_columns(people, id, &store)?;
                id
            }
            // A first account that cannot be made (a required field without a default) refuses, as on every stack.
            Decided::Make { columns } => self.make_person(people, &columns).map_err(|_| refused(clarity_auth::attempt::REFUSED))?,
            Decided::Refused(key) => return Err(refused(key)),
        };
        self.signed_in(id, people.session_seconds);
        if target == "desktop" && remembers(people, inputs) {
            self.remember(people, secret.as_deref(), id);
        }
        Ok(id)
    }

    /// The key and the stored secret (empty without one) of the record whose `column` equals `equals`.
    fn person_by(&self, people: &SigninSource, column: &str, secret: Option<&str>, equals: &str) -> Result<Option<(Id, String)>> {
        let mut conn = self.conn(&people.source)?;
        let d = conn.dialect();
        let wanted = match secret {
            Some(secret) => format!("{}, {}", d.quote(&people.key), d.quote(secret)),
            None => d.quote(&people.key),
        };
        let query = format!("SELECT {wanted} FROM {} WHERE {} = {}", d.quote(&people.table), d.quote(column), d.mark(1));
        let rows = conn.rows(&query, &[sql::text(equals)])?;
        let Some(row) = rows.into_iter().next() else { return Ok(None) };
        let id = sql::get_integer(&people.key, row.first().unwrap_or(&SqlValue::Null))?;
        let stored = match row.get(1) {
            Some(SqlValue::Text(text)) => text.clone(),
            _ => String::new(),
        };
        Ok(Some((id, stored)))
    }

    /// Writes back the fields (by role) a sign-in changed, in one UPDATE.
    /// Writes `store`, values by column, to the person's record `id`.
    fn store_columns(&self, people: &SigninSource, id: Id, store: &Map<String, Value>) -> Result<()> {
        let mut conn = self.conn(&people.source)?;
        let d = conn.dialect();
        let mut sets = Vec::new();
        let mut values = Vec::new();
        for (column, value) in store {
            if let Some(value) = value.as_str() {
                values.push(sql::text(value));
                sets.push(format!("{} = {}", d.quote(column), d.mark(values.len())));
            }
        }
        if sets.is_empty() {
            return Ok(());
        }
        values.push(SqlValue::Integer(id));
        let update = format!("UPDATE {} SET {} WHERE {} = {}", d.quote(&people.table), sets.join(", "), d.quote(&people.key), d.mark(values.len()));
        conn.execute(&update, &values)?;
        Ok(())
    }

    /// The secret columns of the people's record `id` when `table` is the
    /// people's (before a save, to see whether it changes them); none else.
    pub(crate) fn secrets_of(&self, table: &str, id: Id) -> Option<Vec<SqlValue>> {
        let people = self.people.source.as_ref().filter(|p| p.table == table && id != 0)?;
        let columns = secret_columns(people);
        if columns.is_empty() {
            return None;
        }
        let mut conn = self.conn(&people.source).ok()?;
        let d = conn.dialect();
        let wanted: Vec<String> = columns.iter().map(|c| d.quote(c)).collect();
        let query = format!("SELECT {} FROM {} WHERE {} = {}", wanted.join(", "), d.quote(&people.table), d.quote(&people.key), d.mark(1));
        conn.rows(&query, &[SqlValue::Integer(id)]).ok()?.into_iter().next()
    }

    /// A save of a person changed their secret (`before`, as `secrets_of`
    /// read it): served, the person's other sessions close when they changed
    /// their own, every one of theirs when someone else did. On the desktop
    /// and the command line nothing: there is one person, signed in for the run.
    pub(crate) fn secrets_saved(&self, table: &str, id: Id, before: Option<Vec<SqlValue>>) {
        let Some(before) = before else { return };
        if self.secrets_of(table, id).is_some_and(|after| after == before) {
            return;
        }
        CALL.with(|c| {
            let c = c.borrow();
            let Some(call) = c.as_ref() else { return };
            match (call.me, call.token.as_deref()) {
                (Some(me), Some(token)) if me == id => clarity_auth::session_close_others(id, token),
                _ => clarity_auth::session_close_all(id),
            }
        });
    }

    fn no_people(&self, people: &SigninSource) -> Result<bool> {
        let mut conn = self.conn(&people.source)?;
        let d = conn.dialect();
        let rows = conn.rows(&format!("SELECT COUNT(*) FROM {}", d.quote(&people.table)), &[])?;
        let count = rows.first().and_then(|r| r.first()).map_or(Ok(0), |v| sql::get_integer("count", v))?;
        Ok(count == 0)
    }

    /// Inserts a person with only `columns` (values by column); the other columns take their defaults.
    fn make_person(&self, people: &SigninSource, columns: &Map<String, Value>) -> Result<Id> {
        let names: Vec<&str> = columns.keys().map(String::as_str).collect();
        let values: Vec<SqlValue> = columns.values().map(|v| sql::text(v.as_str().unwrap_or_default())).collect();
        let mut conn = self.conn(&people.source)?;
        let d = conn.dialect();
        Ok(conn.insert(&d.insert(&people.table, &people.key, &names), &values)?)
    }

    fn signed_in(&self, id: Id, seconds: u64) {
        let served = CALL.with(|c| {
            let mut c = c.borrow_mut();
            let Some(call) = c.as_mut() else { return false };
            // A new session for the new sign-in; the one it came with is closed.
            if let Some(old) = call.token.take() {
                clarity_auth::session_close(&old);
            }
            if let Ok(token) = clarity_auth::session_open(id, seconds, clarity_auth::now()) {
                call.opened = Some((token.clone(), seconds));
                call.token = Some(token);
            }
            call.me = Some(id);
            true
        });
        if !served {
            self.people.me.set(Some(id));
        }
    }

    /// Signs the person out: served, the call's session is closed and its
    /// cookie cleared; on the desktop nobody is signed in any more.
    pub fn signout(&self) {
        let served = CALL.with(|c| {
            let mut c = c.borrow_mut();
            let Some(call) = c.as_mut() else { return false };
            if let Some(token) = call.token.take() {
                clarity_auth::session_close(&token);
            }
            call.me = None;
            call.opened = None;
            call.closed = true;
            true
        });
        if !served {
            self.people.me.set(None);
            if let Some(people) = self.people.source.as_ref().filter(|p| p.remember_seconds.is_some()) {
                keep_remembered(people, "");
            }
        }
    }
}

/// Whether a sign-in with these inputs is remembered: always, unless the
/// sign-in part asks (`when asked`) and its box's answer is not "true".
/// The block as Clarity.Auth's attempt takes it.
fn block_of(people: &SigninSource) -> clarity_auth::attempt::Block {
    let strategies = people.strategies.iter().map(|s| clarity_auth::attempt::BlockStrategy { name: s.name.clone(), columns: s.columns.clone(), options: options_of(people, s) }).collect();
    clarity_auth::attempt::Block { strategies, tries: people.tries, tries_seconds: people.tries_seconds }
}

/// A provider's settings as Clarity.Auth takes them, the secret read from
/// the program's protected store (DESIGN 5.27); empty when it is not kept.
fn options_of(people: &SigninSource, strategy: &SigninStrategy) -> Map<String, Value> {
    strategy
        .options
        .iter()
        .map(|(key, value)| {
            let value = match value.strip_prefix("setting:").filter(|_| key == "secret") {
                Some(name) => kept_secret(&people.program, name),
                None => value.clone(),
            };
            (key.clone(), Value::from(value))
        })
        .collect()
}

/// A secret setting of the program; empty when the store has none.
fn kept_secret(program: &str, name: &str) -> String {
    #[cfg(feature = "secrets")]
    {
        crate::secrets::get(program, name).unwrap_or_default()
    }
    #[cfg(not(feature = "secrets"))]
    {
        let _ = (program, name);
        String::new()
    }
}

fn remembers(people: &SigninSource, inputs: &Map<String, Value>) -> bool {
    !people.remember_asks || inputs.get("remember").and_then(Value::as_str) == Some("true")
}

/// The remembered sign-in kept for the program; none without one or without a store.
fn remembered_entry(people: &SigninSource) -> Option<String> {
    #[cfg(feature = "secrets")]
    {
        crate::secrets::get(&people.program, REMEMBERED).filter(|e| !e.is_empty())
    }
    #[cfg(not(feature = "secrets"))]
    {
        let _ = people;
        None
    }
}

/// Keeps `entry` as the program's remembered sign-in; empty forgets it.
/// A store that cannot keep it changes nothing else.
fn keep_remembered(people: &SigninSource, entry: &str) {
    #[cfg(feature = "secrets")]
    {
        let _ = crate::secrets::set(&people.program, REMEMBERED, entry);
    }
    #[cfg(not(feature = "secrets"))]
    {
        let _ = (people, entry);
    }
}

/// The columns that keep a secret of some strategy of the project, once each.
fn secret_columns(people: &SigninSource) -> Vec<String> {
    let mut columns: Vec<String> = Vec::new();
    for used in &people.strategies {
        let Some(info) = clarity_auth::catalog().iter().find(|s| s.name == used.name) else { continue };
        for (field, column) in info.fields.iter().zip(&used.columns) {
            if field.role == "secret" && !columns.contains(column) {
                columns.push(column.clone());
            }
        }
    }
    columns
}

/// The session token a request's `Cookie` header carries.
pub fn session_of(cookie_header: &str) -> Option<String> {
    cookie_header.split(';').find_map(|pair| {
        let (name, value) = pair.split_once('=')?;
        (name.trim() == COOKIE).then(|| value.trim().to_string()).filter(|v| !v.is_empty())
    })
}

/// Whether a served call without a session may run: the program's start,
/// its language and facts, signing in and out, and the sign-in window itself.
fn open_without_session(command: &str, args: &Value, page: &str) -> bool {
    match command {
        "started" | "set_language" | "facts" | "splash_status" | "watches" | "read_settings" | "signin" | "signout" => true,
        "open_window" => args.get("window").and_then(Value::as_str) == Some(page),
        _ => args.get("state").and_then(|s| s.get("window")).and_then(Value::as_str) == Some(page),
    }
}

/// Runs one served call (`run`) as the person of its session: `cookie` is
/// the request's `Cookie` header, `address` where it came from. Without a
/// valid session, while people sign in on the web, only the sign-in window
/// and its commands are answered; the program refuses the rest with
/// `notAllowed`. Gives the answer and the `Set-Cookie` header's value when
/// the call signed in (a new session) or out (the cookie cleared).
pub fn served(cookie: Option<&str>, address: &str, command: &str, args: &Value, run: impl FnOnce() -> Value) -> (Value, Option<String>) {
    let token = cookie.and_then(session_of);
    let me = token.as_deref().map(|t| clarity_auth::session_find(t, clarity_auth::now())).filter(|id| *id > 0);
    let token = token.filter(|_| me.is_some());
    let page = WEB_PAGE.lock().unwrap_or_else(|e| e.into_inner()).clone();
    if let Some(page) = page.filter(|_| me.is_none())
        && !open_without_session(command, args, &page)
    {
        return (json!({ "error": not_allowed() }), None);
    }
    let was = CALL.with(|c| c.replace(Some(Call { me, token, address: address.to_string(), opened: None, closed: false })));
    let answer = run();
    let call = CALL.with(|c| c.replace(was));
    let failed = answer.get("error").is_some_and(|e| !e.is_null());
    let header = call.and_then(|call| match (call.opened, call.closed) {
        (Some((token, seconds)), _) if !failed => Some(format!("{COOKIE}={token}; HttpOnly; SameSite=Strict; Path=/; Max-Age={seconds}")),
        (_, true) => Some(format!("{COOKIE}=; HttpOnly; SameSite=Strict; Path=/; Max-Age=0")),
        _ => None,
    });
    (answer, header)
}
