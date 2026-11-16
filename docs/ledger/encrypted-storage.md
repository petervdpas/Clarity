# Encrypted storage

Status: not built. Any storage the application owns (a SQLite source, a file source, a file store, their backups) encrypted on disk, its key taken from a secret setting and never kept in the project.

Progress: 0%

## Code
- none. What it would build on: secret settings and the unconnected window ([settings-secrets](settings-secrets.md), [databases](databases.md)), the vault's scheme (`crates/clarity-vault`: Argon2id, AES-256-GCM per record), backups ([backups](backups.md)), file sources ([formats](formats.md)).
- Docs: none.

## Tests
- none.

## Open
- `encrypted "setting:name"` on storage: NOT BUILT; see Design "The line".
- `forget` on a secret setting: NOT BUILT; see Design "Forget".
- Changing the key, encrypting or decrypting what exists: NOT BUILT; see Design "Changing the key".
- Encrypted backups: NOT BUILT; see Design "Where it applies".
- The spike that decides the SQLite format per stack: NOT BUILT; see Design "First".

## Design

### The line

```
datasource main sqlite "data/people.db"
    encrypted "setting:databaseKey"

filestore scans "data/scans"
    encrypted "setting:databaseKey"
```

- `encrypted "setting:name"` takes the key from a secret setting, and nothing else.
- While the secret is not kept, the storage does not open and the program shows the window where it is entered, as for a secret database connection (`unconnectedNotKept`, `on unconnected`).

### Forget

- `databaseKey: secret, forget` keeps the secret in memory for the run only, never in the system's store, so the window asks at every start.
- The option is on the secret, so it works for every secret (an API token too).

### Where it applies

- A `sqlite` source, a file source, a `filestore`, and a backup of any of them.
- Server databases encrypt on their server; the checker says so.

### How

- SQLite: one file format on every stack and in the IDE, which reads the same database (migrations, Import, backups). Candidate: SQLCipher 4, through SQLite3 Multiple Ciphers (MIT, an amalgamation) where a stack bundles its own SQLite. It lives in each runtime's own SQLite.
- Files and file stores: the vault's scheme (Argon2id for the password, AES-256-GCM per file under a fresh nonce), the same in every runtime and readable by the IDE.

### Changing the key

- `clarity encrypt project main` changes the key, or encrypts or decrypts what exists.
- The IDE does the same beside the source in the project's Settings.
- In the program, changing the secret setting in its settings window re-encrypts.
- A forgotten password loses the data; the dialog says so once when the password is set.

### First

A spike that opens one SQLCipher file from Rust, Go, C#, C and the IDE, before anything else.

### Open questions

- Go: modernc.org/sqlite has no encryption. Cgo with SQLite3 Multiple Ciphers (loses pure Go, complicates cross builds), or ncruces/go-sqlite3 (its own formats, which the IDE would need to read). To decide after the spike.
- C#: SQLitePCLRaw `bundle_e_sqlcipher` (community-built), Zetetic's SQLCipher for .NET (paid), or SQLite3 Multiple Ciphers as Clarity.Runtime.Sqlite's native SQLite.
- File and file-store encryption as a shared crate of its own (`clarity-crypt`) or part of `clarity-formats`.

## Conflicts
- none.

## Decisions
- 2026-11-10: each missing capability is added as a general concept on every stack, not a library one project writes. User: "I want you to make a plan for it, to add them in the most generic way possible so it improves the sort of apps clarity can create!"
- 2026-11-10: encryption is a property of storage, of any kind the application owns.
- 2026-11-10: the key comes only from a secret setting; no new way of asking. A shared computer gets the `forget` option on the secret, not on the storage (instead of `encrypted ask`).
- 2026-11-10: one SQLite file format on every stack and in the IDE, which reads the same database; candidate SQLCipher 4 through SQLite3 Multiple Ciphers where a stack bundles its own SQLite.
- 2026-11-10: files and file stores use the vault's scheme, not a second one; SQLite encryption lives in each runtime's own SQLite, not in a shared library.
- 2026-11-10: a forgotten password loses the data, as with the vault; the dialog says so once when the password is set.

## Was
- docs/APP-CAPABILITIES-PLAN.md step 2, "Order" 2, "Where the code lives" (SQLite encryption), "Overlap with what exists" (encryption key, file encryption, encrypted backups), open questions (Go, C#)
- UNIMPLEMENTED.md 6 "Encrypted storage, mail, jobs" (encrypted storage)
- memory: app-capabilities-plan.md (step 2)
