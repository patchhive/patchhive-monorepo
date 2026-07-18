pub(super) fn is_generated_source(path: &str, content: &str) -> bool {
    let normalized = path.replace('\\', "/").to_ascii_lowercase();
    let filename = normalized.rsplit('/').next().unwrap_or(&normalized);
    if filename.contains(".gen.")
        || filename.contains(".generated.")
        || filename.ends_with("_generated.rs")
        || filename.ends_with("_generated.py")
    {
        return true;
    }

    let header = content
        .lines()
        .take(12)
        .collect::<Vec<_>>()
        .join("\n")
        .to_ascii_lowercase();
    header.contains("@generated")
        || header.contains("automatically generated")
        || (header.contains("generated") && header.contains("do not edit"))
}

pub(super) fn is_embedded_stylesheet_wrapper(
    language: &str,
    name: &str,
    lines: &[&str],
    start: usize,
    end: usize,
) -> bool {
    matches!(language, "javascript" | "typescript")
        && (name.ends_with("Style") || name.ends_with("Styles"))
        && lines[start..=end]
            .iter()
            .any(|line| line.contains("<style"))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum FunctionShape {
    ComplexLogic,
    DeclarativeJsx,
    EmbeddedStylesheet,
    LookupTable,
    SqlSchema,
}

impl FunctionShape {
    pub(super) fn score_adjustment(self) -> i32 {
        match self {
            Self::ComplexLogic => 4,
            Self::DeclarativeJsx => -8,
            Self::EmbeddedStylesheet => -12,
            Self::LookupTable => -7,
            Self::SqlSchema => -10,
        }
    }
}

pub(super) fn classify_function_shape(
    language: &str,
    name: &str,
    lines: &[&str],
    start: usize,
    end: usize,
) -> FunctionShape {
    if is_embedded_stylesheet_wrapper(language, name, lines, start, end) {
        return FunctionShape::EmbeddedStylesheet;
    }

    let body = &lines[start..=end];
    let normalized = body.join("\n").to_ascii_lowercase();
    if looks_like_sql_schema(name, &normalized) {
        return FunctionShape::SqlSchema;
    }
    if looks_like_declarative_jsx(language, body) {
        return FunctionShape::DeclarativeJsx;
    }
    if looks_like_lookup_table(body) {
        return FunctionShape::LookupTable;
    }

    FunctionShape::ComplexLogic
}

pub(super) fn control_flow_markers(lines: &[&str]) -> usize {
    lines
        .iter()
        .filter(|line| {
            let normalized = format!(" {} ", code_outside_strings(line).to_ascii_lowercase());
            [
                " if ",
                " else ",
                " for ",
                " while ",
                " match ",
                " switch ",
                " catch ",
                " return ",
                " break ",
                " continue ",
            ]
            .iter()
            .any(|marker| normalized.contains(marker))
        })
        .count()
}

pub(super) fn is_style_literal_usage(content: &str, offset: usize) -> bool {
    let context = source_line_at(content, offset).to_ascii_lowercase();
    context.contains("classname")
        || context.contains("classlist")
        || context.contains("class:")
        || context.contains("style=")
        || context.contains("styles.")
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum LiteralContext {
    Contract,
    General,
    UiCopy,
}

pub(super) fn classify_literal_context(
    literal: &str,
    content: &str,
    offsets: &[usize],
) -> LiteralContext {
    if looks_like_contract_literal(literal, content, offsets) {
        return LiteralContext::Contract;
    }

    let ui_hits = offsets
        .iter()
        .filter(|offset| looks_like_ui_copy_usage(source_line_at(content, **offset)))
        .count();
    if ui_hits * 2 >= offsets.len() {
        LiteralContext::UiCopy
    } else {
        LiteralContext::General
    }
}

pub(super) fn coherent_contract_usage(
    literal: &str,
    content: &str,
    offsets: &[usize],
) -> Option<&'static str> {
    let mut expected = None;
    for offset in offsets {
        let role = contract_usage_role(literal, content, *offset)?;
        if expected.is_some_and(|current| current != role) {
            return None;
        }
        expected = Some(role);
    }
    expected
}

pub(super) fn is_non_actionable_style_literal(literal: &str) -> bool {
    is_css_custom_property_literal(literal)
        || is_css_function_value(literal)
        || looks_like_css_class_literal(literal)
}

fn looks_like_sql_schema(name: &str, normalized: &str) -> bool {
    let schema_name = {
        let name = name.to_ascii_lowercase();
        name.contains("schema")
            || name.contains("migration")
            || name == "init_db"
            || name == "init_database"
    };
    let schema_statements = [
        "create table",
        "create index",
        "alter table",
        "drop table",
        "pragma ",
    ]
    .iter()
    .filter(|marker| normalized.contains(**marker))
    .count();

    (schema_name && schema_statements >= 1)
        || schema_statements >= 2
        || (normalized.contains("execute_batch") && schema_statements >= 1)
}

fn looks_like_declarative_jsx(language: &str, lines: &[&str]) -> bool {
    if !matches!(language, "javascript" | "typescript") {
        return false;
    }

    let jsx_lines = lines
        .iter()
        .filter(|line| {
            let trimmed = line.trim();
            trimmed.starts_with('<')
                || trimmed.starts_with("</")
                || trimmed.contains("className=")
                || trimmed.contains("return (")
        })
        .count();
    jsx_lines >= 10 && jsx_lines * 4 >= lines.len()
}

fn looks_like_lookup_table(lines: &[&str]) -> bool {
    let match_arms = lines.iter().filter(|line| line.contains("=>")).count();
    let struct_rows = lines
        .iter()
        .filter(|line| looks_like_struct_literal_row(line))
        .count();
    let data_rows = lines
        .iter()
        .filter(|line| {
            let trimmed = line.trim();
            (trimmed.starts_with('(') || trimmed.starts_with('[') || trimmed.starts_with('{'))
                && trimmed.ends_with(',')
        })
        .count();
    let controls = control_flow_markers(lines);

    (match_arms >= 10 && controls <= match_arms / 2)
        || (struct_rows >= 10 && controls <= 5)
        || (data_rows >= 14 && controls <= 4)
}

fn looks_like_struct_literal_row(line: &str) -> bool {
    let trimmed = line.trim();
    let Some(prefix) = trimmed.strip_suffix('{') else {
        return false;
    };
    let type_name = prefix
        .trim()
        .rsplit(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_')
        .next()
        .unwrap_or("");
    type_name
        .chars()
        .next()
        .is_some_and(|ch| ch.is_ascii_uppercase())
}

fn looks_like_contract_literal(literal: &str, content: &str, offsets: &[usize]) -> bool {
    let trimmed = literal.trim();
    let literal_shape = trimmed.starts_with("application/")
        || trimmed.starts_with("text/")
        || trimmed.starts_with("/api/")
        || trimmed.starts_with("X-")
        || is_environment_name(trimmed)
        || is_machine_identifier(trimmed);
    if literal_shape {
        return true;
    }

    let contract_hits = offsets
        .iter()
        .filter(|offset| {
            let line = source_line_at(content, **offset).to_ascii_lowercase();
            [
                "header",
                "content-type",
                "env::var",
                "std::env",
                "statuscode",
                "error_code",
                "route(",
                ".route",
            ]
            .iter()
            .any(|marker| line.contains(marker))
        })
        .count();
    contract_hits * 2 >= offsets.len()
}

fn contract_usage_role(literal: &str, content: &str, offset: usize) -> Option<&'static str> {
    let line = source_line_at(content, offset);
    let normalized = line.to_ascii_lowercase();
    let nearby = source_context_at(content, offset, 2).to_ascii_lowercase();

    if normalized.contains("key:") || normalized.contains("label:") {
        return None;
    }
    if (literal.starts_with("application/") || literal.starts_with("text/"))
        && (nearby.contains("content-type") || nearby.contains("headers"))
    {
        return Some("HTTP header value");
    }
    if normalized.contains(&format!("[\"{literal}\"]"))
        || normalized.contains(&format!("['{literal}']"))
    {
        return Some("structured-data field");
    }
    if normalized.contains("=>") || normalized.contains("matches!") {
        return Some("match discriminant");
    }
    if nearby.contains("api_error")
        || nearby.contains("error_code")
        || normalized.contains("\"type\":")
    {
        return Some("machine-readable error code");
    }
    if literal.starts_with("/api/") && (nearby.contains(".route") || nearby.contains("route(")) {
        return Some("API route");
    }
    if is_environment_name(literal)
        && (nearby.contains("env::var")
            || nearby.contains("std::env")
            || nearby.contains("clean_env")
            || nearby.contains("env("))
    {
        return Some("environment lookup");
    }
    None
}

fn is_environment_name(literal: &str) -> bool {
    literal.len() >= 8
        && literal.contains('_')
        && literal
            .chars()
            .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == '_')
}

