//! An OpenAPI description in the project (DESIGN 5.23, D2): a web API data
//! source names it with `openapi "forms.yaml"` (in the project's `apis`
//! folder) and picks the operations it uses with `pick`, and schemas of
//! their own with `pick schema`. Clarity writes `apis/<source>.clarity` from
//! it: an entity per schema the picks need, living in the web API, and an
//! action per picked operation, in plain Clarity. A new revision of the
//! description writes it again; a pick it no longer has is a warning.

use std::path::{Path, PathBuf};

use clarity_project::openapi;

const FORMS: &str = r##"openapi: 3.0.3
info:
  title: Formidable
  version: 2.1.0
servers:
  - url: https://forms.example.com/api
paths:
  /templates:
    get:
      operationId: listTemplates
      summary: Lists the templates
      parameters:
        - name: status
          in: query
          required: true
          schema: { type: string }
        - name: limit
          in: query
          schema: { type: integer }
      responses:
        "200":
          description: The templates
          content:
            application/json:
              schema:
                type: array
                items: { $ref: "#/components/schemas/Template" }
    post:
      operationId: createTemplate
      requestBody:
        content:
          application/json:
            schema: { $ref: "#/components/schemas/Template" }
      responses:
        "201":
          description: Made
          content:
            application/json:
              schema: { $ref: "#/components/schemas/Template" }
  /templates/{templateId}:
    get:
      operationId: getTemplate
      parameters:
        - name: templateId
          in: path
          required: true
          schema: { type: integer }
      responses:
        "200":
          description: One template
          content:
            application/json:
              schema: { $ref: "#/components/schemas/Template" }
    delete:
      operationId: deleteTemplate
      parameters:
        - { name: templateId, in: path, required: true, schema: { type: integer } }
      responses:
        "204": { description: Gone }
  /entries:
    get:
      operationId: listEntries
      responses:
        "200":
          description: Entries
          content:
            application/json:
              schema: { type: array, items: { $ref: "#/components/schemas/Entry" } }
components:
  schemas:
    Template:
      type: object
      properties:
        id: { type: integer }
        name: { type: string }
        createdAt: { type: string, format: date-time }
        status: { type: string, enum: [draft, published] }
        fieldCount: { type: integer }
        price: { type: number }
        tags: { type: array, items: { type: string } }
        owner: { $ref: "#/components/schemas/User" }
        active: { type: boolean }
    Entry:
      type: object
      properties:
        id: { type: integer }
        template: { type: integer }
        filledOn: { type: string, format: date }
    User:
      type: object
      properties:
        name: { type: string }
"##;

const PROJECT: &str = r#"project Forms
    stack go web desktop
    datasource main sqlite "data/forms.db"
    datasource forms rest "https://forms.example.com/api"
        auth bearer "env:FORMS_TOKEN"
        openapi "forms.yaml"
        pick "listTemplates", "getTemplate", "createTemplate"
"#;

const SCREEN: &str = "module screen
use forms

page Templates: window
    data templates: list<Template>
    button @loadCaption name load at 1,1 span 2 on click: loadClicked

code Templates
    handler loadClicked
        templates = list_templates(\"published\")
";

/// A project folder in the target directory, with the description and a
/// module of the developer's own that uses what is picked.
fn project(name: &str, description: &str, project: &str) -> PathBuf {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("openapi-{name}"));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("forms/apis")).unwrap();
    std::fs::write(root.join("forms.clarity"), project).unwrap();
    std::fs::write(root.join("forms/apis/forms.yaml"), description).unwrap();
    std::fs::write(root.join("forms/screen.clarity"), SCREEN).unwrap();
    std::fs::create_dir_all(root.join("forms/translations")).unwrap();
    std::fs::write(root.join("forms/translations/en.clarity"), "translation en\n    loadCaption \"Load\"\n").unwrap();
    root.join("forms.clarity")
}

