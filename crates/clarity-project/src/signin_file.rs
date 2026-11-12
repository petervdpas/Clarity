//! The `signin` block of the project file (docs/APP-CAPABILITIES-PLAN.md
//! step 4): read, checked against Clarity.Auth's catalog and the stack, and
//! written back.

use clarity_spec::lexer::{Token, TokenKind};
use clarity_spec::signin::{self, Signin, SigninStrategy};

use crate::{Stack, Target, target_word};

fn word(token: &Token) -> Option<&str> {
    (token.kind == TokenKind::Ident).then_some(token.text.as_str())
}

fn text(token: &Token) -> Option<String> {
    (token.kind == TokenKind::Str).then(|| token.text.clone())
}

fn number(token: &Token) -> Option<u32> {
    (token.kind == TokenKind::Number).then(|| token.text.parse::<u32>().ok()).flatten().filter(|n| *n > 0)
}

/// `signin Member on desktop web`: the entity and the targets.
pub fn header(line: u32, args: &[&Token]) -> Result<Signin, String> {
    let entity = match args.first() {
        Some(t) if t.kind == TokenKind::Ident && t.text.chars().next().is_some_and(char::is_uppercase) => t.text.clone(),
        _ => return Err(format!("line {line}: write 'signin' and the entity of people, like: signin Member on desktop web")),
    };
    let mut targets = Vec::new();
    match args.get(1).map(|t| word(t)) {
        None => targets.push(target_word(Target::Web).to_string()),
        Some(Some("on")) if args.len() > 2 => {
            for t in &args[2..] {
                let Some(target) = word(t).filter(|w| crate::TARGETS.iter().any(|x| target_word(*x) == *w)) else {
                    return Err(format!("line {line}: people sign in on desktop, web or cli, like: signin {entity} on desktop web"));
                };
                if targets.iter().any(|x| x == target) {
                    return Err(format!("line {line}: {target} is written twice"));
                }
                targets.push(target.to_string());
            }
        }
        _ => return Err(format!("line {line}: write the targets after 'on', like: signin {entity} on desktop web")),
    }
    let mut made = Signin::new(entity, targets);
    made.line = line;
    Ok(made)
}

