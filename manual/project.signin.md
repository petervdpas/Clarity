# Signing in

Who signs in, and how: the project file's `signin` block, its strategies and options, the sign-in window, and who signed in as code sees it.

## Turning it on

Click **+ Sign-in** in the project tree (once it is on, the tree shows **Sign-in** beside the app and the README). Tick **People sign in**, then choose:

- **People**: the entity whose records are the people, such as `Member`.
- **On**: where people sign in. A served program asks every browser first; the desktop and the command line ask only when ticked.
- **Strategies**: how people prove who they are, several at once. Each one names the fields of the entity it matches.
- **Session**, **Tries** and **Remember**: how long a sign-in lasts, how many failed tries per account and address before signing in waits, and how many days the desktop program remembers who signed in.
- **Language**: whether people may pick their language before they sign in: the sign-in views can hold the language dropdown, and the sign-in part shows it.

The project file then holds a block like this one:

```text
signin Member on desktop web
    password email passwordHash
        minimum 12
    recovery email recoveryCodes
    account login
```

## The strategies

| Strategy | Fields | Works on |
|---|---|---|
| Password | the field the name is found by, and a hashed field for the password | desktop, web, command line |
| Recovery codes | the same name field, and a hashed field for the codes | desktop, web, command line |
| Computer account | the field that holds the system user name | desktop, command line |
| Token | a hashed field for the token | desktop, web, command line |
| GitHub | the field of the GitHub login, or of the e-mail address it has verified | desktop, web |
| Microsoft | the field of the e-mail address of a work, school or personal account | desktop, web |
| OpenID provider | the field of the e-mail address or login it gives (Google, GitLab, Okta, Keycloak) | desktop, web |

A password is kept as an Argon2id hash, codes and tokens by their digests: nobody, the program included, reads them back. Give those fields the **Hashed** box in the entity editor (`hashed` in the entity). With **At least N characters** under Password, a shorter password is refused wherever it is set.

The computer account signs in by itself on the desktop when the computer's user has a record. With no record of the people yet, the first person who signs in with a password, the computer account or a provider becomes the first record.

## Signing in with a provider

GitHub, Microsoft and any OpenID provider sign people in at the provider: the button opens the provider's page in the system's browser, the person signs in there, and the browser comes back to the program, which asks the provider who signed in. The program never sees their password. Tick the provider under **Strategies** and fill in its settings:

- **Client id**: the id of the app registered at the provider (a GitHub OAuth app, a Microsoft app registration, an OpenID client).
- **Secret setting**: the secret setting that holds the app's secret, such as `setting:githubSecret`, never the secret itself. GitHub needs one to sign in; give it its value in the program's settings.
- **Tenant** (Microsoft): the company's id or domain, `organizations` or `consumers`; any Microsoft account without it.
- **Issuer** (OpenID provider): the provider's address, such as `https://accounts.google.com`. Its endpoints are found by discovery.
- **New people may register**: someone without a record signs in and gets one, made from the field the provider matches; off, only people already known sign in.

The field a provider finds people by is an e-mail field (its verified address) or a text field (the login name). In the project file:

```text
signin Member on desktop
    password email passwordHash
    github email
        client "Ov23li..."
        secret "setting:githubSecret"
        register
```

## The sign-in window

People who sign in on the desktop or the web see a window of the kind **Sign in** first. It opens as a window of its own, at its size, without the app's menu or toolbar. It is built as every window is: the window holds a content, and the content shows the sign-in views, one at a time; the first it names opens first. Each view is one way of signing in, and its **Other way** links show the others, as on a website: "Use a recovery code", "Sign in with a password".

New window offers the sign-in window and makes it from the sign-in setup: a view per way that takes something typed (a password, a recovery code, a token), with the language dropdown, the fields, Keep me signed in, its button, the other ways and the refusal; a way that takes nothing typed, such as the computer's account, is a button on the first view; and an other way between each two views. Design the window and the views as any other: their look, texts, logo and layout are yours.

On a sign-in view the palette offers only the sign-in parts, made from the setup, and the decoration (**Heading**, **Label**, **Picture**, **Separator** and **Link**). The sign-in window itself holds its content and that decoration. Each sign-in part shows Clarity's own words, translated, unless you give it a caption, and its properties say what it takes, chosen from the setup. The parts and their rules are in [Sign-in parts](parts.signin.md).

A project has at most one sign-in window, and only with a `signin` block. The sign-in window is the program's to show: nothing opens it ("SignIn is the sign-in window, which the program shows itself; nothing opens it").

## On the splash

