//! The C# backend (G2a): the data dictionary, functions, actions and data
//! events as C#, over the C# runtime copied into the project.

use clarity_gen::{OutFile, generate};
use clarity_project::{Backend, Settings, SourceKind, Ui, starter};
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
            "AppThemes.cs",
            "Clarity.Avalonia/Bindings.cs",
            "Clarity.Avalonia/Locale.cs",
            "Clarity.Avalonia/Messages.cs",
            "Clarity.Avalonia/PartState.cs",
            "Clarity.Avalonia/Pickers.cs",
            "Clarity.Avalonia/Remembered.cs",
            "Clarity.Avalonia/Shell.cs",
            "Clarity.Avalonia/Themes.cs",
            "Clarity.Avalonia/WindowViewModel.cs",
            "Clarity.Runtime/Db.cs",
            "Clarity.Runtime/Dialect.cs",
            "Clarity.Runtime/Errors.cs",
            "Clarity.Runtime/Formats.cs",
            "Clarity.Runtime/Languages.cs",
            "Clarity.Runtime/Locations.cs",
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
            "Texts.cs",
            "Texts.resx",
            "Themes/Clarity.axaml",
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
    has(sales, "        OrderStatus.Approved => Texts.ApprovedBySales,");
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

/// A runtime file added later must reach generated projects too.
#[test]
fn every_csharp_runtime_source_file_is_copied() {
    let files = files();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../runtimes/csharp");
    for library in ["Clarity.Runtime", "Clarity.Avalonia"] {
        for entry in std::fs::read_dir(root.join(library)).unwrap() {
            let path = entry.unwrap().path();
            let file = path.file_name().unwrap().to_string_lossy().to_string();
            // A server driver goes only where a data source uses it (tested below).
            let driver = ["ServerDriver.cs", "PostgresDriver.cs", "MysqlDriver.cs", "MssqlDriver.cs"].contains(&file.as_str());
            if path.extension().is_some_and(|e| e == "cs") && !driver {
                let name = format!("{library}/{file}");
                assert!(files.iter().any(|f| f.path == name), "{name} is not copied");
            }
        }
    }
}

/// A project holds the drivers of the databases its data sources are, with
/// their packages, and no others (as the Rust output enables cargo features).
#[test]
fn a_project_gets_the_drivers_of_its_databases_only() {
    let project = load_sources([("sales.clarity", SALES)]);
    let plain = generate(&project, &settings()).unwrap();
    let csproj = file(&plain, "Shop.csproj");
    assert!(!csproj.contains("Npgsql") && !csproj.contains("DefineConstants"), "{csproj}");
    assert!(!plain.iter().any(|f| f.path.ends_with("Driver.cs") && !f.path.ends_with("SqliteDriver.cs")));

    for (kind, constant, package, driver) in [
        (SourceKind::Postgres, "CLARITY_POSTGRES", "Include=\"Npgsql\"", "PostgresDriver.cs"),
        (SourceKind::Mysql, "CLARITY_MYSQL", "Include=\"MySqlConnector\"", "MysqlDriver.cs"),
        (SourceKind::Mssql, "CLARITY_MSSQL", "Include=\"Microsoft.Data.SqlClient\"", "MssqlDriver.cs"),
    ] {
        let mut settings = settings();
        settings.datasources[0].kind = kind;
        settings.datasources[0].url = "env:SHOP_DATABASE".into();
        let files = generate(&project, &settings).unwrap();
        let csproj = file(&files, "Shop.csproj");
        has(csproj, &format!("<DefineConstants>$(DefineConstants);{constant}</DefineConstants>"));
        has(csproj, package);
        let others = ["Include=\"Npgsql\"", "Include=\"MySqlConnector\"", "Include=\"Microsoft.Data.SqlClient\""];
        assert_eq!(others.iter().filter(|p| csproj.contains(*p)).count(), 1, "{csproj}");
        assert!(files.iter().any(|f| f.path == format!("Clarity.Runtime/{driver}")));
        assert_eq!(files.iter().filter(|f| f.path.ends_with("Driver.cs")).count(), 3, "SQLite, the server's own and their base");
        has(file(&files, "Program.cs"), "\"env:SHOP_DATABASE\")];");
    }
    // SQL Server's package comes with the fix of its vulnerable dependency.
    let mut settings = settings();
    settings.datasources[0].kind = SourceKind::Mssql;
    has(file(&generate(&project, &settings).unwrap(), "Shop.csproj"), "Include=\"System.Security.Cryptography.Pkcs\"");
}

/// An external source (D1e) on C#: opened with its own driver but never
/// migrated, its records read and written by its own table, key and column
/// names, with a read-only one on a server database.
#[test]
fn an_external_source_keeps_its_names_and_is_never_migrated() {
    use clarity_project::DataSource;
    use clarity_spec::SourceRef;
    let erp = "module erp\nuse sales\nsource erp\n\nentity Invoice\n    table \"tblInvoice\"\n    key \"InvoiceID\"\n    number: text(20), column \"InvNo\"\n    total: money\n\nfunction big(limit: money) -> list<Invoice>\n    return find i in Invoice where i.number contains \"INV\" and i.total > limit order by i.number\n";
    let mut project = load_sources([("sales.clarity", SALES), ("erp.clarity", erp)]);
    project.sources = vec![SourceRef::sql("main"), SourceRef { readonly: true, ..SourceRef::sql("erp").external() }];
    let mut settings = settings();
    settings.datasources.push(DataSource {
        name: "erp".into(),
        kind: SourceKind::Mssql,
        url: "env:ERP_DATABASE".into(),
        auth: None,
        external: true,
        readonly: true,
    });
    let files = generate(&project, &settings).unwrap_or_else(|e| panic!("{e:#?}"));
    has(file(&files, "Program.cs"), "(\"erp\", Dialect.Mssql, \"env:ERP_DATABASE\")");
    let schema = file(&files, "Schema.cs");
    assert!(!schema.contains("(\"erp\"") && !schema.contains("tblInvoice"), "{schema}");
    let erp_cs = file(&files, "Erp.cs");
    has(erp_cs, "    public static string SourceName => \"erp\";");
    has(erp_cs, "    public static string TableName => \"tblInvoice\";");
    has(erp_cs, "    public static string KeyColumn => \"InvoiceID\";");
    has(erp_cs, "[\"InvNo\", \"total\"]");
    has(erp_cs, "Cond.Contains(\"InvNo\", \"INV\")");
    has(erp_cs, ".Order(\"InvNo\", false)");
    // Its database's driver comes along, though Clarity owns no table there.
    has(file(&files, "Shop.csproj"), "<DefineConstants>$(DefineConstants);CLARITY_MSSQL</DefineConstants>");
}
