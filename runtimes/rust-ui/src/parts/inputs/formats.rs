//! Showing and reading values in the person's locale (DESIGN 1.5, I1a), as
//! every renderer does (conformance/formats.json; C# Clarity.Runtime's
//! Formats). The locale data is carried here, as the shared cases pin it:
//! dates with two-digit days and months, a 24-hour clock with two-digit
//! hours, no weekday in a long date, no-break spaces beside a currency or
//! percent sign, and a plain space before AM and PM. A language this table
//! does not know is shown as American English.
//!
//! Numbers are held exactly: as whole digits and how many of them are
//! decimals, never as floating point.

use serde_json::Value;

use crate::parts::{FieldFacts, PartHost};

/// The types that are formatted, by the kind the designer sees.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormatKind {
    Integer,
    Decimal,
    Money,
    Date,
    DateTime,
    /// Whole seconds, shown as m:ss or h:mm:ss in every locale.
    Duration,
}

impl FormatKind {
    /// The kind by its name in the model and in formats.json (`dateTime`); none for a name of no formatted kind.
    pub fn named(name: &str) -> Option<FormatKind> {
        Some(match name {
            "integer" => FormatKind::Integer,
            "decimal" => FormatKind::Decimal,
            "money" => FormatKind::Money,
            "date" => FormatKind::Date,
            "dateTime" | "datetime" | "moment" => FormatKind::DateTime,
            "duration" => FormatKind::Duration,
            _ => return None,
        })
    }

    /// The kind of a value's type as a host names it (`PartHost::type_of`): an integer, a decimal, a date, a moment.
    pub fn of_type(name: &str) -> Option<FormatKind> {
        Some(match name {
            "integer" | "int" | "long" | "i32" | "i64" | "Int" => FormatKind::Integer,
            "decimal" | "double" | "f64" | "Decimal" => FormatKind::Decimal,
            "money" => FormatKind::Money,
            "date" | "Date" | "NaiveDate" | "DateOnly" => FormatKind::Date,
            "dateTime" | "datetime" | "moment" | "DateTime" | "NaiveDateTime" => FormatKind::DateTime,
            "duration" => FormatKind::Duration,
            _ => return None,
        })
    }
}

/// How a value of a field is shown: its kind, its scale (decimals), and the named format the field picks, if any (`plain`, `percent`, `long`, `iso`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldFormat {
    pub kind: FormatKind,
    pub scale: Option<u32>,
    pub format: Option<String>,
}

impl FieldFormat {
    pub fn new(kind: FormatKind) -> FieldFormat {
        FieldFormat { kind, scale: None, format: None }
    }

    /// A format as `FieldFacts::format` writes it: its kind, then its scale and its named format when it has them (`decimal 2 percent`, `date long`, `integer`).
    pub fn parse(written: &str) -> Option<FieldFormat> {
        let mut words = written.split_whitespace();
        let mut field = FieldFormat::new(FormatKind::named(words.next()?)?);
        for word in words {
            match word.parse::<u32>() {
                Ok(scale) => field.scale = Some(scale),
                Err(_) => field.format = Some(word.to_string()),
            }
        }
        Some(field)
    }

    /// A format as formats.json writes one: `{"kind": "decimal", "scale": 2, "format": "plain"}`.
    pub fn of_json(field: &Value) -> Option<FieldFormat> {
        Some(FieldFormat {
            kind: FormatKind::named(field.get("kind")?.as_str()?)?,
            scale: field.get("scale").and_then(Value::as_u64).map(|s| s as u32),
            format: field.get("format").and_then(Value::as_str).map(str::to_string),
        })
    }

    fn named(&self, name: &str) -> bool {
        self.format.as_deref() == Some(name)
    }
}

/// The format of the value at `path`: as the host knows its field, else by its type; none for a value that is no number or date.
pub fn format_of(host: &dyn PartHost, path: &str) -> Option<FieldFormat> {
    if let Some(known) = host.field(path).and_then(|f: FieldFacts| f.format).and_then(|f| FieldFormat::parse(&f)) {
        return Some(known);
    }
    host.type_of(path).and_then(|t| FormatKind::of_type(&t)).map(FieldFormat::new)
}