The app's splash screen can be the sign-in window instead: give it the **Sign-in** part (`signin`, in the Basic input palette). It draws itself from the project's strategies, in Clarity's own words: a name, a password and **Sign in** for Password; **Use a recovery code**, then a name and the code, for Recovery codes; a token field for Token; **Use this computer's account** for Computer account. A refused try shows inside it. In the designer it is a placeholder that names what it offers. The block's `shows` line names the strategies it shows, `remember ... when asked` gives it a **Keep me signed in** box, and `language picker` a dropdown of the program's languages above its fields. The splash shows while the app starts, as a splash does, then stays open until someone signs in, and the main window opens. A project has at most one sign-in part.

The splash can also hold a sign-in built by hand. A button signs in with a strategy by its click target, `signin password`, `signin recovery` or `signin token` (`opens="signin password"` in the splash's `.cxaml`), and takes what the strategy needs from the parts of the splash named after its inputs: `name`, `password`, `code`, `token`. The checker says which part is missing.

A project has the splash or a sign-in window, not both: both at once is an error that says to keep one. People who sign in on the desktop or the web need one of the two: "people sign in on desktop, so the project needs a sign-in window: add a window of the kind signin, or sign-in parts on the app's splash".

A menu item or button with the click target `signout` signs out and shows the sign-in window again.

### `signin`

```text
signin Entity [on target target...]
    strategy field [field]
        minimum n
        client "..."
        secret "setting:name"
        tenant "..."
        issuer "https://..."
        register
    session n minutes|hours|days
    tries n per m minutes
    remember n days [when asked]
    shows strategy, strategy...
    language picker
```

| Line | Default | Meaning |
|---|---|---|
| `signin Entity on desktop web cli` | `web` alone without `on` | The entity of people, and the targets people sign in on. Each target once, and each a target of the stack. |
| `password field hashField` | | A name and a password: the person whose `field` (text or email) equals the name, with the password checked against the hashed field (Argon2id). The inputs `name` and `password` give them. |
| `minimum 12` | none | Indented under `password`, the strategy whose secret people choose: text assigned to its hashed field has at least that many characters, else the save is refused with Clarity's `secretTooShort`. |
| `recovery field codesField` | | A recovery code, once each: the codes are kept in the hashed field as their digests, and a sign-in uses one up. The inputs `name` and `code` give them. |
| `account field` | | The computer's own account: the person whose text `field` equals the system user name. Desktop and command line. |
| `token field` | | A token, kept hashed (SHA-256). The input `token` gives it. |
| `github field` | | GitHub, in the system's browser: the person whose `field` equals their GitHub login (a text field) or their verified primary e-mail address (an e-mail field). Indented under it: `client`, `secret` (GitHub needs it), `register`. |
| `microsoft field` | | A Microsoft account (work, school or personal), in the system's browser, matched by its e-mail address or login. Indented: `client`, `tenant` (`common` without it), `secret`, `register`. |
| `openid field` | | Any OpenID provider, in the system's browser, matched by its e-mail address or login. Indented: `issuer` (required), `client`, `secret`, `register`. |
| `session 8 hours` | 8 hours | How long a sign-in lasts, in minutes, hours or days. |
| `tries 5 per 15 minutes` | 5 per 15 minutes | Failed tries per account and address before signing in slows down. |
| `remember 30 days` | not remembered | On the desktop, the program keeps a sign-in that long in the system's key store. With `when asked` only when the person ticks Keep me signed in (Clarity's `signinRemember`): the sign-in part's box, or a sign-in view's `signinRemember`. Needs `desktop` on the `signin` line. |
| `shows password, recovery, account` | every strategy that works on the target, in the block's order | The strategies the sign-in part offers, in that order, the first being the default; only strategies the block names above it. |
| `language picker` | none | A dropdown of the program's languages, each by its own name, headed by Clarity's `languagePicker`; picking one shows every text in it. The sign-in part shows it above its fields, and a sign-in view may hold it (`signinLanguage`). |

A provider's secret is never written in the project file, only the secret setting that holds it. On a sign-in view a sign-in button of a provider shows Clarity's `signinGithub`, `signinMicrosoft` or `signinOpenid`.

The checker refuses, at the line:

