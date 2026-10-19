//! The Avalonia windows of the C# backend (G2b): each window as AXAML with
//! a ViewModel, its parts bound to the data and to their state, its handlers
//! as ViewModel methods.

use clarity_gen::{OutFile, generate};
use clarity_project::{Backend, Settings, Ui, starter};
use clarity_spec::load_sources;
use clarity_spec::model::Locale;

const SHOP: &str = "module shop

enum OrderStatus
    Pending
    Approved label \"Approved by sales\"

entity Customer
    name: text(80), required, help \"The name on invoices\"

entity Order
    number: text(12), required
    status: OrderStatus
    customer: ref Customer
    total: money
    ordered_on: date
    paid: bool
    discount: decimal(6, 4), format percent

function within_credit(order: Order) -> bool
    return order.total < 1000

app Shop
    title \"Shop\"
    main OrderForm
    menu
        item \"File\"
            item \"Orders\" key \"Ctrl+O\" on click -> OrderForm
            item \"Another order\" on click -> OrderForm in new window
            separator
            item \"Quit\" on click -> quit
            item \"Language\" on click -> language
            item \"Theme\" on click -> theme
    toolbar
        button \"Look\" on click -> theme
    footer
        status \"Ready\"

page OrderForm: window
    title \"Order\"
    data order: Order
    data orders: list<Order>
    on open: window_opened
    on return: window_returned
    label \"Number\" at 1,1 span 2 for number_input
    textbox name number_input at 3,1 span 4 bind order.number placeholder \"ORD-1\"
    help at 3,2 span 4 for number_input
    number at 1,3 span 3 bind order.total
    checkbox \"Paid\" at 4,3 span 3 bind order.paid
    switch \"Rush\" at 7,3 span 3 bind order.paid
    dropdown at 1,4 span 4 bind order.status
    dropdown name customer_pick at 5,4 span 4 bind order.customer
    date at 9,4 span 3 bind order.ordered_on
    text at 1,5 span 4 bind order.status
    table name grid at 1,6 span 12 rows 4 bind orders columns number, status, total on select: grid_selected on open: grid_opened
    textbox name number_filter at 1,10 span 4 on change: filter_changed
    button \"Approve\" name approve_button at 9,10 span 2 default on click: approve_clicked
    button \"New\" at 11,10 span 2 on click -> OrderForm
    tabs at 1,11 span 12 rows 2
        tab \"Notes\"
            memo at 1,1 span 12 bind order.number

code OrderForm
    handler window_opened
        number_input.readonly = order.paid

    handler window_returned
        reload orders
        reload order

    handler approve_clicked
        if not within_credit(order)
            message warning \"Order {order.number} is too large\"
            approve_button.enabled = false
            return
        order.status = Approved
        save order
        number_input.error = \"\"
        message \"Approved\"
        filter_changed()

    handler filter_changed
        orders = find o in Order where o.number contains number_filter.text order by o.number

    handler grid_selected
        order = grid.selected

    handler grid_opened
        open OrderForm with order = grid.selected
        close
";

fn settings() -> Settings {
    let mut settings = starter("Shop", "shop");
    settings.stack.backend = Backend::Csharp;
    settings.stack.ui = Ui::Avalonia;
    settings
}

fn files() -> Vec<OutFile> {
    let project = load_sources([("shop.clarity", SHOP)]);
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    generate(&project, &settings()).unwrap_or_else(|errors| panic!("{errors:#?}"))
}

fn file<'a>(files: &'a [OutFile], path: &str) -> &'a str {
    &files.iter().find(|f| f.path == path).unwrap_or_else(|| panic!("no {path} in {:?}", files.iter().map(|f| &f.path).collect::<Vec<_>>())).text
}

fn has(text: &str, expected: &str) {
    assert!(text.contains(expected), "expected\n{expected}\nin\n{text}");
}

#[test]
fn an_avalonia_desktop_application() {
    let files = files();
    let project = file(&files, "Shop.csproj");
    has(project, "<OutputType>WinExe</OutputType>");
    has(project, "<AvaloniaUseCompiledBindingsByDefault>true</AvaloniaUseCompiledBindingsByDefault>");
    // Avalonia.Desktop and CommunityToolkit.Mvvm come with Clarity.Avalonia.
    has(project, "<PackageReference Include=\"Clarity.Avalonia\"");
    // They are not named again.
    assert!(!project.contains("Include=\"Avalonia.Desktop\""), "{project}");
    has(file(&files, "App.axaml"), "<FluentTheme />");
    has(file(&files, "App.axaml.cs"), "desktop.MainWindow = Shell.Start(new OrderFormViewModel(db), Views.Create, TimeSpan.FromMilliseconds(5000));");
    has(file(&files, "Program.cs"), ".StartWithClassicDesktopLifetime(args)");
    has(file(&files, "Views.cs"), "OrderFormViewModel vm => new OrderFormView { DataContext = vm },");
}