/// The person's formats as the host gives them.
pub fn formats_of(host: &dyn PartHost) -> Formats {
    let given = host.formats();
    Formats::new(given.language.as_deref(), given.currency.as_deref())
}

// -- Exact numbers ---------------------------------------------------------

/// A number as whole digits and how many of them are decimals.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Exact {
    pub n: i128,
    pub scale: u32,
}

impl Exact {
    /// A number written as `-1250.5` (no groups, a point); none for anything else.
    pub fn parse(text: &str) -> Option<Exact> {
        let text = text.trim();
        let (negative, digits) = match text.strip_prefix('-') {
            Some(rest) => (true, rest),
            None => (false, text),
        };
        let (whole, part) = digits.split_once('.').unwrap_or((digits, ""));
        if whole.is_empty() || (digits.contains('.') && part.is_empty()) || !whole.bytes().chain(part.bytes()).all(|b| b.is_ascii_digit()) {
            return None;
        }
        let n: i128 = format!("{whole}{part}").parse().ok()?;
        Some(Exact { n: if negative { -n } else { n }, scale: part.len() as u32 })
    }

    /// A JSON value as a number: a number, or text that is one.
    pub fn of(value: &Value) -> Option<Exact> {
        match value {
            Value::Number(n) => match n.as_i64() {
                Some(i) => Some(Exact { n: i as i128, scale: 0 }),
                None => Exact::parse(&n.to_string()).or_else(|| n.as_f64().and_then(|f| Exact::parse(&format!("{f}")))),
            },
            Value::String(s) => Exact::parse(s),
            _ => None,
        }
    }

    /// The same number with `scale` decimals, rounded half away from zero.
    pub fn rounded(self, scale: u32) -> Exact {
        if scale >= self.scale {
            return Exact { n: self.n * 10i128.pow(scale - self.scale), scale };
        }
        let by = 10i128.pow(self.scale - scale);
        let (q, r) = (self.n / by, self.n % by);
        let up = r.abs() * 2 >= by;
        Exact { n: if up { q + self.n.signum() } else { q }, scale }
    }

    /// The decimals it has without its trailing zeros.
    pub fn digits(self) -> u32 {
        let (mut n, mut scale) = (self.n, self.scale);
        while scale > 0 && n % 10 == 0 {
            n /= 10;
            scale -= 1;
        }
        scale
    }

    /// Ten to the `by`th times it: a percent is a hundred times its value.
    pub fn shifted(self, by: i32) -> Exact {
        if by >= 0 {
            let by = by as u32;
            if self.scale >= by { Exact { n: self.n, scale: self.scale - by } } else { Exact { n: self.n * 10i128.pow(by - self.scale), scale: 0 } }
        } else {
            Exact { n: self.n, scale: self.scale + (-by) as u32 }
        }
    }

    /// As invariant text, without trailing zeros: `1250.5`.
    pub fn text(self) -> String {
        let trimmed = self.rounded(self.digits());
        let (whole, part) = split(trimmed);
        let sign = if trimmed.n < 0 { "-" } else { "" };
        if part.is_empty() { format!("{sign}{whole}") } else { format!("{sign}{whole}.{part}") }
    }
}

/// The digits before and after the point of a number, without its sign.
fn split(e: Exact) -> (String, String) {
    let digits = format!("{:0>width$}", e.n.unsigned_abs(), width = e.scale as usize + 1);
    let (whole, part) = digits.split_at(digits.len() - e.scale as usize);
    (whole.to_string(), part.to_string())
}

// -- Locales ---------------------------------------------------------------

/// Where a currency's sign goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Sign {
    /// `$1,250.50`, `-$1,250.50`.
    Before,
    /// `EUR 1.250,50` with the sign first, `EUR -1.250,50`.
    BeforeSpaced,
    /// `1.250,50 EUR` with the sign last.
    After,
}

