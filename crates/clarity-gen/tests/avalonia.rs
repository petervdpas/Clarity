//! The Avalonia windows of the C# backend (G2b): each window as AXAML with
//! a ViewModel, its parts bound to the data and to their state, its handlers
//! as ViewModel methods.

use clarity_gen::{OutFile, generate};
use clarity_project::{Backend, Settings, Ui, starter};
use clarity_spec::keyed::load_keyed;
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
    menu name mainMenu
        item \"File\" name fileItem
            item \"Orders\" name ordersItem key \"Ctrl+O\" on click -> OrderForm
            item \"Another order\" name anotherOrderItem on click -> OrderForm in new window
            separator
            item \"Quit\" name quitItem on click -> quit
            item \"Language\" name languageItem on click -> language
            item \"Theme\" name themeItem on click -> theme
    toolbar name mainToolbar
        button \"Look\" name lookButton on click -> theme
    footer
        status \"Ready\"

page OrderForm: window
    title \"Order\"
    data order: Order
    data orders: list<Order>
    on open: windowOpened
    on return: windowReturned
    label \"Number\" name label1 at 1,1 span 2 for numberInput
    textbox name numberInput at 3,1 span 4 bind order.number placeholder \"ORD-1\"
    help name help1 at 3,2 span 4 for numberInput
    number name number1 at 1,3 span 3 bind order.total
    checkbox \"Paid\" name checkbox1 at 4,3 span 3 bind order.paid
    switch \"Rush\" name switch1 at 7,3 span 3 bind order.paid
    dropdown name dropdown1 at 1,4 span 4 bind order.status
    dropdown name customerPick at 5,4 span 4 bind order.customer
    date name date1 at 9,4 span 3 bind order.ordered_on
    text name text1 at 1,5 span 4 bind order.status
    table name grid at 1,6 span 12 rows 4 bind orders columns number, status, total on select: gridSelected on open: gridOpened
    textbox name numberFilter at 1,10 span 4 on change: filterChanged
    button \"Approve\" name approveButton at 9,10 span 2 default on click: approveClicked
    button \"New\" name button1 at 11,10 span 2 on click -> OrderForm
    tabs name tabs1 at 1,11 span 12 rows 2
        tab \"Notes\"
            memo name memo1 at 1,1 span 12 bind order.number

