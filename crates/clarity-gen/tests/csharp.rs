//! The C# backend (G2a): the data dictionary, functions, actions and data
//! events as C#, over the C# runtime's NuGet packages.

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

/// The version of the runtime's packages, as runtimes/csharp/Directory.Build.props sets it.
fn runtime_version() -> &'static str {
    let props = include_str!("../../../runtimes/csharp/Directory.Build.props");
    let start = props.find("<Version>").unwrap() + "<Version>".len();
    &props[start..start + props[start..].find("</Version>").unwrap()]
}

#[test]
fn writes_a_complete_dotnet_project_on_the_runtime_packages() {
    let files = files();
    let paths: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();
    assert_eq!(
        paths,
        [
            "App.axaml",
            "App.axaml.cs",
            "AppThemes.cs",
            "Modules/Sales.cs",
            "Program.cs",
            "README.md",
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
    // Clarity.Avalonia brings Clarity.Runtime and the packages both stand on.
    has(project, &format!("<PackageReference Include=\"Clarity.Avalonia\" Version=\"{}\" />", runtime_version()));
    assert!(!project.contains("Include=\"Clarity.Runtime\"") && !project.contains("Microsoft.Data.Sqlite"), "{project}");
}

/// While working on the runtime, a project builds on the runtime's own
/// projects instead of the packages: `-p:ClarityRuntime=<runtimes/csharp>`,
/// or the environment variable CLARITY_RUNTIME.
#[test]
fn the_local_runtime_projects_can_stand_in_for_the_packages() {
    let project = file(&files(), "Shop.csproj").to_string();
    has(&project, "<ClarityRuntime Condition=\"'$(ClarityRuntime)' == ''\">$(CLARITY_RUNTIME)</ClarityRuntime>");
    has(
        &project,
        &format!(
            "  <ItemGroup Condition=\"'$(ClarityRuntime)' == ''\">
    <PackageReference Include=\"Clarity.Avalonia\" Version=\"{}\" />
  </ItemGroup>
  <ItemGroup Condition=\"'$(ClarityRuntime)' != ''\">
    <ProjectReference Include=\"$(ClarityRuntime)/Clarity.Avalonia/Clarity.Avalonia.csproj\" />
  </ItemGroup>",
            runtime_version()
        ),
    );
    // Each project it names is there.
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../runtimes/csharp");
    for line in project.lines().filter(|l| l.contains("<ProjectReference ")) {
        let path = line.split("$(ClarityRuntime)/").nth(1).unwrap().split('"').next().unwrap();
        assert!(root.join(path).exists(), "{path}");
    }
}

/// The program builds on the runtime's packages and carries no copy of them.
#[test]
fn no_runtime_source_is_copied() {
    let files = files();
    assert!(!files.iter().any(|f| f.path.starts_with("Clarity.")), "{:?}", files.iter().map(|f| &f.path).collect::<Vec<_>>());
}

#[test]
fn is_deterministic() {
    assert_eq!(files(), files());
}

#[test]
fn enums_and_entities_become_csharp_types() {
    let files = files();
    let sales = file(&files, "Modules/Sales.cs");
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
    let sales = file(&files, "Modules/Sales.cs");
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
    let sales = file(&files, "Modules/Sales.cs");
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
    let sales = file(&files, "Modules/Sales.cs");
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
    settings.stack.ui = Ui::Web;
    assert_eq!(generate(&project, &settings).unwrap_err(), ["C# with the web renderer is not generated yet; C# uses Avalonia"]);
}

/// A project references the package of each server database its data
/// sources are, and no others (Clarity.Runtime.PostgreSql and the others,
/// which bring their own clients); the program turns each on before it opens
/// its databases.
#[test]
fn a_project_gets_the_drivers_of_its_databases_only() {
    let project = load_sources([("sales.clarity", SALES)]);
    let plain = generate(&project, &settings()).unwrap();
    let csproj = file(&plain, "Shop.csproj");
    assert!(!csproj.contains("Clarity.Runtime."), "{csproj}");
    assert!(!file(&plain, "Program.cs").contains(".Use();"));

    let packages = ["Clarity.Runtime.PostgreSql", "Clarity.Runtime.MySql", "Clarity.Runtime.SqlServer"];
    for (kind, package, turned_on) in [
        (SourceKind::Postgres, packages[0], "PostgreSql.Use();"),
        (SourceKind::Mysql, packages[1], "MySql.Use();"),
        (SourceKind::Mssql, packages[2], "SqlServer.Use();"),
    ] {
        let mut settings = settings();
        settings.datasources[0].kind = kind;
        settings.datasources[0].url = "env:SHOP_DATABASE".into();
        let files = generate(&project, &settings).unwrap();
        let csproj = file(&files, "Shop.csproj");
        has(csproj, &format!("    <PackageReference Include=\"{package}\" Version=\"{}\" />\n", runtime_version()));
        has(csproj, &format!("    <ProjectReference Include=\"$(ClarityRuntime)/{package}/{package}.csproj\" />\n"));
        assert_eq!(packages.iter().filter(|p| csproj.contains(&format!("Include=\"{p}\""))).count(), 1, "{csproj}");
        // The clients come with the driver's package.
        assert!(!csproj.contains("Npgsql") && !csproj.contains("MySqlConnector") && !csproj.contains("SqlClient"), "{csproj}");
        let program = file(&files, "Program.cs");
        has(program, "\"env:SHOP_DATABASE\")];");
        // The driver is turned on before the databases open.
        has(program, &format!("        {turned_on}\n        var db = Db.Connect(Databases);"));
    }
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
    let erp_cs = file(&files, "Modules/Erp.cs");
    has(erp_cs, "    public static string SourceName => \"erp\";");
    has(erp_cs, "    public static string TableName => \"tblInvoice\";");
    has(erp_cs, "    public static string KeyColumn => \"InvoiceID\";");
    has(erp_cs, "[\"InvNo\", \"total\"]");
    has(erp_cs, "Cond.Contains(\"InvNo\", \"INV\")");
    has(erp_cs, ".Order(\"InvNo\", false)");
    // Its database's driver comes along, though Clarity owns no table there.
    has(file(&files, "Program.cs"), "        SqlServer.Use();\n");
}

/// A record's id, which code can read, is the record's `Id`.
#[test]
fn a_record_s_id_reads_as_its_id() {
    let source = format!("{SALES}\nfunction same(order: Order, other: Order) -> bool\n    return order.id == other.id\n");
    let files = generate(&load_sources([("sales.clarity", source.as_str())]), &settings()).unwrap_or_else(|e| panic!("{e:#?}"));
    has(file(&files, "Modules/Sales.cs"), "return (order.Id == other.Id);");
}

/// A record given to a reference field is stored as its id.
#[test]
fn a_reference_field_takes_a_record() {
    let source = format!("{SALES}\naction assign(order: Order, customer: Customer)\n    effects db.write\n    order.customer = customer\n    save order\n");
    let files = generate(&load_sources([("sales.clarity", source.as_str())]), &settings()).unwrap_or_else(|e| panic!("{e:#?}"));
    has(file(&files, "Modules/Sales.cs"), "order.Customer = customer.Id;");
}

/// `none`, the empty value of a date or reference (X2).
#[test]
fn none_is_null() {
    let source = format!("{SALES}\nfunction unplaced(order: Order) -> bool\n    return order.customer == none or order.customer.since != none\n\naction clear(order: Order, customer: Customer)\n    effects db.write\n    customer.since = none\n    order.customer = none\n    save order\n\nfunction without_customer() -> list<Order>\n    return find o in Order where o.customer == none\n");
    let files = generate(&load_sources([("sales.clarity", source.as_str())]), &settings()).unwrap_or_else(|e| panic!("{e:#?}"));
    let sales = file(&files, "Modules/Sales.cs");
    for line in ["(order.Customer == null)", "!(db.Load<Customer>(order.Customer).Since == null)", "customer.Since = null;", "order.Customer = null;", "Cond.IsNone(\"customer\")"] {
        has(sales, line);
    }
}

/// `today()` and `now()` read the runtime's clock (X2).
#[test]
fn today_reads_the_clock() {
    let source = format!("{SALES}\naction stamp(customer: Customer)\n    effects db.write\n    if customer.since == none or customer.since > today()\n        customer.since = today()\n    save customer\n");
    let files = generate(&load_sources([("sales.clarity", source.as_str())]), &settings()).unwrap_or_else(|e| panic!("{e:#?}"));
    let sales = file(&files, "Modules/Sales.cs");
    has(sales, "customer.Since = Clock.Today();");
    has(sales, "(customer.Since > Clock.Today())");
}

const MUSIC: &str = "module music\n\nentity Track\n    title: text(80)\n    length: duration, min 0, default 180\n\nfunction album_length(tracks: list<Track>) -> duration\n    let total: duration = 0\n    for track in tracks\n        total = total + track.length\n    return total\n\ncode Track\n    before save\n        if track.length > 7200\n            refuse \"{track.title} lasts {track.length}\"\n\npage Tracks: window\n    data track: Track\n    number at 1,1 span 3 bind track.length\n    text at 4,1 span 3 bind track.length\n";

/// A duration is whole seconds; its slot and its parts show it as 3:45 (X2).
#[test]
fn a_duration_is_seconds_shown_as_minutes_and_seconds() {
    let project = load_sources([("music.clarity", MUSIC)]);
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    let files = generate(&project, &settings()).unwrap_or_else(|e| panic!("{e:#?}"));
    let music = file(&files, "Modules/Music.cs");
    has(music, "public long Length");
    has(music, "long _length = 180L;");
    has(music, ".With(\"track.length\", Slot.OfDuration(track.Length))");
    let shown = file(&files, "Shown.cs");
    has(shown, "public static readonly FormatConverter TrackLength = new(new FieldFormat(FormatKind.Duration));");
    has(shown, "public static readonly FieldFormat TrackLength = new FieldFormat(FormatKind.Duration);");
    let view = file(&files, "Windows/TracksView.axaml");
    has(view, "Format=\"{x:Static local:Edit.TrackLength}\" Value=\"{Binding Track.Length}\"");
    has(view, "Text=\"{Binding Track.Length, Converter={x:Static local:Shown.TrackLength}}\"");
}

/// X2: a query through a reference: the condition and the order read the
/// referred record in a subquery.
#[test]
fn a_query_goes_through_a_reference() {
    let source = format!(
        "{SALES}\nfunction of_customers(name: text) -> list<Order>\n    return find o in Order where o.customer.name contains name and o.customer.credit_limit > 10 order by o.customer.name desc, o.customer.credit_limit\n"
    );
    let files = generate(&load_sources([("sales.clarity", source.as_str())]), &settings()).unwrap_or_else(|e| panic!("{e:#?}"));
    let sales = file(&files, "Modules/Sales.cs");
    has(sales, ".Filter(Cond.Through(\"customer\", \"customer\", \"id\", Cond.Contains(\"name\", name)).And(Cond.Through(\"customer\", \"customer\", \"id\", Cond.Number(\"credit_limit\", Op.Gt, ");
    has(sales, ".OrderThrough([new Hop(\"customer\", \"customer\", \"id\")], \"name\", true, false).OrderThrough([new Hop(\"customer\", \"customer\", \"id\")], \"credit_limit\", false, true)");
}
