//! The C# backend (G2a): the data dictionary, functions, actions and data
//! events as C#, over the C# runtime copied into the project.

use clarity_gen::{OutFile, generate};
use clarity_project::{Backend, Settings, Ui, starter};
use clarity_spec::load_sources;

const SALES: &str = "module sales

enum OrderStatus
    Pending
    Approved label \"Approved by sales\"

entity Customer
    name: text(80), required
    credit_limit: money, default 100
    active: bool, default true
    since: date, format long

entity Order
    number: text(12), unique
    status: OrderStatus
    customer: ref Customer
    total: money
    event: text(20)

function within_credit(order: Order) -> bool
    return order.total <= order.customer.credit_limit

function doubled(n: integer) -> money
    ensure result >= 0
    let x: money = n * 2
    if x < 0
        return 0
    return x + 0.5

function big_orders(limit: money) -> list<Order>
    return find o in Order where o.total >= limit and o.status in [Pending, Approved] order by o.total desc, o.number

function described(order: Order) -> text
    let all = find o in Order where o.customer == order.customer
    let shown = \"Order \" + order.number + \" is \" + order.status
    for other in all
        if other.number contains \"X\" or count(all) > 3
            shown = shown + \"!\"
    return shown

action approve(order: Order)
    require order.status == Pending
    require within_credit(order)
    effects db.write
    order.status = Approved
    save order

code Order
    before save
        if order.status == Approved and not within_credit(order)
            refuse \"Order {order.number} is over its customer's credit limit\"

    after delete
        return

page OrderForm: window
    title \"Order\"
    data order: Order
";

fn settings() -> Settings {
    let mut settings = starter("Shop", "shop");
    settings.stack.backend = Backend::Csharp;
    settings.stack.ui = Ui::Avalonia;
    settings
}

fn files() -> Vec<OutFile> {
    let project = load_sources([("sales.clarity", SALES)]);
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    generate(&project, &settings()).unwrap()
}

fn file<'a>(files: &'a [OutFile], path: &str) -> &'a str {
    &files.iter().find(|f| f.path == path).unwrap_or_else(|| panic!("no {path}")).text
}

/// Asserts `text` holds every line of `expected`, in order and together.
fn has(text: &str, expected: &str) {
    assert!(text.contains(expected), "expected\n{expected}\nin\n{text}");
}

#[test]
fn writes_a_complete_dotnet_project_with_the_runtime_copied_in() {
    let files = files();
    let paths: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();
    assert_eq!(
        paths,
        [
            "App.axaml",
            "App.axaml.cs",
            "Clarity.Avalonia/Bindings.cs",
            "Clarity.Avalonia/Messages.cs",
            "Clarity.Avalonia/PartState.cs",
            "Clarity.Avalonia/Shell.cs",
            "Clarity.Avalonia/WindowViewModel.cs",
            "Clarity.Runtime/Db.cs",
            "Clarity.Runtime/Dialect.cs",
            "Clarity.Runtime/Errors.cs",
            "Clarity.Runtime/Query.cs",
            "Clarity.Runtime/Record.cs",
            "Clarity.Runtime/Sql.cs",
            "Clarity.Runtime/SqliteDriver.cs",
            "Clarity.Runtime/Store.cs",
            "Clarity.Runtime/Text.cs",
            "Program.cs",
            "Sales.cs",
            "Schema.cs",
            "Shop.csproj",
            "Shown.cs",
            "Views.cs",
            "Windows/OrderFormView.axaml",
            "Windows/OrderFormView.axaml.cs",
            "Windows/OrderFormViewModel.cs",
        ]
    );
    let project = file(&files, "Shop.csproj");
    has(project, "<TargetFramework>net10.0</TargetFramework>");
    has(project, "<RootNamespace>Shop</RootNamespace>");
    // The runtime's packages, at the versions the runtime is tested with.
    let runtime = include_str!("../../../runtimes/csharp/Clarity.Runtime/Clarity.Runtime.csproj");
    let sqlite = runtime.lines().find(|l| l.contains("Microsoft.Data.Sqlite")).unwrap().trim();
    has(project, sqlite);
    assert_eq!(file(&files, "Clarity.Runtime/Db.cs"), include_str!("../../../runtimes/csharp/Clarity.Runtime/Db.cs"));
}

#[test]
fn is_deterministic() {
    assert_eq!(files(), files());
}

