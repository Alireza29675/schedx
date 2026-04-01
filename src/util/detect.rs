use std::process::Command;

use crate::model::config::AgentProfile;

/// A known CLI agent that can be auto-detected and registered.
pub struct KnownAgent {
    pub name: &'static str,
    pub bin: &'static str,
    pub args: &'static [&'static str],
    pub prompt_stdin: bool,
    pub description: &'static str,
}

impl KnownAgent {
    pub fn to_profile(&self) -> AgentProfile {
        AgentProfile {
            bin: self.bin.to_string(),
            args: self.args.iter().map(|s| (*s).to_string()).collect(),
            prompt_stdin: self.prompt_stdin,
        }
    }
}

/// Known CLI agents for auto-detection. Cursor is excluded (IDE, not a CLI agent).
pub const KNOWN_CLI_AGENTS: &[KnownAgent] = &[
    KnownAgent {
        name: "claude",
        bin: "claude",
        args: &["-p", "--enable-auto-mode"],
        prompt_stdin: false,
        description: "Claude Code (Anthropic)",
    },
    KnownAgent {
        name: "codex",
        bin: "codex",
        args: &["exec", "--full-auto"],
        prompt_stdin: false,
        description: "Codex CLI (OpenAI)",
    },
    KnownAgent {
        name: "gemini",
        bin: "gemini",
        args: &["-p", "--yolo"],
        prompt_stdin: false,
        description: "Gemini CLI (Google)",
    },
    KnownAgent {
        name: "opencode",
        bin: "opencode",
        args: &["-p"],
        prompt_stdin: false,
        description: "OpenCode",
    },
];

/// Check if a binary is available on `PATH`.
pub fn is_binary_on_path(bin: &str) -> bool {
    Command::new("which")
        .arg(bin)
        .output()
        .is_ok_and(|o| o.status.success())
}
