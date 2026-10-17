//! The Avalonia windows of the C# backend (G2b): each window as AXAML with
//! a ViewModel, its parts bound to the data and to their state, its handlers
//! as ViewModel methods.

use clarity_gen::{OutFile, generate};
use clarity_project::{Backend, Settings, Ui, starter};
use clarity_spec::load_sources;

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
    has(project, "Include=\"Avalonia.Desktop\"");
    has(project, "Include=\"CommunityToolkit.Mvvm\"");
    assert!(files.iter().any(|f| f.path == "Clarity.Avalonia/Shell.cs"));
    has(file(&files, "App.axaml"), "<FluentTheme />");
    has(file(&files, "App.axaml.cs"), "desktop.MainWindow = Shell.Start(new OrderFormViewModel(db), Views.Create, TimeSpan.FromMilliseconds(5000));");
    has(file(&files, "Program.cs"), ".StartWithClassicDesktopLifetime(args)");
    has(file(&files, "Views.cs"), "OrderFormViewModel vm => new OrderFormView { DataContext = vm },");
}

#[test]
fn records_tell_the_window_when_they_change() {
    let files = files();
    let shop = file(&files, "Shop.cs");
    has(shop, "public sealed partial class Order : ObservableObject, IRecord<Order>");
    has(shop, "    public decimal Total { get => _total; set => SetProperty(ref _total, value); }\n    decimal _total;");
    has(shop, "    public string Number { get => _number; set => SetProperty(ref _number, value); }\n    string _number = \"\";");
    has(shop, "public static partial class Choices\n{\n    public static readonly Choice[] OrderStatus =\n    [\n        new(global::Shop.OrderStatus.Pending, \"Pending\"),\n        new(global::Shop.OrderStatus.Approved, \"Approved by sales\"),\n    ];");
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
    has(window, "<TextBlock Grid.Column=\"0\" Grid.Row=\"0\" Grid.ColumnSpan=\"2\" Classes=\"label\" Text=\"Number *\" />");
    has(
        window,
        "<TextBox x:Name=\"numberInput\" Grid.Column=\"2\" Grid.Row=\"0\" Grid.ColumnSpan=\"4\" Text=\"{Binding Order.Number}\" PlaceholderText=\"ORD-1\" IsEnabled=\"{Binding NumberInput.IsEnabled}\" IsVisible=\"{Binding NumberInput.IsVisible}\" IsReadOnly=\"{Binding NumberInput.IsReadOnly}\" />",
    );
    has(window, "<TextBlock Grid.Column=\"2\" Grid.Row=\"1\" Grid.ColumnSpan=\"4\" Classes=\"help\" Classes.error=\"{Binding NumberInput.HasError}\" Text=\"{Binding NumberInput.Shown}\" />");
    has(window, "<TextBox Grid.Column=\"0\" Grid.Row=\"2\" Grid.ColumnSpan=\"3\" Classes=\"number\" Text=\"{Binding Order.Total}\" />");
    has(window, "<CheckBox Grid.Column=\"3\" Grid.Row=\"2\" Grid.ColumnSpan=\"3\" Content=\"Paid\" IsChecked=\"{Binding Order.Paid}\" />");
    // The caption beside the switch, as on the web, so it fits its row.
    has(window, "<ToggleSwitch Grid.Column=\"6\" Grid.Row=\"2\" Grid.ColumnSpan=\"3\" OnContent=\"Rush\" OffContent=\"Rush\" IsChecked=\"{Binding Order.Paid}\" />");
    has(window, "ItemsSource=\"{x:Static local:Choices.OrderStatus}\" DisplayMemberBinding=\"{Binding Label, x:DataType=c:Choice}\" SelectedValueBinding=\"{Binding Value, x:DataType=c:Choice}\" SelectedValue=\"{Binding Order.Status}\"");
    has(window, "ItemsSource=\"{Binding CustomerChoices}\"");
    has(window, "SelectedDate=\"{Binding Order.OrderedOn, Converter={x:Static c:Converters.Date}}\"");
    has(window, "Text=\"{Binding Order.Status, Converter={x:Static local:Shown.Converter}}\"");
    has(window, "<DataGrid x:Name=\"grid\" Grid.Column=\"0\" Grid.Row=\"5\" Grid.ColumnSpan=\"12\" Grid.RowSpan=\"4\" ItemsSource=\"{Binding Orders}\" SelectedItem=\"{Binding Grid.SelectedItem}\"");
    has(window, "<DataGridTextColumn Header=\"Status\" Binding=\"{Binding Status, Converter={x:Static local:Shown.Converter}}\" x:DataType=\"local:Order\" />");
    has(window, "<TextBox x:Name=\"numberFilter\" Grid.Column=\"0\" Grid.Row=\"9\" Grid.ColumnSpan=\"4\" Text=\"{Binding NumberFilter.Text}\"");
    has(window, "Content=\"{Binding ApproveButton.Caption}\" Command=\"{Binding ApproveClickedCommand}\" IsDefault=\"True\"");
    has(window, "<Button Grid.Column=\"10\" Grid.Row=\"9\" Grid.ColumnSpan=\"2\" Content=\"New\" Command=\"{Binding OpenOrderFormCommand}\" />");
    has(window, "<TabItem Header=\"Notes\">");
    has(window, "<TextBox Grid.Column=\"0\" Grid.Row=\"0\" Grid.ColumnSpan=\"12\" Grid.RowSpan=\"3\" Classes=\"memo\" AcceptsReturn=\"True\" TextWrapping=\"Wrap\" Text=\"{Binding Order.Number}\" />");
}

#[test]
fn the_frame_comes_from_the_app() {
    let files = files();
    let window = file(&files, "Windows/OrderFormView.axaml");
    has(window, "<Menu DockPanel.Dock=\"Top\">\n");
    has(window, "<MenuItem Header=\"Orders\" InputGesture=\"Ctrl+O\" HotKey=\"Ctrl+O\" Command=\"{Binding OpenOrderFormCommand}\" />");
    has(window, "<Separator />");
    has(window, "<MenuItem Header=\"Quit\" Command=\"{Binding QuitCommand}\" />");
    // A window opens in a desktop window of its own only where the design says so.
    has(window, "<MenuItem Header=\"Another order\" Command=\"{Binding OpenOrderFormInNewWindowCommand}\" />");
    has(window, "<TextBlock Classes=\"status\" Text=\"Ready\" />");
}

#[test]
fn handlers_are_viewmodel_methods() {
    let files = files();
    let vm = file(&files, "Windows/OrderFormViewModel.cs");
    has(vm, "public sealed partial class OrderFormViewModel : WindowViewModel");
    has(vm, "    public override string Title => \"Order\";");
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
        NumberInput.Error = \"\";
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
    has(code, "        Events.OnCommit(numberFilter, () => Vm.FilterChangedCommand.Execute(null));");
    has(code, "        grid.SelectionChanged += (_, _) => Vm.GridSelectedCommand.Execute(null);");
    has(code, "        grid.DoubleTapped += (_, _) => Vm.GridOpenedCommand.Execute(null);");
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
