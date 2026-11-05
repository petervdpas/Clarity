//! The writer against the layout of the fcdm repository: neutral cases of
//! every rule here, and, when the repository is on this machine (the folder
//! FCDM_REPO names, else ~/Examples/src/fcdm), the classes, interfaces and
//! enums it has, written again and compared with its files.

use super::*;

fn object(domain: &str, name: &str, root: bool, version: i64) -> ObjectSpec {
    ObjectSpec { domain: domain.into(), name: name.into(), root, keyed: root, newtonsoft: true, version, summary: String::new(), obsolete: String::new() }
}

fn property(name: &str, kind: &str) -> PropertySpec {
    PropertySpec { name: name.into(), kind: kind.into(), nullable: false, type_name: String::new(), type_version: 0, initial: String::new(), computed: String::new(), obsolete: String::new() }
}

fn nullable(mut p: PropertySpec) -> PropertySpec {
    p.nullable = true;
    p
}

fn of(mut p: PropertySpec, type_name: &str, version: i64) -> PropertySpec {
    p.type_name = type_name.into();
    p.type_version = version;
    p
}

#[test]
fn a_child_of_three_values_takes_them_on_one_line() {
    let code = write_class(&object("Kitchen", "Step", false, 3), &[property("Ingangsdatum", "Moment"), nullable(property("Einddatum", "Moment")), property("LocatieBk", "BusinessKey")]);
    assert_eq!(
        code,
        "using System;\n\nnamespace Fontys.Fcdm.Kitchen.v3\n{\n    public class Step\n    {\n        public Step(DateTime ingangsdatum, DateTime? einddatum, string locatieBk)\n        {\n            Ingangsdatum = ingangsdatum;\n            Einddatum = einddatum;\n            LocatieBk = locatieBk;\n        }\n\n        public DateTime Ingangsdatum { get; init; }\n\n        public DateTime? Einddatum { get; init; }\n\n        public string LocatieBk { get; init; }\n    }\n}\n"
    );
}

#[test]
fn a_root_takes_its_business_key_first_a_parameter_per_line_and_has_an_interface() {
    let props = [property("Title", "TextValue"), nullable(property("Notes", "TextValue")), of(property("Steps", "HeldObjects"), "Step", 2)];
    let code = write_class(&object("Kitchen", "Recipe", true, 2), &props);
    assert!(code.starts_with("using System.Collections.Generic;\nusing Fontys.Fcdm.Kitchen.Interfaces.v2;\n\nnamespace Fontys.Fcdm.Kitchen.v2\n{\n    public class Recipe : IRecipe\n"), "{code}");
    assert!(code.contains("        public Recipe(\n            string bk,\n            string title,\n            string? notes,\n            IEnumerable<Step> steps)\n        {\n            Bk = bk;\n"), "{code}");
    assert!(code.contains("\n        public string Bk { get; init; }\n\n        public string Title { get; init; }\n"), "{code}");
    let interface = write_interface(&object("Kitchen", "Recipe", true, 2), &props);
    assert_eq!(
        interface,
        "using System.Collections.Generic;\nusing Fontys.Fcdm.Interfaces;\n\nnamespace Fontys.Fcdm.Kitchen.Interfaces.v2\n{\n    public interface IRecipe : IFcdmModel\n    {\n        public string Title { get; }\n\n        public string? Notes { get; }\n\n        public IEnumerable<Kitchen.v2.Step> Steps { get; }\n    }\n}\n"
    );
}

#[test]
fn enums_get_their_converters_computed_properties_are_ignored_and_others_versions_are_named() {
    let mut street = property("Adres", "TextValue");
    street.computed = "$\"{Straat} {Huisnummer}\"".into();
    let props = [of(property("Soort", "EnumChoice"), "Soort", 0), of(property("Oud", "HeldObject"), "Recept", 3), street];
    let code = write_class(&object("Kitchen", "Gang", false, 2), &props);
    assert!(code.starts_with("using System.Text.Json.Serialization;\nusing Newtonsoft.Json.Converters;\n\n"), "{code}");
    assert!(code.contains("        [System.Text.Json.Serialization.JsonConverter(typeof(JsonStringEnumConverter))]\n        [Newtonsoft.Json.JsonConverter(typeof(StringEnumConverter))]\n        public Soort Soort { get; init; }\n"), "{code}");
    assert!(code.contains("public v3.Recept Oud { get; init; }"), "{code}");
    assert!(code.contains("        [JsonIgnore]\n        [Newtonsoft.Json.JsonIgnore]\n        public string Adres => $\"{Straat} {Huisnummer}\";\n"), "{code}");
    assert!(!code.contains("adres"), "a computed property is no constructor parameter: {code}");
}