/// What a locale writes numbers and dates with.
#[derive(Debug, Clone, Copy)]
struct Locale {
    group: &'static str,
    decimal: &'static str,
    /// The text between a percent and its sign.
    percent_gap: &'static str,
    money: Sign,
    /// The short date, with `dd`, `MM`, `yyyy`.
    short: &'static str,
    /// The long date, with `d`, `MMMM`, `yyyy` (and `M` for a month's number).
    long: &'static str,
    /// A 12-hour clock with AM and PM.
    twelve: bool,
    months: [&'static str; 12],
    /// A month with its year, as a calendar's title shows it.
    month_title: &'static str,
    /// The short names of the days of the week, from Sunday.
    days: [&'static str; 7],
    /// The day a week starts on: 0 for Sunday, 1 for Monday.
    first_day: u32,
}

const NBSP: &str = "\u{a0}";

const ENGLISH_MONTHS: [&str; 12] = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];

const EN_US: Locale = Locale { group: ",", decimal: ".", percent_gap: "", money: Sign::Before, short: "MM/dd/yyyy", long: "MMMM d, yyyy", twelve: true, months: ENGLISH_MONTHS, month_title: "MMMM yyyy", days: ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"], first_day: 0 };
const EN_GB: Locale = Locale { group: ",", decimal: ".", percent_gap: "", money: Sign::Before, short: "dd/MM/yyyy", long: "d MMMM yyyy", twelve: false, months: ENGLISH_MONTHS, month_title: "MMMM yyyy", days: ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"], first_day: 1 };
const NL: Locale = Locale {
    group: ".",
    decimal: ",",
    percent_gap: "",
    money: Sign::BeforeSpaced,
    short: "dd-MM-yyyy",
    long: "d MMMM yyyy",
    twelve: false,
    months: ["januari", "februari", "maart", "april", "mei", "juni", "juli", "augustus", "september", "oktober", "november", "december"],
    month_title: "MMMM yyyy", days: ["zo", "ma", "di", "wo", "do", "vr", "za"], first_day: 1,
};
const DE: Locale = Locale {
    group: ".",
    decimal: ",",
    percent_gap: NBSP,
    money: Sign::After,
    short: "dd.MM.yyyy",
    long: "d. MMMM yyyy",
    twelve: false,
    months: ["Januar", "Februar", "M\u{e4}rz", "April", "Mai", "Juni", "Juli", "August", "September", "Oktober", "November", "Dezember"],
    month_title: "MMMM yyyy", days: ["So", "Mo", "Di", "Mi", "Do", "Fr", "Sa"], first_day: 1,
};
const FR: Locale = Locale {
    group: "\u{202f}",
    decimal: ",",
    percent_gap: NBSP,
    money: Sign::After,
    short: "dd/MM/yyyy",
    long: "d MMMM yyyy",
    twelve: false,
    months: ["janvier", "f\u{e9}vrier", "mars", "avril", "mai", "juin", "juillet", "ao\u{fb}t", "septembre", "octobre", "novembre", "d\u{e9}cembre"],
    month_title: "MMMM yyyy", days: ["dim.", "lun.", "mar.", "mer.", "jeu.", "ven.", "sam."], first_day: 1,
};
const ES: Locale = Locale {
    group: ".",
    decimal: ",",
    percent_gap: NBSP,
    money: Sign::After,
    short: "dd/MM/yyyy",
    long: "d 'de' MMMM 'de' yyyy",
    twelve: false,
    months: ["enero", "febrero", "marzo", "abril", "mayo", "junio", "julio", "agosto", "septiembre", "octubre", "noviembre", "diciembre"],
    month_title: "MMMM 'de' yyyy", days: ["dom", "lun", "mar", "mi\u{e9}", "jue", "vie", "s\u{e1}b"], first_day: 1,
};
const IT: Locale = Locale {
    group: ".",
    decimal: ",",
    percent_gap: "",
    money: Sign::After,
    short: "dd/MM/yyyy",
    long: "d MMMM yyyy",
    twelve: false,
    months: ["gennaio", "febbraio", "marzo", "aprile", "maggio", "giugno", "luglio", "agosto", "settembre", "ottobre", "novembre", "dicembre"],
    month_title: "MMMM yyyy", days: ["dom", "lun", "mar", "mer", "gio", "ven", "sab"], first_day: 1,
};
const JA: Locale = Locale {
    group: ",",
    decimal: ".",
    percent_gap: "",
    money: Sign::Before,
    short: "yyyy/MM/dd",
    long: "yyyy\u{5e74}M\u{6708}d\u{65e5}",
    twelve: false,
    months: ["1\u{6708}", "2\u{6708}", "3\u{6708}", "4\u{6708}", "5\u{6708}", "6\u{6708}", "7\u{6708}", "8\u{6708}", "9\u{6708}", "10\u{6708}", "11\u{6708}", "12\u{6708}"],
    month_title: "yyyy\u{5e74}M\u{6708}", days: ["\u{65e5}", "\u{6708}", "\u{706b}", "\u{6c34}", "\u{6728}", "\u{91d1}", "\u{571f}"], first_day: 0,
};

fn locale_of(language: &str) -> Locale {
    let tag = language.replace('_', "-");
    let mut parts = tag.split('-');
    let lang = parts.next().unwrap_or("").to_ascii_lowercase();
    let region = parts.next().unwrap_or("").to_ascii_uppercase();
    match lang.as_str() {
        "en" if region.is_empty() || region == "US" => EN_US,
        "en" => EN_GB,
        "nl" => NL,
        "de" => DE,
        "fr" => FR,
        "es" => ES,
        "it" => IT,
        "ja" => JA,
        _ => EN_US,
    }
}

/// The sign and decimals of a currency, by its ISO 4217 code: EUR has 2 decimals and JPY none; an unknown code is its own sign.
fn currency_of(code: &str) -> (String, u32) {
    let (sign, digits) = match code {
        "EUR" => ("\u{20ac}", 2),
        "USD" => ("$", 2),
        "GBP" => ("\u{a3}", 2),
        "JPY" => ("\u{ffe5}", 0),
        "CHF" => ("CHF", 2),
        "CNY" => ("\u{a5}", 2),
        "INR" => ("\u{20b9}", 2),
        "SEK" | "NOK" | "DKK" => ("kr", 2),
        _ => (code, 2),
    };
    (sign.to_string(), digits)
}

// -- Formats ---------------------------------------------------------------

/// Where values are shown: the person's language (`nl-NL`) and the project's currency (an ISO 4217 code, or none).
#[derive(Debug, Clone)]
pub struct Formats {
    locale: Locale,
    currency: Option<String>,
}

impl Default for Formats {
    fn default() -> Self {
        Formats::new(None, None)
    }
}

impl Formats {
    pub fn new(language: Option<&str>, currency: Option<&str>) -> Formats {
        Formats { locale: locale_of(language.unwrap_or("en-US")), currency: currency.filter(|c| !c.is_empty()).map(str::to_string) }
    }

    /// A stored value as people read it: a number, a date or a date and
    /// time, as JSON holds them. A value that is no number or date of its
    /// kind (a database written by another program) shows as it is.
    pub fn show(&self, field: &FieldFormat, value: &Value) -> String {
        match value {
            Value::Null => String::new(),
            Value::Bool(b) => b.to_string(),
            _ if field.kind == FormatKind::Duration => match Exact::of(value).filter(|e| e.scale == 0) {
                Some(e) => duration(e.n),
                None => as_text(value),
            },
            Value::String(s) if matches!(field.kind, FormatKind::Date | FormatKind::DateTime) => match Moment::parse(s) {
                Some(m) if field.kind == FormatKind::Date => self.date_part(&m, field),
                Some(m) => format!("{} {}", self.date_part(&m, field), self.time_part(&m, field)),
                None => s.clone(),
            },
            _ => match Exact::of(value) {
                Some(e) if !matches!(field.kind, FormatKind::Date | FormatKind::DateTime) => self.show_number(field, e),
                _ => as_text(value),
            },
        }
    }

    /// What a number input shows: as the value is shown, but money without its currency sign, so what is typed and what is shown look alike.
    pub fn edit_text(&self, field: &FieldFormat, value: &Value) -> String {
        if field.kind == FormatKind::Money {
            let plain = FieldFormat { format: Some("plain".to_string()), ..field.clone() };
            return self.show(&plain, value);
        }
        self.show(field, value)
    }

    /// What was typed in a number input, read in the locale: the value to
    /// store (a whole number for integers and durations, else exact text as
    /// `1250.5`), or none when it is no number of that kind. Groups,
    /// spaces, the currency sign and the percent sign may be typed or left out.
    pub fn read(&self, field: &FieldFormat, typed: &str) -> Option<Value> {
        if field.kind == FormatKind::Duration {
            return read_duration(typed).map(Value::from);
        }
        let mut s: String = typed.chars().filter(|c| !c.is_whitespace() && *c != '%' && *c != '\u{202f}').map(|c| if c == '\u{2212}' { '-' } else { c }).collect();
        if let Some(code) = &self.currency {
            let (sign, _) = currency_of(code);
            s = s.replace(&sign, "").replace(code.as_str(), "");
        }
        let group = self.locale.group.trim();
        if !group.is_empty() && group != "\u{202f}" {
            s = s.replace(group, "");
        }
        if self.locale.decimal != "." {
            s = s.replace(self.locale.decimal, ".");
        }
        let number = Exact::parse(&s)?;
        if field.kind == FormatKind::Integer {
            return if s.contains('.') { None } else { i64::try_from(number.n).ok().map(Value::from) };
        }
        let number = if field.named("percent") { number.shifted(-2) } else { number };
        Some(Value::String(number.text()))
    }

    fn show_number(&self, field: &FieldFormat, value: Exact) -> String {
        let plain = field.named("plain");
        match field.kind {
            FormatKind::Integer => self.number(value.rounded(0), !plain),
            FormatKind::Decimal if field.named("percent") => {
                let hundred = value.shifted(2);
                // Without a scale (a local in code) a value keeps its own digits.
                let digits = field.scale.map(|s| s.saturating_sub(2)).unwrap_or_else(|| hundred.digits());
                format!("{}{}%", self.number(hundred.rounded(digits), true), self.locale.percent_gap)
            }
            FormatKind::Decimal => {
                let digits = field.scale.unwrap_or_else(|| value.digits());
                self.number(value.rounded(digits), !plain)
            }
            _ => {
                let (sign, digits) = match &self.currency {
                    Some(code) => {
                        let (s, d) = currency_of(code);
                        (Some(s), d)
                    }
                    None => (None, 2),
                };
                let rounded = value.rounded(digits);
                let Some(sign) = sign.filter(|_| !plain) else {
                    return self.number(rounded, true);
                };
                let negative = rounded.n < 0;
                let amount = self.number(Exact { n: rounded.n.abs(), scale: rounded.scale }, true);
                let minus = if negative { "-" } else { "" };
                match self.locale.money {
                    Sign::Before => format!("{minus}{sign}{amount}"),
                    Sign::BeforeSpaced => format!("{sign}{NBSP}{minus}{amount}"),
                    Sign::After => format!("{minus}{amount}{NBSP}{sign}"),
                }
            }
        }
    }

    /// A number with its decimals as it has them, its groups when `grouped`, in the locale.
    fn number(&self, e: Exact, grouped: bool) -> String {
        let (whole, part) = split(e);
        let whole = if grouped { grouped_digits(&whole, self.locale.group) } else { whole };
        let sign = if e.n < 0 { "-" } else { "" };
        if part.is_empty() { format!("{sign}{whole}") } else { format!("{sign}{whole}{}{part}", self.locale.decimal) }
    }

    fn date_part(&self, m: &Moment, field: &FieldFormat) -> String {
        let pattern = if field.named("iso") {
            "yyyy-MM-dd"
        } else if field.named("long") {
            self.locale.long
        } else {
            self.locale.short
        };
        self.write_date(pattern, m)
    }

    fn write_date(&self, pattern: &str, m: &Moment) -> String {
        let mut out = String::new();
        let chars: Vec<char> = pattern.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            let c = chars[i];
            if c == '\'' {
                i += 1;
                while i < chars.len() && chars[i] != '\'' {
                    out.push(chars[i]);
                    i += 1;
                }
                i += 1;
                continue;
            }
            let mut run = 1;
            while i + run < chars.len() && chars[i + run] == c {
                run += 1;
            }
            match (c, run) {
                ('y', _) => out.push_str(&format!("{:04}", m.year)),
                ('M', 4) => out.push_str(self.locale.months[(m.month - 1) as usize]),
                ('M', 2) => out.push_str(&format!("{:02}", m.month)),
                ('M', _) => out.push_str(&m.month.to_string()),
                ('d', 2) => out.push_str(&format!("{:02}", m.day)),
                ('d', _) => out.push_str(&m.day.to_string()),
                _ => (0..run).for_each(|_| out.push(c)),
            }
            i += run;
        }
        out
    }

    fn time_part(&self, m: &Moment, field: &FieldFormat) -> String {
        if !field.named("iso") && self.locale.twelve {
            let hour = match m.hour % 12 {
                0 => 12,
                h => h,
            };
            return format!("{hour}:{:02} {}", m.minute, if m.hour < 12 { "AM" } else { "PM" });
        }
        format!("{:02}:{:02}", m.hour, m.minute)
    }

    /// A day as the locale writes it short (`17-10-2026`), for a date input.
    pub fn short_date(&self, year: i32, month: u32, day: u32) -> String {
        self.write_date(self.locale.short, &Moment { year, month, day, hour: 0, minute: 0, second: 0 })
    }

    /// A day typed as the locale writes it: in its short order (day, month
    /// and year as the locale has them), as its long date (`October 21,
    /// 2026`, `21 oktober 2026`), or as `2026-10-17`; none for no day.
    pub fn read_date(&self, typed: &str) -> Option<(i32, u32, u32)> {
        let typed = typed.trim();
        if let Some(m) = Moment::parse(typed) {
            return Some((m.year, m.month, m.day));
        }
        let numbers: Vec<&str> = typed.split(|c: char| !c.is_ascii_digit()).filter(|p| !p.is_empty()).collect();
        let lower = typed.to_lowercase();
        let named = (0..12u32).filter(|m| lower.contains(&self.locale.months[*m as usize].to_lowercase())).max_by_key(|m| self.locale.months[*m as usize].len());
        let (pattern, month) = match (numbers.len(), named) {
            (3, _) => (self.locale.short, None),
            (2, Some(m)) => (self.locale.long, Some(m + 1)),
            _ => return None,
        };
        let order: Vec<char> = pattern.chars().filter(|c| matches!(c, 'd' | 'M' | 'y')).filter(|c| month.is_none() || *c != 'M').fold(Vec::new(), |mut seen, c| {
            if seen.last() != Some(&c) {
                seen.push(c);
            }
            seen
        });
        let (mut year, mut day) = (None, None);
        let mut month = month;
        for (which, written) in order.iter().zip(numbers) {
            let n: i32 = written.parse().ok()?;
            match which {
                'y' => year = Some(if written.len() <= 2 { 2000 + n } else { n }),
                'M' => month = Some(n as u32),
                _ => day = Some(n as u32),
            }
        }
        let (year, month, day) = (year?, month?, day?);
        (days_in(year, month)? >= day && day >= 1).then_some((year, month, day))
    }

    /// A month with its year as a calendar's title shows it: `October 2026`, `oktober 2026`.
    pub fn month_title(&self, year: i32, month: u32) -> String {
        self.write_date(self.locale.month_title, &Moment { year, month: month.clamp(1, 12), day: 1, hour: 0, minute: 0, second: 0 })
    }

    /// The day a week starts on: 0 for Sunday, 1 for Monday.
    pub fn first_weekday(&self) -> u32 {
        self.locale.first_day
    }

    /// The short names of the days of the week, from the day a week starts on.
    pub fn weekdays(&self) -> [&'static str; 7] {
        let first = self.locale.first_day as usize;
        std::array::from_fn(|i| self.locale.days[(first + i) % 7])
    }

    /// The name of a month (1 to 12) in the locale.
    pub fn month_name(&self, month: u32) -> &'static str {
        self.locale.months[(month.clamp(1, 12) - 1) as usize]
    }

    /// The abbreviated name of a month (1 to 12), as the year view of a
    /// calendar shows it (C# AbbreviatedMonthNames): `Oct`, `mrt`; a language
    /// without short names (Japanese) gives the whole name.
    pub fn short_month_name(&self, month: u32) -> &'static str {
        let at = (month.clamp(1, 12) - 1) as usize;
        match SHORT_MONTHS.iter().find(|(first, _)| *first == self.locale.months[0]) {
            Some((_, names)) => names[at],
            None => self.locale.months[at],
        }
    }
}