#[test]
fn records_tell_the_window_when_they_change() {
    let files = files();
    let shop = file(&files, "Modules/Shop.cs");
    has(shop, "public sealed partial class Order : ObservableObject, IRecord<Order>");
    has(shop, "    public decimal Total { get => _total; set => SetProperty(ref _total, value); }\n    decimal _total;");
    has(shop, "    public string Number { get => _number; set => SetProperty(ref _number, value); }\n    string _number = \"\";");
    // Labels and choices read their texts each time, so they follow the language.
    has(shop, "public static partial class Choices\n{\n    public static Choice[] OrderStatus =>\n    [\n        new(global::Shop.OrderStatus.Pending, Texts.Pending),\n        new(global::Shop.OrderStatus.Approved, Texts.ApprovedBySales),\n    ];");
    has(shop, "        OrderStatus.Approved => Texts.ApprovedBySales,");
}

#[test]
fn the_grid_has_fixed_rows_and_rows_that_grow() {
    let files = files();
    let window = file(&files, "Windows/OrderFormView.axaml");
    has(window, "x:DataType=\"local:OrderFormViewModel\"");
    // Rows 6 to 9 hold the table, 11 and 12 the tabs: they share the extra height.
    has(window, "<Grid ColumnDefinitions=\"*,*,*,*,*,*,*,*,*,*,*,*\" ColumnSpacing=\"8\" RowSpacing=\"8\" Margin=\"12\">");
    has(window, "<RowDefinition Height=\"36\" />\n");
    has(window, "<RowDefinition Height=\"*\" MinHeight=\"36\" />\n");
    has(window, "<UserControl xmlns=\"https://github.com/avaloniaui\"");
}

#[test]
fn parts_bind_to_the_data_and_to_their_state() {
    let files = files();
    let window = file(&files, "Windows/OrderFormView.axaml");
    // Every text through the application's Texts, so it shows in the language chosen.
    has(window, "<TextBlock Grid.Column=\"0\" Grid.Row=\"0\" Grid.ColumnSpan=\"2\" Classes=\"label\"><Run Text=\"{x:Static local:Texts.Number}\" /><Run Text=\" *\" /></TextBlock>");
    has(
        window,
        "<c:TextInput x:Name=\"numberInput\" Grid.Column=\"2\" Grid.Row=\"0\" Grid.ColumnSpan=\"4\" Value=\"{Binding Order.Number}\" PlaceholderText=\"{x:Static local:Texts.ORD1}\" IsEnabled=\"{Binding NumberInput.IsEnabled}\" IsVisible=\"{Binding NumberInput.IsVisible}\" IsReadOnly=\"{Binding NumberInput.IsReadOnly}\" />",
    );
    has(window, "<TextBlock Grid.Column=\"2\" Grid.Row=\"1\" Grid.ColumnSpan=\"4\" Classes=\"help\" Classes.error=\"{Binding NumberInput.HasError}\" Text=\"{Binding NumberInput.Shown}\" />");
    // Clarity's own inputs: each tells when the person changed its value, and only then.
    has(window, "<c:NumberInput Grid.Column=\"0\" Grid.Row=\"2\" Grid.ColumnSpan=\"3\" Format=\"{x:Static local:Edit.OrderTotal}\" Value=\"{Binding Order.Total}\" />");
    has(window, "<c:CheckInput Grid.Column=\"3\" Grid.Row=\"2\" Grid.ColumnSpan=\"3\" Content=\"{x:Static local:Texts.Paid}\" IsChecked=\"{Binding Order.Paid}\" />");
    // The caption beside the switch, as on the web, so it fits its row.
    has(window, "<c:SwitchInput Grid.Column=\"6\" Grid.Row=\"2\" Grid.ColumnSpan=\"3\" OnContent=\"{x:Static local:Texts.Rush}\" OffContent=\"{x:Static local:Texts.Rush}\" IsChecked=\"{Binding Order.Paid}\" />");
    has(window, "<c:ChoiceInput Grid.Column=\"0\" Grid.Row=\"3\" Grid.ColumnSpan=\"4\" Choices=\"{x:Static local:Choices.OrderStatus}\" Value=\"{Binding Order.Status}\" />");
    has(window, "<c:ChoiceInput x:Name=\"customerPick\" Grid.Column=\"4\" Grid.Row=\"3\" Grid.ColumnSpan=\"4\" Choices=\"{Binding CustomerChoices}\" Value=\"{Binding Order.Customer}\"");
    has(window, "<c:DateInput Grid.Column=\"8\" Grid.Row=\"3\" Grid.ColumnSpan=\"3\" Value=\"{Binding Order.OrderedOn}\" />");
    has(window, "Text=\"{Binding Order.Status, Converter={x:Static local:Shown.Converter}}\"");
    has(window, "<c:ClarityTable x:Name=\"grid\" Grid.Column=\"0\" Grid.Row=\"5\" Grid.ColumnSpan=\"12\" Grid.RowSpan=\"4\" ItemsSource=\"{Binding Orders}\" SelectedItem=\"{Binding Grid.SelectedItem}\"");
    has(window, "<c:TableColumn Field=\"Status\" Header=\"{x:Static local:Texts.Status}\" Kind=\"Choice\" Shown=\"{x:Static local:Shown.Converter}\" />");
    // A value with a format shows in the person's format.
    has(window, "<c:TableColumn Field=\"Total\" Header=\"{x:Static local:Texts.Total}\" Kind=\"Number\" Shown=\"{x:Static local:Shown.OrderTotal}\" />");
    has(window, "<c:TextInput x:Name=\"numberFilter\" Grid.Column=\"0\" Grid.Row=\"9\" Grid.ColumnSpan=\"4\" Value=\"{Binding NumberFilter.Text}\"");
    has(window, "Content=\"{Binding ApproveButton.Caption}\" Command=\"{Binding ApproveClickedCommand}\" IsDefault=\"True\"");
    has(window, "<Button Grid.Column=\"10\" Grid.Row=\"9\" Grid.ColumnSpan=\"2\" Content=\"{x:Static local:Texts.New}\" Command=\"{Binding OpenOrderFormCommand}\" />");
    has(window, "<TabItem Header=\"{x:Static local:Texts.Notes}\">");
    has(window, "<c:TextInput Grid.Column=\"0\" Grid.Row=\"0\" Grid.ColumnSpan=\"12\" Grid.RowSpan=\"3\" Classes=\"memo\" AcceptsReturn=\"True\" TextWrapping=\"Wrap\" Value=\"{Binding Order.Number}\" />");
    // No stock input is placed directly: the library's own decide when `change` runs.
    for stock in ["<TextBox ", "<CheckBox ", "<ToggleSwitch ", "<ComboBox ", "<CalendarDatePicker ", "<RadioButton "] {
        assert!(!window.contains(stock), "{stock} in\n{window}");
    }
}

