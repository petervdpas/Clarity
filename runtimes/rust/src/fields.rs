//! What a screen knows of a program's fields and types (DESIGN 7.9), beside
//! the values: each entity field's label, format, whether it is required, its
//! range and what it refers to, keyed "Entity.field", and the type of each
//! value of a window's data. The generated program carries them as one static
//! table, [`Facts`], which `Program::facts` gives; the parts of Clarity.UI
//! find a field by its path in a window's data, as the C# host does.

/// How a field's values are shown and read: its kind of value, the decimals
/// of a decimal and its written format (`percent`, `long`); as the shared
/// cases of conformance/formats.json name them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FieldFormat {
    pub kind: FormatKind,
    /// The number of decimals of a decimal field; none for the other kinds.
    pub scale: Option<u32>,
    /// The format the field's line writes (`format percent`); none for the kind's default.
    pub format: Option<&'static str>,
}

/// The kinds of value that have formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormatKind {
    Integer,
    Decimal,
    Money,
    Date,
    DateTime,
    Duration,
}

/// What a screen needs to know of a field beyond its value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FieldFacts {
    /// Its label as written: a text key (`@orderTotalLabel`) or the text itself.
    pub label: &'static str,
    /// How its values are shown and read; none for a field without a format.
    pub format: Option<FieldFormat>,
    /// Whether a record needs a value in it.
    pub required: bool,
    /// Its least value, for a number or a date, as Clarity writes it.
    pub min: Option<&'static str>,
    /// Its greatest value.
    pub max: Option<&'static str>,
    /// The entity a reference points at (`Customer`); none for any other field.
    pub refers: Option<&'static str>,
    /// For an entity whose records make a tree: the field that names a record's parent.
    pub parent: Option<&'static str>,
    /// Whether it holds a color (`#3b82f6`), shown with a swatch beside it.
    pub color: bool,
}

/// A list of names and what each is: a window's values and their types, a
/// record's fields and theirs.
pub type Typed = &'static [(&'static str, &'static str)];

/// The facts of a program, as the generator writes them. A type is written
/// as Clarity writes it: a record, entity or enum by its name (`Order`), a
/// list as `list<Order>`, a reference as `ref<Customer>`, a plain value by
/// its keyword (`text`, `money`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Facts {
    /// Each window's and app's data: the name of each value and its type.
    pub windows: &'static [(&'static str, Typed)],
    /// Each entity's and record's fields and their types.
    pub records: &'static [(&'static str, Typed)],
    /// The facts of every entity's field, keyed "Entity.field".
    pub fields: &'static [(&'static str, FieldFacts)],
    /// Each enum's values, as written.
    pub enums: &'static [(&'static str, &'static [&'static str])],
}

fn find<T: Copy>(table: &'static [(&'static str, T)], name: &str) -> Option<T> {
    table.iter().find(|(n, _)| *n == name).map(|(_, t)| *t)
}

/// The type of a list's rows: `Order` of `list<Order>`.
fn row_type(ty: &str) -> Option<&str> {
    ty.strip_prefix("list<").and_then(|t| t.strip_suffix('>'))
}

/// A path's names, each with whether a place follows it: `orders[0].total`
/// is `orders` (placed) and `total`.
fn names(path: &str) -> Vec<(&str, bool)> {
    path.split('.')
        .map(|step| match step.find('[') {
            Some(at) => (&step[..at], true),
            None => (step, false),
        })
        .collect()
}

impl Facts {
    /// A program that carries no facts.
    pub const EMPTY: Facts = Facts { windows: &[], records: &[], fields: &[], enums: &[] };

    /// The type of the value at `path` in the data of `window`: a row of a
    /// list by its place (`orders[0]`) is the list's record; none for a path
    /// the window's data does not have.
    pub fn type_of(&self, window: &str, path: &str) -> Option<&'static str> {
        let mut steps = names(path).into_iter();
        let (first, placed) = steps.next()?;
        let mut ty = find(find(self.windows, window)?, first)?;
        if placed {
            ty = row_type(ty)?;
        }
        for (name, placed) in steps {
            // A field of a list's rows is reached without a place too: `orders.total`.
            let record = row_type(ty).unwrap_or(ty);
            ty = find(find(self.records, record)?, name)?;
            if placed {
                ty = row_type(ty)?;
            }
        }
        Some(ty)
    }

    /// The facts of the field at `path` in the data of `window`, by the
    /// record it is in and its name: `order.total`, `orders[0].total` and
    /// `orders.total` are all "Order.total". None for a value that is no
    /// entity's field.
    pub fn field(&self, window: &str, path: &str) -> Option<FieldFacts> {
        let (owner, name) = path.rsplit_once('.')?;
        let ty = self.type_of(window, owner)?;
        let record = row_type(ty).unwrap_or(ty);
        // The entity is an open set of the program's, so its key is made here.
        let key = format!("{record}.{name}");
        find(self.fields, &key)
    }

    /// The facts of a field by its key, "Entity.field".
    pub fn field_of(&self, key: &str) -> Option<FieldFacts> {
        find(self.fields, key)
    }

    /// The values of the enum `name`, as written; none for a name that is no enum.
    pub fn enum_values(&self, name: &str) -> &'static [&'static str] {
        find(self.enums, name).unwrap_or(&[])
    }
}

impl Default for Facts {
    fn default() -> Self {
        Facts::EMPTY
    }
}
