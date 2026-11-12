# Mail

Status: not built. Mail servers as data sources of one Internet family: `smtp` to send (`send mail`), `imap` to read (`mails m in inbox`), written once in a shared Rust library.

## Code
- none. Designed in APP-CAPABILITIES-PLAN.md step 3. Planned places: `runtimes/rust-mail` (crate `clarity-mail`), `runtimes/rust-mail-ffi` (`clarity_mail.h`, `libclarity_mail`), C# NuGet `Clarity.Mail`, release tag `runtimes/rust-mail/v*`, the native library beside Clarity.UI's; Rust crates `lettre`, `imap`, `mail-parser`, `rustls`.
- What it builds on: `rest` sources with `auth` ([web-apis](web-apis.md)), secret settings ([settings-secrets](settings-secrets.md)), `files f in store` ([files-pictures](files-pictures.md)), formats for attachments ([formats](formats.md)).
- Docs: none.

## Tests
- none. Planned: a fake SMTP and IMAP server in the suites, offline; `conformance/mail.json`.

## Open
- Sending: `datasource outbox smtp "smtp://..."` with `auth basic "setting:..."` and `from "setting:..."`; `send mail to a subject t body b from source` with `cc`, `bcc`, `replyTo`, `html body`, `attach f` (a store's file, or bytes `named`), `header`; a reply with `ok`, `text`, `messageId` that never stops the code: NOT BUILT (step 3a).
- Reading: `datasource inbox imap "imap://..."`; `mails m in source` with `folder`, `where`, `order by`, `limit`; a mail's `id`, `from`, `to`, `cc`, `subject`, `date`, `text`, `html`, `messageId`, `inReplyTo`, `seen`, `attachments` (`read bytes of`); `where` on from, subject, date, seen, messageId sent as an IMAP search; bodies fetched only when read; `mark m seen`, `move m to "Archive"`, `delete m`: NOT BUILT (step 3b).
- TLS by default (`smtps://`, `imaps://`, STARTTLS), plain only to localhost, `trust "certs/bridge.pem"` for a local bridge: NOT BUILT.
- The `maillink` sign-in strategy and mail resets: NOT BUILT; see [signin](signin.md).
- Later on the same line, noted only: `pop3`; file stores on a server (`sftp`). Message streams (`tcp`, `mqtt`, `websocket`) belong to [hardware](hardware.md).
- Open questions: mail login with OAuth (Gmail, Outlook) now or later; Proton through Bridge needs only `basic`.

## Conflicts
- none.

## Decisions
- 2026-11-10: missing capabilities as general concepts on every stack. User: "I want you to make a plan for it, to add them in the most generic way possible so it improves the sort of apps clarity can create!"
- 2026-11-10: written once in a shared Rust library with a C interface, as formats. User: "shared Rust library, go with that"
- 2026-11-10: mail servers are data sources, the first of an Internet family with one auth line and one TLS rule; `mails m in` has the shape of `files f in store` on purpose.
- 2026-11-10: login is `auth basic` from a secret setting.
- 2026-11-10: Clarity's agent never sends mail itself; sending is the built program's (rule: the agent never executes).
- 2026-11-10: per-stack mail libraries (MailKit, libcurl) dropped for the one Rust library.

## Was
- APP-CAPABILITIES-PLAN.md step 3 (3a, 3b, "Both", "Later on the same line"), "Order" 3, "Where the code lives" (mail column), "Overlap with what exists" (`mails m in inbox`, tcp/mqtt/websocket), open questions (mail OAuth)
- UNIMPLEMENTED.md 6 "Encrypted storage, mail, jobs" (mail)
- memory: app-capabilities-plan.md (step 3)