#[test]
fn the_project_file_names_the_description_and_its_picks() {
    let settings = clarity_project::parse(PROJECT).unwrap();
    let forms = &settings.datasources[1];
    assert_eq!(forms.openapi.as_deref(), Some("forms.yaml"));
    assert_eq!(forms.picks, ["listTemplates", "getTemplate", "createTemplate"]);
    assert!(forms.pick_schemas.is_empty());
    assert!(clarity_project::validate(&settings).is_empty(), "{:?}", clarity_project::validate(&settings));
    // Written back as it was read.
    let written = clarity_project::write(Some(PROJECT), &settings).unwrap();
    assert_eq!(written, PROJECT);
    let more = PROJECT.replace("\"createTemplate\"\n", "\"createTemplate\"\n        pick schema \"Entry\"\n");
    let settings = clarity_project::parse(&more).unwrap();
    assert_eq!(settings.datasources[1].pick_schemas, ["Entry"]);
    assert_eq!(clarity_project::write(Some(&more), &settings).unwrap(), more);
    // Only a web API has a description.
    let wrong = PROJECT.replace("    datasource main sqlite \"data/forms.db\"\n", "    datasource main sqlite \"data/forms.db\"\n        openapi \"main.yaml\"\n");
    assert!(clarity_project::parse(&wrong).map(|s| clarity_project::validate(&s)).unwrap_or_default().iter().any(|p| p.contains("only a web API has an openapi description")));
}

#[test]
fn a_description_reads_alike_from_yaml_and_json() {
    let yaml = openapi::read(FORMS).unwrap();
    assert_eq!((yaml.title.as_str(), yaml.version.as_str()), ("Formidable", "2.1.0"));
    let ids: Vec<&str> = yaml.operations.iter().map(|o| o.id.as_str()).collect();
    assert_eq!(ids, ["listTemplates", "createTemplate", "getTemplate", "deleteTemplate", "listEntries"]);
    let names: Vec<&str> = yaml.schemas.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names, ["Template", "Entry", "User"]);
    // The same description written as JSON, in the same order.
    let json = openapi::json_of(FORMS).unwrap();
    assert!(serde_json::from_str::<serde_json::Value>(&json).is_ok(), "{json}");
    assert_eq!(openapi::read(&json).unwrap(), yaml);
}

#[test]
fn the_module_holds_what_the_picks_need() {
    let description = openapi::read(FORMS).unwrap();
    let written = openapi::module("forms", "forms.yaml", &description, &["listTemplates".into(), "getTemplate".into(), "createTemplate".into()], &[]);
    assert!(written.warnings.is_empty(), "{:?}", written.warnings);
    let text = &written.text;
    // Only the schema the picked operations take and give.
    assert!(text.contains("module forms\nsource forms\n"), "{text}");
    assert!(text.contains("entity Template\n"), "{text}");
    assert!(!text.contains("entity Entry") && !text.contains("entity User"), "{text}");
    for line in [
        "    name: text",
        "    created_at: datetime",
        "    status: TemplateStatus",
        "    field_count: integer",
        "    price: decimal(18, 6)",
        "    tags: list<text>",
        "    active: bool",
        "enum TemplateStatus\n    draft\n    published\n",
    ] {
        assert!(text.contains(line), "no {line:?} in\n{text}");
    }
    // What is not read yet is named, not left out silently.
    assert!(text.contains("// owner: a User, which is not read yet"), "{text}");
    // An action per picked operation, on the calls of the language.
    for line in [
        "action list_templates(status: text) -> list<Template>",
        "    let reply = http get \"/templates\" from forms query \"status\": status",
        "    return read list<Template> from json reply.text",
        "action get_template(template_id: integer) -> Template",
        "    let reply = http get \"/templates/{template_id}\" from forms",
        "action create_template(template: Template) -> Template",
        "    let reply = http post \"/templates\" from forms with json of template header \"Content-Type\": \"application/json\"",
        "        refuse \"forms createTemplate answered {reply.status}: {reply.text}\"",
    ] {
        assert!(text.contains(line), "no {line:?} in\n{text}");
    }
    assert!(!text.contains("delete_template") && !text.contains("list_entries"), "{text}");
    // An optional query parameter is named for the developer.
    assert!(text.contains("// limit (query, optional) is not passed yet"), "{text}");
}

