use serde::{Deserialize, Serialize};

/// The action a job performs when triggered.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Action {
    Run {
        command: String,
        shell: bool,
        workdir: Option<String>,
    },
    Prompt {
        text: String,
        agent: Option<String>,
    },
    Webhook {
        url: String,
        method: HttpMethod,
        headers: Vec<(String, String)>,
        body: Option<String>,
    },
}

impl Action {
    pub fn kind_str(&self) -> &'static str {
        match self {
            Self::Run { .. } => "run",
            Self::Prompt { .. } => "prompt",
            Self::Webhook { .. } => "webhook",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
}

impl Default for HttpMethod {
    fn default() -> Self {
        Self::Post
    }
}

impl std::fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Get => write!(f, "GET"),
            Self::Post => write!(f, "POST"),
            Self::Put => write!(f, "PUT"),
            Self::Patch => write!(f, "PATCH"),
            Self::Delete => write!(f, "DELETE"),
        }
    }
}

impl std::str::FromStr for HttpMethod {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "GET" => Ok(Self::Get),
            "POST" => Ok(Self::Post),
            "PUT" => Ok(Self::Put),
            "PATCH" => Ok(Self::Patch),
            "DELETE" => Ok(Self::Delete),
            other => Err(format!("unsupported HTTP method: {other}")),
        }
    }
}
