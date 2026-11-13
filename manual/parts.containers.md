# Containers

The parts of the Containers group of the Parts palette: forms, panels, tabs, content, accordions, scroll boxes, button groups, separators, wizards, popovers, slide-outs and splitters, which hold other parts on a grid of their own.

## A container's grid

A container has its own grid, as many columns wide as its `span`, with the column lines of the grid around it: its frame sits in the space between columns, so a label and an input inside a panel line up with those outside it. In a window's file its parts are elements inside it, and their places count from its own top-left cell:

```xml
<Tabs name="customerTabs" at="1,5" span="12" rows="7">
  <Tab title="@ordersTab">
    <Table name="orderTable" at="1,1" span="12" rows="5" bind="orders" columns="number, total" />
  </Tab>
  <Tab title="@notesTab">
    <Text name="notesText" at="1,1" span="12" bind="customer.notes" />
  </Tab>
</Tabs>
```

Drop a part onto a container on the canvas and it lands on the container's grid, in the innermost container under the pointer. Tabs, steps and accordions have **Add tab**, **Add step** and **Add section** in their properties; pick a tab, step or section on the canvas to set its **Title** or remove it. The checker reports parts that start outside a container's grid, run past its last column or overlap, as it does on the window (see [Windows](design.windows.md#the-grid)).

A view shows in a window through a content part, and nowhere else: to show a view in a tab, step, section or slide-out, put a content in it (see [`content`](#content)).

### `form`

Holds the inputs of one record, on its own grid, nested in panels and tabs if needed. It binds to one record, `bind="customer"`, and every input inside it binds to that record. Pressing Enter in one of its inputs runs its `submit`; when `submit` is not wired, Enter clicks the window's default button. A form without a caption has no frame, so its grid lines up with the grid around it.

- `bind` (a record of the window's data).
- `caption` (a key, none by default): a frame with this caption.
- `labels` (a whole number, none by default): a label column. Each input in the form gets a label in the form's first columns of its row, showing its field's label (the `label` of the field, as `customerNameLabel`), so its inputs start after them. The label is a part named after its input, `nameBoxLabel` for `nameBox`, which code hides or disables as any part. A check box, a switch and a toggle label themselves. **Label column** in the properties.
- `foldable` and `folded` (true or false, false by default): as a panel's.
- Event: `submit`.

```xml
<Form name="customerForm" at="1,1" span="12" rows="4" bind="customer" labels="3" onsubmit="saveCustomer">
  <Textbox name="nameBox" at="4,1" span="9" bind="customer.name" />
  <Textbox name="cityBox" at="4,2" span="9" bind="customer.city" />
</Form>
```

The checker refuses:

- A form inside a form: `form at 1,3: a form cannot hold another form`.
- An input bound to other data than its form's: `textbox at 4,2: binds to 'order', but its form 'customerForm' holds 'customer'`.
- A label column that leaves no room: `a form's label column leaves room for its inputs: 'labels' is 1 to 11 here`.
- An input that starts inside the label column (`'nameBox' starts at column 4 or later: its label takes the columns before`), and a label written for an input the form labels itself (`the form labels 'nameBox' itself; remove this label`).
- A `bind` to a field or a list: `form at 1,1: a form binds to one record, as in 'bind customer'`.

### `panel`

Parts in a frame with the caption, on its own grid.

- `caption` (a key, "Panel" for a new one).
- `foldable` (true or false, false by default): the person folds it to its header with a click on the header, and unfolds it again. **The person can fold it by its header** under Folding in the properties.
- `folded` (true or false, false by default): with `foldable`, it starts folded. **Starts folded**.

```xml
<Panel name="details" at="1,6" span="12" rows="4" caption="@detailsCaption" foldable="true" folded="true">
  <Text name="addressText" at="1,1" span="12" bind="customer.address" />
</Panel>
```

Code reads and sets whether a foldable panel or form is folded as `details.folded`, so a handler can open the section a message is about. In the designer it stays open, to edit what it holds. The designer refuses `folded` without `foldable` (`'folded' needs 'foldable': write 'foldable folded'`).

### `tabs`

Tab pages, each a `<Tab title="@key">` with parts. Code reads and sets the open tab as `customerTabs.tab`, counted from 1. Its `change` runs when the person opens another tab.

- `<Tab title>` (a key): the page's title; **Title** when the tab is picked.
- Event: `change`.

See [A container's grid](#a-containers-grid) for an example. The checker refuses tabs without a tab (`tabs need at least one 'tab'`).

### `content`

A window's content: it shows one view at a time, of the views it lists, each a `<View name="X" />` line. It has no look of its own; the view brings its heading, buttons and tabs. Code shows another of them with `main.swap(OrderEditor)`; `main.tab` is the one it shows, counted from 1 in its list. **Add view** in its properties lists the project's views; pick a view line to change it or wire what the view tells (see [Windows](design.windows.md#views-and-content)).

```xml
<Content name="main" at="1,1" span="12" rows="9">
  <View name="FriendBrowseView" />
  <View name="FriendEditView" />
</Content>
```

The checker refuses content without a view (`content needs at least one 'view', as in view OrderEditor`), a view it cannot find (`there is no view 'OrderEditr'`), a window or dialog named as a view (`OrderForm is a window; content shows a view: write page OrderForm: view`), a view listed twice in a window (`OrderEditor shows once in a window`) or in itself (`OrderEditor shows itself`), parts written inside a view line (`a view line of content holds no parts of its own; they are OrderEditor's`), and a view shown in a tab, step, section or slide-out directly (`a tab does not show a view: a view shows in content; put a content in the tab with view OrderEditor`). Code refuses `main.swap(X)` of a view it does not list: `main shows no view 'X'; list it in main as view X`.

### `accordion`

Sections that fold, one open at a time, each a `<Section title="@key">` with parts. It shows the sections' titles one under the other, with the open section's parts under its title; the first is open to start. A press on a title opens that section, or folds it when it was the open one, and runs `change`. Code reads and sets the open section as `faq.open`, counted from 1 (0 folds them all).

- `orientation` (`horizontal`, stacked without it): a horizontal accordion stands its titles side by side as tall strips, read downwards, with the open section's parts beside its title, as a side panel of folding sections does. **Orientation** in the properties.
- `<Section title color>`: the title (a key), and a color of its header, written out (`color="#3b6ea8"`) or a color of the theme by name (`color="accent"`). Its text is black or white, whichever reads best, and it darkens a little under the pointer. A section without one takes the theme's look. **Color** when the section is picked.
- Event: `change`.

```xml
<Accordion name="parts" at="1,2" span="12" rows="15" orientation="horizontal" onchange="partOpened">
  <Section title="@chaptersSection" color="#3b6ea8">
    <List name="chapterList" at="1,1" span="12" rows="10" bind="chapters" show="title" />
  </Section>
  <Section title="@notesSection">
    <Memo name="notesBox" at="1,1" span="12" rows="10" bind="book.notes" />
  </Section>
</Accordion>
```

The checker refuses an accordion without a section: `an accordion needs at least one 'section'`.

### `scroll`

A scroll box: parts on its own grid, like a panel without a frame, and parts placed below its rows scroll into view.

```xml
<Scroll name="settingsScroll" at="1,1" span="12" rows="8">
  <Switch name="darkSwitch" at="1,1" span="4" caption="@darkCaption" bind="settings.dark" />
  <Switch name="soundSwitch" at="1,12" span="4" caption="@soundCaption" bind="settings.sound" />
</Scroll>
```

### `buttons`

A button group: buttons and toggles joined in one row, on its own grid.

```xml
<Buttons name="alignButtons" at="3,1" span="3">
  <Toggle name="leftToggle" at="1,1" icon="text-align-start" onchange="alignLeft" />
  <Toggle name="rightToggle" at="2,1" icon="text-align-end" onchange="alignRight" />
</Buttons>
```

The checker refuses another part in it (`a button group holds buttons and toggles`) and a button off its one row (`the buttons of a group stand in one row: at row 1, one row high`).

### `separator`

A line between parts, level, or upright with `orientation="vertical"`. It also stands in toolboxes and on the sign-in window and its views.

```xml
<Separator name="footerLine" at="1,9" span="12" />
```

- `orientation` (`horizontal` or `vertical`, level without it).

The checker refuses a caption on it: `a separator has no caption: it is only a line`.

### `steps`

A wizard: its `<Step title="@key">` pages show one at a time, with the titles of all steps above and Clarity's own Back, Next and Finish below (the words `back`, `next` and `finish` in the language files). Code reads and sets the step the person is on as `wizard.step`, counted from 1.

- `<Step title>` (a key).
- Event `next`: runs on the step the person is on when Next is pressed, before the wizard moves on. A `refuse` in it keeps the person on the step and says why; a handler that sets `wizard.step` itself decides where the wizard goes. Without it, Next just moves on. Back goes one step back and runs nothing, so what was entered stays.
- Event `finish`: runs when Finish is pressed on the last step. It saves, closes the dialog with its value, or does what the wizard is for.

```xml
<Dialog class="shop.NewOrder" gives="Order">
  <Data name="order" type="Order" />
  <Steps name="wizard" at="1,1" span="12" rows="10" onnext="checkStep" onfinish="done">
    <Step title="@customerStep">
      <Dropdown name="customerBox" at="1,1" span="6" bind="order.customer" />
    </Step>
    <Step title="@noteStep">
      <Textbox name="noteBox" at="1,1" span="12" bind="order.note" />
    </Step>
  </Steps>
</Dialog>
```

```clarity
code NewOrder
    handler done
        save order
        close with order
```

The checker refuses fewer than two steps: `steps need at least two 'step' blocks: a wizard of one step is a panel`.

### `popover`

Parts on their own grid, outside the window's grid, opened next to the part its `for` names. It has no place, only a size (`span` and `rows`). It is closed until a click opens it (`opens="filters"` on a button or a menu item) or code sets `filters.visible = true`; a click outside it or Escape closes it. In the designer it waits in the tray under the canvas and shows while it is picked.

- `for` (a part name): the part it opens next to, any part of the window; **Next to** in the properties.

```xml
<Button name="filterButton" at="1,7" span="2" caption="@filterButton" opens="filters" />
<Popover name="filters" for="filterButton" span="4" rows="2">
  <Checkbox name="vinylBox" at="1,1" span="4" caption="@onlyVinylCaption" bind="onlyVinyl" onchange="refilter" />
</Popover>
```

The checker refuses a popover without `for` (`a popover opens next to a part: write 'for' and the part's name`), a `for` that names no part of the window, and a popover inside another part (`a popover stands on the window, not in another part`).

### `slideout`

Parts on their own grid that slide in over the window from its right, left, top or bottom edge, as an IDE's side and bottom panels do. It has no place in the window's grid: in the designer it waits in the tray under the canvas, marked hidden, and shows on the window while it is picked.

At the right (or the left) it is as high as the window's content, below the menu bar, toolbar and ribbon and above the footer, with a shadow, and what is under it stays where it is. At the top or bottom it slides down from the top of the content or up from its bottom, as wide as the content. `span` and `rows` give its own grid; its rows are no more than the window holds under its bars, and an edit that makes it taller cuts it to fit (see [Windows](design.windows.md#size)). Its rows take no room in the window, so it never makes the window scroll.

Its caption shows in its header and on its handle: a tab on its edge of the window with the caption written along it (across at the top and bottom), always there, which slides it out with a click (or Enter) and back with another. The handles of one side stand one after the other in a strip the window's parts make room for; at the bottom that strip lies on top of the footer. A slide-out whose caption is empty (`caption=""`) has no handle. A click on another part opens it too (`opens="details"` on a button or menu item), and so does code (`details.visible = true`); Escape, a click beside it, the close button in its header or `details.visible = false` closes it. The focus moves into it when it opens and back when it closes; a screen reader finds it as a dialog named by its caption.

- `caption` (a key, "Details" for a new one).
- `side` (`left`, `right`, `top` or `bottom`; `right` without it): **Side** in the properties.
- `slideWidth` (pixels, 100 to 1600, 420 without it): its width at the left or right; **Width (px)**. Dragging its inner edge on the canvas sets it too.
- `slideHeight` (pixels, 100 to 1600, 300 without it): its height at the top or bottom; **Height (px)**.
- `resizable` (true or false, false by default): the person drags its inner edge between `min` and `max` (pixels, 100 to 1600), as a sidebar's; up and down at the top or bottom. **The person can resize it**, with **Narrowest** and **Widest** (**Lowest** and **Highest** at the top or bottom).
- `pinnable` (true or false, false by default): its header has a pin. Pinned, it stays open beside the window's parts, which make room for it as for a sidebar, and a click beside it leaves it open (Escape closes it while the focus is in it). Code reads and sets `details.pinned`. **The person can pin it open**.
- `beside` (a part name, none by default): it lies against that part instead of the window's edge; see below. **Beside**.
- Events: `open` and `close`, when it opens and closes, so a handler can load what it shows.

The size the person dragged it to and whether it is pinned are kept per window for the next start.

```xml
<Button name="detailsButton" at="1,1" span="2" caption="@detailsButton" opens="details" />
<Slideout name="details" span="6" rows="10" caption="@orderDetails" slideWidth="480" resizable="true" min="300" max="900" pinnable="true" onopen="loadDetails">
  <Textbox name="noteBox" at="1,1" span="6" bind="order.note" />
</Slideout>
```

A slide-out stands on the window, unless it lies beside a part: `beside="chapterTable"` lays it against that part, at its `side` of it (`side="right"` at the part's right edge, sliding outward over what lies beside the part, up to the window's edge; with too little room there it lies over the part from that edge). Its handle then sits just outside that edge, and what lies beside the part (in its grid or the next pane) makes room for it, as the window's parts do for the handles at its edge; at the window's edge it sits just inside the part. It is as high as the container the part stands in (at the top or bottom, as wide), with a lighter shadow than one at the window's edge. It is written in the same container as that part, so it stands in a panel, a splitter pane, a tab, a section or a view. Pinned, it stays open over what lies beside the part; nothing makes room for it. A part that is not shown hides its handle.

```xml
<Slideout name="chapterProps" caption="@chapterCaption" side="right" beside="chapterTable" pinnable="true" span="6" rows="12">
  <Textbox name="chapterTitle" at="1,1" span="6" bind="chapter.title" />
</Slideout>
```

To show a view in a slide-out, put a content in it. The checker refuses:

- A slide-out inside another part without `beside`: `a slide-out stands on the window, or beside a part of the container it is in: write 'beside' and the part`.
- A `beside` part that is not in the same container (`there is no part 'chapterTable' beside this slide-out; it lies against a part of the container it is in`) or has no place of its own (`'filters' has no place of its own to lie against`).
- A slide-out that names a view itself: `a slide-out does not show a view: a view shows in content; put a content in the slide-out with view ChapterEditor`.

The designer refuses a width at the top or bottom (`a slide-out at the top or bottom has a height, not a width: write height 300`), a height at the left or right, a size outside 100 to 1600 pixels (`a slide-out is 100 to 1600 pixels wide`), and a size outside its `min` and `max` (`a slide-out's width lies between its min and max`).

### `splitter`

Two panes, side by side or one above the other, with a divider the person drags. Each pane is a `<Pane>` with parts on its own grid.

- `orientation` (`horizontal` or `vertical`; side by side without it): `vertical` stacks the panes. **Orientation** in the properties.
- `<Pane span>` (columns): side by side, the first pane can start at a width of its own, `span="4"`: its grid has 4 of the splitter's columns and the second pane the rest, so a list on the left and a form on the right keep the window's column lines.
- `<Pane rows>` (rows): one above the other, the first pane starts at a height of its own the same way, `rows="4"`: 4 of the splitter's rows, the second pane the rest, and each pane keeps all of the splitter's columns.

Without either the panes start alike, each with the splitter's columns. On the canvas, drag the divider to set the size the first pane starts with (the designer shows how many columns or rows while you drag), or pick the first pane and type its **Width** or **Height**.

```xml
<Splitter name="customerSplit" at="1,1" span="12" rows="10">
  <Pane span="4">
    <List name="customerList" at="1,1" span="4" rows="10" bind="customers" show="name" />
  </Pane>
  <Pane>
    <Panel name="details" at="1,1" span="8" rows="4" caption="@detailsCaption">
      <Text name="nameText" at="1,1" span="6" bind="customer.name" />
    </Panel>
  </Pane>
</Splitter>
```

The checker refuses a splitter without exactly two panes (`a splitter has exactly two 'pane' blocks`), a size on the second pane (`only the first pane has a size; the second has the rest`), and a first pane as large as the splitter or larger (`the first pane spans 1 to 11 of the splitter's 12 columns`).

## Related pages

- [Parts](design.parts.md): the palette, the properties panel, and what every part has.
- [Windows](design.windows.md): the grid, sizes, views and content.
- [Basic input parts](parts.input.md): the inputs forms hold, and buttons that open popovers and slide-outs.
- [Text parts](parts.text.md): labels and help texts.

## The parts at a glance

<!-- catalog: written from Clarity's part catalog (crates/clarity-spec/src/widgets.rs) by CLARITY_WRITE_MANUAL=1 cargo test -p clarity-spec --test manual_catalog; not edited by hand -->

| Part | Line | Binds | Size | Grows | Properties | Events |
|---|---|---|---|---|---|---|
| Form | `form` | record | 12 x 4 | yes | `foldable`, `labels` | `submit` |
| Panel | `panel` | nothing | 6 x 4 | yes | `foldable` | none |
| Tabs | `tabs` | nothing | 8 x 5 | yes | none | `change` |
| Content | `content` | nothing | 8 x 6 | yes | none | none |
| Accordion | `accordion` | nothing | 6 x 8 | yes | `orientation` | `change` |
| Scroll box | `scroll` | nothing | 6 x 6 | yes | none | none |
| Button group | `buttons` | nothing | 4 x 1 | no | none | none |
| Separator | `separator` | nothing | 12 x 1 | no | `orientation` | none |
| Steps | `steps` | nothing | 12 x 8 | yes | none | `next`, `finish` |
| Popover | `popover` | nothing | 4 x 3 | no | `for` | none |
| Slide-out | `slideout` | nothing | 6 x 12 | no | `side`, `beside`, `slideWidth`, `slideHeight`, `resizable`, `min`, `max`, `pinnable`, `view` | `open`, `close` |
| Splitter | `splitter` | nothing | 12 x 6 | yes | `orientation` | none |

<!-- /catalog -->
