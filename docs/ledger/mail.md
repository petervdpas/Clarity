# Mail

Status: not built. Mail servers as data sources of one Internet family: `smtp` to send (`send mail`), `imap` to read (`mails m in inbox`), written once in a shared Rust library.

Progress: 0%

## Code
- none. What it builds on: `rest` sources with `auth` ([web-apis](web-apis.md)), secret settings ([settings-secrets](settings-secrets.md)), `files f in store` ([files-pictures](files-pictures.md)), formats for attachments ([formats](formats.md)).
- Docs: none.

## Tests
- none.

## Open
- Sending: NOT BUILT; see Design "Sending".
- Reading: NOT BUILT; see Design "Reading".
- TLS and `trust`: NOT BUILT; see Design "Both".
- The `maillink` sign-in strategy and mail resets: NOT BUILT; see [signin](signin.md) Design.

## Design

### The sources

```
datasource outbox smtp "smtp://127.0.0.1:1025"
    auth basic "setting:mailLogin"
    from "setting:mailAddress"

datasource inbox imap "imap://127.0.0.1:1143"
    auth basic "setting:mailLogin"
```

### Sending

```clarity
let sent = send mail to site.privacyEmail subject @erasureSubject body letter from outbox
if not sent.ok
    message @notSent(why = sent.text)
```

- `send mail to address subject text body text from source`, with optional `cc`, `bcc`, `replyTo`, `html body`, `attach f` and `header "Name": value`.
- `attach` takes a file of a file store, or bytes with a name (`attach csv of requests named "requests.csv"`, any format of [formats](formats.md)).
- The reply is like an HTTP reply: `ok`, `text` (why not), `messageId` (stored to match replies later). It never stops the code.

### Reading

```clarity
for m in mails in inbox folder "INBOX" where m.inReplyTo == request.messageId
    request.status = Answered
    request.reply = m.text
    save request
```

- `mails m in source` with `folder`, `where`, `order by`, `limit`: the shape of `files f in store`.
- A mail has `id`, `from`, `to`, `cc`, `subject`, `date`, `text`, `html`, `messageId`, `inReplyTo`, `seen` and `attachments` (each with `name`, `size`, and `read bytes of`).
- `where` on `from`, `subject`, `date`, `seen` and `messageId` goes to the server as an IMAP search; the rest is filtered after fetching.
- Bodies and attachments are fetched only when code reads them.
- `mark m seen`, `move m to "Archive"`, `delete m`.

### Both

- TLS by default (`smtps://`, `imaps://`, or STARTTLS on `smtp://` and `imap://`); plain only to localhost.
- A local bridge with its own certificate names it: `trust "certs/bridge.pem"`.
- Login is `auth basic` from a secret setting.
- The agent never sends mail itself; sending is the built program's.

### Where the code lives

- `runtimes/rust-mail` (crate `clarity-mail`, release tag `runtimes/rust-mail/v*`), used directly by the Rust stack through a clarity-runtime feature.
- `runtimes/rust-mail-ffi` (`include/clarity_mail.h`, `clarity_mail_api()`, `libclarity_mail` beside Clarity.UI's library) for C#, Go, C and C++; C# NuGet `Clarity.Mail`.
- Rust crates `lettre`, `imap`, `mail-parser`, `rustls`, vendored for offline builds; safe for threads, the problem text kept per thread (see [background-jobs](background-jobs.md)).
- What a new C-interface library touches, as Clarity.Formats did: its release workflow, the -ffi build in `release-clarity-ui.yml`, `checks.yml`, `.github/scripts/stage-clarity-ui.sh`, `release-csharp-runtime.yml`; `BASE_LIBRARIES` and the wiring in `crates/clarity-gen`; `runtimes/c/clarity.cmake`, `runtimes/cpp/clarity.cmake`; `Clarity.slnx`; `clarity libraries`.
- Tests: a small fake SMTP and IMAP server in the suites, offline; `conformance/mail.json`.

### Later on the same line

Noted so the shape fits them: `pop3`; a file store on a server (`filestore shared sftp "sftp://..."`). Message streams (`tcp`, `mqtt`, `websocket`) belong to [hardware](hardware.md).

### Open questions

- Mail login with OAuth (Gmail, Outlook) now or later; Proton through Bridge needs only `basic`.

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
- docs/APP-CAPABILITIES-PLAN.md step 3 (3a, 3b, "Both", "Later on the same line"), "Order" 3, "Where the code lives" (mail column), "Overlap with what exists" (`mails m in inbox`, tcp/mqtt/websocket), open questions (mail OAuth)
- UNIMPLEMENTED.md 6 "Encrypted storage, mail, jobs" (mail)
- memory: app-capabilities-plan.md (step 3)
