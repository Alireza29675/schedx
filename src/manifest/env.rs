//! `${VAR}` interpolation for manifest string values.
//!
//! The lookup is injected rather than reading `std::env` directly:
//! edition 2024 makes `set_var` unsafe, so injection keeps tests pure
//! and parallel-safe. Production callers pass a wrapper over
//! `std::env::var`.

/// Expand `${VAR}` references in `input` using `lookup`.
///
/// Syntax:
/// - `${VAR}` expands (`VAR` = `[A-Za-z_][A-Za-z0-9_]*`).
/// - `$${` is an escape producing a literal `${`.
/// - A `$` not followed by `{` is literal.
///
/// Errors:
/// - A missing variable yields `Err` carrying exactly the variable name
///   (the caller formats the message; use [`is_var_name`] to tell these
///   apart).
/// - Syntax errors (unclosed brace, empty or invalid variable name)
///   yield a full sentence, which never matches the variable-name
///   grammar.
pub fn expand(input: &str, lookup: &dyn Fn(&str) -> Option<String>) -> Result<String, String> {
    let mut out = String::with_capacity(input.len());
    let mut rest = input;

    while let Some(pos) = rest.find('$') {
        out.push_str(&rest[..pos]);
        let after = &rest[pos + 1..];

        if let Some(tail) = after.strip_prefix("${") {
            // `$${` escape: emit a literal `${` and keep scanning.
            out.push_str("${");
            rest = tail;
        } else if let Some(tail) = after.strip_prefix('{') {
            let Some(end) = tail.find('}') else {
                return Err(format!("unclosed '${{' in '{input}'"));
            };
            let name = &tail[..end];
            if name.is_empty() {
                return Err(format!("empty variable name in '{input}'"));
            }
            if !is_var_name(name) {
                return Err(format!("invalid variable name '{name}' in '{input}'"));
            }
            match lookup(name) {
                Some(value) => out.push_str(&value),
                None => return Err(name.to_string()),
            }
            rest = &tail[end + 1..];
        } else {
            // A `$` not followed by `{` is literal.
            out.push('$');
            rest = after;
        }
    }

    out.push_str(rest);
    Ok(out)
}

/// Whether `s` matches the variable-name grammar `[A-Za-z_][A-Za-z0-9_]*`.
pub fn is_var_name(s: &str) -> bool {
    let mut chars = s.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lookup(name: &str) -> Option<String> {
        match name {
            "NAME" => Some("world".to_string()),
            "A" => Some("alpha".to_string()),
            "B" => Some("beta".to_string()),
            "_UNDER" => Some("score".to_string()),
            _ => None,
        }
    }

    #[test]
    fn basic_expansion() {
        assert_eq!(expand("hi ${NAME}", &lookup).unwrap(), "hi world");
    }

    #[test]
    fn multiple_vars_in_one_string() {
        assert_eq!(
            expand("${A}-${B}/${_UNDER}", &lookup).unwrap(),
            "alpha-beta/score"
        );
    }

    #[test]
    fn escape_produces_literal() {
        // `$${HOME}` must not look HOME up at all.
        assert_eq!(expand("echo $${HOME}", &lookup).unwrap(), "echo ${HOME}");
    }

    #[test]
    fn lone_dollar_is_literal() {
        assert_eq!(
            expand("cost $5 and $x and end$", &lookup).unwrap(),
            "cost $5 and $x and end$"
        );
    }

    #[test]
    fn missing_var_errors_with_name() {
        assert_eq!(expand("${MISSING}", &lookup).unwrap_err(), "MISSING");
    }

    #[test]
    fn unclosed_brace_errors() {
        let err = expand("oops ${NAME", &lookup).unwrap_err();
        assert!(err.contains("unclosed"), "got: {err}");
        assert!(!is_var_name(&err));
    }

    #[test]
    fn empty_var_name_errors() {
        let err = expand("${}", &lookup).unwrap_err();
        assert!(err.contains("empty variable name"), "got: {err}");
        assert!(!is_var_name(&err));
    }

    #[test]
    fn invalid_var_name_errors() {
        let err = expand("${FOO-BAR}", &lookup).unwrap_err();
        assert!(
            err.contains("invalid variable name 'FOO-BAR'"),
            "got: {err}"
        );
        assert!(!is_var_name(&err));
    }
}
