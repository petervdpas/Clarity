//! A schema read into the engine's rules (describe.rs's `Described`): the
//! tables an XSD declares, typed, as Import takes them; the XML Data
//! Binding wizard made generic. Read with the markup reader (markup.rs).

use crate::describe::{Column, Described};
use crate::doc::{local, Node as Element};
use crate::fit::plain;
use crate::markup;
use crate::{Problem, ProblemKind};

fn not(text: impl Into<String>) -> Problem {
    Problem::new(ProblemKind::NotFitting, text)
}

/// The tables an XSD schema declares, typed. What it reads: global and
/// local elements, named and anonymous complex and simple types, sequence,
/// all and choice (nested), element, attribute, group and attributeGroup
/// references, complexContent extension, and simpleContent (as its base).
pub fn schema(xsd: &str) -> Result<Vec<Described>, Problem> {
    let (_, root, _) = markup::parse(xsd).map_err(|why| not(format!("the schema is no XML: {why}")))?;
    if local(&root.name) != "schema" {
        return Err(not(format!("the schema's root is <{}>, not <xs:schema>", root.name)));
    }
    let mut xsd_prefixes = Vec::new();
    for (name, value) in &root.attributes {
        if value == "http://www.w3.org/2001/XMLSchema" {
            xsd_prefixes.push(name.strip_prefix("xmlns:").unwrap_or(if name == "xmlns" { "" } else { name }).to_string());
        }
    }
    let walker = Walker { schema: &root, xsd_prefixes, tables: Vec::new(), stack: Vec::new() };
    walker.run()
}

/// What an element's type is: a value, or records of a complex type.
enum Type<'a> {
    Simple(Column),
    Complex(&'a Element),
}

struct Walker<'a> {
    schema: &'a Element,
    xsd_prefixes: Vec<String>,
    tables: Vec<Described>,
    /// The named types and global elements being described, with their table's path.
    stack: Vec<(String, String)>,
}

fn children<'b>(e: &'b Element, kind: &'b str) -> impl Iterator<Item = &'b Element> + 'b {
    e.nodes().filter(move |c| local(&c.name) == kind)
}

fn occurs(e: &Element, name: &str) -> u64 {
    match e.attribute(name) {
        Some("unbounded") => u64::MAX,
        Some(n) => n.trim().parse().unwrap_or(1),
        None => 1,
    }
}

fn builtin_kind(name: &str) -> &'static str {
    match name {
        "int" | "integer" | "long" | "short" | "byte" | "nonNegativeInteger" | "positiveInteger" | "negativeInteger" | "nonPositiveInteger" | "unsignedInt" | "unsignedLong" | "unsignedShort" | "unsignedByte" => "integer",
        "decimal" | "double" | "float" => "decimal",
        "date" => "date",
        "dateTime" => "datetime",
        "boolean" => "bool",
        _ => "text",
    }
}

impl<'a> Walker<'a> {
    fn run(mut self) -> Result<Vec<Described>, Problem> {
        let globals: Vec<&Element> = children(self.schema, "element").collect();
        let mut referred = Vec::new();
        refs(self.schema, &mut referred);
        // The roots are the global elements no other refers to (all, when every one is).
        let mut roots: Vec<&Element> = globals.iter().copied().filter(|g| !g.attribute("name").is_some_and(|n| referred.iter().any(|r| r == n))).collect();
        if roots.is_empty() {
            roots = globals;
        }
        for root in roots {
            let Some(name) = root.attribute("name") else { continue };
            if let Type::Complex(ctype) = self.type_of(root, 0) {
                // A type that holds itself refers back to this table.
                let keys = [Some(format!("element:{name}")), root.attribute("type").map(|t| format!("type:{}", local(t)))];
                let pushed = keys.iter().flatten().count();
                self.stack.extend(keys.into_iter().flatten().map(|k| (k, name.to_string())));
                self.table(name, ctype);
                self.stack.truncate(self.stack.len() - pushed);
            }
        }
        Ok(self.tables)
    }

