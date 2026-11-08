//! Reading the adapter-fcdmapi repository's tables and what fills them.
//! Each `DbSet` of its DbContext names a table and the adapter entity kept
//! in it (`Models.Cursus` is entity 1, `Models.V2.Cursus` entity 2), with
//! the columns and key of that entity's model class. Tables are filled from
//! events: each data-intake handler takes events (`IHandleMessages<E>`) and
//! writes through a service, `IService<Fcdm.X, Models.V2.X>` for one
//! table, or a service of its own whose repositories
//! (`IGenericRepository<Models.V2.Cursus>`) name the tables it writes. A
//! deleted event deletes from those tables and adds a row to
//! DeletedEntities. Each such pair is an `intake` row; a table takes its
//! domain from the events that fill it.

use crate::read::SourceFile;
use serde::Serialize;
use std::collections::HashMap;

/// A row of what was read, by `kind`: a `table`, an `intake` (an event that fills a table) or a `note` for the report.
#[derive(Debug, Clone, Default, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TableRow {
    pub kind: String,
    pub path: String,
    /// The adapter entity's class, which is the data object's name.
    pub name: String,
    /// The adapter entity's version: 1 for `Models`, N for `Models.VN`.
    pub entity: i64,
    pub table: String,
    /// For a table: the fcdm domain of the events that fill it; empty when none does.
    pub domain: String,
    pub note: String,
    /// The adapter entity's columns as its model class has them (`Bk: string`), its base class's first.
    pub columns: Vec<String>,
    /// Its primary key: `Bk`, or what the DbContext's `HasKey` names (DeletedEntity: Bk, EntityName).
    pub keys: Vec<String>,
    /// For an intake: the event's class, and the namespaces it can be in (those its handler opens, or the one it names).
    pub event: String,
    pub namespaces: Vec<String>,
    /// For an intake: whether the event deletes from the table (a deleted event), else it writes a row.
    pub deletes: bool,
}

const MODELS: &str = "Fontys.Adapter.FcdmApi.Models";
const MESSAGES: &str = "Fontys.Messages.";
const DELETED: &str = "DeletedEntity";

