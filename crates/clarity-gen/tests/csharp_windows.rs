//! The windows of the C# backend on Clarity.UI (DESIGN 7.9): each window's
//! view model, its parts bound to the data and to their state, its handlers
//! as view model methods; the window itself as markup Clarity.UI draws.

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
    orderedOn: date
    paid: bool
    discount: decimal(6, 4), format percent

function withinCredit(order: Order) -> bool
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
    date name date1 at 9,4 span 3 bind order.orderedOn
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
        if not withinCredit(order)
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
    settings.stack.ui = Ui::Clarity;
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

fn lacks(text: &str, unexpected: &str) {
    assert!(!text.contains(unexpected), "did not expect {unexpected:?} in:\n{text}");
}

fn has(text: &str, expected: &str) {
    assert!(text.contains(expected), "expected\n{expected}\nin\n{text}");
}

#[test]
fn a_clarity_ui_desktop_application() {
    let files = files();
    let project = file(&files, "Shop.csproj");
    has(project, "<OutputType>WinExe</OutputType>");
    // Clarity.Ui brings the native Clarity.UI library, whose desktop host shows the windows, and the runtime.
    has(project, "<PackageReference Include=\"Clarity.Ui\"");
    lacks(project, "Clarity.Ui.Desktop");
    // The model, which the library reads its facts, translations and keys from.
    has(project, "<EmbeddedResource Include=\"model.json\" LogicalName=\"model.json\"");
    has(file(&files, "model.json"), "\"project\"");
    // The windows as markup, the app frames and the themes travel in the program.
    has(project, "<EmbeddedResource Include=\"Windows/*.cxaml\" LogicalName=\"Windows/%(Filename)%(Extension)\" />");
    has(file(&files, "Windows/OrderForm.cxaml"), "<Window ");
    has(file(&files, "Windows/OrderFormViewModel.cs"), "public sealed partial class OrderFormViewModel : WindowViewModel");
    let program = file(&files, "Program.cs");
    has(program, "using Clarity.Ui;");
    lacks(program, "Clarity.Ui.Desktop");
    has(program, "Locale.Start(Texts.BaseLanguage, Texts.Languages, ");
    has(program, "AppThemes.Start();");
    has(program, "DesktopWindow.Run(new OrderFormViewModel(databases));");
    has(file(&files, "AppThemes.cs"), "Themes.Start([new(\"Clarity\", \"Clarity\")]);");
}

