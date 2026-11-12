# Encrypted storage

Status: not built. Any storage the application owns (a SQLite source, a file source, a file store, their backups) encrypted on disk, its key taken from a secret setting and never kept in the project.

## Code
- none. Designed in APP-CAPABILITIES-PLAN.md step 2. What it would build on: secret settings and the unconnected window ([settings-secrets](settings-secrets.md), [databases](databases.md)), the vault's scheme (`crates/clarity-vault`: Argon2id, AES-256-GCM per record), backups ([backups](backups.md)), file sources ([formats](formats.md)).
- Docs: none.

## Tests
- none.

## Open
- `encrypted "setting:name"` on a `sqlite` source, a file source and a `filestore`: NOT BUILT. While the secret is not kept, the storage does not open and the program shows the window where it is entered (`unconnectedNotKept`, `on unconnected`).
- `forget` on a secret setting (`databaseKey: secret, forget`): kept in memory for the run only, asked at every start, for every secret: NOT BUILT.
- Encrypted backups of any encrypted storage: NOT BUILT.
- `clarity encrypt project main` (change the key, encrypt or decrypt what exists), the same beside the source in the IDE's Settings, and re-encrypting when the setting changes in the program: NOT BUILT.
- Server databases encrypt on their server; the checker says so: NOT BUILT.
- A spike first: one SQLCipher file opened from Rust, Go, C#, C and the IDE (APP-CAPABILITIES-PLAN.md "Order" 2).
- Open questions (APP-CAPABILITIES-PLAN.md):
  - Go: modernc.org/sqlite has no encryption; cgo with SQLite3 Multiple Ciphers, or ncruces/go-sqlite3 (its own formats, which the IDE would need to read). To decide after the spike.
  - C#: SQLitePCLRaw `bundle_e_sqlcipher` (community-built), Zetetic's SQLCipher for .NET (paid), or SQLite3 Multiple Ciphers as Clarity.Runtime.Sqlite's native SQLite.
  - Whether file and file-store encryption is a shared crate (`clarity-crypt`) or part of `clarity-formats`; to decide in step 2.

## Conflicts
- none.

## Decisions
- 2026-11-10: each missing capability is added as a general concept on every stack, not a library one project writes. User: "I want you to make a plan for it, to add them in the most generic way possible so it improves the sort of apps clarity can create!"
- 2026-11-10: encryption is a property of storage, of any kind the application owns (APP-CAPABILITIES-PLAN.md "The idea").
- 2026-11-10: the key comes only from a secret setting; no new way of asking. A shared computer gets the `forget` option on the secret, not on the storage (plan: "one new option `forget` on secrets instead of `encrypted ask`").
- 2026-11-10: one SQLite file format on every stack and in the IDE, which reads the same database; candidate SQLCipher 4 through SQLite3 Multiple Ciphers where a stack bundles its own SQLite.
- 2026-11-10: files and file stores use the vault's scheme, not a second one; SQLite encryption lives in each runtime's own SQLite, not in a shared library.
- 2026-11-10: a forgotten password loses the data, as with the vault; the dialog says so once when the password is set.

## Was
- APP-CAPABILITIES-PLAN.md step 2, "Order" 2, "Where the code lives" (SQLite encryption), "Overlap with what exists" (encryption key, file encryption, encrypted backups), open questions (Go, C#)
- UNIMPLEMENTED.md 6 "Encrypted storage, mail, jobs" (encrypted storage)
- memory: app-capabilities-plan.md (step 2)
