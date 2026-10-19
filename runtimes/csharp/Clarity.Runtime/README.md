# Clarity.Runtime

The runtime of C# applications that Clarity generates: what the generated code stands on.

- Records and their storage, on SQLite, PostgreSQL, MySQL and SQL Server
- Queries, and the migration steps a project's data dictionary writes
- Values shown and read in the person's language and format
- Contract checks and refusals with their texts

Generated applications use it; it is not meant to be called by hand. Its behavior is checked against the cases every Clarity stack shares (`conformance/*.json` in the Clarity repository).