fn is_machine_identifier(literal: &str) -> bool {
    literal.len() >= 12
        && literal.contains('_')
        && !literal.contains(' ')
        && literal
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_')
}

fn looks_like_ui_copy_usage(line: &str) -> bool {
    let normalized = line.to_ascii_lowercase();
    [
        "label:",
        "title:",
        "placeholder",
        "<button",
        "<chip",
        "toast",
        "helpertext",
        "emptycopy",
        "description:",
    ]
    .iter()
    .any(|marker| normalized.contains(marker))
}

fn source_line_at(content: &str, offset: usize) -> &str {
    let start = content[..offset]
        .rfind('\n')
        .map(|index| index + 1)
        .unwrap_or(0);
    let end = content[offset..]
        .find('\n')
        .map(|index| offset + index)
        .unwrap_or(content.len());
    &content[start..end]
}

fn source_context_at(content: &str, offset: usize, radius: usize) -> &str {
    let mut start = offset.min(content.len());
    for _ in 0..radius {
        start = content[..start].rfind('\n').unwrap_or(0);
    }

    let mut end = offset.min(content.len());
    for _ in 0..=radius {
        end = content[end..]
            .find('\n')
            .map(|index| end + index + 1)
            .unwrap_or(content.len());
    }
    &content[start..end]
}