#[test]
fn an_added_property_starts_as_its_initial_value_and_obsolete_says_why() {
    let mut brin = nullable(property("BrinBk", "BusinessKey"));
    brin.initial = "null".into();
    let mut lesplaats = object("Kitchen", "Lesplaats", true, 1);
    lesplaats.obsolete = "Use the v2 \"Lesplaats\".".into();
    let code = write_class(&lesplaats, &[property("Code", "TextValue"), brin]);
    assert!(code.contains("    [Obsolete(\"Use the v2 \\\"Lesplaats\\\".\")]\n    public class Lesplaats : ILesplaats\n"), "{code}");
    assert!(code.contains("public Lesplaats(string bk, string code, string? brinBk = null)"), "{code}");
    assert!(code.starts_with("using System;\nusing Fontys.Fcdm.Kitchen.Interfaces;\n\n"), "{code}");
}

#[test]
fn an_enum_lists_its_values_with_their_numbers() {
    let values = [EnumValueSpec { name: "MEDEW".into(), number: 0 }, EnumValueSpec { name: "PARTNER".into(), number: 1 }];
    assert_eq!(
        write_enum(&EnumSpec { domain: "Hr".into(), name: "VoorkeurnaamCode".into() }, &values),
        "namespace Fontys.Fcdm.Hr\n{\n    public enum VoorkeurnaamCode\n    {\n        MEDEW = 0,\n        PARTNER = 1\n    }\n}\n"
    );
}

/// The fcdm repository's folder on this machine, if it is here.
fn repository() -> Option<std::path::PathBuf> {
    let dir = std::env::var_os("FCDM_REPO").map(std::path::PathBuf::from).or_else(|| std::env::var_os("HOME").map(|h| std::path::Path::new(&h).join("Examples/src/fcdm")))?;
    dir.is_dir().then_some(dir)
}

/// A file of the repository as text: no byte order mark, lines ending in `\n`.
fn file(repo: &std::path::Path, path: &str) -> Option<String> {
    let text = std::fs::read_to_string(repo.join(path)).ok()?;
    Some(text.trim_start_matches('\u{feff}').replace("\r\n", "\n"))
}