/// A radio group is the library's, with the enum's choices.
#[test]
fn a_radio_group_picks_an_enum_value() {
    let source = SHOP.replace("    dropdown at 1,4 span 4 bind order.status\n", "    radio name status_pick at 1,4 span 4 bind order.status vertical on change: filter_changed\n");
    let project = load_sources([("shop.clarity", source.as_str())]);
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    let files = generate(&project, &settings()).unwrap_or_else(|e| panic!("{e:#?}"));
    has(
        file(&files, "Windows/OrderFormView.axaml"),
        "<c:RadioInput x:Name=\"statusPick\" Grid.Column=\"0\" Grid.Row=\"3\" Grid.ColumnSpan=\"4\" Choices=\"{x:Static local:Choices.OrderStatus}\" Value=\"{Binding Order.Status}\" Vertical=\"True\"",
    );
    has(file(&files, "Windows/OrderFormView.axaml.cs"), "        statusPick.ValueChanged += (_, _) => Vm.FilterChangedCommand.Execute(null);");
}

#[test]
fn the_frame_comes_from_the_app() {
    let files = files();
    let window = file(&files, "Windows/OrderFormView.axaml");
    has(window, "<Menu DockPanel.Dock=\"Top\">\n");
    has(window, "<MenuItem Header=\"{x:Static local:Texts.Orders}\" InputGesture=\"Ctrl+O\" HotKey=\"Ctrl+O\" Command=\"{Binding OpenOrderFormCommand}\" />");
    has(window, "<Separator />");
    has(window, "<MenuItem Header=\"{x:Static local:Texts.Quit}\" Command=\"{Binding QuitCommand}\" />");
    // A window opens in a desktop window of its own only where the design says so.
    has(window, "<MenuItem Header=\"{x:Static local:Texts.AnotherOrder}\" Command=\"{Binding OpenOrderFormInNewWindowCommand}\" />");
    has(window, "<TextBlock Classes=\"status\" Text=\"{x:Static local:Texts.Ready}\" />");
}