fn code_outside_strings(line: &str) -> String {
    let mut result = String::with_capacity(line.len());
    let mut delimiter = None;
    let mut escaped = false;
    let chars = line.chars().collect::<Vec<_>>();
    let mut index = 0;

    while index < chars.len() {
        let ch = chars[index];
        if let Some(active) = delimiter {
            if escaped {
                escaped = false;
            } else if ch == '\\' && active != '`' {
                escaped = true;
            } else if ch == active {
                delimiter = None;
            }
            result.push(' ');
            index += 1;
            continue;
        }
        if ch == '/' && chars.get(index + 1) == Some(&'/') {
            break;
        }
        if matches!(ch, '"' | '\'' | '`') {
            delimiter = Some(ch);
            result.push(' ');
        } else {
            result.push(ch);
        }
        index += 1;
    }

    result
}

pub(super) fn is_template_placeholder(literal: &str) -> bool {
    literal.len() > 4
        && literal.starts_with("__")
        && literal.ends_with("__")
        && literal[2..literal.len() - 2]
            .chars()
            .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == '_')
}

fn is_css_custom_property_literal(literal: &str) -> bool {
    literal.contains("var(--")
        || literal.strip_prefix("--").is_some_and(|name| {
            !name.is_empty()
                && name
                    .chars()
                    .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_'))
        })
}

fn is_css_function_value(literal: &str) -> bool {
    const CSS_FUNCTIONS: &[&str] = &[
        "calc(",
        "color-mix(",
        "hsl(",
        "hsla(",
        "linear-gradient(",
        "minmax(",
        "radial-gradient(",
        "repeat(",
        "rgb(",
        "rgba(",
        "translate(",
    ];

    CSS_FUNCTIONS
        .iter()
        .any(|function| literal.trim_start().starts_with(function))
}

fn looks_like_css_class_literal(literal: &str) -> bool {
    let tokens = literal.split_whitespace().collect::<Vec<_>>();
    if tokens.is_empty() || !tokens.iter().all(|token| is_css_class_token(token)) {
        return false;
    }

    if tokens.len() == 1 {
        return has_strong_css_class_marker(tokens[0]);
    }

    tokens
        .iter()
        .any(|token| has_strong_css_class_marker(token))
        && tokens.iter().all(|token| {
            has_strong_css_class_marker(token)
                || token.contains('-')
                || token.contains(':')
                || token.contains('[')
        })
}

fn is_css_class_token(token: &str) -> bool {
    !token.is_empty()
        && token.chars().all(|ch| {
            ch.is_ascii_alphanumeric()
                || matches!(
                    ch,
                    '-' | '_' | ':' | '/' | '.' | '[' | ']' | '(' | ')' | '%' | '!' | '='
                )
        })
}

fn has_strong_css_class_marker(token: &str) -> bool {
    const EXACT: &[&str] = &[
        "absolute",
        "antialiased",
        "block",
        "container",
        "fixed",
        "flex",
        "grid",
        "grow",
        "hidden",
        "inline",
        "italic",
        "lowercase",
        "panelbody",
        "relative",
        "shrink",
        "sticky",
        "surface",
        "truncate",
        "uppercase",
    ];
    const PREFIXES: &[&str] = &[
        "accent-",
        "bg-",
        "border-",
        "break-",
        "col-",
        "dark:",
        "data-",
        "feed-",
        "flex-",
        "font-",
        "from-",
        "gap-",
        "grid-",
        "group-",
        "group-data-",
        "h-",
        "items-",
        "justify-",
        "leading-",
        "m-",
        "max-",
        "mb-",
        "md:",
        "min-",
        "ml-",
        "mr-",
        "mt-",
        "mx-",
        "my-",
        "opacity-",
        "overflow-",
        "p-",
        "pb-",
        "peer-",
        "pl-",
        "pr-",
        "pt-",
        "px-",
        "py-",
        "ring-",
        "rounded-",
        "row-",
        "shadow-",
        "sm:",
        "space-",
        "surface-",
        "text-",
        "to-",
        "tracking-",
        "via-",
        "w-",
        "whitespace-",
        "xl:",
    ];
    EXACT.contains(&token) || PREFIXES.iter().any(|prefix| token.starts_with(prefix))
}
