# A complete example

A whole small project, every file of it: a customer list with a filter that opens a form, the form that saves a customer, and a data event that clears the VAT number of a private customer. Every file here is checked as one project, so it is always valid Clarity.

A project file, one module and the files of its app and windows: a customer list with a filter that opens a form, the form that saves a customer, and a data event that clears the VAT number of a private customer.

`shop.clarity`:

```clarity
project Shop
    stack rust desktop
    datasource main sqlite "data/shop.db"
```

`shop/customers.clarity`:

```clarity
module customers

enum CustomerKind
    Private label @customerKindPrivate
    Business label @customerKindBusiness

entity Customer
    name: text(80), required, label @customerNameLabel
    kind: CustomerKind, default Private, label @customerKindLabel
    vatNumber: text(14), help @customerVatNumberHelp, label @customerVatNumberLabel
    notes: text, label @customerNotesLabel

code Customer
    before save
        // Only business customers keep a VAT number, whichever window saves them.
        if customer.kind == Private
            customer.vatNumber = ""

code CustomerBrowse
    handler windowReturned
        // The form may have saved a customer.
        filterChanged()

code CustomerBrowseView
    handler filterChanged
        customers = find c in Customer where c.name contains nameFilter.text order by c.name

    handler openClicked
        open CustomerForm with customer = grid.selected

code CustomerFormView
    handler kindChanged
        vatNumber.enabled = customer.kind == Business

    handler saveClicked
        name.error = ""
        vatNumber.error = ""
        if customer.name == ""
            name.error = @nameIsRequired
            return
        if customer.kind == Business and customer.vatNumber == ""
            vatNumber.error = @neededForBusinessCustomers
            return
        save customer
        message @savedName(name = customer.name)
        close

    handler closeClicked
        close
```

Beside it in `shop/design/` are the app and the windows, each a `.cxaml` file of its own. Each window shows its parts in a view. `open CustomerForm with customer = ...` fills the data of the view the window shows.

`Shop.cxaml`, the app, which starts at the customer list:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<App class="customers.Shop" main="CustomerBrowse">
</App>
```

`CustomerBrowse.cxaml` and `CustomerBrowseView.cxaml`:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<Window class="customers.CustomerBrowse" title="@customerBrowseTitle" onreturn="windowReturned">
  <Content name="main" at="1,1" span="12" rows="10">
    <View name="CustomerBrowseView" />
  </Content>
</Window>
```

```xml
<?xml version="1.0" encoding="UTF-8"?>
<View class="customers.CustomerBrowseView">
  <Data name="customers" type="list&lt;Customer&gt;" />
  <Label name="nameFilterLabel" at="1,1" span="2" caption="@nameFilterLabel" for="nameFilter" />
  <Textbox name="nameFilter" at="3,1" span="4" placeholder="@nameFilterPlaceholder" onchange="filterChanged" />
  <Table name="grid" at="1,2" span="12" rows="8" bind="customers" columns="name, kind" onopen="openClicked" />
  <Button name="newCustomer" at="9,10" span="2" caption="@newCaption" opens="CustomerForm" />
  <Button name="openButton" at="11,10" span="2" caption="@openButtonCaption" onclick="openClicked" />
</View>
```

`CustomerForm.cxaml` and `CustomerFormView.cxaml`:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<Window class="customers.CustomerForm" title="@customerFormTitle">
  <Content name="main" at="1,1" span="12" rows="6">
    <View name="CustomerFormView" />
  </Content>
</Window>
```

```xml
<?xml version="1.0" encoding="UTF-8"?>
<View class="customers.CustomerFormView">
  <Data name="customer" type="Customer" />
  <Form name="customerForm" at="1,1" span="12" rows="4" bind="customer">
    <Label name="nameLabel" at="1,1" span="3" caption="@nameLabel" for="name" />
    <Textbox name="name" at="4,1" span="6" bind="customer.name" />
    <Help name="nameHelp" at="10,1" span="3" for="name" />
    <Label name="kindLabel" at="1,2" span="3" caption="@kindLabel" for="kind" />
    <Radio name="kind" at="4,2" span="6" bind="customer.kind" none="orientation" onchange="kindChanged" />
    <Label name="vatNumberLabel" at="1,3" span="3" caption="@vatNumberLabel" for="vatNumber" />
    <Textbox name="vatNumber" at="4,3" span="4" bind="customer.vatNumber" />
    <Help name="vatNumberHelp" at="8,3" span="5" for="vatNumber" />
    <Label name="notesLabel" at="1,4" span="3" caption="@notesLabel" for="notes" />
    <Textbox name="notes" at="4,4" span="9" bind="customer.notes" />
  </Form>
  <Button name="saveButton" at="9,6" span="2" caption="@saveButtonCaption" key="Ctrl+S" default="true" onclick="saveClicked" />
  <Button name="closeForm" at="11,6" span="2" caption="@closeFormCaption" cancel="true" onclick="closeClicked" />
</View>
```

The full shop example, with orders, a menu and a ribbon, is `examples/shop.clarity`.

See [Your first project](start.first-project.md) to make one like it in the IDE, [Windows](design.windows.md) for the `.cxaml` files and [Handlers](code.handlers.md) for the code behind.
