# Your first project

A short walk from an empty folder to a running application: a task entity, a window whose view holds a form and a table, Preview and Run.

## 1. Create the project

On the **Project** tab, click **New**. The New project dialog asks for the project's **Name**, such as `tasks`, and the **Folder** it goes in. The folder starts as the Projects folder of **Clarity settings**, **Folders** (else the folder of the open project); **Browse...** chooses another one in Clarity's file browser (see [Opening a project](start.overview.md#opening-a-project)). The line under the folder shows what you get as you type, `Creates /home/you/projects/tasks.clarity`, or why it cannot be made there: a `tasks.clarity` or a `tasks` folder that is there already. The project file goes in the folder, beside a folder of the same name for its modules.

The dialog also asks for the stack: a **Backend** and one or more **Targets** (Desktop, Web, CLI). The windows of a new project are always drawn by Clarity.UI, so there is no UI to choose. The Build tools row says whether this machine has what the stack needs. Click **Create project**.

Clarity writes the project file, a first module `tasks/tasks.clarity`, the app, the main window and the view it shows in `tasks/design`, the translations, and a `.gitignore` that keeps the generated `out/` and `data/` folders out of version control. The module starts as one line, `module tasks`; entities and code go there. The app is `tasks/design/Tasks.cxaml`, which names the main window:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<App class="tasks.Tasks" title="@tasksTitle" main="Main">
</App>
```

The main window is a file of its own, `tasks/design/Main.cxaml`; its `class` says it is the window `Main` of module `tasks`. Its content shows the view `MainView`, where the window's parts go:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<Window class="tasks.Main" title="@mainTitle">
  <Content name="main" at="1,1" span="12">
    <View name="MainView" />
  </Content>
</Window>
```

The view starts empty, in `tasks/design/MainView.cxaml`:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<View class="tasks.MainView">
</View>
```

The project file has one SQLite database, `main`, in `data/tasks.db`. See [The project file](project.file.md).

## 2. Add an entity

In the project tree, under Data, click **+ New entity** and call it `Task`. The entity editor opens. Add two fields with **Add field**: `title` of type `text(80)`, ticked Required, and `done` of type `bool` with default `false`. See [Entities](data.entities.md).

A new entity needs a migration step before the application can be built. Click **Settings** on the Project tab; under Migrations, click **Create step 1**. See [Migrations](data.migrations.md).

## 3. Design the view

Open `MainView` from the tree and go to the **Design** tab. On the right, under The window, add two pieces of data: `task` of entity `Task`, and `tasks` of entity `Task` with list ticked.

In the sidebar's Parts tab, drag a **Form** from Containers onto the view and bind it to `task`. Drag a **Textbox** into the form and set its Bind to `task.title`, and a **Checkbox** with Bind `task.done`, each with a **Label** before it. Drag a **Table** below the form, set its Bind to `tasks`, and tick its columns. Drag a **Button**, caption it `Add`, name it `add` and tick "Enter clicks it". Under Events, click **Wire** next to click. Clarity writes an empty handler `addClick` into the module's file and opens the Code panel at it. Fill it in and click **Save**:

```clarity
code MainView
    handler addClick
        save task
        reload tasks
        task = new Task
```

The designer has written the view into `tasks/design/MainView.cxaml`, which now reads like this; see [Windows](design.windows.md) and [Parts](design.parts.md):

```xml
<?xml version="1.0" encoding="UTF-8"?>
<View class="tasks.MainView">
  <Data name="task" type="Task" />
  <Data name="tasks" type="list&lt;Task&gt;" />
  <Form name="form1" at="1,1" span="12" rows="2" bind="task">
    <Label name="label1" at="1,1" span="3" caption="@titleLabel" for="title" />
    <Textbox name="title" at="4,1" span="6" bind="task.title" />
    <Label name="label2" at="1,2" span="3" caption="@doneLabel" for="done" />
    <Checkbox name="done" at="4,2" span="3" bind="task.done" />
  </Form>
  <Button name="addButton" at="11,3" span="2" caption="@addButtonCaption" default="true" onclick="addClick" />
  <Table name="table1" at="1,4" span="12" rows="6" bind="tasks" columns="title, done" />
</View>
```

The view's code stays in the module, `tasks/tasks.clarity`, in the `code MainView` block above. `Main` shows the view and its parts, so Preview and Run show them in the main window.

## 4. Preview and Run

The **Preview** tab shows the window as it will look, filled with sample data. Handlers do not run there.

Click **Generate**: Clarity writes the application's source into `tasks/out`. Then click **Run**: it builds that source and starts it; the Output panel shows the build. Type a task, press Enter, and it appears in the table. See [Run and Build](project.run-build.md).