/// An indented line under `signin`: a strategy and its fields, or an option.
/// `strategy_indent` is the indent of the last strategy line, kept by the reader.
pub fn line(signin: &mut Signin, line: u32, indent: usize, strategy_indent: &mut Option<usize>, keyword: &str, args: &[&Token]) -> Result<(), String> {
    // `minimum 12`, indented under the strategy it belongs to.
    if keyword == "minimum" {
        let Some((last, under)) = signin.strategies.last_mut().zip(*strategy_indent) else {
            return Err(format!("line {line}: write 'minimum' indented under a strategy line, like: minimum 12"));
        };
        if indent <= under {
            return Err(format!("line {line}: write 'minimum' indented under the strategy it belongs to"));
        }
        let fits = signin::strategy_info(&last.name).is_some_and(|info| info.fields.iter().any(|f| f.hash == Some("argon2id")));
        if !fits {
            return Err(format!("line {line}: the {} strategy keeps no secret people choose, so it has no minimum", last.name));
        }
        if last.minimum.is_some() {
            return Err(format!("line {line}: 'minimum' is written twice"));
        }
        let n = match args {
            [n] => number(n),
            _ => None,
        };
        last.minimum = Some(n.ok_or_else(|| format!("line {line}: write the fewest characters a secret may have, like: minimum 12"))?);
        return Ok(());
    }
    // A provider's settings, indented under its line.
    if PROVIDER_SETTINGS.contains(&keyword) && strategy_indent.is_some_and(|i| indent > i) {
        return provider_line(signin, line, keyword, args);
    }
    if strategy_indent.is_some_and(|i| indent > i) {
        return Err(format!("line {line}: unexpected indented line under a strategy; only 'minimum' and a provider's settings go there"));
    }
    match keyword {
        "session" => {
            let minutes = match args {
                [n, unit] => number(n).and_then(|n| match word(unit) {
                    Some("minute" | "minutes") => Some(n),
                    Some("hour" | "hours") => n.checked_mul(60),
                    Some("day" | "days") => n.checked_mul(24 * 60),
                    _ => None,
                }),
                _ => None,
            };
            signin.session_minutes = minutes.ok_or_else(|| format!("line {line}: write how long a sign-in lasts, like: session 8 hours (minutes, hours or days)"))?;
        }
        "tries" => {
            let read = match args {
                [n, per, m, unit] if word(per) == Some("per") && matches!(word(unit), Some("minute" | "minutes")) => number(n).zip(number(m)),
                _ => None,
            };
            let (tries, minutes) = read.ok_or_else(|| format!("line {line}: write how many failed tries in how long, like: tries 5 per 15 minutes"))?;
            signin.tries = tries;
            signin.tries_minutes = minutes;
        }
        "remember" => {
            // `remember 30 days when asked`: the sign-in part asks, with a "keep me signed in" box.
            let (days, asks) = match args {
                [n, unit] if matches!(word(unit), Some("day" | "days")) => (number(n), false),
                [n, unit, when, asked] if matches!(word(unit), Some("day" | "days")) && word(when) == Some("when") && word(asked) == Some("asked") => (number(n), true),
                _ => (None, false),
            };
            signin.remember_days = Some(days.ok_or_else(|| format!("line {line}: write how many days a sign-in is remembered, like: remember 30 days (or remember 30 days when asked)"))?);
            signin.remember_asks = asks;
            if !signin.targets.iter().any(|t| t == "desktop") {
                return Err(format!("line {line}: 'remember' keeps a sign-in on the desktop; add desktop to the signin line"));
            }
        }
        "language" => {
            // `language picker`: the sign-in part lets people pick their language.
            if !matches!(args, [picker] if word(picker) == Some("picker")) {
                return Err(format!("line {line}: write language picker to let people pick their language on the sign-in part"));
            }
            signin.language_picker = true;
        }
        "shows" => {
            let named: Vec<String> = args.iter().filter(|t| t.kind != TokenKind::Comma).filter_map(|t| word(t)).map(str::to_string).collect();
            if named.is_empty() || named.len() != args.iter().filter(|t| t.kind != TokenKind::Comma).count() {
                return Err(format!("line {line}: write the strategies the sign-in part shows, first the default, like: shows password, account"));
            }
            for (i, name) in named.iter().enumerate() {
                if named[..i].contains(name) {
                    return Err(format!("line {line}: {name} is shown twice"));
                }
            }
            // The strategies come first, so what is shown is checked here.
            if named.iter().any(|n| signin.strategy(n).is_none()) {
                let names: Vec<&str> = signin.strategies.iter().map(|s| s.name.as_str()).collect();
                return Err(format!("line {line}: the sign-in part shows only strategies the block names above it; it names {}", names.join(", ")));
            }
            signin.shows = named;
        }
        name => {
            let Some(info) = signin::strategy_info(name) else {
                return Err(format!("line {line}: there is no sign-in strategy '{name}'; there are {}", signin::strategy_names().join(", ")));
            };
            if signin.strategy(name).is_some() {
                return Err(format!("line {line}: the {name} strategy is written twice"));
            }
            let fields: Vec<String> = args.iter().filter_map(|t| word(t)).map(str::to_string).collect();
            if fields.len() != info.fields.len() || fields.len() != args.len() {
                let roles: Vec<&str> = info.fields.iter().map(|f| f.role).collect();
                return Err(format!("line {line}: the {name} strategy names {} field{} of {} ({}), like: {}", info.fields.len(), if info.fields.len() == 1 { "" } else { "s" }, signin.entity, roles.join(", "), example(name)));
            }
            // It applies on the block's targets it works on; on none of them is a mistake.
            let not_here: Vec<&str> = signin.targets.iter().map(String::as_str).filter(|t| !info.targets.contains(t)).collect();
            if not_here.len() == signin.targets.len() {
                return Err(format!("line {line}: the {name} strategy works on {}, not on {}", info.targets.join(", "), not_here.join(", ")));
            }
            signin.strategies.push(SigninStrategy { name: name.to_string(), fields, minimum: None, provider: None });
            *strategy_indent = Some(indent);
        }
    }
    Ok(())
}

/// A strategy's line as an example, from the catalog's roles.
fn example(name: &str) -> String {
    let fields: Vec<&str> = signin::strategy_info(name)
        .map(|info| {
            info.fields
                .iter()
                .map(|f| match (name, f.role) {
                    ("password", "match") => "email",
                    ("password", _) => "passwordHash",
                    ("account", _) => "login",
                    (_, "secret") => "apiToken",
                    _ => "name",
                })
                .collect()
        })
        .unwrap_or_default();
    format!("{name} {}", fields.join(" "))
}

/// What the whole block must hold, once the project file is read.
pub fn finish(signin: &Signin, stack: Option<&Stack>) -> Result<(), String> {
    if signin.strategies.is_empty() {
        return Err(format!("line {}: name at least one strategy under 'signin', indented, like: password email passwordHash", signin.line));
    }
    if let Some(stack) = stack {
        for target in &signin.targets {
            if !stack.targets.iter().any(|t| target_word(*t) == target) {
                return Err(format!("line {}: the stack has no {target} target to sign in on", signin.line));
            }
        }
    }
    Ok(())
}

/// The settings a provider's line may have indented under it.
const PROVIDER_SETTINGS: [&str; 5] = ["client", "secret", "tenant", "issuer", "register"];

