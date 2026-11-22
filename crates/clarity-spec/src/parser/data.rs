//! The parser of the data model: enums and their values, records, entities
//! (their versions, saved-since texts and access), the settings, fields,
//! types and modifiers.

use super::*;

// ---------------------------------------------------------------------------
// Enums and entities

pub(crate) fn parse_enum(node: &Node, name: Ident, diagnostics: &mut Vec<Diagnostic>) -> EnumDecl {
    let mut variants = Vec::new();
    for child in &node.children {
        match parse_variant(child) {
            Ok(variant) => variants.push(variant),
            Err(diag) => diagnostics.push(diag),
        }
    }
    if variants.is_empty() && node.children.is_empty() {
        diagnostics.push(Diagnostic::error(
            format!("enum '{}' has no variants", name.name),
            name.span,
        ));
    }
    EnumDecl {
        name,
        variants,
        span: node.span(),
    }
}

pub(crate) fn parse_variant(node: &Node) -> ParseResult<EnumVariant> {
    no_children(node)?;
    let mut cursor = Cursor::new(node.line);
    let name = cursor.expect_ident("a variant name")?;
    let label = if cursor.eat_keyword("label") {
        cursor.expect_text("a label string")?
    } else {
        default_label(&name.name)
    };
    cursor.expect_end()?;
    Ok(EnumVariant { name, label })
}

/// `record Dot` and its fields (manual/code.engines.md): as an entity's, but values
/// only, so a decimal needs no precision and scale.
pub(crate) fn parse_record(node: &Node, name: Ident, diagnostics: &mut Vec<Diagnostic>) -> Entity {
    let mut fields = Vec::new();
    for child in &node.children {
        if let Some(word @ ("readable" | "writable")) = first_word(child)
            && child.line.tokens.get(1).is_some_and(|t| t.text == "when")
        {
            diagnostics.push(Diagnostic::error(format!("a record is never stored, so it has no '{word} when'"), child.line.span));
            continue;
        }
        if let Some(word @ ("version" | "savedSince")) = first_word(child)
            && child.line.tokens.get(1).is_some_and(|t| t.kind != TokenKind::Colon)
        {
            diagnostics.push(Diagnostic::error(format!("a record is never stored, so it has no '{word}'"), child.line.span));
            continue;
        }
        let field = no_children(child).and_then(|_| {
            let mut cursor = Cursor::new(child.line);
            parse_field_typed(&mut cursor, child.line.span, true)
        });
        match field {
            Ok(field) => fields.push(field),
            Err(diag) => diagnostics.push(diag),
        }
    }
    if node.children.is_empty() {
        diagnostics.push(Diagnostic::error(format!("record '{}' has no fields", name.name), name.span));
    }
    Entity { name, fields, table: None, key: None, settings: false, record: true, people: false, readable: None, writable: None, version: None, version_none: false, saved_since: None, kept_version: None, span: node.span() }
}

pub(crate) fn parse_entity(node: &Node, name: Ident, diagnostics: &mut Vec<Diagnostic>) -> Entity {
    let mut fields = Vec::new();
    let (mut table, mut key) = (None, None);
    let (mut readable, mut writable) = (None, None);
    let (mut version, mut version_none, mut saved_since) = (None, false, None);
    for child in &node.children {
        // `readable when <condition>` and `writable when <condition>`: who may read or change a record.
        if let Some(word @ ("readable" | "writable")) = first_word(child)
            && child.line.tokens.get(1).is_some_and(|t| t.text == "when")
        {
            let slot = if word == "readable" { &mut readable } else { &mut writable };
            if let Err(diag) = parse_access(child, word, slot) {
                diagnostics.push(diag);
            }
            continue;
        }
        // `version "RowVersion"`, `version none` and `savedSince @key`:
        // how a save is checked; field lines have a colon after the name.
        if let Some(word @ ("version" | "savedSince")) = first_word(child)
            && child.line.tokens.get(1).is_some_and(|t| t.kind != TokenKind::Colon)
        {
            let result = if word == "version" { parse_version(child, &mut version, &mut version_none) } else { parse_saved_since(child, &name.name, &mut saved_since) };
            if let Err(diag) = result {
                diagnostics.push(diag);
            }
            continue;
        }
        // `table "tblInvoice"` and `key "InvoiceID"` map the entity onto an
        // existing table; field lines have a colon after the name.
        let setting = match first_word(child) {
            Some(word @ ("table" | "key")) if child.line.tokens.get(1).is_some_and(|t| t.kind == TokenKind::Str) => Some(word),
            _ => None,
        };
        if let Some(word) = setting {
            let mut cursor = Cursor::new(child.line);
            cursor.next();
            let result = cursor.expect_string("a name").and_then(|value| {
                cursor.expect_end()?;
                no_children(child)?;
                let slot = if word == "table" { &mut table } else { &mut key };
                if slot.is_some() {
                    return Err(Diagnostic::error(format!("duplicate '{word}'"), child.line.span));
                }
                *slot = Some(value);
                Ok(())
            });
            if let Err(diag) = result {
                diagnostics.push(diag);
            }
            continue;
        }
        match parse_field(child) {
            Ok(field) => fields.push(field),
            Err(diag) => diagnostics.push(diag),
        }
    }
    if node.children.is_empty() {
        diagnostics.push(Diagnostic::error(
            format!("entity '{}' has no fields", name.name),
            name.span,
        ));
    }
    Entity {
        name,
        fields,
        table,
        key,
        settings: false,
        record: false,
        people: false,
        readable,
        writable,
        version,
        version_none,
        saved_since,
        kept_version: None,
        span: node.span(),
    }
}