/// A value as its own text: a text as it is, anything else as JSON writes it.
fn as_text(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

fn grouped_digits(whole: &str, group: &str) -> String {
    let mut out = String::new();
    let len = whole.len();
    for (i, c) in whole.chars().enumerate() {
        if i > 0 && (len - i) % 3 == 0 {
            out.push_str(group);
        }
        out.push(c);
    }
    out
}

/// Whole seconds as `m:ss`, or `h:mm:ss` from an hour.
fn duration(seconds: i128) -> String {
    let sign = if seconds < 0 { "-" } else { "" };
    let all = seconds.abs();
    let (h, m, s) = (all / 3600, all % 3600 / 60, all % 60);
    if h > 0 { format!("{sign}{h}:{m:02}:{s:02}") } else { format!("{sign}{m}:{s:02}") }
}

/// Typed text as whole seconds: `3:45`, `1:02:03`, or seconds alone.
fn read_duration(typed: &str) -> Option<i64> {
    let parts: Vec<&str> = typed.trim().split(':').collect();
    if parts.len() > 3 || parts.iter().any(|p| p.is_empty() || !p.bytes().all(|b| b.is_ascii_digit())) {
        return None;
    }
    let n = |p: &str| p.parse::<i64>().ok();
    match parts.as_slice() {
        [s] => n(s),
        [m, s] => (s.len() == 2 && n(s)? <= 59).then(|| Some(n(m)? * 60 + n(s)?)).flatten(),
        [h, m, s] => (s.len() == 2 && n(s)? <= 59 && m.len() == 2 && n(m)? <= 59).then(|| Some(n(h)? * 3600 + n(m)? * 60 + n(s)?)).flatten(),
        _ => None,
    }
}

/// A stored date (`2026-10-17`) or date and time (`2026-10-17T09:30:00`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Moment {
    pub year: i32,
    pub month: u32,
    pub day: u32,
    pub hour: u32,
    pub minute: u32,
    pub second: u32,
}