#[test]
fn enums_and_entities_become_csharp_types() {
    let files = files();
    let sales = file(&files, "Sales.cs");
    has(sales, "namespace Shop;");
    has(sales, "public enum OrderStatus\n{\n    Pending,\n    Approved,\n}");
    has(sales, "        OrderStatus.Approved => \"Approved by sales\",");
    has(sales, "public sealed partial class Customer : ObservableObject, IRecord<Customer>\n{");
    has(sales, "    public static string TableName => \"customer\";");
    has(sales, "    public static IReadOnlyList<string> ColumnNames { get; } = [\"name\", \"credit_limit\", \"active\", \"since\"];");
    has(sales, "    public decimal CreditLimit { get => _creditLimit; set => SetProperty(ref _creditLimit, value); }\n    decimal _creditLimit = 100m;");
    has(sales, "    bool _active = true;");
    has(sales, "    public DateOnly? Since { get => _since; set => SetProperty(ref _since, value); }\n    DateOnly? _since;");
    has(sales, "    public long? Customer { get => _customer; set => SetProperty(ref _customer, value); }");
    has(sales, "    public OrderStatus Status { get => _status; set => SetProperty(ref _status, value); }");
    has(sales, "    public object?[] ToValues() => [Number, Sql.Name(Status), Customer, Total, Event];");
    has(sales, "        CreditLimit = Sql.GetDecimal(\"credit_limit\", values[1]),");
    has(sales, "        Status = Sql.GetName<OrderStatus>(\"status\", values[1]),");
    has(sales, "    public static string TableName => \"order\";");
}

#[test]
fn the_schema_comes_from_the_shared_migration_steps() {
    let files = files();
    let schema = file(&files, "Schema.cs");
    has(schema, "public static readonly (string Source, string[][] Steps)[] Sources =");
    has(schema, "        (\"main\",\n        [\n            // 1: the schema from the data dictionary\n            [\n");
    // Verbatim strings: a quote is written twice.
    has(schema, "@\"CREATE TABLE \"\"order\"\" (");
    has(schema, "\"\"customer\"\" INTEGER REFERENCES \"\"customer\"\" (\"\"id\"\")");
}

#[test]
fn functions_and_actions_become_methods_of_logic() {
    let files = files();
    let sales = file(&files, "Sales.cs");
    has(sales, "public static partial class Logic\n{");
    has(sales, "    public static bool WithinCredit(Db db, Order order)\n    {\n        return (order.Total <= db.Load<Customer>(order.Customer).CreditLimit);\n    }");
    has(
        sales,
        "    public static void Approve(Db db, Order order)
    {
        Contracts.Require((order.Status == OrderStatus.Pending), \"order.status == Pending\");
        Contracts.Require(WithinCredit(db, order), \"within_credit(order)\");
        order.Status = OrderStatus.Approved;
        db.Save(order);
    }",
    );
    // Integers widen to decimals by themselves; ensure wraps the body.
    has(
        sales,
        "    public static decimal Doubled(Db db, long n)
    {
        decimal Body()
        {
            decimal x = (n * 2L);
            if ((x < 0L))
            {
                return 0L;
            }
            return (x + 0.5m);
        }
        var result = Body();
        Contracts.Ensure((result >= 0L), \"result >= 0\");
        return result;
    }",
    );
}

#[test]
fn queries_text_and_loops() {
    let files = files();
    let sales = file(&files, "Sales.cs");
    has(
        sales,
        "return db.Find<Order>(new Query().Filter(Cond.Number(\"total\", Op.GtEq, limit).And(Cond.OneOf(\"status\", [Sql.Name(OrderStatus.Pending), Sql.Name(OrderStatus.Approved)]))).OrderNumber(\"total\", true).Order(\"number\", false));",
    );
    has(sales, "var all = db.Find<Order>(new Query().Filter(Cond.Compare(\"customer\", Op.Eq, order.Customer)));");
    // Enums show their label in text.
    has(sales, "var shown = (((\"Order \" + order.Number) + \" is \") + Labels.Of(order.Status));");
    has(sales, "foreach (var other in all)");
    has(sales, "if ((Texts.Contains(other.Number, \"X\") || ((long)all.Count > 3L)))");
    has(sales, "shown = (shown + \"!\");");
    // A field named like a C# keyword is still a property.
    has(sales, "    public string Event { get => _event; set => SetProperty(ref _event, value); }\n    string _event = \"\";");
}

#[test]
fn data_events_run_from_the_record() {
    let files = files();
    let sales = file(&files, "Sales.cs");
    has(sales, "    public static bool HasEvents => true;");
    has(sales, "    public void BeforeSave(Db db) => Logic.BeforeSaveOrder(db, this);");
    has(sales, "    public void AfterDelete(Db db) => Logic.AfterDeleteOrder(db, this);");
    has(
        sales,
        "            throw new RefusedException(new Text(\"Order {order.number} is over its customer's credit limit\").With(\"order.number\", Slot.OfText(order.Number)));",
    );
}

#[test]
fn the_program_opens_and_migrates_every_database() {
    let files = files();
    let program = file(&files, "Program.cs");
    has(program, "[(\"main\", Dialect.Sqlite, \"data/shop.db\")]");
    has(program, "foreach (var (source, steps) in Schema.Sources)\n            db.MigrateSource(source, steps);");
}

#[test]
fn csharp_needs_avalonia_or_the_web_renderer() {
    let project = load_sources([("sales.clarity", SALES)]);
    let mut settings = settings();
    settings.stack.backend = Backend::Go;
    assert_eq!(generate(&project, &settings).unwrap_err(), ["the Go backend is not generated yet; Rust and C# are"]);
}
