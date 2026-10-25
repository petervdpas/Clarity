# Your first project

A short walk from an empty folder to a running application: a task entity, a window with a form and a table, Preview and Run.

## 1. Create the project

On the **Project** tab, click **New**. Choose where the project file goes and give it a name, such as `tasks.clarity`. The folder `tasks` next to it must be new or empty. With a Projects folder set in **Clarity settings**, **Folders**, New only asks for the name, `tasks`, and makes the project there.

The New project dialog asks for the stack: a backend, a UI and one or more targets. The Build tools row says whether this machine has what the stack needs. Click **Create project**.

Clarity writes three things: the project file, a first module `tasks/tasks.clarity`, and a `.gitignore` that keeps the generated `out/` and `data/` folders out of version control. The module holds an app and its main window:

```clarity
module tasks

app Tasks
    title @tasksTitle
    main Main

page Main: window
    title @mainTitle
```

The project file has one SQLite database, `main`, in `data/tasks.db`. See [The project file](project.file.md).

## 2. Add an entity

In the project tree, under Data, click **+ New entity** and call it `Task`. The entity editor opens. Add two fields with **Add field**: `title` of type `text(80)`, ticked Required, and `done` of type `bool` with default `false`. See [Entities](data.entities.md).

A new entity needs a migration step before the application can be built. Click **Settings** on the Project tab; under Migrations, click **Create step 1**. See [Migrations](data.migrations.md).

## 3. Design the window

Open `Main` from the tree and go to the **Design** tab. On the right, under The window, add two pieces of data: `task` of entity `Task`, and `tasks` of entity `Task` with list ticked.

In the sidebar's Parts tab, drag a **Form** from Containers onto the window and bind it to `task`. The Parts tab now lists the fields of `task`: drag Title and Done into the form, and each arrives as a label and a bound input. Drag **Table of tasks** below the form, and tick its columns. Drag a **Button**, caption it `Add`, name it `add` and tick "Enter clicks it". Under Events, click **Wire** next to click. Clarity writes an empty handler `addClick` and opens the Code panel at it. Fill it in and click **Save**:

```clarity
code Main
    handler addClick
        save task
        reload tasks
        task = new Task
```

The window is now text like this; see [Windows](design.windows.md) and [Parts](design.parts.md):

```clarity
page Main: window
    title @mainTitle
    data task: Task
    data tasks: list<Task>

    form name form1 at 1,1 span 12 rows 2 bind task
        label @titleLabel name label1 at 1,1 span 3 for title
        textbox name title at 4,1 span 6 bind task.title
        label @doneLabel name label2 at 1,2 span 3 for done
        checkbox name done at 4,2 span 3 bind task.done
    button @addButtonCaption name addButton at 11,3 span 2 default on click: addClick
    table name table1 at 1,4 span 12 rows 6 bind tasks columns title, done
```

## 4. Preview and Run

The **Preview** tab shows the window as it will look, filled with sample data. Handlers do not run there.

Click **Run**. Clarity generates the application into `tasks/out`, builds it and starts it; the Output panel shows the build. Type a task, press Enter, and it appears in the table. See [Run and Build](project.run-build.md).