/// A provider's setting under its line: `client "..."`, `secret "setting:..."`,
/// `tenant "..."`, `issuer "..."`, `register`.
fn provider_line(signin: &mut Signin, line: u32, keyword: &str, args: &[&Token]) -> Result<(), String> {
    let last = signin.strategies.last_mut().expect("a strategy line comes first");
    let is_provider = signin::strategy_info(&last.name).is_some_and(|info| info.provider);
    if !is_provider {
        return Err(format!("line {line}: the {} strategy is no provider, so it has no {keyword}", last.name));
    }
    let name = last.name.clone();
    let settings = last.provider.get_or_insert_with(Default::default);
    if keyword == "register" {
        if !args.is_empty() {
            return Err(format!("line {line}: write register alone, so a first sign-in makes the person's record"));
        }
        if settings.register {
            return Err(format!("line {line}: 'register' is written twice"));
        }
        settings.register = true;
        return Ok(());
    }
    let value = match args {
        [value] => text(value),
        _ => None,
    }
    .ok_or_else(|| format!("line {line}: write the {keyword} in quotes, like: {keyword} \"...\""))?;
    let slot = match keyword {
        "client" => {
            if !settings.client.is_empty() {
                return Err(format!("line {line}: 'client' is written twice"));
            }
            settings.client = value;
            return Ok(());
        }
        "secret" => {
            if !value.starts_with("setting:") {
                return Err(format!("line {line}: the secret is a secret setting, never the secret itself: secret \"setting:{name}Secret\""));
            }
            &mut settings.secret
        }
        "tenant" if name != "microsoft" => return Err(format!("line {line}: only the microsoft strategy has a tenant")),
        "tenant" => &mut settings.tenant,
        "issuer" if name != "openid" => return Err(format!("line {line}: only the openid strategy has an issuer")),
        _ => &mut settings.issuer,
    };
    if slot.is_some() {
        return Err(format!("line {line}: '{keyword}' is written twice"));
    }
    *slot = Some(value);
    Ok(())
}

/// The block's lines as the writer writes them: (key, indent, text).
pub fn lines(signin: &Signin) -> Vec<(String, usize, String)> {
    let mut header = format!("signin {}", signin.entity);
    if signin.targets != [target_word(Target::Web)] {
        header.push_str(" on ");
        header.push_str(&signin.targets.join(" "));
    }
    let mut lines = vec![("signin".to_string(), 1, header)];
    for s in &signin.strategies {
        lines.push((format!("signin {}", s.name), 2, format!("{} {}", s.name, s.fields.join(" "))));
        if let Some(minimum) = s.minimum {
            lines.push((format!("signin {} minimum", s.name), 3, format!("minimum {minimum}")));
        }
        if let Some(p) = &s.provider {
            let quoted = |word: &str, value: &str| format!("{word} {}", serde_json::Value::from(value));
            if !p.client.is_empty() {
                lines.push((format!("signin {} client", s.name), 3, quoted("client", &p.client)));
            }
            for (word, value) in [("secret", &p.secret), ("tenant", &p.tenant), ("issuer", &p.issuer)] {
                if let Some(value) = value {
                    lines.push((format!("signin {} {word}", s.name), 3, quoted(word, value)));
                }
            }
            if p.register {
                lines.push((format!("signin {} register", s.name), 3, "register".to_string()));
            }
        }
    }
    if signin.session_minutes != signin::DEFAULT_SESSION_MINUTES {
        lines.push(("signin session".into(), 2, format!("session {}", span(signin.session_minutes))));
    }
    if (signin.tries, signin.tries_minutes) != (signin::DEFAULT_TRIES, signin::DEFAULT_TRIES_MINUTES) {
        lines.push(("signin tries".into(), 2, format!("tries {} per {} minutes", signin.tries, signin.tries_minutes)));
    }
    if !signin.shows.is_empty() {
        lines.push(("signin shows".into(), 2, format!("shows {}", signin.shows.join(", "))));
    }
    if signin.language_picker {
        lines.push(("signin language".into(), 2, "language picker".to_string()));
    }
    if let Some(days) = signin.remember_days {
        lines.push(("signin remember".into(), 2, format!("remember {days} days{}", if signin.remember_asks { " when asked" } else { "" })));
    }
    lines
}

/// Minutes in the largest whole unit: `2 days`, `8 hours`, `90 minutes`.
fn span(minutes: u32) -> String {
    let (n, unit) = if minutes % (24 * 60) == 0 { (minutes / (24 * 60), "day") } else if minutes % 60 == 0 { (minutes / 60, "hour") } else { (minutes, "minute") };
    format!("{n} {unit}{}", if n == 1 { "" } else { "s" })
}
