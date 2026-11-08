# Data Modelling

The design goal of this project, kept for whoever works on it next.

## Goal

Maintain the Fontys Canonical Data Model (FCDM) in one place: its domains,
their data objects with a version per change, the enums, the events, the
tables the adapter keeps them in and the API versions that read them. From
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

- **Domain**: one namespace (`Fontys.Fcdm.Hr`), with its model and messages packages and versions.
- **DataObject**: a root (business key, interface) or a child held by another.
- **ObjectVersion**: 1 is the domain's own namespace, N its `vN` namespace. Version N of a domain holds every data object at its newest version up to N.
- **Property**: per version, in order: values, enums, held objects and lists, business keys; nullable, initial, computed, obsolete.
- **EnumType / EnumValue**: per domain, never versioned.
- **EventType / EventField**: base events carry one object version; others have fields. A namespace stays as published, because NServiceBus routes by it.
- **AdapterTable**: a table the adapter keeps. One data object can have several (`Cursussen`, `CursussenV2`); each records its adapter entity (`Models` is 1, `Models.V2` is 2) with that entity's columns and key. A table of the adapter's own (DeletedEntities, Roostercodes) has no data object; it shows on the ERD of each domain whose events fill it (Roostercodes under Hr, DeletedEntities under every domain with deleted events).
- **TableIntake**: an event that fills a table. Tables are filled from events only: a data-intake handler takes the event and writes through a service (one table, or every table its repositories name). A deleted event deletes a row and adds one to DeletedEntities.

## Where the data comes from

- **Import fcdm** reads the fcdm repository (folder setting `fcdmFolder`). Inconsistencies are tidied and reported; a change to a public name is marked for review.
- **Read tables** reads the adapter-fcdmapi repository (folder setting `adapterFolder`): the DbContext's DbSets, the model classes and the data-intake handlers with their services. Import fcdm reads them too when the folder is set.
- The real FCDM lives in the local database, `data/modelling.db` (tracked in git). The source repositories are only read; nothing is written into them.

## The workspace

A tree of domains, data objects, versions, enums and events on the left.
A version opens in the object editor (slide-out, right), with its class,
interface and adapter tables. The diagram view (slide-up, footer) shows the
class diagram or the ERD of a domain version; the version dial stops at the domain's newest version, and **Arrange** lays the boxes out in rows by their links (the Fcdm engine's `arrangeBoxes`), which you can drag on from there. The class diagram and the ERD share each data object's position.

## Order of work

1. Table names and ERD (built: tables, the events that fill them, enums on the diagrams)
2. API versions and FcdmApi-vN.json
3. Adapter entities, DbContext lines and a migration report
4. csproj and Testing projects
5. Swagger preview
6. Events, then a versioning guard

Later, the mapping stage: applications (sources and targets), their
entities and fields, field mappings onto object versions, lineage and
coverage. Never call them systems: the whole is the integration platform.

## Rules

- Stack: Rust desktop. Build only on this stack.
- The engine in `engines/fcdm` writes and reads the C#. Its tests compare with the fcdm and adapter-fcdmapi repositories when they are on the machine; their files are never copied into this repository.
