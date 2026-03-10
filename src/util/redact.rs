/// Headers whose values should be redacted in output.
const SENSITIVE_HEADERS: &[&str] = &["authorization", "token", "api-key", "cookie"];
const SENSITIVE_ARG_WORDS: &[&str] = &["token", "secret", "password", "passwd"];
const SENSITIVE_ARG_EXACT: &[&str] = &[
    "api-key",
    "apikey",
    "auth",
    "authorization",
    "cookie",
    "credentials",
];

/// Redact the value of sensitive headers.
pub fn redact_header_value(key: &str, value: &str) -> String {
    let lower = key.to_lowercase();
    for sensitive in SENSITIVE_HEADERS {
        if lower.contains(sensitive) {
            return "***REDACTED***".to_string();
        }
    }
    value.to_string()
}

/// Redact credentials embedded in a URL (e.g., `https://user:pass@host/...`).
pub fn redact_url(raw: &str) -> String {
    match url::Url::parse(raw) {
        Ok(mut parsed) => {
            if !parsed.username().is_empty() || parsed.password().is_some() {
                let _ = parsed.set_username("***");
                let _ = parsed.set_password(Some("***"));
            }
            parsed.to_string()
        }
        Err(_) => raw.to_string(),
    }
}

/// Redact likely secret-bearing CLI arguments before displaying them.
pub fn redact_cli_args(args: &[String]) -> Vec<String> {
    let mut redacted = Vec::with_capacity(args.len());
    let mut redact_next = false;

    for arg in args {
        if redact_next {
            redacted.push("***REDACTED***".to_string());
            redact_next = false;
            continue;
        }

        if let Some((key, value)) = arg.split_once('=') {
            if is_sensitive_arg_key(key) {
                redacted.push(format!("{key}=***REDACTED***"));
            } else {
                let _ = value;
                redacted.push(arg.clone());
            }
            continue;
        }

        if let Some((key, value)) = arg.split_once(':') {
            if is_sensitive_arg_key(key) {
                let _ = value;
                redacted.push(format!("{key}: ***REDACTED***"));
                continue;
            }
        }

        if is_sensitive_arg_key(arg) {
            redacted.push(arg.clone());
            redact_next = true;
        } else {
            redacted.push(arg.clone());
        }
    }

    redacted
}

fn is_sensitive_arg_key(raw: &str) -> bool {
    let trimmed = raw.trim();
    let normalized = trimmed
        .trim_start_matches('-')
        .trim()
        .to_ascii_lowercase()
        .replace('_', "-");

    if normalized.is_empty() {
        return false;
    }

    if SENSITIVE_ARG_EXACT.contains(&normalized.as_str()) {
        return true;
    }

    if normalized.ends_with("-key") {
        return true;
    }

    normalized
        .split(|c: char| !c.is_ascii_alphanumeric())
        .any(|part| !part.is_empty() && SENSITIVE_ARG_WORDS.contains(&part))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_authorization_header() {
        assert_eq!(
            redact_header_value("Authorization", "Bearer secret"),
            "***REDACTED***"
        );
    }

    #[test]
    fn keeps_safe_header() {
        assert_eq!(
            redact_header_value("Content-Type", "application/json"),
            "application/json"
        );
    }

    #[test]
    fn redacts_url_credentials() {
        let redacted = redact_url("https://user:pass@example.com/api");
        assert!(!redacted.contains("user:pass"));
        assert!(redacted.contains("***"));
    }

    #[test]
    fn keeps_safe_url() {
        let url = "https://example.com/api";
        assert_eq!(redact_url(url), "https://example.com/api");
    }

    #[test]
    fn redacts_sensitive_flag_value_pair() {
        let args = vec!["--token".to_string(), "secret-value".to_string()];
        assert_eq!(
            redact_cli_args(&args),
            vec!["--token".to_string(), "***REDACTED***".to_string()]
        );
    }

    #[test]
    fn redacts_sensitive_assignment_args() {
        let args = vec![
            "--api-key=abc123".to_string(),
            "OPENAI_API_KEY=sk-test".to_string(),
        ];
        assert_eq!(
            redact_cli_args(&args),
            vec![
                "--api-key=***REDACTED***".to_string(),
                "OPENAI_API_KEY=***REDACTED***".to_string(),
            ]
        );
    }

    #[test]
    fn keeps_safe_cli_args() {
        let args = vec!["--model".to_string(), "gpt-4.1".to_string()];
        assert_eq!(redact_cli_args(&args), args);
    }
}