/// `version "RowVersion"` or `version none` of an entity.
pub(crate) fn parse_version(node: &Node, version: &mut Option<String>, none: &mut bool) -> ParseResult<()> {
    no_children(node)?;
    let mut cursor = Cursor::new(node.line);
    cursor.next();
    let named = if cursor.eat_keyword("none") { None } else { Some(cursor.expect_string("the version column in quotes, as in version \"RowVersion\", or none")?) };
    cursor.expect_end()?;
    if version.is_some() || *none {
        return Err(Diagnostic::error("duplicate 'version'", node.line.span));
    }
    match named {
        Some(column) => *version = Some(column),
        None => *none = true,
    }
    Ok(())
}

/// `savedSince @customerChangedSince` of an entity: a key, as every text people read.
pub(crate) fn parse_saved_since(node: &Node, entity: &str, slot: &mut Option<String>) -> ParseResult<()> {
    no_children(node)?;
    let mut cursor = Cursor::new(node.line);
    cursor.next();
    if cursor.peek_kind() != Some(TokenKind::Key) {
        let key = crate::names::camel_case(&format!("{entity} changed since"));
        return Err(cursor.error_here(format!("the text of savedSince is a key: write savedSince @{key}, with its text in the language files")));
    }
    let key = cursor.expect(TokenKind::Key, "a key")?.text.clone();
    cursor.expect_end()?;
    if slot.is_some() {
        return Err(Diagnostic::error("duplicate 'savedSince'", node.line.span));
    }
    *slot = Some(key);
    Ok(())
}

/// `readable when <condition>` or `writable when <condition>` of an entity.
pub(crate) fn parse_access(node: &Node, word: &str, slot: &mut Option<crate::code::Expr>) -> ParseResult<()> {
    no_children(node)?;
    let mut cursor = Cursor::new(node.line);
    cursor.next();
    cursor.next();
    let condition = crate::code::parse::parse_expr(&mut cursor)?;
    cursor.expect_end()?;
    if slot.is_some() {
        return Err(Diagnostic::error(format!("'{word} when' is written twice; join the conditions with 'or'"), node.line.span));
    }
    *slot = Some(condition);
    Ok(())
}

/// The `settings` block (manual/data.settings.md): fields as an entity's, and Clarity's
/// own `language`, `theme`, `agentReadsWhileClosed` and `agentChangesWhileClosed`
/// without a type; read as the record type `Settings`.
pub(crate) fn parse_settings(node: &Node, diagnostics: &mut Vec<Diagnostic>) -> Entity {
    let mut fields = Vec::new();
    for child in &node.children {
        // Labeled with Clarity's own words, which head the pickers too.
        let own = match child.line.tokens.as_slice() {
            [word] if word.text == "language" => Some((FieldType::Language, "@languagePicker")),
            [word] if word.text == "theme" => Some((FieldType::Theme, "@themePicker")),
            // What AI agents may do while the app is closed (docs/ledger/app-mcp.md).
            [word] if word.text == "agentReadsWhileClosed" => Some((FieldType::Bool, "@agentReadsWhileClosedSetting")),
            [word] if word.text == "agentChangesWhileClosed" => Some((FieldType::Bool, "@agentChangesWhileClosedSetting")),
            _ => None,
        };
        let field = match own {
            Some((ty, label)) => no_children(child).map(|_| {
                let word = &child.line.tokens[0];
                Field {
                    name: Ident { name: word.text.clone(), span: word.span },
                    type_text: if ty == FieldType::Bool { "bool".into() } else { word.text.clone() },
                    ty,
                    modifiers: Modifiers { label: Some(label.to_string()), ..Modifiers::default() },
                    label: label.to_string(),
                    span: child.line.span,
                }
            }),
            None => parse_field(child),
        };
        match field {
            Ok(field) => fields.push(field),
            Err(diag) => diagnostics.push(diag),
        }
    }
    let span = node.span();
    Entity {
        name: Ident { name: SETTINGS_ENTITY.to_string(), span: node.line.tokens.first().map_or(span, |t| t.span) },
        fields,
        table: None,
        key: None,
        settings: true,
        record: false,
        people: false,
        readable: None,
        writable: None,
        version: None,
        version_none: false,
        saved_since: None,
        kept_version: None,
        span,
    }
}

