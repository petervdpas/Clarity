# Signing in

Who signs in, and how.

## Turning it on

Open **Project settings** and find **Who is using it**. Tick **People sign in**, then choose:

- **People**: the entity whose records are the people, such as `Member`.
- **On**: where people sign in. A served program asks every browser first; the desktop and the command line ask only when ticked.
- **Strategies**: how people prove who they are, several at once. Each one names the fields of the entity it matches.
- **Session**, **Tries** and **Remember**: how long a sign-in lasts, how many failed tries per account and address before signing in waits, and how many days the desktop program remembers who signed in.

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

A password is kept as an Argon2id hash, codes and tokens by their digests: nobody, the program included, reads them back. Give those fields the **Hashed** box in the entity editor (`hashed` in the entity). With **At least N characters** under Password, a shorter password is refused wherever it is set.

The computer account signs in by itself on the desktop when the computer's user has a record. With no record of the people yet, the first person who signs in with a password or the computer account becomes the first record.

## The sign-in window

People who sign in on the desktop or the web see a window of the kind **Sign in** first. New window offers it; it starts with the **Sign-in** part across the window. Design it as any window: its look, texts and logo are yours.

The **Sign-in** part (`signin`, in the Basic input palette) draws itself from the project's strategies, in Clarity's own words: a name, a password and **Sign in** for Password; **Use a recovery code**, then a name and the code, for Recovery codes; a token field for Token; **Use this computer's account** for Computer account. A refused try shows inside it. The part keeps only its place and look; the project's sign-in settings say what it offers: `shows password, recovery` in the block names the strategies it shows, first the default (without it, every one that works where the program runs), and `remember 30 days when asked` gives it a **Keep me signed in** box. A project has one sign-in part, on the sign-in window or the splash.

The window can also be built by hand, for a layout of your own. A button signs in with a strategy by its click target, `signin password`, `signin recovery` or `signin token`, and takes what the strategy needs from the parts of the window named after it: `name`, `password`, `code`, `token`. The checker says which part is missing.

The app's splash screen can be the sign-in window instead: give it the sign-in part, or the parts and buttons of a hand-built one. It shows while the app starts, as a splash does, then stays open until someone signs in, and the main window opens. A project has one or the other, not both.

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

See also [The project file](project.file.md), [Entities and enums](data.entities.md) and [Functions and actions](code.functions.md). The full rules are in [the language reference](../LANGUAGE.md#signing-in).