/// The tables of the adapter's DbContext, in its order, then the events that fill them, then notes.
pub fn read_tables(files: &[SourceFile]) -> Vec<TableRow> {
    let mut tables: Vec<TableRow> = Vec::new();
    let mut notes = Vec::new();
    let models = Models::of(files);
    let mut keys: HashMap<String, Vec<String>> = HashMap::new();
    for f in files {
        let text = f.text.trim_start_matches('\u{feff}');
        if !text.contains(": DbContext") {
            continue;
        }
        let names = Usings::of(text);
        keys.extend(has_keys(text));
        for line in text.lines() {
            if let Some((entity, table)) = db_set(line) {
                let (ns, class) = names.resolve(entity, MODELS);
                match entity_version(&ns) {
                    Some(n) => tables.push(TableRow { kind: "table".into(), path: f.path.clone(), name: class, entity: n, table, ..Default::default() }),
                    None => notes.push(note(&f.path, format!("{table}: its entity {entity} is no adapter model"))),
                }
            }
        }
    }
    for t in &mut tables {
        let ns = if t.entity == 1 { MODELS.to_string() } else { format!("{MODELS}.V{}", t.entity) };
        let (columns, key) = models.columns(&ns, &t.name);
        t.columns = columns;
        t.keys = keys.get(&t.name).cloned().unwrap_or(key);
    }
    let table_of = |class: &str, entity: i64| tables.iter().find(|t| t.name == class && t.entity == entity).map(|t| t.table.clone());
    let deleted_table = tables.iter().find(|t| t.name == DELETED).map(|t| t.table.clone());
    let services = services(files, &models);
    let mut intakes = Vec::new();
    let fill = |path: &str, names: &Usings, written_event: &str, written: &[(String, i64)], intakes: &mut Vec<TableRow>, notes: &mut Vec<TableRow>| {
        let (namespaces, event) = names.candidates(written_event);
        let deletes = event.ends_with("DeletedEvent");
        for (class, entity) in written {
            match table_of(class, *entity) {
                Some(table) => intakes.push(TableRow { kind: "intake".into(), path: path.into(), name: class.clone(), entity: *entity, table, event: event.clone(), namespaces: namespaces.clone(), deletes, ..Default::default() }),
                None => notes.push(note(path, format!("{event} fills entity {class} {entity}, which has no table in the DbContext"))),
            }
        }
        if let (true, Some(table)) = (deletes, &deleted_table) {
            intakes.push(TableRow { kind: "intake".into(), path: path.into(), name: DELETED.into(), entity: 1, table: table.clone(), event, namespaces, ..Default::default() });
        }
    };
    for f in files {
        let text = f.text.trim_start_matches('\u{feff}');
        if namespace(text).contains("Tests") {
            continue;
        }
        let names = Usings::of(text);
        // A generic handler: `class XChangedHandler(...) : GenericHandler<XChangedEvent, X, Models.V2.X>(...)`.
        for part in text.split("GenericHandler<").skip(1) {
            let Some((inside, _)) = part.split_once('>') else { continue };
            let args: Vec<&str> = inside.split(',').map(str::trim).collect();
            let [event, _, target] = args.as_slice() else { continue };
            let class = event.rsplit('.').next().unwrap_or_default();
            if class.starts_with('T') && class.chars().nth(1).is_some_and(char::is_uppercase) {
                continue;
            }
            let (ns, model) = names.resolve(target, MODELS);
            if let (Some(n), true) = (entity_version(&ns), models.has(&ns, &model)) {
                fill(&f.path, &names, event, &[(model, n)], &mut intakes, &mut notes);
            }
        }
        // A handler of its own, not the generic one.
        if !text.contains("IHandleMessages<") || text.contains("abstract class GenericHandler<") {
            continue;
        }
        let events = handled(text);
        let mut written: Vec<(String, i64)> = Vec::new();
        for ty in service_fields(text) {
            if let Some(inside) = ty.strip_prefix("IService<").and_then(|r| r.strip_suffix('>')) {
                let Some((_, target)) = inside.split_once(',') else { continue };
                let (ns, class) = names.resolve(target.trim(), MODELS);
                if let (Some(n), true) = (entity_version(&ns), models.has(&ns, &class)) {
                    written.push((class, n));
                }
            } else if let Some(found) = services.get(&ty) {
                written.extend(found.iter().cloned());
            }
        }
        if written.is_empty() {
            if !events.is_empty() {
                notes.push(note(&f.path, format!("{}: its handler writes through no service the reader knows", events.join(", "))));
            }
            continue;
        }
        for written_event in &events {
            fill(&f.path, &names, written_event, &written, &mut intakes, &mut notes);
        }
    }
    // A table's domain: that of the first event that writes it.
    for t in &mut tables {
        if let Some(domain) = intakes.iter().filter(|i| i.table == t.table && !i.deletes && t.name != DELETED).find_map(|i| i.namespaces.first().and_then(|n| domain_of(n))) {
            t.domain = domain;
        }
    }
    tables.extend(intakes);
    tables.extend(notes);
    tables
}

fn note(path: &str, text: String) -> TableRow {
    TableRow { kind: "note".into(), path: path.into(), note: text, ..Default::default() }
}

/// The domain of an event namespace: `Fontys.Messages.Hr.Events.v4` and `Fontys.Messages.Hr.v2.Events` are Hr's.
fn domain_of(ns: &str) -> Option<String> {
    ns.strip_prefix(MESSAGES)?.split('.').next().filter(|d| !d.is_empty()).map(str::to_string)
}

