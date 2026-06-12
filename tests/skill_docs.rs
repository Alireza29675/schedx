//! Drift guards for the canonical agent skill (`skills/schedx/`).
//!
//! Two invariants that silently break a skill in production, locked here so CI
//! catches them:
//!   1. Every `schedx <command>` the skill documents must still exist in the
//!      CLI — a renamed or removed command fails the build.
//!   2. An installed `SKILL.md` must keep its YAML frontmatter at byte 0, with
//!      `name` and `description` parseable — the native loaders read it there.

mod helpers;

use std::collections::BTreeSet;

use helpers::TestEnv;

const SKILL_MD: &str = include_str!("../skills/schedx/SKILL.md");
const REFERENCE_MD: &str = include_str!("../skills/schedx/reference.md");

/// Every `schedx <subcommand>` token that appears inside a fenced code block.
///
/// Only fenced commands are scanned: prose mentions like "schedx is a
/// scheduler" or "schedx never phones home" are not commands and must not be
/// treated as ones (they would otherwise fail the guard for the wrong reason).
fn documented_subcommands(markdown: &str) -> BTreeSet<String> {
    let mut subs = BTreeSet::new();
    let mut in_fence = false;
    for line in markdown.lines() {
        if line.trim_start().starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if !in_fence {
            continue;
        }
        let mut rest = line;
        while let Some(idx) = rest.find("schedx ") {
            let after = &rest[idx + "schedx ".len()..];
            let token: String = after
                .chars()
                .take_while(|c| c.is_ascii_lowercase() || *c == '-')
                .collect();
            if !token.is_empty() && !token.starts_with('-') {
                subs.insert(token);
            }
            rest = after;
        }
    }
    subs
}

#[test]
fn documented_commands_all_exist_in_the_cli() {
    let mut subs = documented_subcommands(SKILL_MD);
    subs.extend(documented_subcommands(REFERENCE_MD));
    assert!(
        subs.contains("add") && subs.contains("up") && subs.contains("setup"),
        "extractor found {} commands but is missing obvious ones — parser likely broke: {subs:?}",
        subs.len()
    );

    let env = TestEnv::new();
    for sub in &subs {
        // `--help` exits 0 for a real subcommand and errors (exit 2) for an
        // unknown one, so this catches any command the docs name but the CLI
        // no longer has.
        let output = env
            .cmd()
            .args([sub.as_str(), "--help"])
            .output()
            .expect("failed to run schedx");
        assert!(
            output.status.success(),
            "skill docs reference `schedx {sub}`, but it is not a valid CLI command \
             (a rename or removal must be reflected in skills/schedx/)"
        );
    }
}

#[test]
fn setup_installs_skill_with_frontmatter_at_byte_zero() {
    let env = TestEnv::new();
    let home = env.home();

    // Codex installs into the shared `~/.agents/skills/schedx/` path.
    env.cmd()
        .env("HOME", &home)
        .args(["setup", "--agent", "codex"])
        .assert()
        .success();

    let skill = home.join(".agents/skills/schedx/SKILL.md");
    assert!(skill.exists(), "skill not installed at {}", skill.display());
    let content = std::fs::read_to_string(&skill).expect("failed to read installed SKILL.md");

    // The version marker must NOT push the frontmatter off byte 0.
    assert!(
        content.starts_with("---\n"),
        "installed SKILL.md must start with YAML frontmatter at byte 0, got: {:?}",
        &content[..content.len().min(40)]
    );

    // The frontmatter must be valid YAML with the skill's identity intact.
    let body = content.strip_prefix("---\n").unwrap();
    let close = body
        .find("\n---")
        .expect("SKILL.md frontmatter must be closed with `---`");
    let frontmatter: serde_norway::Value =
        serde_norway::from_str(&body[..close]).expect("SKILL.md frontmatter must be valid YAML");
    assert_eq!(
        frontmatter
            .get("name")
            .and_then(serde_norway::Value::as_str),
        Some("schedx"),
        "frontmatter `name` must survive installation"
    );
    assert!(
        frontmatter
            .get("description")
            .and_then(serde_norway::Value::as_str)
            .is_some_and(|d| !d.is_empty()),
        "frontmatter `description` must survive installation"
    );

    // The stamped version must match the binary that wrote it (drift-proof).
    let version = env!("CARGO_PKG_VERSION");
    assert!(
        content.contains(&format!("<!-- schedx-skill v{version} -->")),
        "installed skill must carry the current version marker v{version}"
    );
}

#[test]
fn setup_all_installs_every_supported_agent() {
    // `--all` must install for every supported agent, detected or not — the
    // docs promise it and the flag name implies it. Cursor installs into the
    // current directory, so run from an isolated CWD to avoid polluting.
    let env = TestEnv::new();
    let home = env.home();
    let cwd = tempfile::tempdir().expect("failed to make cwd");

    env.cmd()
        .env("HOME", &home)
        .current_dir(cwd.path())
        .args(["setup", "--all"])
        .assert()
        .success();

    // Claude → ~/.claude, the shared trio → ~/.agents, cursor → project CWD.
    assert!(
        home.join(".claude/skills/schedx/SKILL.md").exists(),
        "claude skill missing"
    );
    assert!(
        home.join(".agents/skills/schedx/SKILL.md").exists(),
        "shared agents skill missing"
    );
    assert!(
        cwd.path().join(".cursor/skills/schedx/SKILL.md").exists(),
        "cursor skill should land in the current project dir"
    );
}

#[test]
fn setup_json_is_a_single_document() {
    // `--json` must emit exactly ONE JSON document so it is parseable by jq /
    // serde. (It used to print the skills array and then a separate agents
    // object — two concatenated top-level values that broke every consumer.)
    let env = TestEnv::new();
    let home = env.home();
    let cwd = tempfile::tempdir().expect("failed to make cwd");

    let output = env
        .cmd()
        .env("HOME", &home)
        .current_dir(cwd.path())
        .args(["setup", "--agent", "codex", "--json"])
        .output()
        .expect("failed to run setup --json");

    // serde_json::from_slice rejects trailing data, so a clean parse is proof
    // the output is a single document — it would fail on two concatenated ones.
    let doc: serde_json::Value = serde_json::from_slice(&output.stdout)
        .expect("setup --json must be exactly one JSON document");
    assert!(
        doc.get("skills").and_then(|s| s.as_array()).is_some(),
        "the combined document must carry a skills array"
    );
    assert!(
        doc.get("agents").is_some(),
        "the combined document must fold in the agent detection"
    );
}

#[test]
fn setup_list_reports_installed_version() {
    let env = TestEnv::new();
    let home = env.home();
    env.cmd()
        .env("HOME", &home)
        .args(["setup", "--agent", "codex"])
        .assert()
        .success();

    let output = env
        .cmd()
        .env("HOME", &home)
        .args(["setup", "--list", "--json"])
        .output()
        .expect("failed to run setup --list");
    let entries: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("--list --json must be valid JSON");
    let version = env!("CARGO_PKG_VERSION");
    let codex = entries
        .as_array()
        .and_then(|a| a.iter().find(|e| e["agent"] == "codex"))
        .expect("codex entry must be present");
    assert!(
        codex["status"]
            .as_str()
            .is_some_and(|s| s.contains(version)),
        "codex should report installed at v{version}, got {:?}",
        codex["status"]
    );
}
