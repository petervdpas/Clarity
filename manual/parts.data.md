# Data parts

The parts of the Data group: tables, lists, trees, charts, trends and graphs of the window's data, and the progress bar, busy indicator and status item that show how things stand.

## Showing a list

Drag a part from the **Data** group of the Parts palette onto the window, then pick what it shows under **Bound to** in its properties. A table, list, tree, tree table, chart and graph show a list the window holds (`<Data name="orders" type="list&lt;Order&gt;" />`); a trend, a progress bar and a status item show one number. The properties offer only the data and fields that part accepts.

Inside a tab page, a pane, a panel, a form, a section, a step or a scroll box, a list, table, tree or tree table has no border of its own: the container frames it. **Frame** (`frame`) gives it one anyway. On the window it always has one.

**Striped rows** (`striped`) draws every other row (the 2nd, the 4th, ...) with a faint stripe of the theme's text color, in a light theme and a dark one; a chosen row or the row under the pointer shows as without it. A table, a list, a tree and a tree table have it; a list of cards has no stripes.

## Binding

`bind` connects a part to the window's data:

- A `table`, `list`, `tree`, `treetable`, `chart` or `graph` binds a list: `bind="orders"`. Its `columns` or `show` name fields of the listed entity.
- An input or `text` binds one field of a single record, `bind="customer.name"`, or a plain value by its name, `bind="shown"`. The type must be one the part accepts: a checkbox needs a `bool`. A plain `decimal` shows the digits it has.
- A `form` binds a single record: `bind="customer"`.

What the person types goes into the data, and handlers read and change the data by name. A part that cannot be bound, such as a label, may not have `bind`. An input that is not bound yet is allowed while designing.

## Choosing and opening rows