/// The events a handler takes: `IHandleMessages<EventsV2.OnderwijsproductDeletedEvent>`, as written; no commands, no type parameters.
fn handled(text: &str) -> Vec<String> {
    let mut found = Vec::new();
    for part in text.split("IHandleMessages<").skip(1) {
        let Some((ty, _)) = part.split_once('>') else { continue };
        let ty = ty.trim().to_string();
        let class = ty.rsplit('.').next().unwrap_or_default();
        // `TEvent` is a type parameter, as `T` and an upper-case letter says.
        let parameter = class.starts_with('T') && class.chars().nth(1).is_some_and(char::is_uppercase);
        if class.ends_with("Event") && !parameter && !found.contains(&ty) {
            found.push(ty);
        }
    }
    found
}

/// The types of a handler's services: `private readonly IService<Lesplaats, Models.V2.Lesplaats> _service;`.
fn service_fields(text: &str) -> Vec<String> {
    text.lines()
        .filter_map(|l| l.trim().strip_prefix("private readonly "))
        .filter_map(|rest| rest.rsplit_once(' ').map(|(ty, _)| ty.trim().to_string()))
        .filter(|ty| ty.contains("Service"))
        .collect()
}

/// The services of their own (`class OnderwijsproductService : IOnderwijsproductService`), by
/// interface, with the adapter entities their repositories write.
fn services(files: &[SourceFile], models: &Models) -> HashMap<String, Vec<(String, i64)>> {
    let mut found: HashMap<String, Vec<(String, i64)>> = HashMap::new();
    for f in files {
        let text = f.text.trim_start_matches('\u{feff}');
        if !text.contains("IGenericRepository<") || namespace(text).contains("Tests") {
            continue;
        }
        let names = Usings::of(text);
        let interfaces: Vec<String> = text
            .lines()
            .filter_map(|l| {
                let rest = l.trim().split_once("class ")?.1;
                let after = rest.split_once(':')?.1;
                let first = after.split([',', '{', ' ']).find(|w| !w.is_empty())?;
                (first.starts_with('I') && first.ends_with("Service")).then(|| first.to_string())
            })
            .collect();
        if interfaces.is_empty() {
            continue;
        }
        let mut written: Vec<(String, i64)> = Vec::new();
        for part in text.split("IGenericRepository<").skip(1) {
            let Some((ty, _)) = part.split_once('>') else { continue };
            let (ns, class) = names.resolve(ty.trim(), MODELS);
            if let (Some(n), true) = (entity_version(&ns), models.has(&ns, &class)) {
                if !written.contains(&(class.clone(), n)) {
                    written.push((class, n));
                }
            }
        }
        for interface in interfaces {
            let list = found.entry(interface).or_default();
            for w in &written {
                if !list.contains(w) {
                    list.push(w.clone());
                }
            }
        }
    }
    found
}

/// The namespace a file declares.
fn namespace(text: &str) -> String {
    text.lines().find_map(|l| l.trim().strip_prefix("namespace ")).map(|n| n.trim().trim_end_matches(';').to_string()).unwrap_or_default()
}

/// The adapter's model classes: per namespace and class, its base class and its properties.
struct Models {
    classes: HashMap<(String, String), (String, Vec<(String, bool)>)>,
}