code OrderForm
    handler windowOpened
        numberInput.readonly = order.paid

    handler windowReturned
        reload orders
        reload order

    handler approveClicked
        if not within_credit(order)
            message warning \"Order {order.number} is too large\"
            approveButton.enabled = false
            return
        order.status = Approved
        save order
        numberInput.error = \"\"
        message \"Approved\"
        filterChanged()

    handler filterChanged
        orders = find o in Order where o.number contains numberFilter.text order by o.number

    handler gridSelected
        order = grid.selected

    handler gridOpened
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
    let project = load_keyed([("shop.clarity", SHOP)], &[], "en");
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
    // The desktop's own (native windows and sound) come with Clarity.Avalonia.Desktop,
    // Avalonia and CommunityToolkit.Mvvm with the Clarity.Avalonia it brings.
    has(project, "<PackageReference Include=\"Clarity.Avalonia.Desktop\"");
    // They are not named again.
    assert!(!project.contains("Include=\"Avalonia.Desktop\""), "{project}");
    has(file(&files, "App.axaml"), "<FluentTheme />");
    has(file(&files, "App.axaml.cs"), "desktop.MainWindow = Shell.Start(new OrderFormViewModel(db), Views.Create, TimeSpan.FromMilliseconds(5000));");
    has(file(&files, "Program.cs"), ".StartWithClassicDesktopLifetime(args)");
    has(file(&files, "Program.cs"), "AppBuilder.Configure<App>().UseClarityDesktop().LogToTrace()");
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
    has(shop, "public static partial class Choices\n{\n    public static Choice[] OrderStatus =>\n    [\n        new(global::Shop.OrderStatus.Pending, Texts.OrderStatusPending),\n        new(global::Shop.OrderStatus.Approved, Texts.OrderStatusApproved),\n    ];");
    has(shop, "        OrderStatus.Approved => Texts.OrderStatusApproved,");
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
    // A named label shows its caption as a handler sets it, still marked when its input is required.
    has(window, "<TextBlock x:Name=\"label1\" Grid.Column=\"0\" Grid.Row=\"0\" Grid.ColumnSpan=\"2\" Classes=\"label\"><Run Text=\"{Binding Label1.Caption}\" /><Run Text=\" *\" /></TextBlock>");
    has(
        window,
        "<c:TextInput x:Name=\"numberInput\" Grid.Column=\"2\" Grid.Row=\"0\" Grid.ColumnSpan=\"4\" Value=\"{Binding Order.Number}\" PlaceholderText=\"{x:Static local:Texts.NumberInputPlaceholder}\" IsEnabled=\"{Binding NumberInput.IsEnabled}\" IsVisible=\"{Binding NumberInput.IsVisible}\" IsReadOnly=\"{Binding NumberInput.IsReadOnly}\" />",
    );
    has(window, "<TextBlock x:Name=\"help1\" Grid.Column=\"2\" Grid.Row=\"1\" Grid.ColumnSpan=\"4\" Classes=\"help\" Classes.error=\"{Binding NumberInput.HasError}\" Text=\"{Binding NumberInput.Shown}\" />");
    // Clarity's own inputs: each tells when the person changed its value, and only then.
    has(window, "<c:NumberInput x:Name=\"number1\" Grid.Column=\"0\" Grid.Row=\"2\" Grid.ColumnSpan=\"3\" Format=\"{x:Static local:Edit.OrderTotal}\" Value=\"{Binding Order.Total}\"");
    has(window, "<c:CheckInput x:Name=\"checkbox1\" Grid.Column=\"3\" Grid.Row=\"2\" Grid.ColumnSpan=\"3\" Content=\"{x:Static local:Texts.Checkbox1Caption}\" IsChecked=\"{Binding Order.Paid}\"");
    // The caption beside the switch, as on the web, so it fits its row.
    has(window, "<c:SwitchInput x:Name=\"switch1\" Grid.Column=\"6\" Grid.Row=\"2\" Grid.ColumnSpan=\"3\" OnContent=\"{x:Static local:Texts.Switch1Caption}\" OffContent=\"{x:Static local:Texts.Switch1Caption}\" IsChecked=\"{Binding Order.Paid}\"");
    has(window, "<c:ChoiceInput x:Name=\"dropdown1\" Grid.Column=\"0\" Grid.Row=\"3\" Grid.ColumnSpan=\"4\" Choices=\"{x:Static local:Choices.OrderStatus}\" Value=\"{Binding Order.Status}\"");
    has(window, "<c:ChoiceInput x:Name=\"customerPick\" Grid.Column=\"4\" Grid.Row=\"3\" Grid.ColumnSpan=\"4\" Choices=\"{Binding CustomerChoices}\" Value=\"{Binding Order.Customer}\"");
    has(window, "<c:DateInput x:Name=\"date1\" Grid.Column=\"8\" Grid.Row=\"3\" Grid.ColumnSpan=\"3\" Value=\"{Binding Order.OrderedOn}\"");
    has(window, "Text=\"{Binding Order.Status, Converter={x:Static local:Shown.Converter}}\"");
    has(window, "<c:ClarityTable x:Name=\"grid\" Grid.Column=\"0\" Grid.Row=\"5\" Grid.ColumnSpan=\"12\" Grid.RowSpan=\"4\" ItemsSource=\"{Binding Orders}\" SelectedItem=\"{Binding Grid.SelectedItem}\"");
    has(window, "<c:TableColumn Field=\"Status\" Header=\"{x:Static local:Texts.OrderStatusLabel}\" Kind=\"Choice\" Shown=\"{x:Static local:Shown.Converter}\" />");
    // A value with a format shows in the person's format.
    has(window, "<c:TableColumn Field=\"Total\" Header=\"{x:Static local:Texts.OrderTotalLabel}\" Kind=\"Number\" Shown=\"{x:Static local:Shown.OrderTotal}\" />");
    has(window, "<c:TextInput x:Name=\"numberFilter\" Grid.Column=\"0\" Grid.Row=\"9\" Grid.ColumnSpan=\"4\" Value=\"{Binding NumberFilter.Text}\"");
    has(window, "Content=\"{Binding ApproveButton.Caption}\" Command=\"{Binding ApproveClickedCommand}\" IsDefault=\"True\"");
    has(window, "<Button x:Name=\"button1\" Grid.Column=\"10\" Grid.Row=\"9\" Grid.ColumnSpan=\"2\" Content=\"{Binding Button1.Caption}\" Command=\"{Binding Open_OrderFormCommand}\"");
    has(window, "<TabItem Header=\"{x:Static local:Texts.OrderFormNotesTab}\">");
    has(window, "<c:TextInput x:Name=\"memo1\" Grid.Column=\"0\" Grid.Row=\"0\" Grid.ColumnSpan=\"12\" Grid.RowSpan=\"3\" Classes=\"memo\" AcceptsReturn=\"True\" TextWrapping=\"Wrap\" Value=\"{Binding Order.Number}\"");
    // No stock input is placed directly: the library's own decide when `change` runs.
    for stock in ["<TextBox ", "<CheckBox ", "<ToggleSwitch ", "<ComboBox ", "<CalendarDatePicker ", "<RadioButton "] {
        assert!(!window.contains(stock), "{stock} in\n{window}");
    }
}