#[test]
fn handlers_are_viewmodel_methods() {
    let files = files();
    let vm = file(&files, "Windows/OrderFormViewModel.cs");
    has(vm, "public sealed partial class OrderFormViewModel : WindowViewModel");
    has(vm, "    public override string Title => Texts.Order;");
    has(vm, "    public Order Order { get => _order; set => SetProperty(ref _order, value); }\n    Order _order = new();");
    has(vm, "    public List<Order> Orders { get => _orders; set => SetProperty(ref _orders, value); }");
    has(vm, "    public PartState NumberInput { get; } = new(\"number_input\");");
    has(vm, "    public PartState ApproveButton { get; } = new(\"approve_button\", caption: \"Approve\");");
    has(vm, "        _orders = Db.All<Order>();");
    has(vm, "        CustomerChoices = Choice.Of(Db.All<Customer>(), r => r.Name);");
    has(vm, "    [RelayCommand]\n    void ApproveClicked() => Handle(OnApproveClicked);");
    has(
        vm,
        "    void OnApproveClicked()
    {
        if (!Logic.WithinCredit(Db, Order))
        {
            Say(Level.Warning, new Text(\"Order {order.number} is too large\").With(\"order.number\", Slot.OfText(Order.Number)));
            ApproveButton.IsEnabled = false;
            return;
        }
        Order.Status = OrderStatus.Approved;
        Db.Save(Order);
        NumberInput.ErrorText = Text.Plain(\"\");
        Say(Level.Info, Text.Plain(\"Approved\"));
        OnFilterChanged();
    }",
    );
    has(vm, "        Orders = Db.Find<Order>(new Query().Filter(Cond.Contains(\"number\", NumberFilter.Text)).Order(\"number\", false));");
    has(vm, "        Order = Selected<Order>(Grid);");
    has(vm, "        Open(new OrderFormViewModel(Db) { Order = Selected<Order>(Grid) });\n        Close();");
    has(vm, "        Orders = Db.All<Order>();\n        if (Order.Id != 0)\n            Order = Db.Get<Order>(Order.Id);");
    has(vm, "        NumberInput.IsReadOnly = Order.Paid;");
    has(vm, "    protected override void OnOpen() => OnWindowOpened();");
    has(vm, "    protected override void OnReturn() => OnWindowReturned();");
    has(vm, "    [RelayCommand]\n    void OpenOrderForm() => Handle(() => Open(new OrderFormViewModel(Db)));");
    has(vm, "    [RelayCommand]\n    void Quit() => Handle(QuitApplication);");
    has(vm, "    [RelayCommand]\n    void OpenOrderFormInNewWindow() => Handle(() => Open(new OrderFormViewModel(Db), newWindow: true));");
}

#[test]
fn events_without_a_command_are_wired_in_the_code_behind() {
    let files = files();
    let code = file(&files, "Windows/OrderFormView.axaml.cs");
    has(code, "public partial class OrderFormView : UserControl");
    // Every input of the library tells the same way when the person changed it.
    has(code, "        numberFilter.ValueChanged += (_, _) => Vm.FilterChangedCommand.Execute(null);");
    has(code, "        grid.Selected += (_, _) => Vm.GridSelectedCommand.Execute(null);");
    has(code, "        grid.Opened += (_, _) => Vm.GridOpenedCommand.Execute(null);");
}

/// One desktop window (DESIGN.md, Page kinds): the runtime's shell is the only
/// `Window`; every Clarity window is a view in it.
#[test]
fn the_application_has_one_desktop_window() {
    let files = files();
    for f in files.iter().filter(|f| !f.path.starts_with("Clarity.")) {
        let derives = f.text.lines().any(|l| l.trim_end().ends_with(": Window") || l.contains(": Window,"));
        assert!(!derives && !f.text.contains("new Window") && !f.text.contains("<Window "), "{} makes a window", f.path);
    }
    has(file(&files, "Windows/OrderFormView.axaml"), "<UserControl ");
}

const NL: &str = "translation nl
    \"Order\" \"Bestelling\"
    \"Approved by sales\" \"Goedgekeurd door verkoop\"
    \"Order {order.number} is too large\" \"Bestelling {order.number} is te groot\"
";

fn translated() -> Vec<OutFile> {
    let mut project = load_sources([("shop.clarity", SHOP)]);
    clarity_spec::translations::attach(&mut project, [("translations/nl.clarity", NL)]);
    project.locale = Locale {
        language: Some("en".into()),
        currency: Some("EUR".into()),
    };
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    generate(&project, &settings()).unwrap_or_else(|errors| panic!("{errors:#?}"))
}

/// Texts as Visual Studio keeps them (DESIGN 1.5): a .resx file per language
/// and a typed accessor, found by what a text says as written.
#[test]
fn texts_are_resx_files_with_a_typed_accessor() {
    let files = translated();
    let base = file(&files, "Texts.resx");
    has(base, "<resheader name=\"resmimetype\">\n    <value>text/microsoft-resx</value>\n  </resheader>");
    has(base, "  <data name=\"ApprovedBySales\" xml:space=\"preserve\">\n    <value>Approved by sales</value>\n    <comment>enum OrderStatus</comment>\n  </data>");
    has(base, "  <data name=\"OrderOrderNumberIsTooLarge\" xml:space=\"preserve\">\n    <value>Order {order.number} is too large</value>");
    let nl = file(&files, "Texts.nl.resx");
    has(nl, "  <data name=\"Order\" xml:space=\"preserve\">\n    <value>Bestelling</value>");
    has(nl, "    <value>Bestelling {order.number} is te groot</value>");
    // A text without a translation falls back to the base file.
    assert!(!nl.contains("name=\"Paid\""), "{nl}");

    let texts = file(&files, "Texts.cs");
    has(texts, "public static class Texts");
    has(texts, "    static readonly ResourceManager Resources = new(\"Shop.Texts\", typeof(Texts).Assembly);");
    has(texts, "    public const string? BaseLanguage = \"en\";");
    has(texts, "    public static readonly string[] Languages = [\"nl\"];");
    has(texts, "        [\"Order {order.number} is too large\"] = \"OrderOrderNumberIsTooLarge\",");
    has(texts, "    /// <summary>Approved by sales</summary>\n    public static string ApprovedBySales => Locale.Translate(\"Approved by sales\");");
    // The renderer's own words are texts too.
    has(texts, "    public static string Yes => Locale.Translate(\"Yes\");");
}

#[test]
fn the_application_starts_in_the_language_and_theme_kept() {
    let files = translated();
    let app = file(&files, "App.axaml.cs");
    has(
        app,
        "            Remembered.Use(\"Shop\");
            Locale.Start(Texts.BaseLanguage, Texts.Languages, \"EUR\", Texts.In, Labels.Named);
            Themes.Start(AppThemes.Offered);
            var db = Program.OpenDatabases();",
    );
    // Without a currency, money has no sign.
    let plain = self::files();
    has(file(&plain, "App.axaml.cs"), "Locale.Start(Texts.BaseLanguage, Texts.Languages, null, Texts.In, Labels.Named);");
    has(file(&plain, "Texts.cs"), "    public static readonly string[] Languages = [];");
}

#[test]
fn values_show_and_read_in_the_persons_format() {
    let files = files();
    let shown = file(&files, "Shown.cs");
    has(shown, "    public static readonly FormatConverter OrderTotal = new(new FieldFormat(FormatKind.Money));");
    has(shown, "    public static readonly FormatConverter OrderOrderedOn = new(new FieldFormat(FormatKind.Date));");
    has(shown, "    public static readonly FormatConverter OrderDiscount = new(new FieldFormat(FormatKind.Decimal, 4, \"percent\"));");
    has(shown, "public static class Edit\n{");
    has(shown, "    public static readonly FieldFormat OrderTotal = new FieldFormat(FormatKind.Money);");
    // An enum value in text with slots shows its label.
    has(shown, "        \"OrderStatus\" => Enum.TryParse<OrderStatus>(value, out var v) ? Of(v) : value,");
}

/// The click targets `language` and `theme` pick in place (one window): a
/// submenu on a menu item, a flyout under a button.
#[test]
fn the_language_and_theme_targets_pick_in_place() {
    let files = files();
    let window = file(&files, "Windows/OrderFormView.axaml");
    has(window, "<MenuItem x:Name=\"picker1\" Header=\"{x:Static local:Texts.Language}\" />");
    has(window, "<MenuItem x:Name=\"picker2\" Header=\"{x:Static local:Texts.Theme}\" />");
    has(window, "<Button x:Name=\"picker3\" Content=\"{x:Static local:Texts.Look}\" />");
    let code = file(&files, "Windows/OrderFormView.axaml.cs");
    has(code, "        Pickers.Language(picker1);\n        Pickers.Theme(picker2);\n        Pickers.Theme(picker3);\n");
}

/// The selected row goes on into paths and calls, as any record does.
#[test]
fn the_selected_row_is_a_record_in_any_expression() {
    let source = SHOP.replace(
        "    handler grid_opened\n",
        "    handler grid_opened\n        if within_credit(grid.selected)\n            message \"{grid.selected.number} of {grid.selected.customer.name} fits\"\n",
    );
    let project = load_sources([("shop.clarity", source.as_str())]);
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    let files = generate(&project, &settings()).unwrap_or_else(|e| panic!("{e:#?}"));
    let vm = file(&files, "Windows/OrderFormViewModel.cs");
    has(vm, "if (Logic.WithinCredit(Db, Selected<Order>(Grid)))");
    has(vm, ".With(\"grid.selected.number\", Slot.OfText(Selected<Order>(Grid).Number))");
    has(vm, "Db.Load<Customer>(Selected<Order>(Grid).Customer).Name");
}

/// An unbound checkbox holds its own state in its PartState (X2).
#[test]
fn an_unbound_checkbox_is_checked_in_its_state() {
    let source = SHOP
        .replace("    checkbox \"Paid\" at 4,3 span 3 bind order.paid\n", "    checkbox \"Paid\" at 4,3 span 3 bind order.paid\n    checkbox \"Only mine\" name mine at 10,3 span 3\n")
        .replace("    handler grid_selected\n", "    handler grid_selected\n        if mine.checked\n            mine.checked = false\n");
    let project = load_sources([("shop.clarity", source.as_str())]);
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    let files = generate(&project, &settings()).unwrap_or_else(|e| panic!("{e:#?}"));
    has(file(&files, "Windows/OrderFormView.axaml"), "<c:CheckInput x:Name=\"mine\" Grid.Column=\"9\" Grid.Row=\"2\" Grid.ColumnSpan=\"3\" Content=\"{x:Static local:Texts.OnlyMine}\" IsChecked=\"{Binding Mine.IsChecked}\"");
    let vm = file(&files, "Windows/OrderFormViewModel.cs");
    has(vm, "if (Mine.IsChecked)");
    has(vm, "Mine.IsChecked = false;");
}

const TODO: &str = "module todo

enum Priority
    Low
    High

entity Person
    name: text(80)

entity Task
    title: text(120), required
    done: bool
    priority: Priority
    due: date
    hours: decimal(5, 1)
    owner: ref Person
    added: datetime
    urgent: bool

page Main: window
    data tasks: list<Task>
    data task: Task
    date name added_pick at 1,7 span 4 bind task.added on change: task_changed
    table name grid at 1,1 span 12 rows 6 bind tasks columns title, done, priority, due, hours, owner, added, urgent edit title, done, priority, due, hours, owner, added, urgent as switch on change: task_changed

code Main
    handler task_changed
        save grid.selected
";

#[test]
fn edited_columns_hold_the_inputs_a_form_would() {
    let project = load_sources([("todo.clarity", TODO)]);
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    let mut settings = settings();
    settings.name = "Todo".into();
    let files = generate(&project, &settings).unwrap_or_else(|errors| panic!("{errors:#?}"));
    let window = file(&files, "Windows/MainView.axaml");
    // The table is Clarity's own part: the window only describes its columns.
    has(window, "<c:ClarityTable x:Name=\"grid\" Grid.Column=\"0\" Grid.Row=\"0\" Grid.ColumnSpan=\"12\" Grid.RowSpan=\"6\" ItemsSource=\"{Binding Tasks}\" SelectedItem=\"{Binding Grid.SelectedItem}\"");
    let column = |attributes: &str| format!("<c:TableColumn {attributes} />");
    has(window, &column("Field=\"Title\" Header=\"{x:Static local:Texts.Title}\" Kind=\"Text\" Edit=\"True\""));
    has(window, &column("Field=\"Done\" Header=\"{x:Static local:Texts.Done}\" Kind=\"Bool\" Edit=\"True\""));
    // A bool column written `as switch` is a switch, as the switch part of a form.
    has(window, &column("Field=\"Urgent\" Header=\"{x:Static local:Texts.Urgent}\" Kind=\"Bool\" Edit=\"True\" Switch=\"True\""));
    has(window, &column("Field=\"Priority\" Header=\"{x:Static local:Texts.Priority}\" Kind=\"Choice\" Edit=\"True\" Choices=\"{x:Static local:Choices.Priority}\""));
    has(window, &column("Field=\"Due\" Header=\"{x:Static local:Texts.Due}\" Kind=\"Date\" Edit=\"True\""));
    // A moment is picked as a day and a time, in a cell and in a form alike.
    has(window, &column("Field=\"Added\" Header=\"{x:Static local:Texts.Added}\" Kind=\"Moment\" Edit=\"True\""));
    has(window, "<c:DateTimeInput x:Name=\"addedPick\" Grid.Column=\"0\" Grid.Row=\"6\" Grid.ColumnSpan=\"4\" Value=\"{Binding Task.Added}\"");
    has(window, &column("Field=\"Hours\" Header=\"{x:Static local:Texts.Hours}\" Kind=\"Number\" Edit=\"True\" Number=\"{x:Static local:Edit.TaskHours}\""));
    // A reference picks from the records the window reads, as a dropdown in a form does.
    has(window, &column("Field=\"Owner\" Header=\"{x:Static local:Texts.Owner}\" Kind=\"Choice\" Edit=\"True\" ChoicesFrom=\"PersonChoices\""));
    has(file(&files, "Windows/MainViewModel.cs"), "PersonChoices = Choice.Of(Db.All<Person>(), r => ");
    // An edit by the person (not by a handler) selects its row, then the table's change runs.
    let code = file(&files, "Windows/MainView.axaml.cs");
    // The cells' own inputs tell what the person edited; a handler's changes never count.
    assert!(!code.contains("Handling"), "{code}");
    has(code, "grid.Edited += _ => Vm.TaskChangedCommand.Execute(null);");
    has(code, "addedPick.ValueChanged += (_, _) => Vm.TaskChangedCommand.Execute(null);");
}

/// A window that says its size opens at it: its view model tells the shell.
#[test]
fn a_window_opens_at_its_size() {
    let vm = file(&files(), "Windows/OrderFormViewModel.cs").to_string();
    assert!(!vm.contains("WindowSize"), "{vm}");
    let sized = SHOP.replace("page OrderForm: window\n", "page OrderForm: window\n    size 640 x 480\n");
    assert_ne!(sized, SHOP);
    let project = load_sources([("shop.clarity", sized.as_str())]);
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    let files = generate(&project, &settings()).unwrap_or_else(|e| panic!("{e:#?}"));
    has(file(&files, "Windows/OrderFormViewModel.cs"), "    public override global::Avalonia.Size? WindowSize => new(640, 480);\n");
}

/// A click can pass values to its handler: each written call is a command of
/// its own, with its values written into the code.
#[test]
fn a_click_passes_values_to_its_handler() {
    let source = SHOP
        .replace(
            "    button \"New\" at 11,10 span 2 on click -> OrderForm\n",
            "    button \"New\" at 11,10 span 2 on click -> OrderForm\n    button \"7\" name d7 at 5,10 span 1 on click: digit(7)\n    button \"Half\" name half at 6,10 span 1 on click: scale(0.5, \"half\", Approved)\n    textbox name one at 7,10 span 1 on change: digit(1)\n",
        )
        .replace(
            "    handler grid_selected\n",
            "    handler digit(d: integer)\n        number_input.error = \"\"\n\n    handler scale(by: decimal, word: text, status: OrderStatus)\n        digit(3)\n\n    handler grid_selected\n",
        );
    assert_ne!(source, SHOP);
    let project = load_sources([("shop.clarity", source.as_str())]);
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    let files = generate(&project, &settings()).unwrap_or_else(|e| panic!("{e:#?}"));
    let vm = file(&files, "Windows/OrderFormViewModel.cs");
    has(vm, "    void OnDigit(long d)\n");
    has(vm, "    void OnScale(decimal by, string word, OrderStatus status)\n");
    has(vm, "        OnDigit(3L);\n");
    has(vm, "    [RelayCommand]\n    void Digit7() => Handle(() => OnDigit(7L));");
    has(vm, "    [RelayCommand]\n    void Digit1() => Handle(() => OnDigit(1L));");
    has(vm, "    [RelayCommand]\n    void Scale0_5HalfApproved() => Handle(() => OnScale(0.5m, \"half\", OrderStatus.Approved));");
    // A handler that takes values is only run with them.
    assert!(!vm.contains("void Digit() =>"), "{vm}");
    let axaml = file(&files, "Windows/OrderFormView.axaml");
    has(axaml, "Command=\"{Binding Digit7Command}\"");
    has(axaml, "Command=\"{Binding Scale0_5HalfApprovedCommand}\"");
    has(file(&files, "Windows/OrderFormView.axaml.cs"), "Vm.Digit1Command.Execute(null)");
}

/// A window with plain values besides records, as each stack's tests use it.
const CALC: &str = "
page Calc: window
    title \"Calc\"
    data shown: decimal
    data pending: OrderStatus
    data fresh: bool, default true
    data word: text, default \"hi\"
    data count: integer, default 2
    data rate: decimal, default 0.5
    text name display at 1,1 span 4 bind shown
    button \"7\" name d7 at 1,2 span 1 on click: digit(7)

code Calc
    handler digit(d: integer)
        if fresh
            shown = 0
            fresh = false
        shown = shown * 10 + d
        pending = Approved
        count = count + 1
        word = \"x\"
";

/// A window holds plain values: properties of its ViewModel, at their
/// default or empty when it opens, read and changed by its handlers.
#[test]
fn a_window_holds_plain_values() {
    let source = format!("{SHOP}{CALC}");
    let project = load_sources([("shop.clarity", source.as_str())]);
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    let files = generate(&project, &settings()).unwrap_or_else(|e| panic!("{e:#?}"));
    let vm = file(&files, "Windows/CalcViewModel.cs");
    has(vm, "    public decimal Shown { get => _shown; set => SetProperty(ref _shown, value); }\n    decimal _shown = default;\n");
    for start in ["    OrderStatus _pending = default;\n", "    bool _fresh = true;\n", "    string _word = \"hi\";\n", "    long _count = 2L;\n", "    decimal _rate = 0.5m;\n"] {
        has(vm, start);
    }
    has(vm, "        Fresh = false;\n");
    has(vm, "        Pending = OrderStatus.Approved;\n");
    has(file(&files, "Windows/CalcView.axaml"), "Text=\"{Binding Shown, Converter={x:Static local:Shown.Converter}}\"");
}

/// A character key (`7`, `+`) is what is typed, from the keyboard or the
/// number pad; a key with modifiers is a gesture, as Avalonia names keys.
#[test]
fn keys_are_written_as_avalonia_names_them() {
    let source = SHOP.replace(
        "    button \"New\" at 11,10 span 2 on click -> OrderForm\n",
        "    button \"New\" at 11,10 span 2 on click -> OrderForm\n    button \"7\" name d7 at 1,13 span 1 key \"7\" on click: approve_clicked\n    button \"+\" name plus at 2,13 span 1 key \"+\" on click: approve_clicked\n    button \"Back\" name back at 3,13 span 1 key \"Backspace\" on click: approve_clicked\n    button \"Save\" name keep at 4,13 span 1 key \"Ctrl+S\" on click: approve_clicked\n    button \"Zoom\" name zoom at 5,13 span 1 key \"Ctrl++\" on click: approve_clicked\n",
    );
    assert_ne!(source, SHOP);
    let project = load_sources([("shop.clarity", source.as_str())]);
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    let files = generate(&project, &settings()).unwrap_or_else(|e| panic!("{e:#?}"));
    let axaml = file(&files, "Windows/OrderFormView.axaml");
    has(axaml, "c:Keys.Typed=\"7\"");
    has(axaml, "c:Keys.Typed=\"+\"");
    has(axaml, "HotKey=\"Back\"");
    has(axaml, "HotKey=\"Ctrl+S\"");
    has(axaml, "HotKey=\"Ctrl+OemPlus\"");
    assert!(!axaml.contains("HotKey=\"7\"") && !axaml.contains("HotKey=\"D7\""), "{axaml}");
}

/// round(value, places) rounds through the runtime, as every stack does.
#[test]
fn round_rounds_through_the_runtime() {
    let source = format!("{SHOP}\nfunction rounded(x: decimal) -> decimal\n    return round(x, 2)\n");
    let project = load_sources([("m.clarity", source.as_str())]);
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    let files = generate(&project, &settings()).unwrap_or_else(|e| panic!("{e:#?}"));
    let code: String = files.iter().map(|f| f.text.as_str()).collect();
    assert!(code.contains("Numbers.Round(x, 2L)"), "{code}");
}

/// A display draws its number in seven-segment digits through the runtime.
#[test]
fn a_display_is_the_runtimes_segment_display() {
    let source = format!("{SHOP}{CALC}").replace(
        "    text name display at 1,1 span 4 bind shown\n",
        "    display name screen at 1,1 span 4 rows 1 bind shown digits 10\n",
    );
    let project = load_sources([("shop.clarity", source.as_str())]);
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    let files = generate(&project, &settings()).unwrap_or_else(|e| panic!("{e:#?}"));
    has(file(&files, "Windows/CalcView.axaml"), "<c:SegmentDisplay x:Name=\"screen\" Grid.Column=\"0\" Grid.Row=\"0\" Grid.ColumnSpan=\"4\" Value=\"{Binding Shown}\" Digits=\"10\"");
}

/// A window with a canvas its draw handler draws, as each stack's tests use it.
const DIAL: &str = "
page Dial: window
    data angle: decimal, default 30
    data speed: integer, default 80
    canvas name dial at 1,1 span 6 rows 6 units 200 x 100 on draw: draw_dial
    button \"Faster\" name faster at 7,1 span 2 on click: faster_clicked

code Dial
    handler draw_dial
        dial.clear(\"#101810\")
        dial.line(50, 50, 50 + 35 * cos(angle), 50 - 35 * sin(angle), \"text\", 3)
        dial.text(50, 90, \"{speed} km/h\")

    handler faster_clicked
        speed = speed + 10
";

/// A draw handler adds shapes to its canvas's drawing, which the runtime's
/// CanvasView shows; the window draws it anew after every handler.
#[test]
fn a_canvas_is_drawn_by_its_draw_handler() {
    let source = format!("{SHOP}{DIAL}");
    let project = load_sources([("shop.clarity", source.as_str())]);
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    let files = generate(&project, &settings()).unwrap_or_else(|e| panic!("{e:#?}"));
    let vm = file(&files, "Windows/DialViewModel.cs");
    has(vm, "        Dial.Draw(new Shape.Clear(\"#101810\"));");
    has(vm, "Dial.Draw(new Shape.Line((double)(50L), (double)(50L), Numbers.Units((50L + (35L * Numbers.Cos(Angle))))");
    has(vm, "\"text\", (double)(3L)));");
    // Left out: the text's color and size.
    has(vm, ", \"text\", 10));");
    has(vm, "    protected override void Redraw()\n    {\n        Dial.BeginDrawing();\n        OnDrawDial();\n        Dial.EndDrawing();\n    }");
    has(file(&files, "Windows/DialView.axaml"), "<c:CanvasView x:Name=\"dial\" Grid.Column=\"0\" Grid.Row=\"0\" Grid.ColumnSpan=\"6\" Grid.RowSpan=\"6\" Drawing=\"{Binding Dial.Drawing}\" UnitsWidth=\"200\" UnitsHeight=\"100\"");
}