impl Moment {
    pub fn parse(value: &str) -> Option<Moment> {
        let b = value.as_bytes();
        let digits = |from: usize, len: usize| -> Option<u32> {
            let part = value.get(from..from + len)?;
            part.bytes().all(|c| c.is_ascii_digit()).then(|| part.parse().ok()).flatten()
        };
        if b.len() < 10 || b[4] != b'-' || b[7] != b'-' {
            return None;
        }
        let (year, month, day) = (digits(0, 4)? as i32, digits(5, 2)?, digits(8, 2)?);
        let (mut hour, mut minute, mut second) = (0, 0, 0);
        if b.len() >= 16 && (b[10] == b'T' || b[10] == b' ') && b[13] == b':' {
            hour = digits(11, 2)?;
            minute = digits(14, 2)?;
            if b.len() >= 19 && b[16] == b':' {
                second = digits(17, 2)?;
            }
        }
        if !(1..=12).contains(&month) || day < 1 || day > days_in(year, month)? || hour > 23 || minute > 59 || second > 59 {
            return None;
        }
        Some(Moment { year, month, day, hour, minute, second })
    }
}

/// How many days a month (1 to 12) of a year has; none for no month.
pub fn days_in(year: i32, month: u32) -> Option<u32> {
    let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
    Some(match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return None,
    })
}

/// The short names of the months as a year view shows them, by the
/// language's name of January; a language not here shows the whole name.
const SHORT_MONTHS: [(&str, [&str; 12]); 6] = [
    ("January", ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"]),
    ("januari", ["jan", "feb", "mrt", "apr", "mei", "jun", "jul", "aug", "sep", "okt", "nov", "dec"]),
    ("Januar", ["Jan.", "Feb.", "M\u{e4}rz", "Apr.", "Mai", "Juni", "Juli", "Aug.", "Sept.", "Okt.", "Nov.", "Dez."]),
    ("janvier", ["janv.", "f\u{e9}vr.", "mars", "avr.", "mai", "juin", "juil.", "ao\u{fb}t", "sept.", "oct.", "nov.", "d\u{e9}c."]),
    ("enero", ["ene", "feb", "mar", "abr", "may", "jun", "jul", "ago", "sept", "oct", "nov", "dic"]),
    ("gennaio", ["gen", "feb", "mar", "apr", "mag", "giu", "lug", "ago", "set", "ott", "nov", "dic"]),
];