#[test]
fn the_repository_s_files_come_out_as_they_are() {
    let Some(repo) = repository() else {
        eprintln!("the fcdm repository is not on this machine: skipped");
        return;
    };
    let hr = |name: &str, kind: &str| property(name, kind);
    // Hr's v2 Medewerker, its interface, a v3 child and an enum.
    let medewerker = [
        hr("Medewerkernummer", "TextValue"),
        nullable(hr("Voorletters", "TextValue")),
        nullable(hr("Voornamen", "TextValue")),
        nullable(hr("Roepnaam", "TextValue")),
        hr("Geboortenaam", "TextValue"),
        nullable(hr("Tussenvoegsels", "TextValue")),
        of(hr("VoorkeurnaamCode", "EnumChoice"), "VoorkeurnaamCode", 0),
        nullable(hr("AchternaamPartner", "TextValue")),
        nullable(hr("TussenvoegselsPartner", "TextValue")),
        nullable(hr("VolledigeNaam", "TextValue")),
        hr("IsOverleden", "YesNo"),
    ];
    // Its interface gets the enum converters most interfaces have; the rest is as the file has it.
    let v2 = object("Hr", "Medewerker", true, 2);
    let interface = write_interface(&v2, &medewerker);
    let without = interface.replace("        [System.Text.Json.Serialization.JsonConverter(typeof(JsonStringEnumConverter))]\n        [Newtonsoft.Json.JsonConverter(typeof(StringEnumConverter))]\n", "").replace("using System.Text.Json.Serialization;\n", "").replace("using Newtonsoft.Json.Converters;\n", "");
    assert_eq!(Some(without), file(&repo, "Fontys.Fcdm.Hr/Interfaces/v2/IMedewerker.cs"));
    let locatie = [hr("Ingangsdatum", "Moment"), nullable(hr("Einddatum", "Moment")), hr("LocatieBk", "BusinessKey")];
    assert_eq!(Some(write_class(&object("Hr", "ArbeidscontractLocatie", false, 3), &locatie)), file(&repo, "Fontys.Fcdm.Hr/v3/ArbeidscontractLocatie.cs"));
    let values = ["MEDEW", "PARTNER", "MEDEW_PARTNER", "PARTNER_MEDEW"].iter().enumerate().map(|(i, n)| EnumValueSpec { name: n.to_string(), number: i as i64 }).collect::<Vec<_>>();
    assert_eq!(Some(write_enum(&EnumSpec { domain: "Hr".into(), name: "VoorkeurnaamCode".into() }, &values)), file(&repo, "Fontys.Fcdm.Hr/VoorkeurnaamCode.cs"));
}

/// Every file of the repository read: its domains, data objects, versions and enums, and the report of what was tidied.
#[test]
fn the_repository_reads_whole() {
    let Some(repo) = repository() else {
        eprintln!("the fcdm repository is not on this machine: skipped");
        return;
    };
    let mut files = Vec::new();
    fn walk(dir: &std::path::Path, root: &std::path::Path, out: &mut Vec<read::SourceFile>) {
        for entry in std::fs::read_dir(dir).unwrap().flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') || name == "bin" || name == "obj" {
                continue;
            }
            if path.is_dir() {
                walk(&path, root, out);
            } else if name.ends_with(".cs") || name.ends_with(".csproj") {
                let relative = path.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/");
                out.push(read::SourceFile { path: relative, text: std::fs::read_to_string(&path).unwrap_or_default() });
            }
        }
    }
    walk(&repo, &repo, &mut files);
    let rows = read::read_files(&files);
    let count = |kind: &str| rows.iter().filter(|r| r.kind == kind).count();
    let domains: std::collections::BTreeSet<&str> = rows.iter().filter(|r| r.kind == "domain").map(|r| r.domain.as_str()).collect();
    eprintln!("domains {domains:?}, objects {}, properties {}, enums {}, values {}, notes {}", count("object"), count("property"), count("enum"), count("value"), count("note"));
    for n in rows.iter().filter(|r| r.kind == "note") {
        eprintln!("  note{}: {} ({})", if n.renames { " (renames)" } else { "" }, n.note, n.path);
    }
    assert!(domains.contains("Hr") && domains.contains("Onderwijs"));
    assert!(count("object") > 70, "objects {}", count("object"));
    // A property whose kind fell through is a type the reader does not know yet.
    let unknown: Vec<String> = rows.iter().filter(|r| r.kind == "property" && r.property_kind == "HeldObject" && !rows.iter().any(|o| o.kind == "object" && o.name == r.type_name)).map(|r| format!("{}.{}: {}", r.note, r.name, r.type_name)).collect();
    assert!(unknown.is_empty(), "held objects that are no data object: {unknown:#?}");
}