`select` runs when another row is chosen, and `open` on a double click or Enter. Rows of a part with `on open` show a hand under the pointer and say how to open them (Clarity's word `openRow`), so people see they lead somewhere. Code reads the chosen row as the part's `selected` (`grid.selected`), the record itself, and sets it to choose a row: the first row that is the record given (a stored one by its id, a record by its fields).

## Reordering rows

A `list`, `table`, `tree` or `treetable` lets the person put its rows in their own order when it has `reorder` and an integer field of the listed entity, which keeps the order. It is an option: without it the rows stay where the data has them. In the designer it is the **Reorder by the person** checkbox, with the field it is kept in.

```clarity
entity Task
    title: text, label @title
    position: integer
```

```xml
<!-- todo/design/TasksView.cxaml -->
<View class="todo.TasksView">
  <Data name="tasks" type="list&lt;Task&gt;" />
  <List name="taskList" at="1,1" span="4" rows="10" bind="tasks" reorder="position" show="title" onreorder="tasksMoved" />
</View>
```

```clarity
code Tasks
    handler loadTasks
        tasks = find t in Task order by t.position

code TasksView
    handler tasksMoved
        for t in tasks
            save t
```

- **Moving a row.** The person drags a row with the pointer, and a line shows where it lands; or chooses a row and presses Alt+Up or Alt+Down. In a tree a drop on the middle of a row puts the dragged row under it, as its last row; Alt+Left puts a row after its parent, a level up, and Alt+Right puts it under the row before it. A row takes the rows under it along. Every renderer moves alike (`conformance/reorder.json`).
- **What it changes.** The list in the window's data takes the new order, and every row gets its number in the field: its place at its level, counted from 1, or from the number `from` gives (`reorder by chapter from 0`, for a book whose prologue is chapter 0). In a tree the moved row also gets its new parent. A move that would change no number or parent is no move: nothing happens.
- **`on reorder`** runs after the move, with the moved row chosen (`taskList.selected`). Saving is up to the handler, as with a table that edits. Load the rows in their order: `order by t.position`.
- **Undo.** A move and what its `on reorder` saved are one edit, which Ctrl+Z takes back, unless the part has `no undo`.
- **A table** that reorders has no `sortable` and no `order by`: its rows keep the person's order. With `page`, a row moves on its page by dragging, and to the page before or after with Alt and the arrows.
- **In the designer** the rows are samples, and do not move.

The checker refuses a `reorder` field the entity does not have or that is no integer ("the order is kept in an integer field"), a reordering part whose rows are records, which are never stored ("build the list in code"), and a reordering table that is `sortable` or has `order by`. A part that reorders without `on reorder` is a warning, since its order is lost when the data is read again.

## Dragging rows between parts

A list, table or tree takes rows dragged onto it from another one of the window with `on drop from fragmentList: linkFragment`, one per part it takes rows from (in a `.cxaml` file, **Takes rows from** in its properties). The person drags a row out of `fragmentList` and lets it go on a row of this part: the dragged row is chosen in `fragmentList` and the row it landed on in this part (beside the rows, its chosen row stays), then the handler runs and reads both as `fragmentList.selected` and `chapterList.selected`, to link them. The rows themselves stay where they are.

A memo takes rows too (`on drop from fragmentList: fragmentDropped`): the caret goes where the row was let go in its text, then the handler runs, so `manuscriptBox.insert(fragmentList.selected.text)` puts the fragment in there.

The checker refuses a part name that is not in the window ("there is no part 'x' in this window to drag rows from"), a part that has no rows ("rows come from a list, table or tree"), and the part itself ("a part's own rows move with `reorder by`, not `on drop from` itself").

## The parts

### `table`

Rows of a list under column headers, with cells that can be edited, links, sorting, pages, and buttons that add and remove rows.

```xml
<Table name="name" at="col,row" span="n" rows="n" bind="list" columns="field, field" widths="{...}" edit="field" switches="field" links="[...]" resizable="true" sortable="true" add="true" remove="true" confirm="true" striped="true" order="{...}" reorder="field" page="50" frame="true" noUndo="true" onselect="handler" onopen="handler" onchange="handler" onreorder="handler" onadd="handler" onremove="handler" />
```

In Clarity's words the same table reads `table name albums at 1,2 span 12 rows 8 bind albums columns title 240, artist, year 60 resizable sortable order by year desc`.

| Property | Type | Default | What it does |
|---|---|---|---|
| `bind` | a list of records | none | The rows. |
| `columns` | fields, each with an optional width | none | The columns, in the order written; **Move up** and **Move down** beside a ticked column change it. |
| `edit` | columns | none | Columns whose cells are inputs; `as switch` makes a bool column a switch. |
| `link` | column: handler, ... | none | Columns whose cells are links that run a handler. |
| `resizable` | yes or no | no | The person drags the header edges to resize the columns. |
| `sortable` | yes or no | no | A click on a header sorts by that column. |
| `order by` | a column, `desc` | the list's order | The order the rows show in when the window opens. |
| `page` | 1 to 1000 | all rows | The rows shown so many at a time. |
| `add` | yes or no | no | An Add button above the header, at the right, runs `on add`. |
| `remove` | yes or no, `confirm` | no | A trash button on every row runs `on remove`; `confirm` asks first. |
| `reorder by` | an integer field, `from n` | none | The person puts the rows in their own order; see Reordering rows. |
| `striped`, `frame`, `no undo` | yes or no | no | See above; `no undo` takes Ctrl+Z away from its edits. |

Events: `select`, `open`, `change` (a cell was edited), `reorder`, `add`, `remove`.

```xml
<Table name="grid" at="1,2" span="12" rows="8" bind="tasks" columns="title, done" edit="done" onchange="taskChanged" />
```

```clarity
code Tasks
    handler taskChanged
        save grid.selected
```

#### Columns

- A width in pixels follows a field: `title 240`. A column without one shares the width that is left. Widths are 20 to 2000 pixels. On the design canvas, dragging the right edge of a header sets the width, and a double click there takes it away. In a `.cxaml` file the widths are `widths="{&quot;title&quot;:240}"`.
- `link chapter: toChapter` makes a column's cells links, in the theme's accent and underlined, with a hand under the pointer: a click on one chooses its row and runs the handler, which finds the row as the table's `selected` (`placeTable.selected.chapter`). Several columns are written `link chapter: toChapter, layer: toLayer`. In the properties these are **Link handlers**. A link column is one the table shows and does not edit.
- `resizable`: the widths the person gives are remembered for them, per window and table, and a double click on an edge gives the designed widths back.
- `sortable`: a click on a header sorts the rows by that column, a second click the other way. Text sorts ignoring case, numbers as numbers, an enum or `ref` by the label it shows, and an empty value comes last either way. Sorting changes only how the rows show: a row keeps its place in the data, so `grid.selected` and every event name the record the person sees, and `save grid.selected` saves it.
- `order by field [desc]`: the order the rows show in when the window opens, as in queries. The field must be one of the `columns`.

#### Editing in a table

`edit` names the columns whose cells are inputs, so a row can be changed where it is shown (**Edited in the table** in the properties).

- A `bool` column holds a checkbox, or a switch when it is written `as switch`: `edit title, done as switch`. An enum column is a dropdown of its values, and a `ref` column a dropdown of the records it can point at, as a dropdown in a form shows them. Dates are picked with a date picker, as in a date input. A moment (`datetime`) is a day from the date picker and a time typed on a 24-hour clock (`14:30`). Text and numbers are edited in place, like a text box or number input. A color column shows its swatch and is edited with a color input.
- An edited column is one of the `columns`, of one of these types: text, `email`, `url`, `color`, `integer`, `decimal`, `money`, `bool`, `date`, `datetime`, an enum or a `ref`. Pictures and durations are not edited in a cell.
- Only a value that differs is an edit: a checkbox or dropdown when it changes, a text or number cell when it is left or Enter is pressed. An edit changes the row on screen. Then the row is selected and `on change` runs, so `grid.selected` is the changed record. Saving it is up to the handler, as with a form: `save grid.selected`. When the record breaks a rule, `save` refuses it as anywhere else.

#### Adding and removing rows

With `add` an Add button stands above the header at the right, and with `remove` every row has a trash button in a last column of its own (**Add button above it** and **Trash button on every row** in the properties). The Add button runs `on add`; a trash button picks its row and runs `on remove`, which reads the row as `table.selected`. Insert and Delete do the same from the keys, and a read-only or disabled table runs neither. The handlers do the adding and deleting, so each needs its handler.

```xml
<!-- modelling/design/ClassEditor.cxaml -->
<View class="modelling.ClassEditor">
  <Data name="editedClass" type="ModelClass" />
  <Data name="properties" type="list&lt;Property&gt;" />
  <Table name="propertyTable" at="1,1" span="6" rows="6" bind="properties" columns="name, type" edit="name, type" add="true" remove="true" onadd="propertyAdded" onremove="propertyRemoved" onchange="propertyChanged" />
</View>
```

```clarity
code ClassEditor
    handler propertyAdded
        let p = new Property
        p.owner = editedClass
        save p
        properties = find x in Property where x.owner == editedClass

    handler propertyRemoved
        delete propertyTable.selected
        properties = find x in Property where x.owner == editedClass
```

With `remove confirm` (**Ask first**) the trash button and Delete ask first, in Clarity's dialog: "Remove {row}?" with the row's first column, and Remove and Keep; `on remove` runs on Remove only. For a question of its own (a dialog page with more in it, say) the `on remove` handler asks in code instead, with `ask` or `open`.

The button says Clarity's word `addRow` ("Add") with the plus icon, and the trash button's tooltip is `removeRow` ("Remove"); the question is `removeRowAsk` and its other button `keepRow`. Each language file gives them its own words, as `addRow "Toevoegen"` and `removeRow "Verwijderen"`.

#### Pages

`page 50` shows the rows 50 at a time (1 to 1000), with a bar below the table to go to the first, previous, next and last page and the rows it shows, as `51-100 / 230`. The rows are ordered first, then paged. The table keeps its page when its rows come again, goes back to the first page when a header click sorts it, and shows the page of a row that code selects. The bar's buttons say Clarity's words `firstPage`, `previousPage`, `nextPage` and `lastPage`. Every renderer pages alike (`conformance/table-pages.json`).

The checker refuses:

- an edited column that is not one of the columns, or of a type a cell cannot edit (its message says "a table edits text, numbers, bool, dates, colors, enum and ref columns"), and a switch on a column that is no `bool` or is not edited;
- edits of a list of records, which are never stored ("edit them in parts bound to a record");
- a link on a column the table does not show, and an `order by` on one;
- `add` without `on add`, and `remove` without `on remove`;
- a page of fewer than 1 or more than 1000 rows, and a column narrower than 20 or wider than 2000 pixels.

A table that edits without `on change` is a warning: its edits are lost when the data is read again.

### `list`

The rows of a list, one line each, or as small cards.

```xml
<List name="name" at="col,row" span="n" rows="n" bind="list" show="field, field" cards="true" reorder="field" striped="true" frame="true" noUndo="true" onselect="handler" onopen="handler" onreorder="handler" />
```

| Property | Type | Default | What it does |
|---|---|---|---|
| `bind` | a list of records | none | The rows. |
| `show` | fields | the record's text | The fields each row shows (**Shown fields**, in their order with Move up and Move down). |
| `cards` | yes or no | no | Each row a small card: the first field of `show` its title, the others lines below it, in the theme's `part card` and `part card_line` looks (see [Themes](themes.md)). |
| `reorder by` | an integer field | none | See Reordering rows. |
| `striped`, `frame`, `no undo` | yes or no | no | See above. |

Events: `select`, `open`, `reorder`.

```xml
<List name="taskList" at="1,1" span="4" rows="10" bind="tasks" show="title, kind" cards="true" onselect="taskPicked" />
```

The checker refuses `cards` without `show` ("write 'show title, kind cards'") and a field of `show` the listed entity does not have.

### `tree`

A list of records as a tree: rows under their parents, each with a toggle that shows or hides the rows under it. All rows start closed; the toggle's words are Clarity's `expandRow` and `collapseRow`.

```xml
<Tree name="name" at="col,row" span="n" rows="n" bind="list" childList="field" show="field" foldButtons="true" reorder="field" striped="true" frame="true" noUndo="true" onselect="handler" onopen="handler" onreorder="handler" />
```

| Property | Type | Default | What it does |
|---|---|---|---|
| `bind` | a list of records | none | The rows. |
| `children` | a list field of the rows' own kind | none | Arranges the rows by their parent (in a `.cxaml` file `childList`). |
| `key`, `parent` | two text or two integer fields | none | Arranges the rows by key instead (`rowKey`, `parentKey`). |
| `show` | fields | the row's text | What each row shows. |
| `fold buttons` | yes or no | no | Expand all and Collapse all above its rows (**Expand and collapse all**); code turns them on and off with `tree.foldButtons`. |
| `reorder by` | an integer field | none | See Reordering rows. |
| `striped`, `frame`, `no undo` | yes or no | no | See above. |

Events: `select`, `open`, `reorder`.

The entity has one reference to its own kind, a row's parent, and a list field of its own kind that stands for the rows under it:

```clarity
entity Folder
    name: text(80), required
    size: integer
    parent: ref Folder
    children: list<Folder>
```

```xml
<!-- files/design/FilesView.cxaml -->
<View class="files.FilesView">
  <Data name="folders" type="list&lt;Folder&gt;" />
  <Tree name="folderTree" at="1,1" span="4" rows="10" bind="folders" childList="children" />
  <TreeTable name="folderTable" at="5,1" span="8" rows="10" bind="folders" columns="name, size" widths="{&quot;name&quot;:200}" childList="children" />
</View>
```

```clarity
code Files
    handler loadFolders
        folders = find f in Folder order by f.name
```

- A row whose parent is in the list shows under it; any other row shows at the top. Each level keeps the order of the list. A loop of parents is broken at its row that comes first in the list.
- `folderTree.selected` is the record picked, as for a table, and selecting a row in code (`folderTree.selected = folder`) opens the rows above it. Every renderer arranges the rows alike (`conformance/trees.json`).

#### By key

A tree can also be arranged by key: `key` names the field that names a row and `parent` the field that holds the key of the row it shows under, text or whole numbers of the same type. The rows need no reference to their own kind, so code builds one tree of different things as a list of records; a stored record's `id` names it too. In the designer they are **Key** and **Parent**, offered when the list's rows have text or integer fields.

```xml
<Tree name="nav" at="1,1" span="4" rows="20" bind="nodes" rowKey="key" parentKey="parent" show="label" />
<TreeTable name="grid" at="5,1" span="8" rows="20" bind="nodes" rowKey="key" parentKey="parent" columns="label, kind" widths="{&quot;label&quot;:200}" />
```

A row shows under the first row whose key its parent field holds; any other row shows at the top. Empty text names no row, so a row whose parent is empty is at the top. Rows keep the order of the list, and a loop of parents is broken as above.

#### A tree of levels

A tree can show records of several entities, each level the records of one entity under the row of the level around it, with `<Level>` elements inside the tree in its `.cxaml` file. It binds nothing: its rows are its levels'.

```xml
<Tree name="modelTree" at="1,2" span="6" rows="12">
  <Level entity="Project" show="name" order="name">
    <Level entity="Domain" under="project" show="name" order="name">
      <Level group="@enumsNode">
        <Level entity="EnumType" under="domain" show="name" order="name" />
      </Level>
    </Level>
  </Level>
  <Select entity="Project" handler="projectPicked" />
  <Select entity="EnumType" handler="enumPicked" />
</Tree>
```

- `entity` names the records of a level, `under` the field of those records that refers to the row of the level around it (none at the top), `show` what a row shows (a field, or a text key such as `@classNode` whose slots are fields; without it the entity's first text field, else its id), and `order` the field its rows are ordered by, with `descending="true"` from high to low.
- `group` is a heading, a text key, shown when the levels inside it have rows.
- `<Select entity="Domain" handler="domainPicked" />` runs that handler with the record when a row of that entity is picked: `handler domainPicked(d: Domain)`.
- The rows load when the window opens, and again where code says `modelTree.reload()`.

The checker refuses a tree of levels without a name or with a `bind`, a level whose entity does not exist, an `under` field that does not refer to the level around it (or one on a top level, or none below the top), a `show` or `order` field the entity does not have, and a `Select` of an entity the tree does not show.

The checker refuses, for a tree arranged by `children`: a field the entity does not have, one that is no list of the rows' own kind, and an entity without exactly one `ref` to its own kind ("needs one field 'ref Folder' to know a row's parent"); for a tree by key: `children` together with `key` and `parent`, only one of the two, and fields that are not text or integer or not of the same type. A tree with neither is refused too: "a tree names the list of its own kind its rows have, as in 'children children', or the key and parent fields of its rows, as in 'key id parent parentId'".

### `treetable`

A table whose rows are a tree: the columns of a table under their headers, the first one indented, with each row's toggle. It is arranged as a tree is, by `children` or by `key` and `parent`, with the same checks.

```xml
<TreeTable name="name" at="col,row" span="n" rows="n" bind="list" childList="field" columns="field, field" widths="{...}" resizable="true" foldButtons="true" add="true" remove="true" confirm="true" reorder="field" striped="true" frame="true" noUndo="true" onselect="handler" onopen="handler" onreorder="handler" onadd="handler" onremove="handler" />
```

| Property | Type | Default | What it does |
|---|---|---|---|
| `bind`, `children` or `key` and `parent` | as for a tree | none | The rows and how they are arranged. |
| `columns` | fields, each with an optional width | none | The columns, the first one indented. |
| `resizable` | yes or no | no | The person resizes the columns (**Resizable columns**). |
| `fold buttons` | yes or no | no | Expand all and Collapse all above its rows. |
| `add`, `remove` | yes or no | no | The Add button and the trash buttons, as on a table. |
| `reorder by` | an integer field | none | See Reordering rows. |
| `striped`, `frame`, `no undo` | yes or no | no | See above. |

Events: `select`, `open`, `reorder`, `add`, `remove`.

```xml
<TreeTable name="folderTable" at="5,1" span="8" rows="10" bind="folders" columns="name, size" childList="children" foldButtons="true" />
```

### `chart`

Bars, a line or a pie of a list's rows grouped by a field, each group given a number; or a list of numbers drawn as they are.

```xml
<Chart name="name" at="col,row" span="n" rows="n" bind="list" chart="bar|line|pie" by="field" measure="{...}" points="n" max="n" />
```

| Property | Type | Default | What it does |
|---|---|---|---|
| `chart` | `bar`, `line` or `pie` | `bar` | What it draws (**Chart**: Bars, Line, Pie). In Clarity's words it is the word after `chart`: `chart line`. |
| `by` | a field | none | The field whose values become the bars, points or slices (**Grouped by**). |
| `count`, `sum field`, `average field` | a measure | `count` | Each group's number (**Number**). In a `.cxaml` file `measure="{&quot;field&quot;:&quot;total&quot;,&quot;kind&quot;:&quot;sum&quot;}"`, and `none="measure"` for a chart of numbers. |
| `points` | 2 to 1000 | all | For a list of numbers: its last n numbers. |
| `max` | a number | its largest value | For a list of numbers: the top of the axis (**Top**). |

```xml
<!-- shop/design/DashboardView.cxaml -->
<View class="shop.DashboardView">
  <Data name="orders" type="list&lt;Order&gt;" />
  <Chart name="byStatus" at="1,1" span="6" rows="6" bind="orders" by="status" />
  <Chart name="perDay" at="7,1" span="6" rows="6" bind="orders" chart="line" by="placed" measure="{&quot;field&quot;:&quot;total&quot;,&quot;kind&quot;:&quot;sum&quot;}" />
  <Chart name="share" at="1,7" span="4" rows="6" bind="orders" chart="pie" by="customer" measure="{&quot;field&quot;:&quot;total&quot;,&quot;kind&quot;:&quot;average&quot;}" />
</View>
```

- Rows with no value in the `by` field are left out. `count` gives each group its number of rows, `sum field` the sum of a number field, `average field` its average over the rows that have one.
- The groups come in the order of their values: numbers and dates low to high, text ignoring case, an enum's values as it declares them, no before yes, a reference by the text of the record it names. Each shows as people read it: an enum by its label, a date in their format.
- A bar or line chart has an axis in steps of 1, 2, 2.5 or 5 times a power of ten that always takes in 0. A pie's slices take the colors of the palette's fourth row, in order; a group of 0 or less has no slice. Each bar, point and slice tells its group and number when pointed at.
- The chart draws again when its rows change. Every renderer groups and draws alike (`conformance/charts.json`).

#### A chart of numbers

Bound to a list of numbers (`<Data name="readings" type="list&lt;decimal&gt;" />`, or such a field of a record), a bar or line chart draws the numbers as they are, in order, without groups: `<Chart name="history" at="1,1" span="6" rows="4" bind="readings" chart="line" points="30" max="100" none="measure" />`. A line of more than 30 points has no dot at each. A timer that adds a number to the list every so many seconds makes it a live graph; see [Time](parts.time.md).

The checker refuses: a chart of records without `by` ("as in 'by status'") or without a number ("as in 'count' or 'sum total'"); a `by` field the entity does not have, or one that is a picture, file or list ("a chart cannot group by"); a sum or average of a field that is no integer, decimal, money or duration; a chart of a list of values that are not numbers ("list<integer> or list<decimal>"), with `by`, or as a pie ("a list of numbers is drawn as a bar or a line"); and `points` outside 2 to 1000.

### `trend`

One number's last values as a line, taken every so many seconds while the window is open.

```xml
<Trend name="name" at="col,row" span="n" rows="n" bind="number" points="n" max="n" every="seconds" />
```

| Property | Type | Default | What it does |
|---|---|---|---|
| `bind` | an integer, decimal, money or duration | none | The number it follows. |
| `points` | 2 to 1000 | 60 | How many values it keeps, the newest at the right. |
| `every` | 0.1 to 86400 seconds | 1 | Seconds between the values it takes (**Every (s)**). |
| `max` | a number | its largest value | The top of its axis (**Top**). |

```xml
<!-- monitor/design/MonitorView.cxaml -->
<View class="monitor.MonitorView">
  <Data name="cpu" type="decimal" />
  <Timer name="clock" at="1,1" span="1" every="1" ontick="sampled" />
  <Trend name="cpuTrend" at="1,1" span="6" rows="4" bind="cpu" max="100" points="60" />
</View>
```

A trend does not wait for code: every `every` seconds it takes the value its number has then, what code last set. The designer shows a still sample line. The checker refuses `every` outside 0.1 to 86400 seconds ("a trend takes a value every 0.1 to 86400 seconds") and `points` outside 2 to 1000.

### `graph`

Records as nodes joined by edges: the records of the list it binds are its nodes, the records of the list `edges` names are its edges. Both are records of any kind the project declares, with whatever fields they need; the graph only has to know which fields play which part.

```xml
<Graph name="name" at="col,row" span="n" rows="n" bind="list" edges="list" nodeKey="field" nodeLabel="field" nodeKind="field" edgeFrom="field" edgeTo="field" edgeLabel="field" edgeKind="field" layout="force|layered|radial|circle" nodeLooks="{...}" edgeLooks="{...}" show="field, field" limit="n" hops="n" legend="true" opens="target" onclick="handler" onselect="handler" onopen="handler" />
```

| Property | Type | Default | What it does |
|---|---|---|---|
| `bind` | a list of records | none | The nodes. |
| `edges` | a list of records | none | The edges. |
| `nodeKey` | a field of a node | `id` | What its edges name it by. |
| `nodeLabel` | a field of a node | its key | What it shows. |
| `nodeKind` | a field of a node | none | Whose value chooses its look. |
| `edgeFrom`, `edgeTo` | fields of an edge | `source`, `target` | The fields that hold the keys of the nodes it joins. |
| `edgeLabel`, `edgeKind` | fields of an edge | none | Its label and its kind. |
| `layout` | `force`, `layered`, `radial`, `circle` | `force` | How it places its nodes. |
| `nodeLooks`, `edgeLooks` | a look per kind | none | How each kind is drawn. |
| `show` | fields | every field that plays no part | What pointing at a node tells (**Shown fields**). |
| `limit` | 1 to 5000 | 500 | The most nodes it shows. |
| `hops` | 1 to 10 | all | Only the nodes this many edges from the picked one. |
| `legend` | yes or no | no | Its kinds as chips that hide and show them. |
| `opens` | a slide-out or window | none | What a click on a node opens (**Opens**), as a button's. |

Events: `click`, `select`, `open`.

```clarity
entity Thing
    name: text
    kind: text
    owner: text

entity Link
    source: integer
    target: integer
    field: text
    since: integer
```

```xml
<!-- ideas/design/MapView.cxaml -->
<View class="ideas.MapView">
  <Data name="things" type="list&lt;Thing&gt;" />
  <Data name="links" type="list&lt;Link&gt;" />
  <Graph name="map" at="1,1" span="12" rows="10" bind="things" edges="links" nodeLabel="name" nodeKind="kind" edgeLabel="field" layout="force" nodeLooks="{&quot;person&quot;:{&quot;shape&quot;:&quot;box&quot;,&quot;color&quot;:&quot;accent&quot;}}" onselect="picked" onopen="opened" />
</View>
```

- An edge joins the node whose key its `edgeFrom` field holds to the node whose key its `edgeTo` field holds. A key is matched by its value, and a reference field may hold it, so a list can be its own edges: `bind="folders" edges="folders" edgeFrom="id" edgeTo="parent"` draws folders under their parents. An edge whose ends are not both shown is left out.
- Pointing at a node tells its label and its fields, the ones `show` names or else every field that plays no part; pointing at an edge tells its label, the nodes it joins and its fields.
- `force` pushes the nodes apart and pulls them together along their edges, `layered` puts them in layers along the edges' direction, `radial` in rings around the picked node, `circle` on one circle. Each places the same graph the same way every time.
- `nodeLooks` gives the nodes of a kind, by the kind's value, a `shape` (`circle`, `square`, `diamond`, or `box` with the label inside), a `color` (a color of the theme or written out) and a `size` across in pixels; a kind without a look is a circle in the next color of the palette's fourth row. `edgeLooks` gives an edge kind a `line` (`solid`, `dashed`, `dotted`), a `color` and `arrow` (an arrow at its target unless `false`). The kind `""` is the look of nodes or edges without a kind.
- A graph cut short by its `limit` (the first nodes of the list) says so in its corner (Clarity's word `graphCut`).
- The person zooms with the buttons in its corner (Clarity's words `zoomOut`, `zoomWhole`, `zoomIn`), with Ctrl and the wheel, and with + and -; 0 shows it whole again. The wheel or a drag of the background pans.
- A drag of a node moves it, and it stays where it was left: as the graph changes, and the next time the window opens, kept for the person by the window's and the graph's name (as a table keeps its column widths). A node is kept by its record (its key, or in a graph of lists its entity and id), not by its place in the list. Picking, hops and the legend move no node: the whole graph is laid out, and they only keep some of it from showing.
- A click on a node picks it and runs `on select` when it is another one; every click runs `on click`, or opens what `opens` names: `opens="nodeDetails"` slides out a slide-out with the picked record's details. A double click or Enter runs `on open`. Code reads the picked record as `map.selected`, as a table's. The graph draws again when its lists change.
- `hops` shows only the nodes at most that many edges from the picked node, either way along them, with a control in its corner to show fewer or more (Clarity's words `graphHops`, `graphFewerHops`, `graphMoreHops`); with nothing picked it shows all of it. `legend` shows each kind as a chip in its corner, in its color; a click hides or shows that kind (`graphKind`). The picked node always shows.

#### A graph of the data model

Instead of a list it binds and its edges, a graph can name lists of records of different entities, `lists` (**Lists of the data model**): their records are its nodes, and the data model gives the rest. A record's kind is its entity, its label its entity's first text field (as a reference shows a record), and each reference field to an entity the graph shows is an edge, labelled with the field. No code is needed: a window's lists hold every record when it opens.

```xml
<View class="shop.CustomerMapView">
  <Data name="customers" type="list&lt;Customer&gt;" />
  <Data name="orders" type="list&lt;Order&gt;" />
  <Data name="kindPicked" type="text" />
  <Data name="idPicked" type="integer" />
  <Graph name="map" at="1,1" span="12" rows="10" lists="customers, orders" hops="2" legend="true" pickedKind="kindPicked" pickedId="idPicked" onopen="opened" />
</View>
```

- A record in two lists is one node; records of two entities with the same id are two. A reference to an entity the graph does not show is no edge.
- A graph of lists binds nothing and names no edges or node fields. Its looks, layout, `show` (fields of any of its entities) and `limit` are a graph's as above.
- `pickedKind` and `pickedId` (**Picked entity** and **Picked id**) name text and whole-number data: picking a node writes its entity and id there, and code that sets them picks that node. The handlers of `on select` and `on open` read them: `if kindPicked == "Order"`.

The checker refuses: a graph with neither `bind` nor `lists`; a field a node or edge does not have ("an edge names the nodes it joins in its fields (edgeFrom, edgeTo)"); edge fields or looks without `edges` ("name the list of its edges first"); `edges` that is no list of records; `limit` outside 1 to 5000 and `hops` outside 1 to 10; on a graph of lists, `bind`, `edges` or node fields, a list that is no list of records, a `show` field none of its entities has, and `pickedKind` or `pickedId` that is not text or whole-number data; on a graph it binds, `pickedKind` and `pickedId` ("a graph it binds has its picked record in selected").

### `progress`

A bar for a number it binds, from 0 to `max`.

```xml
<Progress name="name" at="col,row" span="n" bind="number" max="n" />
```

| Property | Type | Default | What it does |
|---|---|---|---|
| `bind` | an integer or decimal, a field or a plain value | none | The number it shows. |
| `max` | a number above 0 | 100 | The number the bar is full at (**Full at**). |

It has no events.

```xml
<Progress name="uploadProgress" at="1,10" span="6" bind="done" max="250" />
```

The checker refuses a progress bar that binds nothing ("bind one, as in 'bind done'") and a `max` that is not a number above 0.

### `busy`

A running indicator with its caption ("Busy..." for a new one), which shows that something is going on. It binds nothing; code shows and hides it with `visible`.

```xml
<Busy name="name" at="col,row" span="n" caption="@key" />
```

It has no properties of its own and no events.

```xml
<Busy name="loading" at="1,1" span="3" caption="@loadingCaption" />
```

```clarity
code Orders
    handler refreshClicked
        loading.visible = true
        orders = find o in Order order by o.placed
        loading.visible = false
```

### `statusItem`

A panel of a status bar, made for the footer and placed anywhere a part goes. From its left it shows its icon (when it has one), a mark, and its caption, and at its right end a small gauge of the number it binds, as full as a progress bar of the same value and `max`; an item that binds nothing has no gauge.

```xml
<StatusItem name="name" at="col,row" span="n" caption="@key" icon="name" bind="number" max="n" onclick="handler" />
```

| Property | Type | Default | What it does |
|---|---|---|---|
| `bind` | an integer or decimal, or nothing | none | The number of its gauge. |
| `max` | a number above 0 | 100 | The number its gauge is full at (**Highest**). |
| `icon` | an icon's name | none | The icon at its left. |

Event: `click`, which runs a handler, or opens a window or slide-out as a button's does.

```xml
<StatusItem name="progressItem" at="1,1" span="3" caption="@progressCaption" icon="gauge" bind="entryProgress" max="100" onclick="progressClicked" />
```

- Code sets what it says, `progressItem.caption = @entryProgress(percent = progress)`, as any caption.
- `reviewItem.checked = true` shows a check mark in the theme's `success` color; false takes it away. A status item that binds its gauge still has its own check mark.
- `reviewItem.error = @changedSinceReview` shows a warning in the theme's `warning` color in place of the check mark, its icon turns that color, and the error is its tooltip; `reviewItem.error = ""` clears it. Its `tooltip` shows while it has no error.
- A screen reader reads its caption, and as its value its warning, else how full its gauge is (`60%`).

The checker refuses a `max` that is not a number above 0, and an icon Clarity does not have. Where it stands in the footer is in [The app and its frame](design.frame.md).

## See also

- [Parts](design.parts.md): the Parts palette and the properties panel.
- [Time](parts.time.md): the timer, which feeds trends and charts of numbers.
- [Handlers](code.handlers.md): the code behind, and the part properties code reads and sets.
- [Queries](code.queries.md): `find`, which fills the lists these parts show.
- [Entities and enums](data.entities.md): the records and fields they show.

## The parts at a glance

<!-- catalog: written from Clarity's part catalog (crates/clarity-spec/src/widgets.rs) by CLARITY_WRITE_MANUAL=1 cargo test -p clarity-spec --test manual_catalog; not edited by hand -->

| Part | Line | Binds | Size | Grows | Properties | Events |
|---|---|---|---|---|---|---|
| Table | `table` | collection | 12 x 5 | yes | `columns`, `edit`, `link`, `noUndo`, `resizable`, `sortable`, `add`, `remove`, `striped`, `orderBy`, `reorder`, `page`, `frame` | `select`, `open`, `change`, `reorder`, `add`, `remove` |
| Tree | `tree` | collection | 4 x 8 | yes | `children`, `show`, `striped`, `foldButtons`, `reorder`, `noUndo`, `frame` | `select`, `open`, `reorder` |
| Tree table | `treetable` | collection | 12 x 8 | yes | `children`, `columns`, `resizable`, `striped`, `foldButtons`, `reorder`, `add`, `remove`, `noUndo`, `frame` | `select`, `open`, `reorder`, `add`, `remove` |
| Chart | `chart` | collection | 6 x 6 | yes | `by`, `measure`, `points`, `max` | none |
| Graph | `graph` | collection | 8 x 8 | yes | `edges`, `nodes`, `layout`, `looks`, `limit`, `show`, `lists`, `hops`, `legend` | `click`, `select`, `open` |
| Trend | `trend` | field (integer, decimal, money, duration) | 6 x 4 | no | `points`, `max`, `every` | none |
| List | `list` | collection | 4 x 5 | yes | `show`, `cards`, `striped`, `reorder`, `noUndo`, `frame` | `select`, `open`, `reorder` |
| Progress | `progress` | field (integer, decimal) | 6 x 1 | no | `max` | none |
| Busy | `busy` | nothing | 3 x 1 | no | none | none |
| Status item | `statusItem` | field (integer, decimal) | 3 x 1 | no | `icon`, `max` | `click` |

<!-- /catalog -->
