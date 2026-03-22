import { CopyButton } from "./components/CopyButton";

export default function Home() {
  return (
    <div className="container grid-frame">
      {/* Hero */}
      <header className="hero">
        <div className="meta-label" style={{ marginBottom: "1rem" }}>
          Scheduler CLI
        </div>
        <div className="hero-header">
          <div>
            <h1 className="logo">schedx</h1>
          </div>
          <div className="hero-install">
            <span className="meta-label" style={{ margin: 0 }}>
              Install
            </span>
            <code>curl -sL schedx.run/get | bash</code>
            <CopyButton text="curl -sL schedx.run/get | bash" />
          </div>
        </div>
        <p
          style={{
            fontFamily: "var(--font-mono)",
            color: "var(--fg-muted)",
            maxWidth: "460px",
            fontSize: "13px",
            lineHeight: 1.5,
          }}
        >
          Recurring jobs, one-shot tasks, agent prompts, and webhooks.
          <br />
          One CLI. Local-first. Built for humans and agents.
        </p>
      </header>

      {/* The Idea */}
      <section className="idea">
        <p>
          Agents can do anything right now.{" "}
          <span style={{ color: "var(--accent)" }}>schedx</span> gives them the
          dimension of time.
        </p>
        <div className="idea-meta">
          <span className="meta-label">Cron</span>
          <span className="meta-label">Intervals</span>
          <span className="meta-label">One-shot</span>
        </div>
      </section>

      {/* How It Works */}
      <section>
        <h2>How It Works</h2>
        <div className="blocks-grid">
          <div className="block">
            <div className="block-header">
              <span className="meta-label">--run</span>
              <span className="block-num">001</span>
            </div>
            <h3>Commands</h3>
            <p
              style={{
                color: "var(--fg-muted)",
                marginBottom: "2rem",
                fontSize: "14px",
              }}
            >
              Schedule any shell command on a cron expression or human interval.
            </p>
            <div className="copyable">
              <CopyButton text={`schedx add "0 2 * * *" \\\n  --run "./backup.sh"`} />
              <pre>
                <span className="hl-cmd">schedx</span> add{" "}
                <span className="hl-val">&quot;0 2 * * *&quot;</span> \{"\n"}
                {"  "}
                <span className="hl-flag">--run</span>{" "}
                <span className="hl-val">&quot;./backup.sh&quot;</span>
              </pre>
            </div>
          </div>
          <div className="block">
            <div className="block-header">
              <span className="meta-label">--prompt</span>
              <span className="block-num">002</span>
            </div>
            <h3>Agent Prompts</h3>
            <p
              style={{
                color: "var(--fg-muted)",
                marginBottom: "2rem",
                fontSize: "14px",
              }}
            >
              Ask an AI agent to do something on a schedule. Prompts are
              first-class.
            </p>
            <div className="copyable">
              <CopyButton text={`schedx add "0 6 * * *" \\\n  --prompt "Summarize what I did yesterday and add it to my Notion" \\\n  --agent claude`} />
              <pre>
                <span className="hl-cmd">schedx</span> add{" "}
                <span className="hl-val">&quot;0 6 * * *&quot;</span> \{"\n"}
                {"  "}
                <span className="hl-flag">--prompt</span>{" "}
                <span className="hl-val">&quot;Summarize yesterday,{"\n"}
                {"  "}add to my Notion&quot;</span> \{"\n"}
                {"  "}
                <span className="hl-flag">--agent</span>{" "}
                <span className="hl-val">claude</span>
              </pre>
            </div>
          </div>
          <div className="block">
            <div className="block-header">
              <span className="meta-label">--webhook</span>
              <span className="block-num">003</span>
            </div>
            <h3>Webhooks</h3>
            <p
              style={{
                color: "var(--fg-muted)",
                marginBottom: "2rem",
                fontSize: "14px",
              }}
            >
              Fire HTTP requests on a schedule. Built-in HTTPS, headers, and
              body.
            </p>
            <div className="copyable">
              <CopyButton text={`schedx add "0 9 * * 1-5" \\\n  --webhook https://hooks.slack.com/...`} />
              <pre>
                <span className="hl-cmd">schedx</span> add{" "}
                <span className="hl-val">&quot;0 9 * * 1-5&quot;</span> \{"\n"}
                {"  "}
                <span className="hl-flag">--webhook</span>{" "}
                <span className="hl-val">https://hooks.slack.com/...</span>
              </pre>
            </div>
          </div>
        </div>
      </section>

      {/* Real Workflows */}
      <section>
        <h2>Real Workflows</h2>
        <div className="workflow-list">
          <div className="workflow-item">
            <div className="workflow-info">
              <span className="meta-label" style={{ marginBottom: "1rem" }}>
                Every weekday at 7am
              </span>
              <h3>Morning News Briefing</h3>
              <p>
                An agent reads from diverse news sources, compiles a balanced
                summary, and emails it to you before your day starts.
              </p>
            </div>
            <div className="workflow-code copyable">
              <CopyButton text={`schedx add "0 7 * * 1-5" \\\n  --prompt "Read top stories from Reuters, AP, Al Jazeera, and Ars Technica. Write a balanced 5-minute briefing and email it to me."`} />
              <pre>{`schedx add "0 7 * * 1-5" \\
  --prompt "Read top stories from
  Reuters, AP, Al Jazeera, and
  Ars Technica. Write a balanced
  5-minute briefing and email it
  to me."`}</pre>
            </div>
          </div>

          <div className="workflow-item">
            <div className="workflow-info">
              <span className="meta-label" style={{ marginBottom: "1rem" }}>
                Nightly at 2am
              </span>
              <h3>Multi-Agent Security Audit</h3>
              <p>
                Two agents run independent security scans. A third judges both
                reports and only pings you if something needs attention.
              </p>
            </div>
            <div className="workflow-code copyable">
              <CopyButton text={`schedx add "0 2 * * *" \\\n  --run "claude -p 'Full security scan: ports, logins, processes' > /tmp/a1.md && codex -p 'Review network and firewall' > /tmp/a2.md && claude -p 'Judge these reports. Alert only if needed.'"`} />
              <pre>{`schedx add "0 2 * * *" \\
  --run "claude -p 'Full security
  scan: ports, logins, processes'
  > /tmp/a1.md &&
  codex -p 'Review network and
  firewall' > /tmp/a2.md &&
  claude -p 'Judge these reports.
  Alert only if needed.'"`}</pre>
            </div>
          </div>

          <div className="workflow-item">
            <div className="workflow-info">
              <span className="meta-label" style={{ marginBottom: "1rem" }}>
                30 minutes after deploy
              </span>
              <h3>Post-Deploy Canary Check</h3>
              <p>
                Schedule a one-shot follow-up after a deploy. The agent checks
                health endpoints and error rates, then reports back.
              </p>
            </div>
            <div className="workflow-code copyable">
              <CopyButton text={`schedx add "in 30m" \\\n  --prompt "Check the /health endpoint and error rates for the last 30 minutes. Did the deploy go clean?"`} />
              <pre>{`schedx add "in 30m" \\
  --prompt "Check the /health
  endpoint and error rates for
  the last 30 minutes. Did the
  deploy go clean?"`}</pre>
            </div>
          </div>
        </div>
      </section>

      {/* Comparison */}
      <section>
        <h2>How It Compares</h2>
        <table className="compare-table">
          <thead>
            <tr>
              <th>Capability</th>
              <th>schedx</th>
              <th>cron</th>
              <th>at</th>
            </tr>
          </thead>
          <tbody>
            <tr>
              <td>Scheduling</td>
              <td className="row-highlight">Cron + intervals + one-shot</td>
              <td>Cron only</td>
              <td>One-shot only</td>
            </tr>
            <tr>
              <td>Agent Prompts</td>
              <td className="row-highlight">Built-in (--prompt)</td>
              <td>No</td>
              <td>No</td>
            </tr>
            <tr>
              <td>Webhooks</td>
              <td className="row-highlight">Built-in (--webhook)</td>
              <td>Wrap curl manually</td>
              <td>No</td>
            </tr>
            <tr>
              <td>Job History</td>
              <td className="row-highlight">Per-job run history + logs</td>
              <td>No</td>
              <td>No</td>
            </tr>
            <tr>
              <td>JSON Output</td>
              <td className="row-highlight">--json on every command</td>
              <td>No</td>
              <td>No</td>
            </tr>
            <tr>
              <td>Lifecycle</td>
              <td className="row-highlight">Pause, resume, skip</td>
              <td>Edit crontab</td>
              <td>Remove only</td>
            </tr>
          </tbody>
        </table>
      </section>

      {/* Footer / Install */}
      <section className="footer-section">
        <div className="cta-group">
          <div className="install-box">
            <code>curl -sL schedx.run/get | bash</code>
            <CopyButton text="curl -sL schedx.run/get | bash" />
            <span>[Bin]</span>
          </div>
          <div className="install-box">
            <code>cargo install schedx --locked</code>
            <CopyButton text="cargo install schedx --locked" />
            <span>[Rust]</span>
          </div>
        </div>
        <div className="links">
          <a href="https://github.com/Alireza29675/schedx">GitHub</a>
          <a href="https://github.com/Alireza29675/schedx/blob/main/docs/EXAMPLES.md">
            Examples
          </a>
        </div>
      </section>
    </div>
  );
}