/// A radio group is the library's, with the enum's choices.
#[test]
fn a_radio_group_picks_an_enum_value() {
    let source = SHOP.replace("    dropdown name dropdown1 at 1,4 span 4 bind order.status\n", "    radio name statusPick at 1,4 span 4 bind order.status vertical on change: filterChanged\n");
    let project = load_keyed([("shop.clarity", source.as_str())], &[], "en");
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
    has(window, "<MenuItem Header=\"{x:Static local:Texts.OrdersItem}\" InputGesture=\"Ctrl+O\" c:Keys.Frame=\"Ctrl+O\" Command=\"{Binding Open_OrderFormCommand}\" IsEnabled=\"{Binding OrdersItem.IsEnabled}\" IsVisible=\"{Binding OrdersItem.IsVisible}\" />");
    // The frame answers its keys after the window's own buttons, a ribbon's on every tab (DESIGN 5.28).
    has(window, "<DockPanel c:Keys.FrameRoot=\"True\">");
    has(window, "<Separator />");
    has(window, "<MenuItem Header=\"{x:Static local:Texts.QuitItem}\" Command=\"{Binding Quit_Command}\" IsEnabled=\"{Binding QuitItem.IsEnabled}\" IsVisible=\"{Binding QuitItem.IsVisible}\" />");
    // A window opens in a desktop window of its own only where the design says so.
    has(window, "<MenuItem Header=\"{x:Static local:Texts.AnotherOrderItem}\" Command=\"{Binding OpenNew_OrderFormCommand}\" IsEnabled=\"{Binding AnotherOrderItem.IsEnabled}\" IsVisible=\"{Binding AnotherOrderItem.IsVisible}\" />");
    has(window, "<TextBlock Classes=\"status\" Text=\"{x:Static local:Texts.ShopStatus}\" />");
}

/// The menu and the toolbar are parts: code hides and disables them by name.
#[test]
fn the_menu_and_the_toolbar_follow_their_part_state() {
    let files = files();
    let window = file(&files, "Windows/OrderFormView.axaml");
    has(window, "<Menu x:Name=\"mainMenu\" DockPanel.Dock=\"Top\" IsEnabled=\"{Binding MainMenu.IsEnabled}\" IsVisible=\"{Binding MainMenu.IsVisible}\">\n");
    has(window, "<Border x:Name=\"mainToolbar\" DockPanel.Dock=\"Top\" Classes=\"toolbar\" IsEnabled=\"{Binding MainToolbar.IsEnabled}\" IsVisible=\"{Binding MainToolbar.IsVisible}\">\n");
    let vm = file(&files, "Windows/OrderFormViewModel.cs");
    has(vm, "    public PartState MainMenu { get; } = new(\"mainMenu\");\n");
    has(vm, "    public PartState MainToolbar { get; } = new(\"mainToolbar\");\n");
}