    /// A global declaration of `kind` (element, complexType, ...) by its qualified name.
    fn global(&self, kind: &str, qname: &str) -> Option<&'a Element> {
        let name = local(qname);
        self.schema.nodes().find(|c| local(&c.name) == kind && c.attribute("name") == Some(name))
    }

    /// The built-in type a qualified name is, when its prefix is the schema namespace's.
    fn builtin<'q>(&self, qname: &'q str) -> Option<&'q str> {
        let prefix = qname.rsplit_once(':').map_or("", |(p, _)| p);
        self.xsd_prefixes.iter().any(|p| p == prefix).then(|| local(qname))
    }

    /// A simple type named by `qname`, as a value column (without its name).
    fn simple_named(&self, qname: &str, depth: usize) -> Column {
        if let Some(name) = self.builtin(qname) {
            return Column { kind: builtin_kind(name).into(), ..Column::default() };
        }
        match self.global("simpleType", qname) {
            Some(st) if depth < 20 => {
                let mut c = self.simple(st, depth + 1);
                if c.kind == "enum" {
                    c.type_name = Some(local(qname).to_string());
                }
                c
            }
            // A complex type with simple content stands for its base.
            _ => match self.global("complexType", qname) {
                Some(ct) if depth < 20 => self.simple_content(ct, depth + 1).unwrap_or_else(|| Column { kind: "text".into(), ..Column::default() }),
                _ => Column { kind: builtin_kind(local(qname)).into(), ..Column::default() },
            },
        }
    }

    /// An `xs:simpleType`: its base and facets.
    fn simple(&self, st: &Element, depth: usize) -> Column {
        let Some(restriction) = children(st, "restriction").next() else {
            // A list or a union is text.
            return Column { kind: "text".into(), ..Column::default() };
        };
        self.restricted(restriction, depth)
    }

    fn restricted(&self, restriction: &Element, depth: usize) -> Column {
        let mut c = match (restriction.attribute("base"), children(restriction, "simpleType").next()) {
            (Some(base), _) => self.simple_named(base, depth),
            (None, Some(st)) if depth < 20 => self.simple(st, depth + 1),
            _ => Column { kind: "text".into(), ..Column::default() },
        };
        let facet = |name: &str| children(restriction, name).next().and_then(|f| f.attribute("value")).and_then(|v| v.trim().parse::<u32>().ok());
        if let Some(size) = facet("maxLength").or_else(|| facet("length")) {
            c.size = Some(size);
        }
        if c.kind == "decimal" {
            if let Some(p) = facet("totalDigits") {
                c.digits = Some((p, facet("fractionDigits").unwrap_or(0).min(p)));
            }
        }
        let values: Vec<String> = children(restriction, "enumeration").filter_map(|e| e.attribute("value")).map(str::to_string).collect();
        // Only text becomes an enum; numbers keep their kind.
        if !values.is_empty() && matches!(c.kind.as_str(), "text" | "enum") {
            c.kind = "enum".into();
            c.values = values;
            c.size = None;
            c.type_name = None;
        }
        c
    }

    /// The base of a complex type with simple content, as a value; none when it has none.
    fn simple_content(&self, ct: &Element, depth: usize) -> Option<Column> {
        let content = children(ct, "simpleContent").next()?;
        let derived = content.nodes().find(|e| matches!(local(&e.name), "extension" | "restriction"))?;
        Some(if local(&derived.name) == "restriction" { self.restricted(derived, depth) } else { self.simple_named(derived.attribute("base").unwrap_or("xs:string"), depth) })
    }

    /// The type of an element declaration (its `type`, or the type inside it).
    fn type_of(&self, el: &'a Element, depth: usize) -> Type<'a> {
        let text = || Type::Simple(Column { kind: "text".into(), ..Column::default() });
        if let Some(qname) = el.attribute("type") {
            if self.builtin(qname).is_some() || self.global("simpleType", qname).is_some() {
                return Type::Simple(self.simple_named(qname, depth));
            }
            return match self.global("complexType", qname) {
                Some(ct) => match self.simple_content(ct, depth) {
                    Some(c) => Type::Simple(c),
                    None => Type::Complex(ct),
                },
                None => text(),
            };
        }
        if let Some(ct) = children(el, "complexType").next() {
            return match self.simple_content(ct, depth) {
                Some(c) => Type::Simple(c),
                None => Type::Complex(ct),
            };
        }
        match children(el, "simpleType").next() {
            Some(st) => Type::Simple(self.simple(st, depth)),
            None => text(),
        }
    }

    /// Describes a complex type as the table at `path`, then the tables inside it.
    fn table(&mut self, path: &str, ctype: &'a Element) {
        let at = self.tables.len();
        self.tables.push(Described { path: path.to_string(), count: 0, columns: Vec::new() });
        let mut attributes = Vec::new();
        let mut elements = Vec::new();
        self.content(path, ctype, false, &mut attributes, &mut elements, 0);
        let names: Vec<String> = elements.iter().map(|c: &Column| plain(&c.name)).collect();
        for a in attributes.iter_mut() {
            if names.contains(&plain(&a.name)) {
                a.column = format!("@{}", a.name);
            }
        }
        attributes.extend(elements);
        self.tables[at].columns = attributes;
    }

    /// The attributes and elements a complex type (or a part of one) declares.
    fn content(&mut self, path: &str, part: &'a Element, choice: bool, attributes: &mut Vec<Column>, elements: &mut Vec<Column>, depth: usize) {
        if depth > 20 {
            return;
        }
        for child in part.nodes() {
            match local(&child.name) {
                "sequence" | "all" => self.content(path, child, choice, attributes, elements, depth + 1),
                "choice" => self.content(path, child, true, attributes, elements, depth + 1),
                "group" | "attributeGroup" => {
                    let kind = local(&child.name);
                    if let Some(found) = child.attribute("ref").and_then(|r| self.global(kind, r)) {
                        self.content(path, found, choice, attributes, elements, depth + 1);
                    }
                }
                "complexContent" => {
                    for derived in child.nodes() {
                        let is_extension = local(&derived.name) == "extension";
                        if is_extension {
                            if let Some(base) = derived.attribute("base").and_then(|b| self.global("complexType", b)) {
                                self.content(path, base, choice, attributes, elements, depth + 1);
                            }
                        }
                        if is_extension || local(&derived.name) == "restriction" {
                            self.content(path, derived, choice, attributes, elements, depth + 1);
                        }
                    }
                }
                "attribute" => {
                    if let Some(c) = self.attribute(child) {
                        if !attributes.iter().any(|a| a.name == c.name) {
                            attributes.push(c);
                        }
                    }
                }
                "element" => {
                    if let Some(c) = self.element(path, child, choice) {
                        if !elements.iter().any(|e| e.name == c.name) {
                            elements.push(c);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    fn attribute(&self, at: &'a Element) -> Option<Column> {
        let declared = match at.attribute("ref") {
            Some(r) => self.global("attribute", r)?,
            None => at,
        };
        let name = declared.attribute("name")?;
        if at.attribute("use") == Some("prohibited") {
            return None;
        }
        let mut c = match (declared.attribute("type"), children(declared, "simpleType").next()) {
            (Some(t), _) => self.simple_named(t, 0),
            (None, Some(st)) => self.simple(st, 0),
            _ => Column { kind: "text".into(), ..Column::default() },
        };
        c.name = name.to_string();
        c.column = name.to_string();
        c.required = at.attribute("use") == Some("required");
        Some(c)
    }

    /// An element inside a complex type: a value column, a record column
    /// with its own table, or a list. An element holding only one repeated
    /// element (`<lines><line/><line/></lines>`) is the list of those.
    fn element(&mut self, path: &str, at: &'a Element, choice: bool) -> Option<Column> {
        let declared = match at.attribute("ref") {
            Some(r) => self.global("element", r)?,
            None => at,
        };
        let name = declared.attribute("name")?.to_string();
        let many = occurs(at, "maxOccurs") > 1;
        let required = !choice && !many && occurs(at, "minOccurs") >= 1;
        let inner = format!("{path}/{name}");
        let key = match (at.attribute("ref"), declared.attribute("type")) {
            (Some(r), _) => Some(format!("element:{}", local(r))),
            (None, Some(t)) => Some(format!("type:{}", local(t))),
            _ => None,
        };
        let mut c = match self.type_of(declared, 0) {
            Type::Simple(mut c) => {
                c.list = many;
                c
            }
            Type::Complex(ctype) => {
                if let Some((item, item_many)) = self.only_item(ctype) {
                    if item_many {
                        let mut c = self.element(&inner, item, false)?;
                        c.list = true;
                        c.required = false;
                        c.name = name.clone();
                        c.column = name.clone();
                        return Some(c);
                    }
                }
                let table = self.nested(&inner, key, ctype);
                Column { kind: "record".into(), table: Some(table), list: many, ..Column::default() }
            }
        };
        c.name = name.clone();
        c.column = name;
        c.required = required;
        Some(c)
    }

    /// The one element a complex type holds, with whether it repeats; none
    /// when it holds attributes or more than one element.
    fn only_item(&self, ctype: &'a Element) -> Option<(&'a Element, bool)> {
        if children(ctype, "attribute").next().is_some() || children(ctype, "attributeGroup").next().is_some() {
            return None;
        }
        let groups: Vec<&Element> = ctype.nodes().filter(|e| matches!(local(&e.name), "sequence" | "all" | "choice")).collect();
        let [group] = groups.as_slice() else { return None };
        if ctype.nodes().count() != 1 {
            return None;
        }
        let items: Vec<&Element> = group.nodes().collect();
        let [item] = items.as_slice() else { return None };
        if local(&item.name) != "element" {
            return None;
        }
        Some((item, occurs(item, "maxOccurs") > 1 || occurs(group, "maxOccurs") > 1))
    }

    /// The table of a complex type inside another, at `path`; a type already
    /// being described (a recursive one) is the table it has there.
    fn nested(&mut self, path: &str, key: Option<String>, ctype: &'a Element) -> String {
        if let Some(key) = &key {
            if let Some((_, there)) = self.stack.iter().find(|(k, _)| k == key) {
                return there.clone();
            }
        }
        if let Some(key) = key.clone() {
            self.stack.push((key, path.to_string()));
        }
        self.table(path, ctype);
        if key.is_some() {
            self.stack.pop();
        }
        path.to_string()
    }
}

/// Every element and group a schema refers to by `ref`.
fn refs(e: &Element, out: &mut Vec<String>) {
    for child in e.nodes() {
        if local(&child.name) == "element" {
            if let Some(r) = child.attribute("ref") {
                out.push(local(r).to_string());
            }
        }
        refs(child, out);
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::describe::to_json;

    const ORDERS_XSD: &str = r#"<?xml version="1.0"?>
<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
  <xs:element name="shop">
    <xs:complexType><xs:sequence>
      <xs:element name="orders"><xs:complexType><xs:sequence>
        <xs:element name="order" type="OrderType" maxOccurs="unbounded"/>
      </xs:sequence></xs:complexType></xs:element>
    </xs:sequence></xs:complexType>
  </xs:element>
  <xs:simpleType name="Status"><xs:restriction base="xs:string">
    <xs:enumeration value="open"/><xs:enumeration value="shipped"/>
  </xs:restriction></xs:simpleType>
  <xs:complexType name="OrderType">
    <xs:sequence>
      <xs:element name="placed" type="xs:date"/>
      <xs:element name="total" minOccurs="0"><xs:simpleType><xs:restriction base="xs:decimal"><xs:totalDigits value="10"/><xs:fractionDigits value="2"/></xs:restriction></xs:simpleType></xs:element>
      <xs:element name="note" minOccurs="0"><xs:simpleType><xs:restriction base="xs:string"><xs:maxLength value="200"/></xs:restriction></xs:simpleType></xs:element>
      <xs:element name="customer"><xs:complexType><xs:sequence><xs:element name="name" type="xs:string"/></xs:sequence></xs:complexType></xs:element>
      <xs:element name="lines"><xs:complexType><xs:sequence>
        <xs:element name="line" maxOccurs="unbounded"><xs:complexType>
          <xs:attribute name="product" type="xs:string" use="required"/>
          <xs:attribute name="quantity" type="xs:int"/>
        </xs:complexType></xs:element>
      </xs:sequence></xs:complexType></xs:element>
      <xs:element name="tag" type="xs:string" minOccurs="0" maxOccurs="unbounded"/>
      <xs:choice><xs:element name="paid" type="xs:boolean"/><xs:element name="due" type="xs:dateTime"/></xs:choice>
    </xs:sequence>
    <xs:attribute name="number" type="xs:string" use="required"/>
    <xs:attribute name="status" type="Status"/>
  </xs:complexType>
</xs:schema>"#;

    #[test]
    fn an_xsd_gives_typed_tables() {
        let t = schema(ORDERS_XSD).unwrap();
        let paths: Vec<&str> = t.iter().map(|t| t.path.as_str()).collect();
        assert_eq!(paths, ["shop", "shop/orders/order", "shop/orders/order/customer", "shop/orders/order/lines/line"]);
        assert_eq!(
            to_json(&t[1..2]),
            concat!(
                r#"[{"path":"shop/orders/order","count":0,"columns":["#,
                r#"{"name":"number","column":"number","kind":"text","required":true},"#,
                r#"{"name":"status","column":"status","kind":"enum","values":["open","shipped"],"type":"Status"},"#,
                r#"{"name":"placed","column":"placed","kind":"date","required":true},"#,
                r#"{"name":"total","column":"total","kind":"decimal","precision":10,"scale":2},"#,
                r#"{"name":"note","column":"note","kind":"text","size":200},"#,
                r#"{"name":"customer","column":"customer","kind":"record","required":true,"table":"shop/orders/order/customer"},"#,
                r#"{"name":"lines","column":"lines","kind":"record","list":true,"table":"shop/orders/order/lines/line"},"#,
                r#"{"name":"tag","column":"tag","kind":"text","list":true},"#,
                r#"{"name":"paid","column":"paid","kind":"bool"},"#,
                r#"{"name":"due","column":"due","kind":"datetime"}]}]"#
            )
        );
        assert_eq!(t[0].columns[0].table.as_deref(), Some("shop/orders/order"));
        assert!(t[0].columns[0].list);
        let line = &t[3].columns;
        assert_eq!((line[0].kind.as_str(), line[0].required, line[1].kind.as_str()), ("text", true, "integer"));
    }

    #[test]
    fn an_xsd_reads_refs_extensions_and_recursion() {
        let xsd = r#"<xsd:schema xmlns:xsd="http://www.w3.org/2001/XMLSchema">
          <xsd:element name="folder" type="Folder"/>
          <xsd:element name="title" type="xsd:string"/>
          <xsd:complexType name="Named"><xsd:attribute name="name" type="xsd:string"/></xsd:complexType>
          <xsd:complexType name="Folder"><xsd:complexContent><xsd:extension base="Named"><xsd:sequence>
            <xsd:element ref="title"/>
            <xsd:element name="folder" type="Folder" minOccurs="0" maxOccurs="unbounded"/>
            <xsd:element name="name" type="xsd:string"/>
          </xsd:sequence></xsd:extension></xsd:complexContent></xsd:complexType>
        </xsd:schema>"#;
        let t = schema(xsd).unwrap();
        assert_eq!(t.len(), 1, "{t:?}");
        let cols: Vec<(&str, &str, &str)> = t[0].columns.iter().map(|c| (c.name.as_str(), c.column.as_str(), c.kind.as_str())).collect();
        assert_eq!(cols, [("name", "@name", "text"), ("title", "title", "text"), ("folder", "folder", "record"), ("name", "name", "text")]);
        assert_eq!(t[0].columns[2].table.as_deref(), Some("folder"));
    }

    #[test]
    fn what_is_no_schema_says_why() {
        assert_eq!(schema("<a/>").unwrap_err().text, "the schema's root is <a>, not <xs:schema>");
        assert!(schema("<a").is_err());
    }
}