/// Every class read from the repository, written again: how many come out as their file is, and how the others differ.
#[test]
fn the_repository_s_classes_round_trip() {
    let Some(repo) = repository() else {
        eprintln!("the fcdm repository is not on this machine: skipped");
        return;
    };
    let mut files = Vec::new();
    fn walk(dir: &std::path::Path, root: &std::path::Path, out: &mut Vec<read::SourceFile>) {
        for entry in std::fs::read_dir(dir).unwrap().flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') || name == "bin" || name == "obj" {
                continue;
            }
            if path.is_dir() {
                walk(&path, root, out);
            } else if name.ends_with(".cs") || name.ends_with(".csproj") {
                let relative = path.strip_prefix(root).unwrap().to_string_lossy().to_string();
                out.push(read::SourceFile { path: relative, text: std::fs::read_to_string(&path).unwrap_or_default() });
            }
        }
    }
    walk(&repo, &repo, &mut files);
    let rows = read::read_files(&files);
    let objects: Vec<&read::ReadRow> = rows.iter().filter(|r| r.kind == "object").collect();
    let newest = |name: &str, upto: i64| objects.iter().filter(|o| o.name == name && o.version <= upto).map(|o| o.version).max().unwrap_or(0);
    let (mut same, mut differ) = (0, Vec::new());
    for (i, o) in rows.iter().enumerate().filter(|(_, r)| r.kind == "object") {
        let props: Vec<PropertySpec> = rows[i + 1..]
            .iter()
            .take_while(|r| r.kind == "property" || r.kind == "note")
            .filter(|r| r.kind == "property")
            .map(|r| PropertySpec {
                name: r.name.clone(),
                kind: r.property_kind.clone(),
                nullable: r.nullable,
                type_name: if r.property_kind == "BusinessKey" { String::new() } else { r.type_name.clone() },
                type_version: if r.property_kind.starts_with("Held") { newest(&r.type_name, if r.type_version > 0 { r.type_version } else { o.version }) } else { 0 },
                initial: r.initial.clone(),
                computed: r.computed.clone(),
                obsolete: r.obsolete.clone(),
            })
            .collect();
        let spec = ObjectSpec { domain: o.domain.clone(), name: o.name.clone(), root: o.root, keyed: o.keyed, newtonsoft: rows.iter().any(|d| d.kind == "domain" && d.role == "model" && d.domain == o.domain && d.newtonsoft), version: o.version, summary: String::new(), obsolete: o.obsolete.clone() };
        if o.root {
            let at = if o.version > 1 { format!("{}/Interfaces/v{}/I{}.cs", o.path.split('/').next().unwrap(), o.version, o.name) } else { format!("{}/Interfaces/I{}.cs", o.path.split('/').next().unwrap(), o.name) };
            let interface = write_interface(&spec, &props);
            let disk = file(&repo, &at).unwrap_or_default();
            if interface == disk {
                same += 1;
            } else {
                let first = interface.lines().zip(disk.lines()).position(|(a, b)| a != b).unwrap_or_else(|| interface.lines().count().min(disk.lines().count()));
                differ.push(format!("{at}: line {}: wrote {:?}, file has {:?}", first + 1, interface.lines().nth(first).unwrap_or(""), disk.lines().nth(first).unwrap_or("")));
            }
        }
        let written = write_class(&spec, &props);
        let on_disk = file(&repo, &o.path).unwrap_or_default();
        if written == on_disk {
            same += 1;
        } else {
            let first = written.lines().zip(on_disk.lines()).position(|(a, b)| a != b).unwrap_or_else(|| written.lines().count().min(on_disk.lines().count()));
            differ.push(format!("{}: line {}: wrote {:?}, file has {:?}", o.path, first + 1, written.lines().nth(first).unwrap_or(""), on_disk.lines().nth(first).unwrap_or("")));
        }
    }
    eprintln!("{same} classes and interfaces as their file, {} differ, each for what its notes say", differ.len());
    // Every difference is a tidy the import reported for that file.
    let noted: Vec<&str> = rows.iter().filter(|r| r.kind == "note").map(|r| r.path.as_str()).collect();
    let unexplained: Vec<&String> = differ.iter().filter(|d| !noted.iter().any(|p| d.starts_with(&format!("{p}:")))).collect();
    assert!(unexplained.is_empty(), "differences no note explains: {unexplained:#?}");
}

#[test]
fn a_file_is_written_with_its_byte_order_mark_and_lf() {
    assert_eq!(with_mark("a\r\nb\n"), b"\xEF\xBB\xBFa\nb\n".to_vec());
    assert_eq!(with_mark("\u{feff}a\n"), b"\xEF\xBB\xBFa\n".to_vec(), "one mark, not two");
}