#[test]
fn handlers_are_viewmodel_methods() {
    let files = files();
    let vm = file(&files, "Windows/OrderFormViewModel.cs");
    has(vm, "public sealed partial class OrderFormViewModel : WindowViewModel");
    has(vm, "    public override string Title => Texts.OrderFormTitle;");
    has(vm, "    public Order Order { get => _order; set => SetProperty(ref _order, value); }\n    Order _order = new();");
    has(vm, "    public List<Order> Orders { get => _orders; set => SetProperty(ref _orders, value); }");
    has(vm, "    public PartState NumberInput { get; } = new(\"numberInput\");");
    has(vm, "    public PartState ApproveButton { get; } = new(\"approveButton\", caption: \"approveButtonCaption\");");
    has(vm, "        _orders = Db.All<Order>();");
    has(vm, "        CustomerChoices = Choice.Of(Db.All<Customer>(), r => r.Name);");
    has(vm, "    [RelayCommand]\n    void ApproveClicked() => Handle(OnApproveClicked);");
    has(
        vm,
        "    void OnApproveClicked()
    {
        if (!Logic.WithinCredit(Db, Order))
        {
            Say(Level.Warning, Text.Of(\"orderNumberIsTooLarge\").With(\"number\", Slot.OfText(Order.Number)));
            ApproveButton.IsEnabled = false;
            return;
        }
        Order.Status = OrderStatus.Approved;
        Db.Save(Order);
        NumberInput.ErrorText = Text.Plain(\"\");
        Say(Level.Info, Text.Of(\"approved\"));
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
    has(vm, "    [RelayCommand]\n    void Open_OrderForm() => Handle(() => Open(new OrderFormViewModel(Db)));");
    has(vm, "    [RelayCommand]\n    void Quit_() => Handle(QuitApplication);");
    has(vm, "    [RelayCommand]\n    void OpenNew_OrderForm() => Handle(() => Open(new OrderFormViewModel(Db), newWindow: true));");
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
    let mut project = load_keyed([("shop.clarity", SHOP)], &[("translations/nl.clarity", NL)], "en");
    project.locale = Locale {
        language: Some("en".into()),
        currency: Some("EUR".into()),
    };
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    generate(&project, &settings()).unwrap_or_else(|errors| panic!("{errors:#?}"))
}

/// Texts as Visual Studio keeps them (DESIGN 1.5): a .resx file for the
/// default language and one per other language, and a typed accessor, each
/// key a resource by its name.
#[test]
fn texts_are_resx_files_with_a_typed_accessor() {
    let files = translated();
    let base = file(&files, "Texts.resx");
    has(base, "<resheader name=\"resmimetype\">\n    <value>text/microsoft-resx</value>\n  </resheader>");
    has(base, "  <data name=\"OrderStatusApproved\" xml:space=\"preserve\">\n    <value>Approved by sales</value>\n    <comment>enum OrderStatus</comment>\n  </data>");
    has(base, "  <data name=\"OrderNumberIsTooLarge\" xml:space=\"preserve\">\n    <value>Order {number} is too large</value>");
    let nl = file(&files, "Texts.nl.resx");
    has(nl, "  <data name=\"OrderFormTitle\" xml:space=\"preserve\">\n    <value>Bestelling</value>");
    has(nl, "    <value>Bestelling {number} is te groot</value>");
    // A key the language leaves empty falls back to the default's file.
    assert!(!nl.contains("name=\"Checkbox1Caption\""), "{nl}");

    let texts = file(&files, "Texts.cs");
    has(texts, "public static class Texts");
    has(texts, "    static readonly ResourceManager Resources = new(\"Shop.Texts\", typeof(Texts).Assembly);");
    has(texts, "    public const string? BaseLanguage = \"en\";");
    has(texts, "    public static readonly string[] Languages = [\"nl\"];");
    has(texts, "        [\"orderNumberIsTooLarge\"] = \"OrderNumberIsTooLarge\",");
    has(texts, "    /// <summary>Approved by sales</summary>\n    public static string OrderStatusApproved => Locale.Translate(\"orderStatusApproved\");");
    // The renderer's own words are keys too.
    has(texts, "    public static string Yes => Locale.Translate(\"yes\");");
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
    has(window, "<MenuItem x:Name=\"picker1\" Header=\"{x:Static local:Texts.LanguageItem}\" IsEnabled=\"{Binding LanguageItem.IsEnabled}\" IsVisible=\"{Binding LanguageItem.IsVisible}\" />");
    has(window, "<MenuItem x:Name=\"picker2\" Header=\"{x:Static local:Texts.ThemeItem}\" IsEnabled=\"{Binding ThemeItem.IsEnabled}\" IsVisible=\"{Binding ThemeItem.IsVisible}\" />");
    has(window, "<Button x:Name=\"picker3\" Content=\"{x:Static local:Texts.LookButtonCaption}\" />");
    let code = file(&files, "Windows/OrderFormView.axaml.cs");
    has(code, "        Pickers.Language(picker1);\n        Pickers.Theme(picker2);\n        Pickers.Theme(picker3);\n");
}

/// The selected row goes on into paths and calls, as any record does.
#[test]
fn the_selected_row_is_a_record_in_any_expression() {
    let source = SHOP.replace(
        "    handler gridOpened\n",
        "    handler gridOpened\n        if within_credit(grid.selected)\n            message \"{grid.selected.number} of {grid.selected.customer.name} fits\"\n",
    );
    let project = load_keyed([("shop.clarity", source.as_str())], &[], "en");
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    let files = generate(&project, &settings()).unwrap_or_else(|e| panic!("{e:#?}"));
    let vm = file(&files, "Windows/OrderFormViewModel.cs");
    has(vm, "if (Logic.WithinCredit(Db, Selected<Order>(Grid)))");
    has(vm, ".With(\"number\", Slot.OfText(Selected<Order>(Grid).Number))");
    has(vm, "Db.Load<Customer>(Selected<Order>(Grid).Customer).Name");
}

/// An unbound checkbox holds its own state in its PartState (X2).
#[test]
fn an_unbound_checkbox_is_checked_in_its_state() {
    let source = SHOP
        .replace("    checkbox \"Paid\" name checkbox1 at 4,3 span 3 bind order.paid\n", "    checkbox \"Paid\" name checkbox1 at 4,3 span 3 bind order.paid\n    checkbox \"Only mine\" name mine at 10,3 span 3\n")
        .replace("    handler gridSelected\n", "    handler gridSelected\n        if mine.checked\n            mine.checked = false\n");
    let project = load_keyed([("shop.clarity", source.as_str())], &[], "en");
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    let files = generate(&project, &settings()).unwrap_or_else(|e| panic!("{e:#?}"));
    has(file(&files, "Windows/OrderFormView.axaml"), "<c:CheckInput x:Name=\"mine\" Grid.Column=\"9\" Grid.Row=\"2\" Grid.ColumnSpan=\"3\" Content=\"{x:Static local:Texts.MineCaption}\" IsChecked=\"{Binding Mine.IsChecked}\"");
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
    date name addedPick at 1,7 span 4 bind task.added on change: taskChanged
    table name grid at 1,1 span 12 rows 6 bind tasks columns title, done, priority, due, hours, owner, added, urgent edit title, done, priority, due, hours, owner, added, urgent as switch on change: taskChanged

code Main
    handler taskChanged
        save grid.selected
";

#[test]
fn edited_columns_hold_the_inputs_a_form_would() {
    let project = load_keyed([("todo.clarity", TODO)], &[], "en");
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    let mut settings = settings();
    settings.name = "Todo".into();
    let files = generate(&project, &settings).unwrap_or_else(|errors| panic!("{errors:#?}"));
    let window = file(&files, "Windows/MainView.axaml");
    // The table is Clarity's own part: the window only describes its columns.
    has(window, "<c:ClarityTable x:Name=\"grid\" Grid.Column=\"0\" Grid.Row=\"0\" Grid.ColumnSpan=\"12\" Grid.RowSpan=\"6\" ItemsSource=\"{Binding Tasks}\" SelectedItem=\"{Binding Grid.SelectedItem}\"");
    let column = |attributes: &str| format!("<c:TableColumn {attributes} />");
    has(window, &column("Field=\"Title\" Header=\"{x:Static local:Texts.TaskTitleLabel}\" Kind=\"Text\" Edit=\"True\""));
    has(window, &column("Field=\"Done\" Header=\"{x:Static local:Texts.TaskDoneLabel}\" Kind=\"Bool\" Edit=\"True\""));
    // A bool column written `as switch` is a switch, as the switch part of a form.
    has(window, &column("Field=\"Urgent\" Header=\"{x:Static local:Texts.TaskUrgentLabel}\" Kind=\"Bool\" Edit=\"True\" Switch=\"True\""));
    has(window, &column("Field=\"Priority\" Header=\"{x:Static local:Texts.TaskPriorityLabel}\" Kind=\"Choice\" Edit=\"True\" Choices=\"{x:Static local:Choices.Priority}\""));
    has(window, &column("Field=\"Due\" Header=\"{x:Static local:Texts.TaskDueLabel}\" Kind=\"Date\" Edit=\"True\""));
    // A moment is picked as a day and a time, in a cell and in a form alike.
    has(window, &column("Field=\"Added\" Header=\"{x:Static local:Texts.TaskAddedLabel}\" Kind=\"Moment\" Edit=\"True\""));
    has(window, "<c:DateTimeInput x:Name=\"addedPick\" Grid.Column=\"0\" Grid.Row=\"6\" Grid.ColumnSpan=\"4\" Value=\"{Binding Task.Added}\"");
    has(window, &column("Field=\"Hours\" Header=\"{x:Static local:Texts.TaskHoursLabel}\" Kind=\"Number\" Edit=\"True\" Number=\"{x:Static local:Edit.TaskHours}\""));
    // A reference picks from the records the window reads, as a dropdown in a form does.
    has(window, &column("Field=\"Owner\" Header=\"{x:Static local:Texts.TaskOwnerLabel}\" Kind=\"Choice\" Edit=\"True\" ChoicesFrom=\"PersonChoices\""));
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
    let project = load_keyed([("shop.clarity", sized.as_str())], &[], "en");
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
            "    button \"New\" name button1 at 11,10 span 2 on click -> OrderForm\n",
            "    button \"New\" name button1 at 11,10 span 2 on click -> OrderForm\n    button \"7\" name d7 at 5,10 span 1 on click: digit(7)\n    button \"Half\" name half at 6,10 span 1 on click: scale(0.5, \"half\", Approved)\n    textbox name one at 7,10 span 1 on change: digit(1)\n",
        )
        .replace(
            "    handler gridSelected\n",
            "    handler digit(d: integer)\n        numberInput.error = \"\"\n\n    handler scale(by: decimal, word: text, status: OrderStatus)\n        digit(3)\n\n    handler gridSelected\n",
        );
    assert_ne!(source, SHOP);
    let project = load_keyed([("shop.clarity", source.as_str())], &[], "en");
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

/// A function that returns texts, plain and with a slot: both are translated.
const VERDICT: &str = "
function verdict(n: integer) -> text
    if n > 2
        return \"Draw\"
    return \"Turn {n}\"
";

/// A window holds plain values: properties of its ViewModel, at their
/// default or empty when it opens, read and changed by its handlers.
#[test]
fn a_window_holds_plain_values() {
    let source = format!("{SHOP}{CALC}{VERDICT}");
    let project = load_keyed([("shop.clarity", source.as_str())], &[], "en");
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    let files = generate(&project, &settings()).unwrap_or_else(|e| panic!("{e:#?}"));
    let vm = file(&files, "Windows/CalcViewModel.cs");
    has(vm, "    public decimal Shown { get => _shown; set => SetProperty(ref _shown, value); }\n    decimal _shown = default;\n");
    for start in ["    OrderStatus _pending = default;\n", "    bool _fresh = true;\n", "    string _word = Texts.CalcWord;\n", "    long _count = 2L;\n", "    decimal _rate = 0.5m;\n"] {
        has(vm, start);
    }
    has(vm, "        Fresh = false;\n");
    has(vm, "        Pending = OrderStatus.Approved;\n");
    // A text value shows its text in the person's language.
    has(vm, "        Word = Locale.Show(Text.Of(\"x\"));\n");
    has(file(&files, "Windows/CalcView.axaml"), "Text=\"{Binding Shown, Converter={x:Static local:Shown.Converter}}\"");
    // The texts code makes are translated.
    let code: String = files.iter().filter(|f| f.path.ends_with(".cs")).map(|f| f.text.as_str()).collect();
    has(&code, "Locale.Show(Text.Of(\"draw\"))");
    has(&code, "Locale.Show(Text.Of(\"turnN\").With(\"n\", ");
}

/// A character key (`7`, `+`) is what is typed, from the keyboard or the
/// number pad; a key with modifiers is a gesture, as Avalonia names keys.
#[test]
fn keys_are_written_as_avalonia_names_them() {
    let source = SHOP.replace(
        "    button \"New\" name button1 at 11,10 span 2 on click -> OrderForm\n",
        "    button \"New\" name button1 at 11,10 span 2 on click -> OrderForm\n    button \"7\" name d7 at 1,13 span 1 key \"7\" on click: approveClicked\n    button \"+\" name plus at 2,13 span 1 key \"+\" on click: approveClicked\n    button \"Back\" name back at 3,13 span 1 key \"Backspace\" on click: approveClicked\n    button \"Save\" name keep at 4,13 span 1 key \"Ctrl+S\" on click: approveClicked\n    button \"Zoom\" name zoom at 5,13 span 1 key \"Ctrl++\" on click: approveClicked\n",
    );
    assert_ne!(source, SHOP);
    let project = load_keyed([("shop.clarity", source.as_str())], &[], "en");
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
    let project = load_keyed([("m.clarity", source.as_str())], &[], "en");
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
    let project = load_keyed([("shop.clarity", source.as_str())], &[], "en");
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    let files = generate(&project, &settings()).unwrap_or_else(|e| panic!("{e:#?}"));
    has(file(&files, "Windows/CalcView.axaml"), "<c:SegmentDisplay x:Name=\"screen\" Grid.Column=\"0\" Grid.Row=\"0\" Grid.ColumnSpan=\"4\" Value=\"{Binding Shown}\" Digits=\"10\"");
}

/// A window with a canvas its draw handler draws, as each stack's tests use it.
const DIAL: &str = "
page Dial: window
    data angle: decimal, default 30
    data speed: integer, default 80
    canvas name dial at 1,1 span 6 rows 6 units 200 x 100 on draw: drawDial
    button \"Faster\" name faster at 7,1 span 2 on click: fasterClicked

code Dial
    handler drawDial
        dial.clear(\"#101810\")
        dial.line(50, 50, 50 + 35 * cos(angle), 50 - 35 * sin(angle), \"text\", 3)
        dial.text(50, 90, \"{speed} km/h\")

    handler fasterClicked
        speed = speed + 10
";

/// A draw handler adds shapes to its canvas's drawing, which the runtime's
/// CanvasView shows; the window draws it anew after every handler.
#[test]
fn a_canvas_is_drawn_by_its_draw_handler() {
    let source = format!("{SHOP}{DIAL}");
    let project = load_keyed([("shop.clarity", source.as_str())], &[], "en");
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

/// A canvas with `on click` runs its handler, which reads where it was clicked.
#[test]
fn a_click_on_a_canvas_runs_its_handler_with_where() {
    let source = format!("{SHOP}{DIAL}").replace("on draw: drawDial\n", "on draw: drawDial on click: dialClicked\n") + "\n    handler dialClicked\n        speed = dial.x + dial.y\n";
    let project = load_keyed([("shop.clarity", source.as_str())], &[], "en");
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    let files = generate(&project, &settings()).unwrap_or_else(|e| panic!("{e:#?}"));
    let vm = file(&files, "Windows/DialViewModel.cs");
    has(vm, "Dial.X + Dial.Y");
    // The canvas gives the window where it was clicked, then runs the handler.
    has(vm, "    [RelayCommand]\n    void ClickAt_Dial(Pointer at) => Handle(() =>\n    {\n        Dial.Pointer = at;\n        OnDialClicked();\n    }, [at]);");
    has(file(&files, "Windows/DialView.axaml"), "UnitsWidth=\"200\" UnitsHeight=\"100\" Command=\"{Binding ClickAt_DialCommand}\"");
}

/// A text over more rows than one is for reading: it wraps from the top,
/// keeps its paragraphs and scrolls when it is longer than its rows. A text
/// of one row stays one line.
#[test]
fn a_text_over_several_rows_wraps_and_scrolls() {
    let source = format!(
        "{SHOP}
page Rules: window
    title \"Rules\"
    data rules: text, default \"one\"
    text name rulesText at 1,1 span 4 rows 3 bind rules
    text name shortText at 1,4 span 4 bind rules
"
    );
    let project = load_keyed([("shop.clarity", source.as_str())], &[], "en");
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    let files = generate(&project, &settings()).unwrap_or_else(|e| panic!("{e:#?}"));
    let view = file(&files, "Windows/RulesView.axaml");
    has(
        view,
        "<ScrollViewer Grid.Column=\"0\" Grid.Row=\"0\" Grid.ColumnSpan=\"4\" Grid.RowSpan=\"3\" HorizontalScrollBarVisibility=\"Disabled\" VerticalScrollBarVisibility=\"Auto\" IsVisible=\"{Binding RulesText.IsVisible}\">",
    );
    has(view, "<TextBlock x:Name=\"rulesText\" Classes=\"value lines\" Text=\"{Binding Rules, Converter={x:Static local:Shown.Converter}}\" IsEnabled=\"{Binding RulesText.IsEnabled}\" />");
    has(view, "<TextBlock x:Name=\"shortText\" Grid.Column=\"0\" Grid.Row=\"3\" Grid.ColumnSpan=\"4\" Classes=\"value\" ");
    // The style of the lines: wrapped, from the top.
    let app = file(&files, "App.axaml");
    has(app, "<Style Selector=\"TextBlock.value.lines\">\n      <Setter Property=\"VerticalAlignment\" Value=\"Top\" />\n      <Setter Property=\"TextWrapping\" Value=\"Wrap\" />\n    </Style>");
}

#[test]
fn a_table_has_its_widths_options_and_first_order() {
    let text = TODO
        .replace("columns title, done,", "columns title 240, done,")
        .replace("urgent as switch on change:", "urgent as switch resizable sortable order by due desc on change:");
    let project = load_keyed([("todo.clarity", text.as_str())], &[], "en");
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    let mut settings = settings();
    settings.name = "Todo".into();
    let files = generate(&project, &settings).unwrap_or_else(|errors| panic!("{errors:#?}"));
    let window = file(&files, "Windows/MainView.axaml");
    // The person's widths are kept per window and table.
    has(window, "SelectedItem=\"{Binding Grid.SelectedItem}\" Resizable=\"True\" Sortable=\"True\" OrderBy=\"Due\" OrderDescending=\"True\" Remember=\"Main.grid\"");
    has(window, "<c:TableColumn Field=\"Title\" Header=\"{x:Static local:Texts.TaskTitleLabel}\" Kind=\"Text\" Width=\"240\" Edit=\"True\" />");
    has(window, "<c:TableColumn Field=\"Done\" Header=\"{x:Static local:Texts.TaskDoneLabel}\" Kind=\"Bool\" Edit=\"True\" />");
}

/// A window can be a mirror, in a browser, of the one its server keeps
/// (DESIGN 7.6): made without a database, it loads nothing itself.
#[test]
fn a_window_made_as_a_mirror_loads_nothing() {
    let files = files();
    let vm = file(&files, "Windows/OrderFormViewModel.cs");
    has(vm, "    public OrderFormViewModel(Db? db) : base(db)\n    {\n        if (IsMirror)\n            return;\n");
}

/// The web target (DESIGN 7.6): the windows live on a server, in sessions,
/// and the browser shows mirrors of them. The output is the application as a
/// library, the server that keeps the sessions, and the browser program.
#[test]
fn the_web_target_is_the_application_a_server_and_a_browser_program() {
    let project = load_keyed([("shop.clarity", SHOP)], &[], "en");
    let mut settings = settings();
    settings.stack.targets = vec![clarity_project::Target::Web];
    settings.port = Some(9090);
    let files = generate(&project, &settings).unwrap_or_else(|errors| panic!("{errors:#?}"));
    // The application, as a library: its windows, without a desktop program.
    let app_project = file(&files, "App/Shop.csproj");
    has(app_project, "    <OutputType>Library</OutputType>\n");
    // Nothing of the desktop's own: no native windows or sound to carry into the browser.
    has(app_project, "<PackageReference Include=\"Clarity.Avalonia\" Version=");
    has(app_project, "<ProjectReference Include=\"$(ClarityRuntime)/Clarity.Avalonia/Clarity.Avalonia.csproj\" />");
    assert!(!app_project.contains("Clarity.Avalonia.Desktop"), "{app_project}");
    assert!(!files.iter().any(|f| f.path == "Program.cs" || f.path == "App/Program.cs"), "no desktop program");
    has(file(&files, "App/Windows/OrderFormViewModel.cs"), "public OrderFormViewModel(Db? db) : base(db)");
    // The databases are the server's: the browser program carries no driver.
    assert!(!app_project.contains("Clarity.Runtime.Sqlite"), "{app_project}");
    assert!(!files.iter().any(|f| f.path == "App/Databases.cs"), "the databases open on the server");
    let databases = file(&files, "Server/Databases.cs");
    has(databases, "public static Db Open()\n");
    has(databases, "        Sqlite.Use();\n        var db = Db.Connect(Sources);");
    let server_project = file(&files, "Server/Shop.Server.csproj");
    has(server_project, "<PackageReference Include=\"Clarity.Runtime.Sqlite\" Version=");
    has(server_project, "<ProjectReference Include=\"$(ClarityRuntime)/Clarity.Runtime.Sqlite/Clarity.Runtime.Sqlite.csproj\" />");
    let app = file(&files, "App/App.axaml.cs");
    has(app, "if (ApplicationLifetime is ISingleViewApplicationLifetime single && Sessions is { } link)");
    has(app, "holder.Content = await MirrorShell.Start(link, Views.Create, TimeSpan.FromMilliseconds(");
    // The server keeps the sessions, at the project's port.
    has(file(&files, "Server/Shop.Server.csproj"), "<ProjectReference Include=\"../App/Shop.csproj\" />");
    // Its paths quoted: on Windows a person's folder often has spaces in it.
    has(file(&files, "Server/Shop.Server.csproj"), "<Exec Command=\"dotnet publish &quot;../Browser/Shop.Browser.csproj&quot; -c $(Configuration) -o &quot;$(MSBuildProjectDirectory)/browser/$(Configuration)&quot; &quot;-p:ClarityRuntime=$(ClarityRuntime)&quot; --nologo\" />");
    let server = file(&files, "Server/Program.cs");
    has(server, "using var db = Databases.Open();\n");
    has(server, "new WebServer(new Sessions(() => new OrderFormViewModel(db)), files).Serve(\"Shop\", WebServer.Address(args, \"127.0.0.1:9090\"));");
    // The browser program shows the mirrors, its sessions on the server the page came from.
    let browser = file(&files, "Browser/Shop.Browser.csproj");
    has(browser, "<Project Sdk=\"Microsoft.NET.Sdk.WebAssembly\">");
    has(browser, "<TargetFramework>net10.0-browser</TargetFramework>");
    has(browser, "<PackageReference Include=\"Avalonia.Browser\" Version=\"12.1.4\" />");
    // A browser has none of the system's fonts: the program carries one.
    has(browser, "<PackageReference Include=\"Avalonia.Fonts.Inter\" Version=\"12.1.4\" />");
    // Trimmed in Release, for the first load: what the windows reach by reflection stays whole.
    has(browser, "<PublishTrimmed Condition=\"'$(Configuration)' == 'Release'\">true</PublishTrimmed>");
    has(browser, "<JsonSerializerIsReflectionEnabledByDefault>true</JsonSerializerIsReflectionEnabledByDefault>");
    for kept in ["Shop", "Clarity.Avalonia", "Clarity.Runtime"] {
        has(browser, &format!("<TrimmerRootAssembly Include=\"{kept}\" />"));
    }
    assert!(!browser.contains("<PublishTrimmed>false</PublishTrimmed>"), "{browser}");
    // What the trimmer warns of, code reached by name at runtime, is kept
    // whole above (and DataGrid's reaches only the windows' records), so a
    // Release build has nothing to warn of.
    has(browser, "<SuppressTrimAnalysisWarnings>true</SuppressTrimAnalysisWarnings>");
    // On Windows its build files go to a short folder of their own: the
    // WebAssembly steps nest deep, past Windows' 260 characters when the
    // project's own folder is deep too. One folder per project's place.
    let props = file(&files, "Browser/Directory.Build.props");
    has(props, "<PropertyGroup Condition=\"'$(OS)' == 'Windows_NT' And '$(LOCALAPPDATA)' != ''\">");
    has(props, "<ClarityBuild>$(LOCALAPPDATA)\\Clarity\\build\\Shop-$([MSBuild]::StableStringHash($(MSBuildThisFileDirectory)))\\</ClarityBuild>");
    has(props, "<BaseIntermediateOutputPath>$(ClarityBuild)obj\\</BaseIntermediateOutputPath>");
    has(props, "<BaseOutputPath>$(ClarityBuild)bin\\</BaseOutputPath>");
    // A Directory.Build.props of the developer's further up still counts.
    has(props, "<Import Project=\"$([MSBuild]::GetPathOfFileAbove('Directory.Build.props', '$(MSBuildThisFileDirectory)../'))\" Condition=\"'$([MSBuild]::GetPathOfFileAbove(&quot;Directory.Build.props&quot;, &quot;$(MSBuildThisFileDirectory)../&quot;))' != ''\" />");
    has(file(&files, "README.md"), "On Windows the browser program's build files go to `%LOCALAPPDATA%\\Clarity\\build\\Shop-<number>`");
    has(file(&files, "Browser/Program.cs"), "AppBuilder.Configure<App>().WithInterFont().With(new Avalonia.Media.FontManagerOptions { DefaultFamilyName = \"fonts:Inter#Inter\" }).StartBrowserAppAsync(\"out\")");
    has(file(&files, "Browser/Program.cs"), "App.Sessions = new HttpSessionLink(new HttpClient { BaseAddress = new Uri(origin) });");
    has(file(&files, "Browser/wwwroot/index.html"), "<script type=\"module\" src=\"./main.js\"></script>");
    // What belongs to the browser: the tab's title, downloads, files in a tab, the person's storage.
    let program = file(&files, "Browser/Program.cs");
    has(program, "await JSHost.ImportAsync(\"clarity\", \"/clarity.js\");\n        var page = new Page();\n        BrowserPage.Current = page;");
    has(program, "Clarity.Runtime.KeptSettings.Store = (page.Read, page.Write);\n        Sound.Output = new PageSoundOutput(page);");
    has(file(&files, "Browser/wwwroot/clarity.js"), "export function play(data, volume, repeat)");
    has(file(&files, "Browser/Page.cs"), "internal sealed partial class Page : IBrowserPage");
    has(file(&files, "Browser/wwwroot/clarity.js"), "export function download(name, data)");
}

/// On the web each person's settings stay in their browser (DESIGN 7.6): the
/// browser starts them from its storage and sends them with each request, and
/// the server's handlers read them.
#[test]
fn on_the_web_each_persons_settings_travel_with_their_requests() {
    let source = format!("{SHOP}\nsettings\n    speed: integer, default 2\n");
    let project = load_keyed([("shop.clarity", source.as_str())], &[], "en");
    assert!(project.diagnostics.iter().all(|d| d.severity != clarity_spec::Severity::Error), "{:#?}", project.diagnostics);
    let mut settings = settings();
    settings.stack.targets = vec![clarity_project::Target::Web];
    let files = generate(&project, &settings).unwrap_or_else(|errors| panic!("{errors:#?}"));
    has(file(&files, "App/Modules/Shop.cs"), "    public void Apply(JsonObject sent)\n    {\n        var settings = this;\n");
    has(file(&files, "Server/Program.cs"), "new Sessions(() => new OrderFormViewModel(db)) { Settings = (() => Settings.Current.ToJson(), sent => Settings.Current.Apply(sent)) }");
    let app = file(&files, "App/App.axaml.cs");
    has(app, "            Settings.Start();\n");
    has(app, ", settings: (() => Settings.Current.ToJson(), sent => Settings.Current.Apply(sent)));");
}