/// Clarity's own settings, written in the settings block as a bare word.
pub const OWN_SETTINGS: [&str; 4] = ["language", "theme", "agentReadsWhileClosed", "agentChangesWhileClosed"];

/// The record type of the project's settings.
pub const SETTINGS_ENTITY: &str = "Settings";

/// Every window's own data with the settings.
pub const SETTINGS_DATA: &str = "settings";

pub(crate) fn parse_field(node: &Node) -> ParseResult<Field> {
    no_children(node)?;
    let mut cursor = Cursor::new(node.line);
    parse_field_at(&mut cursor, node.line.span)
}

/// `name: type, modifiers` from the cursor to the end of the line.
pub(crate) fn parse_field_at(cursor: &mut Cursor, span: Span) -> ParseResult<Field> {
    parse_field_typed(cursor, span, false)
}

/// A field of an entity, or of a record (`value`), whose decimal needs no precision and scale.
pub(crate) fn parse_field_typed(cursor: &mut Cursor, span: Span, value: bool) -> ParseResult<Field> {
    let name = cursor.expect_ident("a field name")?;
    cursor.expect(TokenKind::Colon, "':' after the field name")?;
    let ty = parse_type_of(cursor, value)?;
    let mut modifiers = Modifiers::default();
    while cursor.eat(TokenKind::Comma) {
        parse_modifier(cursor, &mut modifiers)?;
    }
    cursor.expect_end()?;
    let label = modifiers
        .label
        .clone()
        .unwrap_or_else(|| default_label(&name.name));
    Ok(Field {
        name,
        type_text: crate::dictionary::type_text(&ty),
        ty,
        modifiers,
        label,
        span,
    })
}

pub(crate) fn parse_type(cursor: &mut Cursor) -> ParseResult<FieldType> {
    parse_type_of(cursor, false)
}

pub(crate) fn parse_type_of(cursor: &mut Cursor, value: bool) -> ParseResult<FieldType> {
    let ident = cursor.expect_ident("a type")?;
    let ty = match ident.name.as_str() {
        "text" => {
            let max_length = if cursor.eat(TokenKind::LParen) {
                let n = cursor.expect_number("a maximum length")?;
                cursor.expect(TokenKind::RParen, "')'")?;
                Some(n)
            } else {
                None
            };
            FieldType::Text { max_length }
        }
        // A record's decimal is a value, with no column to size.
        "decimal" if value && cursor.peek_kind() != Some(TokenKind::LParen) => FieldType::Decimal { precision: 28, scale: 0 },
        "decimal" => {
            cursor.expect(
                TokenKind::LParen,
                "'(' with precision and scale, as in decimal(10, 2)",
            )?;
            let precision = cursor.expect_number("a precision")?;
            cursor.expect(TokenKind::Comma, "',' and a scale")?;
            let scale = cursor.expect_number("a scale")?;
            cursor.expect(TokenKind::RParen, "')'")?;
            FieldType::Decimal { precision, scale }
        }
        "integer" => FieldType::Integer,
        "money" => FieldType::Money,
        "bool" => FieldType::Bool,
        "date" => FieldType::Date,
        "datetime" => FieldType::DateTime,
        "duration" => FieldType::Duration,
        "image" => FieldType::Image,
        "email" => FieldType::Email,
        "url" => FieldType::Url,
        "color" => FieldType::Color,
        "secret" => FieldType::Secret,
        "folder" => FieldType::Folder,
        "blob" => FieldType::Blob,
        "file" => FieldType::File,
        "id" => FieldType::Id,
        "ref" => FieldType::Ref {
            entity: cursor.expect_ident("an entity name after 'ref'")?,
        },
        "list" => {
            cursor.expect(TokenKind::Lt, "'<' after 'list'")?;
            let item = cursor.expect_ident("an entity name")?;
            cursor.expect(TokenKind::Gt, "'>'")?;
            match crate::model::ListItem::of_word(&item.name) {
                Some(item) => FieldType::Values { item },
                None if VALUE_WORDS.contains(&item.name.as_str()) => {
                    return Err(Diagnostic::error("a list field holds text, integer, decimal, date or an enum", item.span));
                }
                // An entity, or an enum once the project is loaded.
                None => FieldType::List { item },
            }
        }
        _ => FieldType::Enum { name: ident },
    };
    Ok(ty)
}

