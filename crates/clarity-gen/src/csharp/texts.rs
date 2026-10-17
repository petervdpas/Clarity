//! The texts of the application as .NET keeps them (DESIGN 1.5, I1c): a
//! `.resx` file for the texts as written and one per language, and a typed
//! accessor class `Texts` with a property per text, as Visual Studio makes
//! them. Every text is found by what it says as written, through an explicit
//! map from that text to its resource name, so a message made while the
//! application runs is translated the same way as a label.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use clarity_spec::Project;
use clarity_spec::translations::{Used, texts};

use super::HEADER;
use super::names::{pascal, string};

/// Members of `Texts` itself, which a text's property cannot be called.
const MEMBERS: [&str; 10] =
    ["Texts", "Resources", "Names", "BaseLanguage", "Languages", "In", "Equals", "GetHashCode", "ToString", "ReferenceEquals"];

/// The longest resource name made from a text.
const LONGEST: usize = 40;

/// The texts of a project with their resource names.
pub struct Texts {
    used: Vec<Used>,
    /// Each text as written, with its resource name.
    names: BTreeMap<String, String>,
}

impl Texts {
    pub fn of(project: &Project) -> Self {
        let used = texts(project);
        let mut names = BTreeMap::new();
        // .NET finds a resource by its name ignoring case.
        let mut taken: Vec<String> = MEMBERS.iter().map(|m| m.to_lowercase()).collect();
        for u in &used {
            let mut name: String = pascal(&u.text).chars().take(LONGEST).collect();
            if taken.contains(&name.to_lowercase()) {
                let mut n = 2;
                while taken.contains(&format!("{name}{n}").to_lowercase()) {
                    n += 1;
                }
                name = format!("{name}{n}");
            }
            taken.push(name.to_lowercase());
            names.insert(u.text.clone(), name);
        }
        Texts { used, names }
    }

    /// The property that gives `text` in the language shown: `Texts.Customers`.
    pub fn property(&self, text: &str) -> Option<String> {
        self.names.get(text).map(|name| format!("Texts.{name}"))
    }

    /// `text` as an AXAML attribute value that follows the language: the
    /// accessor's property, or the text itself when the project does not
    /// collect it.
    pub fn attr(&self, text: &str) -> String {
        match self.names.get(text) {
            Some(name) => format!("{{x:Static local:Texts.{name}}}"),
            None => plain_attr(text),
        }
    }

    /// `text` as a C# expression that follows the language.
    pub fn code(&self, text: &str) -> String {
        self.property(text).unwrap_or_else(|| string(text))
    }

    /// The accessor class.
    pub fn class(&self, project: &Project, namespace: &str) -> String {
        let base = project.locale.language.as_deref().map_or_else(|| "null".to_string(), string);
        let languages = project.translations.keys().map(|l| string(l)).collect::<Vec<_>>().join(", ");
        let mut names = String::new();
        let mut properties = String::new();
        for u in &self.used {
            let name = &self.names[&u.text];
            let _ = writeln!(names, "        [{}] = {},", string(&u.text), string(name));
            let _ = write!(
                properties,
                "\n    /// <summary>{}</summary>\n    public static string {name} => Locale.Translate({});\n",
                xml(&u.text.replace('\n', " ")),
                string(&u.text)
            );
        }
        format!(
            "{HEADER}
using System.Globalization;
using System.Resources;
using Clarity.Avalonia;

namespace {namespace};

/// <summary>
/// Every text the application shows, in the language shown: the texts as
/// written are in Texts.resx, each language's in Texts.&lt;language&gt;.resx.
/// </summary>
public static class Texts
{{
    static readonly ResourceManager Resources = new({resources}, typeof(Texts).Assembly);

    /// <summary>The language the texts are written in.</summary>
    public const string? BaseLanguage = {base};

    /// <summary>The languages the texts are translated to.</summary>
    public static readonly string[] Languages = [{languages}];

    /// <summary>The resource name of each text, by the text as written.</summary>
    static readonly Dictionary<string, string> Names = new(StringComparer.Ordinal)
    {{
{names}    }};

    /// <summary><paramref name=\"text\"/> in <paramref name=\"language\"/>, or null when it is not a text of the application.</summary>
    public static string? In(string text, CultureInfo language) =>
        Names.TryGetValue(text, out var name) ? Resources.GetString(name, language) : null;
{properties}}}
",
            resources = string(&format!("{namespace}.Texts"))
        )
    }

    /// `Texts.resx`, the texts as written, with where each is used.
    pub fn base_resx(&self) -> String {
        resx(self.used.iter().map(|u| (self.names[&u.text].as_str(), u.text.as_str(), Some(u.places.join(", ")))))
    }

    /// `Texts.<language>.resx`: the translations of one language. A text
    /// without one is left out, so it shows as written.
    pub fn language_resx(&self, translations: &BTreeMap<String, String>) -> String {
        resx(self.used.iter().filter_map(|u| {
            let translation = translations.get(&u.text).filter(|t| !t.is_empty())?;
            Some((self.names[&u.text].as_str(), translation.as_str(), None))
        }))
    }
}

/// Text in XML.
fn xml(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// Text as an AXAML attribute value: escaped, and a leading brace kept as text.
pub fn plain_attr(text: &str) -> String {
    let escaped = xml(text);
    if escaped.starts_with('{') { format!("{{}}{escaped}") } else { escaped }
}

fn resx<'a>(entries: impl Iterator<Item = (&'a str, &'a str, Option<String>)>) -> String {
    let mut data = String::new();
    for (name, value, comment) in entries {
        let _ = write!(data, "  <data name=\"{}\" xml:space=\"preserve\">\n    <value>{}</value>\n", xml(name), xml(value));
        if let Some(comment) = comment {
            let _ = writeln!(data, "    <comment>{}</comment>", xml(&comment));
        }
        data.push_str("  </data>\n");
    }
    format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>
<!-- Generated by Clarity. Do not edit: change the texts and translations in Clarity instead. -->
<root>
  <resheader name=\"resmimetype\">
    <value>text/microsoft-resx</value>
  </resheader>
  <resheader name=\"version\">
    <value>2.0</value>
  </resheader>
  <resheader name=\"reader\">
    <value>System.Resources.ResXResourceReader, System.Windows.Forms, Version=4.0.0.0, Culture=neutral, PublicKeyToken=b77a5c561934e089</value>
  </resheader>
  <resheader name=\"writer\">
    <value>System.Resources.ResXResourceWriter, System.Windows.Forms, Version=4.0.0.0, Culture=neutral, PublicKeyToken=b77a5c561934e089</value>
  </resheader>
{data}</root>
"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// .NET finds resources by name ignoring case, so "Email" and "E-mail" need two names.
    #[test]
    fn names_differ_ignoring_case() {
        let project = clarity_spec::load_sources([(
            "a.clarity",
            "module a\n\nentity Friend\n    email: email, label \"Email\"\n    mail: email, label \"E-mail\"\n",
        )]);
        let texts = Texts::of(&project);
        let (a, b) = (texts.property("Email").unwrap(), texts.property("E-mail").unwrap());
        assert_ne!(a.to_lowercase(), b.to_lowercase(), "{a} {b}");
    }
}
