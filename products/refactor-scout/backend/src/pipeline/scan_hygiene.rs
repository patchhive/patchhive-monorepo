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

pub(super) fn is_non_actionable_style_literal(literal: &str) -> bool {
    is_css_custom_property_literal(literal)
        || is_css_function_value(literal)
        || looks_like_css_class_literal(literal)
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
