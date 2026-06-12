//! Shared action-input validation and construction.
//!
//! Used by `add` (CLI flags) and the declarative manifest commands (`up`),
//! so both paths enforce identical limits and security checks.

use anyhow::{Result, bail};

use crate::model::action::{Action, HttpMethod};
use crate::store::config::load_config;

/// Input limits from spec.
pub(crate) const MAX_COMMAND_LEN: usize = 32 * 1024;
pub(crate) const MAX_PROMPT_LEN: usize = 128 * 1024;
pub(crate) const MAX_TAGS: usize = 20;
pub(crate) const MAX_TAG_LEN: usize = 64;

/// Validate tag count and per-tag length.
pub(crate) fn validate_tags(tags: &[String]) -> Result<()> {
    if tags.len() > MAX_TAGS {
        bail!("Error: Maximum {MAX_TAGS} tags per job.");
    }
    for tag in tags {
        if tag.len() > MAX_TAG_LEN {
            bail!("Error: Tag '{tag}' exceeds maximum length of {MAX_TAG_LEN} characters.");
        }
    }
    Ok(())
}

/// Validate an optional on-failure command's length.
pub(crate) fn validate_on_failure(cmd: Option<&str>) -> Result<()> {
    if let Some(cmd) = cmd {
        if cmd.len() > MAX_COMMAND_LEN {
            bail!("Error: On-failure command exceeds maximum length of {MAX_COMMAND_LEN} bytes.");
        }
    }
    Ok(())
}

/// Build a run action, enforcing the command length limit.
pub(crate) fn build_run_action(
    command: String,
    shell: bool,
    workdir: Option<String>,
) -> Result<Action> {
    if command.len() > MAX_COMMAND_LEN {
        bail!("Error: Command exceeds maximum length of {MAX_COMMAND_LEN} bytes.");
    }
    Ok(Action::Run {
        command,
        shell,
        workdir,
    })
}

/// Build a prompt action, enforcing the prompt length limit.
pub(crate) fn build_prompt_action(text: String, agent: Option<String>) -> Result<Action> {
    if text.len() > MAX_PROMPT_LEN {
        bail!("Error: Prompt exceeds maximum length of {MAX_PROMPT_LEN} bytes.");
    }
    Ok(Action::Prompt { text, agent })
}

/// Build a webhook action: validates the URL scheme and enforces the
/// HTTPS-by-default security policy (`allow_insecure_http` config gate).
pub(crate) fn build_webhook_action(
    url: &str,
    method: HttpMethod,
    headers: Vec<(String, String)>,
    body: Option<String>,
) -> Result<Action> {
    let parsed = url::Url::parse(url)
        .map_err(|e| anyhow::anyhow!("Error: Invalid webhook URL '{url}': {e}"))?;
    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        bail!("Error: Only http:// and https:// webhook URLs are supported.");
    }

    // Security check: HTTP requires config flag
    if parsed.scheme() == "http" {
        let config = load_config()?;
        if !config.allow_insecure_http {
            bail!(
                "Error: HTTP webhooks are blocked by default for security.\n\
                 To allow: schedx config allow_insecure_http true"
            );
        }
    }

    Ok(Action::Webhook {
        url: url.to_string(),
        method,
        headers,
        body,
    })
}

/// Parse an optional HTTP method string, defaulting to POST.
pub(crate) fn parse_method(method: Option<&str>) -> Result<HttpMethod> {
    method
        .map(str::parse::<HttpMethod>)
        .transpose()
        .map_err(|e| anyhow::anyhow!("Error: {e}"))
        .map(Option::unwrap_or_default)
}

/// Parse repeatable `"Key: Value"` header lines (CLI form).
pub(crate) fn parse_header_lines(headers: &[String]) -> Result<Vec<(String, String)>> {
    let mut result = Vec::new();
    for h in headers {
        let (key, value) = h.split_once(':').ok_or_else(|| {
            anyhow::anyhow!("Error: Invalid header format '{h}'. Expected 'Key: Value'.")
        })?;
        result.push((key.trim().to_string(), value.trim().to_string()));
    }
    Ok(result)
}
