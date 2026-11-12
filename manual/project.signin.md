# Signing in

Who signs in, and how.

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
| GitHub | the field of the GitHub login, or of the e-mail address it has verified | desktop |
| Microsoft | the field of the e-mail address of a work, school or personal account | desktop |
| OpenID provider | the field of the e-mail address or login it gives (Google, GitLab, Okta, Keycloak) | desktop |

A password is kept as an Argon2id hash, codes and tokens by their digests: nobody, the program included, reads them back. Give those fields the **Hashed** box in the entity editor (`hashed` in the entity). With **At least N characters** under Password, a shorter password is refused wherever it is set.

## Signing in with a provider

GitHub, Microsoft and any OpenID provider sign people in at the provider: the button opens the provider's page in the system's browser, the person signs in there, and the browser comes back to the program, which asks the provider who signed in. The program never sees their password. Tick the provider under **Strategies** and fill in its settings:

- **Client id**: the id of the app registered at the provider (a GitHub OAuth app, a Microsoft app registration, an OpenID client).
- **Secret setting**: the secret setting that holds the app's secret, such as `setting:githubSecret`, never the secret itself. GitHub needs one to sign in; give it its value in the program's settings.
- **Tenant** (Microsoft): the company's id or domain, `organizations` or `consumers`; any Microsoft account without it.
- **Issuer** (OpenID provider): the provider's address, such as `https://accounts.google.com`.
- **New people may register**: someone without a record signs in and gets one; off, only people already known sign in.

The field a provider finds people by is an e-mail field (its verified address) or a text field (the login name). In the project file:

```text
signin Member on desktop
    password email passwordHash
    github email
        client "Ov23li..."
        secret "setting:githubSecret"
        register
```

On the sign-in view the palette offers each provider's button: **Sign in with GitHub**, **Sign in with Microsoft**, **Sign in with your account**. A served program signs in with providers in a later step.

The computer account signs in by itself on the desktop when the computer's user has a record. With no record of the people yet, the first person who signs in with a password or the computer account becomes the first record.

## The sign-in window

People who sign in on the desktop or the web see a window of the kind **Sign in** first. It opens as a window of its own, at its size, without the app's menu or toolbar. It is built as every window is: the window holds a content, and the content shows the **sign-in views**, one at a time. Each view is one way of signing in, and its **Other way** links show the others, as on a website: "Use a recovery code", "Sign in with a password".

New window offers the sign-in window and makes it from the sign-in setup: a view per way that takes something typed (a password, a recovery code, a token), with the language dropdown, the fields, Keep me signed in, its button, the other ways and the refusal; a way that takes nothing typed, such as the computer's account, is a button on the first view. Design the window and the views as any other: their look, texts, logo and layout are yours.

The designer guides what they hold. On a sign-in view the palette offers only the sign-in parts, made from the setup, and the decoration:

- **Language**: the language dropdown, when the setup has **Language** ticked.
- A field per input of the strategies: **Name**, **Password**, **Recovery code**, **Token**.
- **Keep me signed in**, when the setup remembers people only when asked.
- A button per strategy: **Sign in with a password**, **Use this computer's account**, and so on.
- **Other way**: a link that shows another sign-in view.
- **Refusal**: where a refused try says why. Without one on the view, the refusal shows as a message.
- **Heading**, **Label**, **Picture**, **Separator** and **Link**, for the decoration.

The sign-in window itself holds its content and that decoration. Each sign-in part shows Clarity's own words, translated, unless you give it a caption. Its properties say what it takes: the input of a field, the strategy of a button, the view an other way shows, chosen from the setup. The checker says when a view lacks a field its button needs, or holds a part twice.

## On the splash

The app's splash screen can be the sign-in window instead: give it the **Sign-in** part (`signin`, in the Basic input palette). It draws itself from the project's strategies, in Clarity's own words: a name, a password and **Sign in** for Password; **Use a recovery code**, then a name and the code, for Recovery codes; a token field for Token; **Use this computer's account** for Computer account. A refused try shows inside it. In the designer it is a placeholder that names what it offers. `shows password, recovery` in the block names the strategies it shows, first the default (without it, every one that works where the program runs), `remember 30 days when asked` gives it a **Keep me signed in** box, and `language picker` a dropdown of the program's languages above its fields. The splash shows while the app starts, as a splash does, then stays open until someone signs in, and the main window opens. A project has the splash or a sign-in window, not both.

The splash can also hold a sign-in built by hand. A button signs in with a strategy by its click target, `signin password`, `signin recovery` or `signin token`, and takes what the strategy needs from the parts of the splash named after it: `name`, `password`, `code`, `token`. The checker says which part is missing.

A menu item or button with the click target `signout` signs out and shows the sign-in window again.

## Who signed in, in code

- `me` is the record of who signed in: `me.name`, `me.role`.
- `allow when me.role == Admin` on an action or a command, and **Allow** on a window, view or command of the frame, say who may run or open it. Anything else is refused, whatever a browser sends.
- `readable when` and `writable when` on an entity refuse records that are not the person's. They check; they do not filter: a list shows what the code's `where` gives.
- Setting a password is code that assigns the hashed field, in a window someone may open: `member.passwordHash = typed`. A changed password signs that person out everywhere else.
- `newRecoveryCodes(10)` makes codes to show the person once and keep in the codes field; `newToken()` makes a token.

## The command line

When the command line is ticked under **On**, a command signs in before it runs: by the computer account, with `--token` (or the environment variable `CLARITY_TOKEN`), or with `--user name` and the password typed without showing.

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

See also [The project file](project.file.md), [Entities and enums](data.entities.md) and [Functions and actions](code.functions.md). The full rules are in [the language reference](../docs/LANGUAGE.md#signing-in).
