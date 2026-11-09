# Data Modelling

The design goal of this project, kept for whoever works on it next.

## Goal

Maintain the Fontys Canonical Data Model (FCDM) in one place: its domains,
their data objects with a class definition per version, the enums, the
events, the tables the adapter keeps them in and the API versions that
make views on them. From
that model the app writes what the developers would otherwise write by
hand.

## Scope: the app stops at exports

The app writes into an export folder:

- the fcdm classes, interfaces and enums, and the events of the messages projects
- the NuGet projects (csproj, Testing)
- the adapter entities and the DbContext lines
- FcdmApi-vN.json per API version
- a Swagger preview

A developer places those files in the right repository. The existing
pipeline (migrations, view SQL, scaffolding, controllers, clients) stays as
it is.

## Data model

A project is a repository. There are two:

- **FCDM** (the fcdm repository) holds the blueprints, by domain.
- **FcdmApi** (the adapter-fcdmapi repository) holds the adapter's tables, by the same domains, and the API versions.

The entities:

- **Project**: a repository's model. Source and target projects come later.
- **Domain**: one namespace (`Fontys.Fcdm.Hr`) of a project, with its model and messages packages and versions. Algemeen holds DeletedEntity.
- **DataObject**: a root (business key, interface) or a child held by another.
- **ClassDefinition**: a blueprint of a data object in FCDM. Version 1 is the domain's own namespace, N its `vN` namespace. Version N of a domain holds every data object at its newest version up to N.
- **Property**: per class definition, in order: values, enums, held objects and lists, business keys; nullable, initial, computed, obsolete.
- **EnumType / EnumValue**: per domain, never versioned.
- **EventDefinition / EventField**: per data object. Base events carry one class definition; others have fields. A namespace stays as published, because NServiceBus routes by it. **Legacy** marks an event of the old shape (Arbeidrelaties, ToekomstigeArbeidrelaties under Medewerker).
- **AdapterTable / AdapterColumn**: a table of the adapter in FcdmApi (`CursussenV2`), its class (`Fontys.Adapter.FcdmApi.Models.V2.Cursus`), its columns, and its blueprint: the class definition its mapper fills it from. A table without a mapper has no blueprint.
- **ApiVersion**: a version of an API project (`FcdmApi-v5.json`, version `5.23`).
- **ApiView**: the view an API version makes on a table (`SELECT viewProperties FROM tableName`, named as the entity): class, property names, key and the roles that may read it. A table is in an API version by its view there; roles and relations may differ per version.
- **ApiRelation / ApiEnum**: per view, its relations (relatesTo, relatedByMany, relatedByOne) and the int columns the API shows as enums.

## Where the data comes from

The app is the source. The fcdm repository and the adapter (its DbContext,
models, mappers and five API versions) were read in once; from here on the
model is kept in the app and written out to the repositories, never read
back. The real model lives in the local database, `data/modelling.db`
(tracked in git).

## The workspace

A tree of projects on the left (a tree of levels): their domains, data
objects and enums, and their API versions. A data object holds its class definitions, event
definitions and tables; a table holds its views. The middle is the window's
content: it shows the editor of what is picked in the tree, one at a time.

- A project opens in the project editor, a domain in the domain editor.
- A class definition opens in the object editor: its properties, class, interface and the tables filled from it.
- An event opens in the event editor, an enum in the enum editor.
- A table opens in the table editor: its class, blueprint and columns, and its views with their names, roles, relations and enum columns; **Add view** puts it in an API version.
- An API version (under API versions of its project) opens in the API editor: its views in order and its `FcdmApi-vN.json`, which **Write** puts into the adapter folder (setting `adapterFolder`) where the generator reads it. **New API version** starts as a copy of the newest.

The diagram view (slide-up, footer) shows the class diagram or the ERD of a
domain version (a table per class definition a table is filled from); the
version dial stops at the domain's newest version, and **Arrange** lays the
boxes out in rows by their links (the Fcdm engine's `arrangeBoxes`), which
you can drag on from there. The class diagram and the ERD share each data
object's position.

## Order of work

1. Projects, tables and views, API versions and FcdmApi-vN.json (built: read in once, edited and written by the app)
2. Writing the C# into a chosen folder
3. Adapter entities, DbContext lines and a migration report
4. csproj and Testing projects
5. Swagger preview
6. Events, then a versioning guard

Later, the mapping stage: applications (sources and targets), their
entities and fields, field mappings onto object versions, lineage and
coverage. Never call them systems: the whole is the integration platform.

## Rules

- Stack: Rust desktop. Build only on this stack.
- The engine in `engines/fcdm` writes the C#. Everything else is Clarity code in the app.
- The repositories' files are never copied into this repository.
