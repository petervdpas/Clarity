//! The texts of the application as .NET keeps them (DESIGN 1.5, I1c): a
//! `.resx` file for the default language and one per other language, and a
//! typed accessor class `Texts` with a property per key, as Visual Studio
//! makes them. A key is the resource's name, so a text made while the
//! application runs (a message by its key) is found the same way as a label;
//! a language that leaves a key empty falls back to the default's value.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use clarity_spec::Project;
use clarity_spec::translations::{KeyUse, default_language, key_of, keys};

use super::HEADER;
use super::names::{pascal, string};

/// Members of `Texts` itself, which a key's property cannot be called.
const MEMBERS: [&str; 10] =
    ["Texts", "Resources", "Names", "BaseLanguage", "Languages", "In", "Equals", "GetHashCode", "ToString", "ReferenceEquals"];

/// The keys of a project with their property names.
pub struct Texts {
    used: Vec<KeyUse>,
    /// Each key with its property's name.
    names: BTreeMap<String, String>,
}

impl Texts {
    pub fn of(project: &Project) -> Self {
        let used = keys(project);
        let mut names = BTreeMap::new();
        // .NET finds a resource by its name ignoring case.
        let mut taken: Vec<String> = MEMBERS.iter().map(|m| m.to_lowercase()).collect();
        for u in &used {
            let mut name = pascal(&u.key);
            if taken.contains(&name.to_lowercase()) {
                let mut n = 2;
                while taken.contains(&format!("{name}{n}").to_lowercase()) {
                    n += 1;
                }
                name = format!("{name}{n}");
            }
            taken.push(name.to_lowercase());
            names.insert(u.key.clone(), name);
        }
        Texts { used, names }
    }

    /// The property that gives a text in the language shown, `Texts.SaveCaption`,
    /// when it is a key (`@saveCaption`).
    pub fn property(&self, text: &str) -> Option<String> {
        key_of(text).and_then(|key| self.names.get(key)).map(|name| format!("Texts.{name}"))
    }

    /// A text as an AXAML attribute value that follows the language: the
    /// accessor's property for a key, or the text itself.
    pub fn attr(&self, text: &str) -> String {
        match key_of(text).and_then(|key| self.names.get(key)) {
            Some(name) => format!("{{x:Static local:Texts.{name}}}"),
            None => plain_attr(text),
        }
    }

    /// A text as a C# expression that follows the language.
    pub fn code(&self, text: &str) -> String {
        self.property(text).unwrap_or_else(|| string(text))
    }

    /// What a part's state keeps of a text: its key, or the text itself.
    pub fn kept(text: &str) -> String {
        string(key_of(text).unwrap_or(text))
    }

    /// The accessor class.
    pub fn class(&self, project: &Project, namespace: &str) -> String {
        let base = string(&default_language(project));
        let languages = project.translations.keys().filter(|l| **l != default_language(project)).map(|l| string(l)).collect::<Vec<_>>().join(", ");
        let defaults = project.translations.get(&default_language(project));
        let mut names = String::new();
        let mut properties = String::new();
        for u in &self.used {
            let name = &self.names[&u.key];
            let _ = writeln!(names, "        [{}] = {},", string(&u.key), string(name));
            let value = defaults.and_then(|d| d.get(&u.key)).cloned().unwrap_or_default();
            let _ = write!(
                properties,
                "\n    /// <summary>{}</summary>\n    public static string {name} => Locale.Translate({});\n",
                xml(&value.replace('\n', " ")),
                string(&u.key)
            );
        }
        format!(
            "{HEADER}
using System.Globalization;
using System.Resources;
using Clarity.Avalonia;

namespace {namespace};

/// <summary>
/// Every text the application shows, by its key, in the language shown: the
/// default language's in Texts.resx, each other language's in
/// Texts.&lt;language&gt;.resx.
/// </summary>
public static class Texts
{{
    static readonly ResourceManager Resources = new({resources}, typeof(Texts).Assembly);

    /// <summary>The default language: the one the project is written in.</summary>
    public const string? BaseLanguage = {base};

    /// <summary>The other languages the texts are translated to.</summary>
    public static readonly string[] Languages = [{languages}];

    /// <summary>The resource name of each key.</summary>
    static readonly Dictionary<string, string> Names = new(StringComparer.Ordinal)
    {{
{names}    }};

    /// <summary>The value of <paramref name=\"key\"/> in <paramref name=\"language\"/>, or null when it is no key of the application.</summary>
    public static string? In(string key, CultureInfo language) =>
        Names.TryGetValue(key, out var name) ? Resources.GetString(name, language) : null;
{properties}}}
",
            resources = string(&format!("{namespace}.Texts"))
        )
    }

    /// `Texts.resx`: the default language's value of every key, with where each is used.
    pub fn base_resx(&self, project: &Project) -> String {
        let defaults = project.translations.get(&default_language(project)).cloned().unwrap_or_default();
        resx(self.used.iter().map(|u| (self.names[&u.key].as_str(), defaults.get(&u.key).cloned().unwrap_or_default(), Some(u.places.join(", ")))))
    }

    /// `Texts.<language>.resx`: another language's values. A key it leaves
    /// empty is left out, so it shows the default's.
    pub fn language_resx(&self, values: &BTreeMap<String, String>) -> String {
        resx(self.used.iter().filter_map(|u| {
            let value = values.get(&u.key).filter(|t| !t.is_empty())?;
            Some((self.names[&u.key].as_str(), value.clone(), None))
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

fn resx<'a>(entries: impl Iterator<Item = (&'a str, String, Option<String>)>) -> String {
    let mut data = String::new();
    for (name, value, comment) in entries {
        let _ = write!(data, "  <data name=\"{}\" xml:space=\"preserve\">\n    <value>{}</value>\n", xml(name), xml(&value));
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

    /// .NET finds resources by name ignoring case, so keys that differ only in case need two names.
    #[test]
    fn names_differ_ignoring_case() {
        let project = clarity_spec::load_sources([(
            "a.clarity",
            "module a\n\nentity Friend\n    email: email, label @eMail\n    mail: email, label @email\n",
        )]);
        let texts = Texts::of(&project);
        let (a, b) = (texts.property("@eMail").unwrap(), texts.property("@email").unwrap());
        assert_ne!(a.to_lowercase(), b.to_lowercase(), "{a} {b}");
    }
}
