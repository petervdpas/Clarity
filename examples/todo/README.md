# Todo

A task list with the people who finish the tasks. Each task has a title, a
done switch and the person who finished it; each person has a name and a
colour. The data is kept in a SQLite database, and each person's own
settings are kept on their machine.

## What you see and can do

A splash screen shows while the app starts: the logo, the name, a tagline,
the app's version and what is loading. Then the Main window opens.

- Main: a search box and a table of tasks, edited in place. The search keeps the tasks with every word typed in their title. Add makes a task to type over, the trash removes one. Done tasks show only while the setting says so. **Users** opens the Users window.
- Users: a table of people. Picking or adding one slides out the person editor with the name and a colour picker, saved with **Save** (`Ctrl+S`). A person who finished tasks cannot be removed; any other is removed after a question asked from the code.
- Preferences: a dialog with the language, the theme and whether done tasks show.

The app's menu has File (Users, Language, Theme, Preferences, Quit with
`Ctrl+Q`), and a ribbon with buttons for Users, Language and Theme. The
Users window has a menu and ribbon of its own.

## Stack

C#, desktop. An example builds only for the stack its project file names.

## What it shows

- A splash screen (`Welcome.cxaml`), shown for 2.5 seconds, whose status the app's `onstart` handler sets.
- An app icon (`icon "todo.svg"`) and version (`version "1.0.0"`), shown on the splash as `{app.version}`.
- Settings per person: language, theme and `showDone`, edited in a dialog.
- A question in code (`ask ... then deleteConfirmed`) before a delete.
- Two themes, Light and Dark, with the splash screen's own colours and styles.
- Three languages.
- Migrations that change the model over time: three yes/no fields for who finished a task gave way to a Person entity.

## Files

- `../todo.clarity`: the project file (title, icon, version, stack, output folder, the two themes, the SQLite data source).
- `todo.clarity`: the Person and Task entities, the settings, and the code behind the windows, the person editor and the app.
- `themes.clarity`: the Light and Dark themes.
- `design/`: `Todo.cxaml` (the app with its menu, ribbon and splash), `Welcome.cxaml` (the splash screen), `Main.cxaml` and `MainView.cxaml` (the tasks), `Users.cxaml` and `UsersView.cxaml` (the people, with the slide-out), `PersonEditor.cxaml` (a person's name and colour), `Preferences.cxaml` (the settings dialog).
- `resources/todo.svg`: the app's icon and the splash picture.
- `translations/`: English, Dutch and German.
- `migrations/main/`: six steps: tasks, who finishes a task, finished by a person, a colour for each person, search on tasks, and names in camelCase.