impl Models {
    fn of(files: &[SourceFile]) -> Models {
        let mut classes = HashMap::new();
        for f in files {
            let text = f.text.trim_start_matches('\u{feff}');
            let Some(ns) = text.lines().find_map(|l| l.trim().strip_prefix("namespace ")).map(|n| n.trim().trim_end_matches(';').to_string()) else { continue };
            if !ns.starts_with(MODELS) {
                continue;
            }
            let mut current: Option<(String, String)> = None;
            let mut keyed = false;
            for line in text.lines() {
                let line = line.trim();
                if let Some(rest) = line.split_once("class ").map(|(_, r)| r).filter(|_| line.starts_with("public")) {
                    let (name, base) = rest.split_once(':').map_or((rest, ""), |(n, b)| (n, b));
                    let name = name.trim().to_string();
                    classes.insert((ns.clone(), name.clone()), (base.trim().to_string(), Vec::new()));
                    current = Some((ns.clone(), name));
                    continue;
                }
                if line == "[Key]" {
                    keyed = true;
                    continue;
                }
                if let (Some(at), Some(head)) = (&current, line.strip_prefix("public ").and_then(|r| r.split_once("{ get").map(|(h, _)| h))) {
                    let words: Vec<&str> = head.split_whitespace().filter(|w| !matches!(*w, "virtual" | "override" | "required")).collect();
                    if let [ty, name] = words.as_slice() {
                        if let Some((_, props)) = classes.get_mut(at) {
                            props.push((format!("{name}: {ty}"), keyed));
                        }
                    }
                }
                if !line.starts_with('[') && !line.is_empty() {
                    keyed = false;
                }
            }
        }
        Models { classes }
    }

    /// Whether `name` in `ns` is a model class: an entity, not a type parameter.
    fn has(&self, ns: &str, name: &str) -> bool {
        self.classes.contains_key(&(ns.to_string(), name.to_string()))
    }

    /// The columns of class `name` in `ns`, its base class's first, and the properties marked `[Key]`.
    fn columns(&self, ns: &str, name: &str) -> (Vec<String>, Vec<String>) {
        let Some((base, own)) = self.classes.get(&(ns.to_string(), name.to_string())) else { return (Vec::new(), Vec::new()) };
        let (mut columns, mut keys) = if base.is_empty() { (Vec::new(), Vec::new()) } else { self.columns(MODELS, base) };
        for (column, keyed) in own {
            if *keyed {
                keys.push(column.split(':').next().unwrap_or_default().to_string());
            }
            columns.push(column.clone());
        }
        (columns, keys)
    }
}

/// `.HasKey(nameof(Models.DeletedEntity.Bk), nameof(Models.DeletedEntity.EntityName))` in a DbContext, by class.
fn has_keys(text: &str) -> Vec<(String, Vec<String>)> {
    let mut found = Vec::new();
    for line in text.lines() {
        let Some(rest) = line.split_once(".HasKey(").map(|(_, r)| r) else { continue };
        let mut class = String::new();
        let mut keys = Vec::new();
        for part in rest.split("nameof(").skip(1) {
            let path = part.split(')').next().unwrap_or_default();
            let mut names: Vec<&str> = path.rsplitn(3, '.').collect();
            names.reverse();
            if let [.., c, key] = names.as_slice() {
                class = c.to_string();
                keys.push(key.to_string());
            }
        }
        if !keys.is_empty() {
            found.push((class, keys));
        }
    }
    found
}

/// The usings of a file: aliases (`using V2 = Fontys.Adapter.FcdmApi.Models.V2;`) and the namespaces it opens.
struct Usings {
    aliases: HashMap<String, String>,
    opened: Vec<String>,
}

impl Usings {
    fn of(text: &str) -> Usings {
        let mut aliases = HashMap::new();
        let mut opened = Vec::new();
        for line in text.lines() {
            let Some(rest) = line.trim().strip_prefix("using ").and_then(|r| r.strip_suffix(';')) else { continue };
            match rest.split_once('=') {
                Some((alias, ns)) => {
                    aliases.insert(alias.trim().to_string(), ns.trim().to_string());
                }
                None => opened.push(rest.trim().to_string()),
            }
        }
        Usings { aliases, opened }
    }

    /// The namespace a qualified type names (`V2.Cursus`, `Models.Account`), or none for a bare class.
    fn qualified(&self, written: &str) -> Option<(String, String)> {
        let (prefix, class) = written.rsplit_once('.')?;
        let (first, rest) = prefix.split_once('.').map_or((prefix, ""), |(a, b)| (a, b));
        let ns = match self.aliases.get(first) {
            Some(full) if rest.is_empty() => full.clone(),
            Some(full) => format!("{full}.{rest}"),
            None if first == "Models" => format!("Fontys.Adapter.FcdmApi.{prefix}"),
            None => prefix.to_string(),
        };
        Some((ns, class.to_string()))
    }

