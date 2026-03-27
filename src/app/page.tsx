import { CopyButton } from "./components/CopyButton";
import { AgentChat } from "./components/AgentChat";

export default function Home() {
  return (
    <div className="container grid-frame">
      {/* Hero */}
      <header className="hero">
        <div className="hero-content">
          <div className="hero-left">
            <div className="meta-label" style={{ marginBottom: "1rem" }}>
              The scheduler for the autonomous age
            </div>
            <h1 className="logo">
              <svg className="logo-icon" width="40" height="40" viewBox="0 0 24 24" aria-hidden="true">
                <rect x="2" y="2" width="5" height="5" fill="#b8e0ff" />
                <rect x="9" y="2" width="5" height="5" fill="#1a1a1a" />
                <rect x="16" y="2" width="5" height="5" fill="#b8e0ff" />
                <rect x="2" y="9" width="5" height="5" fill="#1a1a1a" />
                <rect x="9" y="9" width="5" height="5" fill="#b8e0ff" />
                <rect x="16" y="9" width="5" height="5" fill="#1a1a1a" />
                <rect x="2" y="16" width="5" height="5" fill="#b8e0ff" />
                <rect x="9" y="16" width="5" height="5" fill="#1a1a1a" />
                <rect x="16" y="16" width="5" height="5" fill="#b8e0ff" />
              </svg>
              sched<span className="logo-x">x</span>
            </h1>
            <p className="hero-desc">
              Schedule commands, agent prompts, and webhooks.
              <br />
              One CLI. Local-first. Built for humans and agents.
            </p>
            <div className="hero-install">
              <code>curl -sL schedx.run/get | bash</code>
              <CopyButton text="curl -sL schedx.run/get | bash" />
            </div>
            <div className="hero-links">
              <a
                href="https://github.com/Alireza29675/schedx"
                className="hero-btn hero-btn-star"
                target="_blank"
                rel="noopener noreferrer"
              >
                Star on GitHub
                <span className="star-tooltip">help it get seen :)</span>
              </a>
              <a
                href="https://github.com/Alireza29675/schedx/blob/main/docs/EXAMPLES.md"
                className="hero-btn hero-btn-secondary"
                target="_blank"
                rel="noopener noreferrer"
              >
                Examples
              </a>
            </div>
          </div>
          <div className="hero-right">
            <AgentChat />
          </div>
        </div>
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
              Schedule any shell command. Cron expressions or plain English
              intervals.
            </p>
            <div className="copyable">
              <CopyButton
                text={`schedx add "every 6h" \\\n  --run "./backup.sh"`}
              />
              <pre>
                <span className="hl-cmd">schedx</span> add{" "}
                <span className="hl-val">&quot;every 6h&quot;</span> \{"\n"}
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
              Tell an agent what to do and when. Prompts are first-class
              citizens.
            </p>
            <div className="copyable">
              <CopyButton
                text={`schedx add "0 6 * * *" \\\n  --prompt "Summarize what I did yesterday and add it to my Notion" \\\n  --agent claude`}
              />
              <pre>
                <span className="hl-cmd">schedx</span> add{" "}
                <span className="hl-val">&quot;0 6 * * *&quot;</span> \{"\n"}
                {"  "}
                <span className="hl-flag">--prompt</span>{" "}
                <span className="hl-val">
                  &quot;Summarize yesterday,{"\n"}
                  {"  "}add to my Notion&quot;
                </span>{" "}
                \{"\n"}
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
              Fire webhooks on a schedule. Headers, body, and HTTPS built in.
            </p>
            <div className="copyable">
              <CopyButton
                text={`schedx add "in 2h" \\\n  --webhook https://hooks.slack.com/T00`}
              />
              <pre>
                <span className="hl-cmd">schedx</span> add{" "}
                <span className="hl-val">&quot;in 2h&quot;</span> \{"\n"}
                {"  "}
                <span className="hl-flag">--webhook</span>{" "}
                <span className="hl-val">https://hooks.slack.com/T00</span>
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
              <CopyButton
                text={`schedx add "0 7 * * 1-5" \\\n  --prompt "Read top stories from Reuters, AP, Al Jazeera, and Ars Technica. Write a balanced 5-minute briefing and email it to me."`}
              />
              <pre>
                <span className="hl-cmd">schedx</span> add{" "}
                <span className="hl-val">&quot;0 7 * * 1-5&quot;</span> \{"\n"}
                {"  "}
                <span className="hl-flag">--prompt</span>{" "}
                <span className="hl-str">
                  &quot;Read top stories from{"\n"}
                  {"  "}Reuters, AP, Al Jazeera, and{"\n"}
                  {"  "}Ars Technica. Write a balanced{"\n"}
                  {"  "}5-minute briefing and email it{"\n"}
                  {"  "}to me.&quot;
                </span>
              </pre>
            </div>
          </div>

          <div className="workflow-item">
            <div className="workflow-info">
              <span className="meta-label" style={{ marginBottom: "1rem" }}>
                Nightly at 2am
              </span>
              <h3>Nightly Multi-Agent Security Sweep</h3>
              <p>
                Two agents run independent security scans. A third judges both
                reports and only pings you if something needs attention.
              </p>
            </div>
            <div className="workflow-code copyable">
              <CopyButton
                text={`schedx add "0 2 * * *" \\\n  --run "claude -p 'Full security scan: ports, logins, processes' > /tmp/a1.md && codex -p 'Review network and firewall' > /tmp/a2.md && claude -p 'Judge these reports. Alert only if needed.'"`}
              />
              <pre>
                <span className="hl-cmd">schedx</span> add{" "}
                <span className="hl-val">&quot;0 2 * * *&quot;</span> \{"\n"}
                {"  "}
                <span className="hl-flag">--run</span>{" "}
                <span className="hl-str">
                  &quot;claude -p &apos;Full security{"\n"}
                  {"  "}scan: ports, logins, processes&apos;{"\n"}
                  {"  "}&gt; /tmp/a1.md
                </span>{" "}
                <span className="hl-op">&amp;&amp;</span>
                {"\n"}
                {"  "}
                <span className="hl-str">
                  codex -p &apos;Review network and{"\n"}
                  {"  "}firewall&apos; &gt; /tmp/a2.md
                </span>{" "}
                <span className="hl-op">&amp;&amp;</span>
                {"\n"}
                {"  "}
                <span className="hl-str">
                  claude -p &apos;Judge these reports.{"\n"}
                  {"  "}Alert only if needed.&apos;&quot;
                </span>
              </pre>
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
              <CopyButton
                text={`schedx add "in 30m" \\\n  --prompt "Check the /health endpoint and error rates for the last 30 minutes. Did the deploy go clean?"`}
              />
              <pre>
                <span className="hl-cmd">schedx</span> add{" "}
                <span className="hl-val">&quot;in 30m&quot;</span> \{"\n"}
                {"  "}
                <span className="hl-flag">--prompt</span>{" "}
                <span className="hl-str">
                  &quot;Check the /health{"\n"}
                  {"  "}endpoint and error rates for{"\n"}
                  {"  "}the last 30 minutes. Did the{"\n"}
                  {"  "}deploy go clean?&quot;
                </span>
              </pre>
            </div>
          </div>
        </div>

        {/* Mid-page CTA */}
        <div className="mid-cta">
          <div className="install-box">
            <code>curl -sL schedx.run/get | bash</code>
            <CopyButton text="curl -sL schedx.run/get | bash" />
            <span>[Bin]</span>
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
          <div className="footer-meta">
            MIT Licensed. Written in Rust. File-based. Zero dependencies.
          </div>
        </div>
        <div className="links">
          <a
            href="https://github.com/Alireza29675/schedx"
            target="_blank"
            rel="noopener noreferrer"
          >
            GitHub
          </a>
          <a
            href="https://github.com/Alireza29675/schedx/blob/main/docs/EXAMPLES.md"
            target="_blank"
            rel="noopener noreferrer"
          >
            Examples
          </a>
          <a
            href="https://alireza.cc"
            target="_blank"
            rel="noopener noreferrer"
          >
            alireza.cc
          </a>
        </div>
      </section>
    </div>
  );
}