#[test]
fn clarity_writes_the_module_and_the_project_checks() {
    let file = project("writes", FORMS, PROJECT);
    let warnings = clarity_project::refresh_apis(&file).unwrap();
    assert!(warnings.is_empty(), "{warnings:?}");
    let written = file.with_extension("").join("apis/forms.clarity");
    assert!(written.is_file());
    let project = clarity_project::load_modules(&clarity_project::module_dir(&file)).unwrap();
    let errors: Vec<&str> = project.diagnostics.iter().filter(|d| d.severity == clarity_spec::Severity::Error).map(|d| d.message.as_str()).collect();
    assert!(errors.is_empty(), "{errors:?}\n{}", std::fs::read_to_string(&written).unwrap());
    // Template lives in the web API: no migration covers it.
    assert!(project.in_web_api("Template"));
}

#[test]
fn a_new_revision_updates_the_entities() {
    let file = project("revision", FORMS, PROJECT);
    clarity_project::refresh_apis(&file).unwrap();
    let written = file.with_extension("").join("apis/forms.clarity");
    let first = std::fs::read_to_string(&written).unwrap();
    assert!(!first.contains("description: text"));
    // Version 2.2.0 gives a template a description and drops getTemplate.
    let revision = FORMS
        .replace("version: 2.1.0", "version: 2.2.0")
        .replace("        name: { type: string }\n        createdAt", "        name: { type: string }\n        description: { type: string }\n        createdAt")
        .replace("    get:\n      operationId: getTemplate", "    get:\n      operationId: fetchTemplate");
    std::fs::write(file.with_extension("").join("apis/forms.yaml"), revision).unwrap();
    let warnings = clarity_project::refresh_apis(&file).unwrap();
    assert_eq!(warnings, ["data source 'forms' picks 'getTemplate', which forms.yaml (Formidable 2.2.0) does not have"]);
    let second = std::fs::read_to_string(&written).unwrap();
    assert!(second.contains("    description: text"), "{second}");
    assert!(second.contains("Formidable 2.2.0"), "{second}");
    assert!(!second.contains("get_template"), "{second}");
    // Nothing changed: the file is left as it is.
    let modified = std::fs::metadata(&written).unwrap().modified().unwrap();
    clarity_project::refresh_apis(&file).unwrap();
    assert_eq!(std::fs::metadata(&written).unwrap().modified().unwrap(), modified);
}

/// What the picking shows: every operation and schema of the description,
/// whether the project picks it, and the schemas the picked operations need.
#[test]
fn the_picking_lists_the_description() {
    let file = project("listing", FORMS, PROJECT);
    let listing = clarity_project::api_listing(&file, "forms").unwrap();
    assert_eq!(listing.file.as_deref(), Some("forms.yaml"));
    assert_eq!((listing.title.as_str(), listing.version.as_str()), ("Formidable", "2.1.0"));
    let picked: Vec<&str> = listing.operations.iter().filter(|o| o.picked).map(|o| o.id.as_str()).collect();
    assert_eq!(picked, ["listTemplates", "createTemplate", "getTemplate"]);
    let delete = listing.operations.iter().find(|o| o.id == "deleteTemplate").unwrap();
    assert_eq!((delete.method.as_str(), delete.path.as_str(), delete.picked), ("delete", "/templates/{templateId}", false));
    let template = listing.schemas.iter().find(|s| s.name == "Template").unwrap();
    assert!(template.needed && !template.picked);
    let entry = listing.schemas.iter().find(|s| s.name == "Entry").unwrap();
    assert!(!entry.needed && !entry.picked);
    // A description chosen elsewhere is copied into the apis folder.
    let elsewhere = file.with_extension("").join("../other.json");
    std::fs::write(&elsewhere, openapi::json_of(FORMS).unwrap()).unwrap();
    assert_eq!(clarity_project::add_api_file(&file, &elsewhere).unwrap(), "other.json");
    assert!(file.with_extension("").join("apis/other.json").is_file());
    assert!(clarity_project::add_api_file(&file, &file).unwrap_err().contains("no OpenAPI"));
}