pub(crate) fn parse_modifier(cursor: &mut Cursor, modifiers: &mut Modifiers) -> ParseResult<()> {
    let keyword = cursor.expect_ident("a modifier")?;
    let duplicate = || {
        Diagnostic::error(
            format!("duplicate modifier '{}'", keyword.name),
            keyword.span,
        )
    };

    fn set<T>(
        slot: &mut Option<T>,
        value: T,
        duplicate: impl Fn() -> Diagnostic,
    ) -> ParseResult<()> {
        if slot.is_some() {
            return Err(duplicate());
        }
        *slot = Some(value);
        Ok(())
    }

    fn flag(slot: &mut bool, duplicate: impl Fn() -> Diagnostic) -> ParseResult<()> {
        if *slot {
            return Err(duplicate());
        }
        *slot = true;
        Ok(())
    }

    match keyword.name.as_str() {
        "required" => flag(&mut modifiers.required, duplicate),
        "unique" => flag(&mut modifiers.unique, duplicate),
        "owned" => flag(&mut modifiers.owned, duplicate),
        "computed" => flag(&mut modifiers.computed, duplicate),
        "search" => flag(&mut modifiers.search, duplicate),
        "cascade" => flag(&mut modifiers.cascade, duplicate),
        "hashed" => flag(&mut modifiers.hashed, duplicate),
        "hidden" => flag(&mut modifiers.hidden, duplicate),
        "min" => set(&mut modifiers.min, parse_literal(cursor)?, duplicate),
        "max" => set(&mut modifiers.max, parse_literal(cursor)?, duplicate),
        "default" => set(&mut modifiers.default, parse_literal(cursor)?, duplicate),
        "pattern" => set(
            &mut modifiers.pattern,
            cursor.expect_string("a pattern string")?,
            duplicate,
        ),
        "label" => set(
            &mut modifiers.label,
            cursor.expect_text("a label string")?,
            duplicate,
        ),
        "help" => set(
            &mut modifiers.help,
            cursor.expect_text("a help string")?,
            duplicate,
        ),
        "format" => set(
            &mut modifiers.format,
            cursor.expect_ident("a format name, such as short or plain")?.name,
            duplicate,
        ),
        "column" => set(
            &mut modifiers.column,
            cursor.expect_string("a column name")?,
            duplicate,
        ),
        "extensions" => {
            let at = cursor.peek().map(|t| t.span).unwrap_or(keyword.span);
            let written = cursor.expect_string("the files it holds, as in 'extensions \".md, .csv\"'")?;
            let found = crate::files::patterns_of(&written);
            if let Some(problem) = crate::files::patterns_problem(&found) {
                return Err(Diagnostic::error(problem, at));
            }
            modifiers.extensions = found;
            Ok(())
        }
        "store" => set(
            &mut modifiers.store,
            cursor.expect_ident("a file store, as in 'store photos'")?.name,
            duplicate,
        ),
        other => Err(Diagnostic::error(
            format!("unknown modifier '{other}'"),
            keyword.span,
        )),
    }
}

pub(crate) fn parse_literal(cursor: &mut Cursor) -> ParseResult<Literal> {
    let Some(token) = cursor.peek() else {
        return Err(cursor.error_here("expected a value"));
    };
    let literal = match token.kind {
        TokenKind::Number => Literal::Number(token.text.clone()),
        TokenKind::Str => Literal::Str(token.text.clone()),
        TokenKind::Key => Literal::Str(format!("@{}", token.text)),
        TokenKind::Ident if token.text == "true" => Literal::Bool(true),
        TokenKind::Ident if token.text == "false" => Literal::Bool(false),
        TokenKind::Ident => Literal::Ident(token.text.clone()),
        _ => return Err(cursor.error_here("expected a value")),
    };
    cursor.next();
    Ok(literal)
}