/// Nothing of another UI framework is in a C# application.
#[test]
fn a_csharp_application_names_no_other_ui_framework() {
    for f in files() {
        assert!(!f.text.to_lowercase().contains("avalonia") && !f.path.ends_with(".axaml"), "{}:\n{}", f.path, f.text);
    }
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

/// The menu and the toolbar are parts: code hides and disables them by name.
#[test]
fn the_menu_and_the_toolbar_follow_their_part_state() {
    let files = files();
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
    has(vm, "    public PartState NumberInput { get; } = new(\"numberInput\") { Field = (\"order\", \"number\") };");
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
    let program = file(&files, "Program.cs");
    has(
        program,
        "        Remembered.Use(\"Shop\");
        Locale.Start(Texts.BaseLanguage, Texts.Languages, \"EUR\", Texts.In, Labels.Named);
        AppThemes.Start();",
    );
    // Without a currency, money has no sign.
    let plain = self::files();
    has(file(&plain, "Program.cs"), "Locale.Start(Texts.BaseLanguage, Texts.Languages, null, Texts.In, Labels.Named);");
    has(file(&plain, "Texts.cs"), "    public static readonly string[] Languages = [];");
}

#[test]
fn an_enum_value_in_text_shows_its_label() {
    let files = files();
    // An enum value in text with slots shows its label.
    has(file(&files, "Labels.cs"), "        \"OrderStatus\" => Enum.TryParse<OrderStatus>(value, out var v) ? Of(v) : value,");
}

/// The selected row goes on into paths and calls, as any record does.
#[test]
fn the_selected_row_is_a_record_in_any_expression() {
    let source = SHOP.replace(
        "    handler gridOpened\n",
        "    handler gridOpened\n        if withinCredit(grid.selected)\n            message \"{grid.selected.number} of {grid.selected.customer.name} fits\"\n",
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

/// A reference column picks from the records the window reads, as a dropdown in a form does.
#[test]
fn a_reference_column_offers_the_records() {
    let project = load_keyed([("todo.clarity", TODO)], &[], "en");
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    let mut settings = settings();
    settings.name = "Todo".into();
    let files = generate(&project, &settings).unwrap_or_else(|errors| panic!("{errors:#?}"));
    has(file(&files, "Windows/MainViewModel.cs"), "PersonChoices = Choice.Of(Db.All<Person>(), r => ");
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
    has(file(&files, "Windows/OrderFormViewModel.cs"), "    public override Extent? WindowSize => new(640, 480);\n");
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
    // The command runs the call; Run_ runs it for the screen and says whether the handler refused.
    has(vm, "    [RelayCommand]\n    void Digit7() => Run_Digit7();\n\n    public bool Run_Digit7() => Handle(() => OnDigit(7L), command: nameof(Digit7));");
    has(vm, "    [RelayCommand]\n    void Digit1() => Run_Digit1();\n\n    public bool Run_Digit1() => Handle(() => OnDigit(1L), command: nameof(Digit1));");
    has(
        vm,
        "    [RelayCommand]\n    void Scale0_5HalfApproved() => Run_Scale0_5HalfApproved();\n\n    public bool Run_Scale0_5HalfApproved() => Handle(() => OnScale(0.5m, \"half\", OrderStatus.Approved), command: nameof(Scale0_5HalfApproved));",
    );
    // A handler that takes values is only run with them.
    assert!(!vm.contains("void Digit() =>"), "{vm}");
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

/// A function that returns texts, plain and with a slot.
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
    // A text value shows its text in the person's language, and shows it
    // again when the language changes (its default too, a key).
    // A text that code only keeps or returns is no key: the checker asks one
    // only for a text shown as written (a message, a caption).
    has(vm, "        ShowText(nameof(Word), Text.Plain(\"x\"), shown => Word = shown);\n");
    has(vm, "        ShowText(nameof(Word), Text.Of(\"calcWord\"), shown => Word = shown);\n");
    // A slot is still filled in.
    let code: String = files.iter().filter(|f| f.path.ends_with(".cs")).map(|f| f.text.as_str()).collect();
    has(&code, "return \"Draw\";");
    has(&code, "new Text(\"Turn {n}\").With(\"n\", ");
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

/// A draw handler adds shapes to its canvas's drawing; the window draws it
/// anew after every handler.
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
    // Left out, the text's color, size, alignment and style take the record's defaults, so code
    // that uses no newer value builds on an older runtime.
    has(vm, "new Text(\"{speed} km/h\").With(\"speed\", Slot.OfInteger(Speed, null))));");
    lacks(vm, "\"center\"");
    has(vm, "    protected override void Redraw()\n    {\n        Dial.BeginDrawing();\n        OnDrawDial();\n        Dial.EndDrawing();\n    }");
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
    // A click on a canvas is an edit Ctrl+Z takes back (DESIGN 5.29).
    has(vm, "    [RelayCommand]\n    void ClickAt_Dial(Pointer at) => EditOf(\"dial\", \"dial\", ends: true, () => Handle(() =>\n    {\n        Dial.Pointer = at;\n        OnDialClicked();\n    }, [at]));");
}

/// A window loads its data as it is made: there is no mirror of it any
/// more, the browser asks the server's window itself (DESIGN 7.9, B2).
#[test]
fn a_window_loads_its_data_as_it_is_made() {
    let files = files();
    let vm = file(&files, "Windows/OrderFormViewModel.cs");
    has(vm, "    public OrderFormViewModel(Db? db) : base(db)\n    {\n");
    assert!(!vm.contains("IsMirror"), "{vm}");
}


const GENRES: &str = "module music

entity Genre
    name: text(40), required
    parent: ref Genre
    children: list<Genre>
    rank: integer, min 1, max 10
    share: decimal(5, 2), format percent
    shade: color

page Genres: window
    data genres: list<Genre>
    treetable name genreTree at 1,1 span 12 rows 8 bind genres children children columns name
";

/// The facts of every field of every entity (DESIGN 7.9), which Clarity.UI's
/// parts read beside the values: label, format, required, range, what a
/// reference refers to, and for records that make a tree their parent field.
#[test]
fn every_field_s_facts_are_in_fields() {
    let fields = file(&files(), "Fields.cs").to_string();
    has(&fields, "using Clarity.Runtime;");
    has(&fields, "public static class Fields\n{");
    has(&fields, "public static readonly IReadOnlyDictionary<string, FieldFacts> All = new Dictionary<string, FieldFacts>\n    {");
    has(&fields, "[\"Order.total\"] = new(\"@orderTotalLabel\", new FieldFormat(FormatKind.Money), Required: false, Min: null, Max: null, Refers: null, Parent: null),");
    has(&fields, "[\"Order.customer\"] = new(\"@orderCustomerLabel\", null, Required: false, Min: null, Max: null, Refers: \"Customer\", Parent: null),");
    has(&fields, "[\"Order.orderedOn\"] = new(\"@orderOrderedOnLabel\", new FieldFormat(FormatKind.Date), Required: false,");
    has(&fields, "[\"Customer.name\"] = new(\"@customerNameLabel\", null, Required: true,");

    let project = load_keyed([("music.clarity", GENRES)], &[], "en");
    assert!(project.diagnostics.iter().all(|d| d.severity != clarity_spec::diagnostic::Severity::Error), "{:#?}", project.diagnostics);
    let files = generate(&project, &settings()).unwrap_or_else(|errors| panic!("{errors:#?}"));
    let fields = file(&files, "Fields.cs");
    has(fields, "[\"Genre.name\"] = new(\"@genreNameLabel\", null, Required: true, Min: null, Max: null, Refers: null, Parent: \"parent\"),");
    has(fields, "[\"Genre.parent\"] = new(\"@genreParentLabel\", null, Required: false, Min: null, Max: null, Refers: \"Genre\", Parent: \"parent\"),");
    has(fields, "[\"Genre.rank\"] = new(\"@genreRankLabel\", new FieldFormat(FormatKind.Integer), Required: false, Min: \"1\", Max: \"10\", Refers: null, Parent: \"parent\"),");
    has(fields, "[\"Genre.share\"] = new(\"@genreShareLabel\", new FieldFormat(FormatKind.Decimal, 2, \"percent\"), Required: false,");
    // A color field says so: a table or list shows a swatch beside its value.
    has(fields, "[\"Genre.shade\"] = new(\"@genreShadeLabel\", null, Required: false, Min: null, Max: null, Refers: null, Parent: \"parent\", Color: true),");
}

/// What an engine says into a text value (noughts' status line) is kept as
/// said, a key of the project's texts, so it shows again when the person
/// picks another language; its default, a key, too.
#[test]
fn a_said_text_follows_the_language() {
    let file = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/noughts.clarity");
    let project = clarity_project::load_modules(&clarity_project::module_dir(&file)).unwrap();
    let settings = clarity_project::load(&file).settings.expect("the example has a project file");
    let files = generate(&project, &settings).unwrap_or_else(|e| panic!("{e:#?}"));
    let vm = file_of(&files, "GameViewModel.cs");
    has(vm, "        ShowText(nameof(Status), Text.Of(Engines.Board.CallIn<string>(Instance, \"play\", square)), shown => Status = shown);\n");
    has(vm, "        ShowText(nameof(Status), Text.Of(\"gameStatus\"), shown => Status = shown);\n");
}

fn file_of<'a>(files: &'a [OutFile], name: &str) -> &'a str {
    &files.iter().find(|f| f.path.ends_with(name)).unwrap_or_else(|| panic!("no {name}")).text
}

/// Names never clash in the view model (data-4, S8): a handler named as a
/// part, or as a member every view model has, gets a command of its own name
/// with `_`, and a button can open the keys window.
#[test]
fn a_handler_named_as_a_part_or_a_view_model_member_still_builds() {
    let source = SHOP
        .replace(
            "    button \"New\" name button1 at 11,10 span 2 on click -> OrderForm\n",
            "    button \"New\" name button1 at 11,10 span 2 on click -> OrderForm\n    button \"Red\" name redInk at 1,13 span 2 on click: redInk\n    button \"Opened\" name button2 at 3,13 span 2 on click: opened\n    button \"Keys\" name keysButton at 5,13 span 2 on click -> keys\n",
        )
        .replace("    handler gridSelected\n", "    handler redInk\n        redInk.enabled = false\n\n    handler opened\n        redInk()\n\n    handler gridSelected\n");
    let mut project = load_keyed([("shop.clarity", source.as_str())], &[], "en");
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    project.keys_editable = true;
    let mut settings = settings();
    settings.keys_editable = true;
    let files = generate(&project, &settings).unwrap_or_else(|e| panic!("{e:#?}"));
    let vm = file(&files, "Windows/OrderFormViewModel.cs");
    has(vm, "public PartState RedInk { get; }");
    has(vm, "    [RelayCommand]\n    void RedInk_() => Handle(OnRedInk);\n");
    has(vm, "RedInk.IsEnabled = false;");
    has(vm, "    [RelayCommand]\n    void Opened_() => Handle(OnOpened);\n");
    has(vm, "OnRedInk();");
    has(vm, "    void Keys_() => ShowKeys();\n");
}