    /// A type as written as its namespace and class; a bare class is in the first namespace opened under `within`.
    fn resolve(&self, written: &str, within: &str) -> (String, String) {
        self.qualified(written).unwrap_or_else(|| (self.opened.iter().find(|n| n.starts_with(within)).cloned().unwrap_or_default(), written.to_string()))
    }

    /// The namespaces an event as written can be in, and its class: the one it names, or every messages namespace the file opens.
    fn candidates(&self, written: &str) -> (Vec<String>, String) {
        match self.qualified(written) {
            Some((ns, class)) => (vec![ns], class),
            None => (self.opened.iter().filter(|n| n.starts_with(MESSAGES)).cloned().collect(), written.to_string()),
        }
    }
}

/// `public DbSet<V2.Cursus> CursussenV2 => Set<V2.Cursus>();` as the entity written and the table.
fn db_set(line: &str) -> Option<(&str, String)> {
    let line = line.trim();
    if line.starts_with("//") {
        return None;
    }
    let rest = &line[line.find("DbSet<")? + "DbSet<".len()..];
    let (entity, after) = rest.split_once('>')?;
    let table: String = after.trim_start().chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
    (!table.is_empty()).then_some((entity.trim(), table))
}

/// The adapter entity's version of a namespace: `...Models` 1, `...Models.V2` 2; none outside the models.
fn entity_version(ns: &str) -> Option<i64> {
    let rest = ns.strip_prefix(MODELS)?;
    if rest.is_empty() {
        return Some(1);
    }
    rest.strip_prefix(".V")?.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(path: &str, text: &str) -> SourceFile {
        SourceFile { path: path.into(), text: text.into() }
    }

    const CONTEXT: &str = "\u{feff}using Microsoft.EntityFrameworkCore;
using V1 = Fontys.Adapter.FcdmApi.Models;
using V2 = Fontys.Adapter.FcdmApi.Models.V2;

namespace Fontys.Adapter.FcdmApi.Data
{
    public class FcdmDbContext : DbContext
    {
        public DbSet<V1.DeletedEntity> DeletedEntities => Set<V1.DeletedEntity>();
        public DbSet<V1.Cursus> Cursussen => Set<V1.Cursus>();
        public DbSet<V2.Cursus> CursussenV2 => Set<V2.Cursus>();
        // public DbSet<V1.Oud> Ouden => Set<V1.Oud>();
        public DbSet<V2.Onderwijsproduct> OnderwijsproductenV2 => Set<V2.Onderwijsproduct>();
        public DbSet<V1.Roostercode> Roostercodes => Set<V1.Roostercode>();
        void OnModelCreating(ModelBuilder modelBuilder) { modelBuilder.Entity<Models.DeletedEntity>()
                .HasKey(nameof(Models.DeletedEntity.Bk), nameof(Models.DeletedEntity.EntityName)); }
    }
}
";

    const CURSUS_HANDLER: &str = "using Fontys.Fcdm.Onderwijs;
using Fontys.Messages.Onderwijs.Events;
namespace Fontys.Adapter.FcdmApi.DataIntake.Handlers
{
    public class CursusHandler : IHandleMessages<CursusDeletedEvent>
    {
        private readonly IService<Cursus, Models.V2.Cursus> _service;
    }
}
";

    const PRODUCT_HANDLER: &str = "using EventsV3 = Fontys.Messages.Onderwijs.Events.v3;
namespace Fontys.Adapter.FcdmApi.DataIntake.Handlers
{
    public class OnderwijsproductHandler : IHandleMessages<EventsV3.OnderwijsproductChangedEvent>, IHandleMessages<InitialLoadCommand>
    {
        private readonly IOnderwijsproductService _service;
        private readonly ILogger _logger;
    }
}
";

    const PRODUCT_SERVICE: &str = "using DataModel = Fontys.Adapter.FcdmApi.Models;
namespace Fontys.Adapter.FcdmApi.Data.Services
{
    public class OnderwijsproductService : IOnderwijsproductService
    {
        private readonly IGenericRepository<DataModel.V2.Onderwijsproduct> _products;
        private readonly IGenericRepository<DataModel.V2.Cursus> _cursussen;
    }
}
";

    const ROOSTER_HANDLER: &str = "using Fontys.Messages.Hr.Events;
using Fontys.Messages.Hr.Events.v4;
namespace Fontys.Adapter.FcdmApi.DataIntake.Handlers
{
    public class RoostercodeHandler
        : IHandleMessages<RoostercodeChangedEvent>
    {
        private readonly IRoostercodeService _roostercodeService;
    }
}
";

    const ROOSTER_SERVICE: &str = "using Fontys.Adapter.FcdmApi.Models;
namespace Fontys.Adapter.FcdmApi.Data.Services
{
    public class RoostercodeService : IRoostercodeService
    {
        private readonly IGenericRepository<Roostercode> _roostercodeRepository;
    }
}
";

    /// The model classes the tables keep: a handler's or service's type that is none is a type parameter.
    const MODELS_V1: &str = "namespace Fontys.Adapter.FcdmApi.Models\n{\n    public class DeletedEntity : BaseEntity { }\n    public class Cursus : BaseEntity { }\n    public class Roostercode : BaseEntity { }\n}\n";
    const MODELS_V2: &str = "namespace Fontys.Adapter.FcdmApi.Models.V2\n{\n    public class Cursus : BaseEntity { }\n    public class Onderwijsproduct : BaseEntity { }\n}\n";

    const GENERIC_HANDLERS: &str = "using Fontys.Fcdm.Onderwijs;
using Fontys.Messages.Onderwijs.Events.v4;
namespace Fontys.Adapter.FcdmApi.DataIntake.Handlers.Generic
{
    public class CursusChangedHandler(IService<Cursus, Models.V2.Cursus> service, ILoggerFactory loggerFactory, GenericHandlerSettings settings)
        : GenericHandler<CursusChangedEvent, Cursus, Models.V2.Cursus>(service, loggerFactory, settings) { }

    public class CursusDeletedHandler(IService<Cursus, Models.V2.Cursus> service, ILoggerFactory loggerFactory, GenericHandlerSettings settings)
        : GenericHandler<CursusDeletedEvent, Cursus, Models.V2.Cursus>(service, loggerFactory, settings) { }
}
";

    const GENERIC_BASE: &str = "namespace Fontys.Adapter.FcdmApi.DataIntake.Handlers.Generic
{
    public abstract class GenericHandler<TEvent, TFcdmModel, TDatabaseEntity> : IHandleMessages<TEvent>
    {
        private readonly IService<TFcdmModel, TDatabaseEntity> _service;
    }
}
";

    fn rows() -> Vec<TableRow> {
        read_tables(&[
            file("DataIntake/Handlers/Generic/Handlers.cs", GENERIC_HANDLERS),
            file("DataIntake/Handlers/Generic/GenericHandler.cs", GENERIC_BASE),
            file("Models/Models.cs", MODELS_V1),
            file("Models/V2/Models.cs", MODELS_V2),
            file("Data/FcdmDbContext.cs", CONTEXT),
            file("DataIntake/Handlers/CursusHandler.cs", CURSUS_HANDLER),
            file("DataIntake/Handlers/OnderwijsproductHandler.cs", PRODUCT_HANDLER),
            file("Data/Services/OnderwijsproductService.cs", PRODUCT_SERVICE),
            file("DataIntake/Handlers/RoostercodeHandler.cs", ROOSTER_HANDLER),
            file("Data/Services/RoostercodeService.cs", ROOSTER_SERVICE),
        ])
    }

    fn intakes(rows: &[TableRow], table: &str) -> Vec<(String, bool)> {
        rows.iter().filter(|r| r.kind == "intake" && r.table == table).map(|r| (r.event.clone(), r.deletes)).collect()
    }

    #[test]
    fn each_table_names_its_entity_in_the_db_context_s_order() {
        let rows = rows();
        let tables: Vec<(&str, i64, &str)> = rows.iter().filter(|r| r.kind == "table").map(|r| (r.name.as_str(), r.entity, r.table.as_str())).collect();
        assert_eq!(tables, [("DeletedEntity", 1, "DeletedEntities"), ("Cursus", 1, "Cursussen"), ("Cursus", 2, "CursussenV2"), ("Onderwijsproduct", 2, "OnderwijsproductenV2"), ("Roostercode", 1, "Roostercodes")]);
    }

    #[test]
    fn events_fill_tables_through_their_handler_s_service() {
        let rows = rows();
        // A generic service writes one table; a service of its own every table its repositories name.
        // A generic handler names its event and entity in GenericHandler<Event, Fcdm, Model>.
        assert_eq!(
            intakes(&rows, "CursussenV2"),
            [("CursusChangedEvent".to_string(), false), ("CursusDeletedEvent".to_string(), true), ("CursusDeletedEvent".to_string(), true), ("OnderwijsproductChangedEvent".to_string(), false)]
        );
        let generic = rows.iter().find(|r| r.event == "CursusChangedEvent").unwrap();
        assert_eq!(generic.namespaces, ["Fontys.Messages.Onderwijs.Events.v4"]);
        assert!(rows.iter().all(|r| r.note.is_empty() || !r.note.contains("TEvent")), "{rows:#?}");
        assert_eq!(intakes(&rows, "OnderwijsproductenV2"), [("OnderwijsproductChangedEvent".to_string(), false)]);
        assert_eq!(intakes(&rows, "Roostercodes"), [("RoostercodeChangedEvent".to_string(), false)]);
        // A deleted event adds its row to DeletedEntities; no table is filled from nothing.
        assert_eq!(intakes(&rows, "DeletedEntities"), [("CursusDeletedEvent".to_string(), false), ("CursusDeletedEvent".to_string(), false)]);
        assert!(intakes(&rows, "Cursussen").is_empty());
        // An event named through an alias is in that namespace; a bare one in any the handler opens.
        let product = rows.iter().find(|r| r.event == "OnderwijsproductChangedEvent").unwrap();
        assert_eq!(product.namespaces, ["Fontys.Messages.Onderwijs.Events.v3"]);
        let rooster = rows.iter().find(|r| r.event == "RoostercodeChangedEvent").unwrap();
        assert_eq!(rooster.namespaces, ["Fontys.Messages.Hr.Events", "Fontys.Messages.Hr.Events.v4"]);
        // A table takes its domain from the events that write it.
        let domain = |t: &str| rows.iter().find(|r| r.kind == "table" && r.table == t).unwrap().domain.clone();
        assert_eq!((domain("Roostercodes"), domain("CursussenV2"), domain("Cursussen"), domain("DeletedEntities")), ("Hr".into(), "Onderwijs".into(), String::new(), String::new()));
    }

    /// A table's columns are its model class's, its base class's first; its key is Bk, or what HasKey names.
    #[test]
    fn a_table_has_its_model_s_columns_and_key() {
        let base = "namespace Fontys.Adapter.FcdmApi.Models\n{\n    public abstract class BaseEntity\n    {\n        [Key]\n        public string Bk { get; set; }\n\n        public DateTimeOffset LastModified { get; set; }\n    }\n}\n";
        let deleted = "namespace Fontys.Adapter.FcdmApi.Models\n{\n    public class DeletedEntity : BaseEntity\n    {\n        public string EntityName { get; set; }\n    }\n}\n";
        let cursus = "namespace Fontys.Adapter.FcdmApi.Models.V2\n{\n    public class Cursus : BaseEntity\n    {\n        public string Code { get; set; }\n\n        public string? Naam { get; set; }\n    }\n}\n";
        let files = [file("Data/FcdmDbContext.cs", CONTEXT), file("Models/BaseEntity.cs", base), file("Models/DeletedEntity.cs", deleted), file("Models/V2/Cursus.cs", cursus)];
        let rows = read_tables(&files);
        let deleted = rows.iter().find(|r| r.table == "DeletedEntities").unwrap();
        assert_eq!(deleted.columns, ["Bk: string", "LastModified: DateTimeOffset", "EntityName: string"]);
        assert_eq!(deleted.keys, ["Bk", "EntityName"]);
        let cursus = rows.iter().find(|r| r.table == "CursussenV2").unwrap();
        assert_eq!(cursus.columns, ["Bk: string", "LastModified: DateTimeOffset", "Code: string", "Naam: string?"]);
        assert_eq!(cursus.keys, ["Bk"]);
    }

    /// The adapter-fcdmapi repository's folder on this machine, if it is here.
    fn repository() -> Option<std::path::PathBuf> {
        let dir = std::env::var_os("ADAPTER_REPO").map(std::path::PathBuf::from).or_else(|| std::env::var_os("HOME").map(|h| std::path::Path::new(&h).join("Examples/src/adapter-fcdmapi")))?;
        dir.is_dir().then_some(dir)
    }

    fn sources(dir: &std::path::Path, root: &std::path::Path, out: &mut Vec<SourceFile>) {
        for entry in std::fs::read_dir(dir).unwrap().flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if path.is_dir() {
                if !matches!(name.as_str(), "bin" | "obj" | "Migrations") {
                    sources(&path, root, out);
                }
            } else if name.ends_with(".cs") {
                let rel = path.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/");
                out.push(SourceFile { path: rel, text: std::fs::read_to_string(&path).unwrap_or_default() });
            }
        }
    }

    #[test]
    fn the_repository_s_tables_are_read_with_the_events_that_fill_them() {
        let Some(repo) = repository() else {
            eprintln!("the adapter-fcdmapi repository is not on this machine: skipped");
            return;
        };
        let mut files = Vec::new();
        sources(&repo, &repo, &mut files);
        let rows = read_tables(&files);
        let tables: Vec<_> = rows.iter().filter(|r| r.kind == "table").collect();
        let expected = std::fs::read_to_string(repo.join("Fontys.Adapter.FcdmApi.Data/FcdmDbContext.cs")).unwrap().lines().filter(|l| l.trim_start().starts_with("public DbSet<")).count();
        assert_eq!(tables.len(), expected);
        let events = |t: &str| intakes(&rows, t).into_iter().map(|(e, _)| e).collect::<Vec<_>>();
        assert_eq!(events("Roostercodes"), ["RoostercodeChangedEvent"]);
        assert!(events("DeletedEntities").contains(&"AccountDeletedEvent".to_string()), "{:?}", events("DeletedEntities"));
        assert!(events("CursussenV2").contains(&"CursusDeletedEvent".to_string()), "{:?}", events("CursussenV2"));
        let deleted = tables.iter().find(|r| r.table == "DeletedEntities").unwrap();
        assert_eq!(deleted.keys, ["Bk", "EntityName"]);
        let filled = tables.iter().filter(|t| !intakes(&rows, &t.table).is_empty()).count();
        eprintln!("{filled} of {} tables filled from events", tables.len());
        for t in tables.iter().filter(|t| intakes(&rows, &t.table).is_empty()) {
            eprintln!("no event fills {}", t.table);
        }
        for r in rows.iter().filter(|r| r.kind == "note") {
            eprintln!("note: {} ({})", r.note, r.path);
        }
    }
}