- a block without a strategy ("name at least one strategy under 'signin', indented, like: password email passwordHash"), an unknown strategy (the message lists them) and a strategy written twice;
- a strategy with the wrong number of fields ("the password strategy names 2 fields of Member (match, secret), like: password email passwordHash"), and one that works on none of the block's targets ("the account strategy works on desktop, cli, not on web");
- a target the stack does not have ("the stack has no cli target to sign in on");
- `minimum` that is not under a strategy whose secret people choose ("the token strategy keeps no secret people choose, so it has no minimum");
- a provider setting under a strategy that is no provider, `tenant` on anything but `microsoft`, `issuer` on anything but `openid`, a `secret` that is not a `setting:` ("the secret is a secret setting, never the secret itself: secret \"setting:githubSecret\""), and each setting written twice;
- `remember` without `desktop` on the `signin` line, and `shows` naming a strategy the block does not.

And against the project:

- the entity must exist and be stored ("there is no entity Member for 'signin Member': write entity Member with the fields its strategies name");
- each field a strategy names is a field of the entity ("Member has no field email for the password strategy"), of a type the strategy takes (text or email for a name, text for a secret);
- a secret field is `hashed` ("passwordHash keeps the password strategy's secret, so it is hashed: passwordHash: text, hashed"), and a field a strategy looks people up by is not;
- a salted password hash or a list of codes is not `unique` ("passwordHash is hashed with a salt, so no two are equal: take 'unique' away");
- a provider names its `client`, an OpenID provider its `issuer`, GitHub its `secret`, and that secret is a secret setting of the project ("add githubSecret: secret to the settings");
- a strategy that can make the first account (password, account, the providers) makes it from what was typed, the other fields at their defaults: a required field without a default is an error that names it ("Member's role is required and has no default, so the password strategy could not make the first account: give role a default, or take 'required' away").

## Who signed in, in code

```clarity
entity Member
    name: text(80), label @memberNameLabel
    email: email, unique, required, label @memberEmailLabel
    passwordHash: text, hashed, label @memberPasswordHashLabel
    role: Role, default Plain, label @memberRoleLabel
    readable when me.role == Admin or member.id == me.id
    writable when me.role == Admin
```

```clarity
action promote(member: Member)
    allow when me.role == Admin
    effects db.write
    member.role = Admin
    save member
```

- `me` is the record of who signed in, in handlers, actions, data events and commands: `me.name`, `me.role`. It cannot be set. A function, which takes values only, does not read it ("'me' is not allowed in a function; make it an action"), and neither does an engine. Without `signin` it is an error: "me is who signed in: write signin and the entity of people in the project file, like: signin Member".
- `allow when <condition>` on an action or a command, before its statements, and **Allow** (`allow="<condition>"`) on a window, a view and a command of a menu, toolbar or ribbon, say who may run or open it. The condition is a bool and reads `me`. The program refuses what is not allowed with Clarity's `notAllowed`, whatever a browser sends.
- `readable when <condition>` and `writable when <condition>` on an entity, with its record named as in its data events (`member` for `Member`), each once: the program refuses a find or a save of a record that does not fit with Clarity's `notYours`. They check; they do not filter: a list shows what the code's `where` gives, so code writes `where n.owner == me`.
- `hashed` on a text field: assigning text stores its hash (`member.passwordHash = typed`); code never reads it and no part shows it. Setting a password is code that assigns the hashed field, in a window someone may open. A changed password signs that person out everywhere else.
- `newToken()` gives a new random token as text, and `newRecoveryCodes(10)` that many recovery codes as lines of text, to show the person once and assign to the hashed field.
- A refused sign-in says Clarity's `signinRefused`, and too many tries `signinTooManyTries`.

## The command line

When the command line is ticked under **On**, a command signs in before it runs: by the computer account, with `--token` (or the environment variable `CLARITY_TOKEN`), or with `--user name` and the password typed without showing (asked with Clarity's `cliPassword`).

## People from Clarity's command line

On a computer without a browser at hand, such as a server, `clarity users` changes the people of a project whose people live in a SQLite database it keeps:

| Command | Does |
|---|---|
| `clarity users club.clarity list` | Lists the people by the field they are found by |
| `clarity users club.clarity add ann@example.com` | Adds a person, and asks their password when people sign in with one |
| `clarity users club.clarity password ann@example.com` | Gives a new password, asked without showing it; the project's minimum holds |
| `clarity users club.clarity token ann@example.com` | Gives a new token and shows it once |
| `clarity users club.clarity codes ann@example.com` | Gives ten new recovery codes and shows them once |
| `clarity users club.clarity remove ann@example.com` | Removes the person |

`--stdin` after `add` or `password` reads the password from standard input, for scripts. People in a server database are changed in the program itself.

See also [The project file](project.file.md), [Sign-in parts](parts.signin.md), [Entities and enums](data.entities.md), [Settings](data.settings.md) and [Functions and actions](code.functions.md).
