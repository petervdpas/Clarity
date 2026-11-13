# Toasts

How to show a short message in a corner of the window that goes by itself, each kind with its own text, lifespan, level, icon and corner.

## What a toast is

A toast says something in passing: a record was saved, an import is done, a line was skipped. It shows in a corner of the window and goes by itself after its lifespan, or earlier by its small x. Your project declares each toast by name, and code shows it by that name, so one app can have `saved` that goes after 3 seconds at the bottom right and `imported` that stays 8 seconds at the top left.

```clarity
toast saved
    text @savedToast
    lifespan 3000
    level success
    icon "check"
    corner bottomRight

toast imported
    text @importedToast
    lifespan 8000
    corner topLeft
```

Code shows one by its name, and fills the slots of its text:

```clarity
code Card
    handler saveClicked
        save customer
        toast saved(name = customer.name)
```

with `savedToast "Saved {name}"` in the language files.

## What a toast says

- **Text:** a key, with its words in each language file. Slots in braces are filled where code shows the toast. A toast needs a text.
- **Lifespan:** how long it shows, in milliseconds, from 500 to 120000. Without one it shows 5000 milliseconds.
- **Level:** `info` (the default), `success`, `warning` or `error`, which sets its color. Even a warning or error toast goes by itself; for a message the person must close, write `message warning` or `message error`, which shows in Clarity's dialog.
- **Icon:** one of Clarity's icons, as a button's, before the text.
- **Corner:** `topRight` (the default), `topLeft`, `bottomRight` or `bottomLeft`. Toasts in the same corner stack, the oldest nearest the corner.

A plain `message @key` shows as a toast too, at the top right for 5 seconds. A toast that says what an AI agent changed has Undo and shows for 10 seconds.

## In the designer

The project tree has **Toasts**: each toast with its level and lifespan, and **+ New toast**, which asks its name and adds it with a text key of its own. A toast opens as a form: its text key and its words in the default language, its lifespan in milliseconds, its level, its icon and its corner, with a sample of how it shows. Each change writes one line of the declaration; **Delete** takes the toast away. An agent writes toasts as any declaration of a module.

## What the checker says

A toast without a text, a lifespan out of range, an icon Clarity does not know, two toasts with one name, a `toast` statement naming a toast the project does not have, and a slot the toast's text does not have are each an error where it is written. In a command, `toast` writes its text to standard error, as `message` does.
