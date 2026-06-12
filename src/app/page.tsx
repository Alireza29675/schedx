import { CopyButton } from "./components/CopyButton";
import { AgentChat } from "./components/AgentChat";
import { SchedxLogo } from "./components/SchedxLogo";

export default function Home() {
  return (
    <div className="container grid-frame">
      {/* Hero */}
      <header className="hero">
        <div className="hero-content">
          <div className="hero-left">
            <SchedxLogo />
            <p className="hero-desc">
              Schedule commands, agent prompts, and webhooks.
              <br />
              One CLI. Local-first. Built for humans and agents.
            </p>
            <div className="hero-install">
              <code>curl -sL schedx.run/get | bash</code>
              <CopyButton text="curl -sL schedx.run/get | bash" umamiEvent="copy-install-command" />
            </div>
            <div className="hero-links">
              <a
                href="https://github.com/Alireza29675/schedx"
                className="hero-btn hero-btn-star"
                target="_blank"
                rel="noopener noreferrer"
                data-umami-event="click-github"
              >
                Star on GitHub
              </a>
              <a
                href="https://github.com/Alireza29675/schedx/blob/main/docs/EXAMPLES.md"
                className="hero-btn hero-btn-secondary"
                target="_blank"
                rel="noopener noreferrer"
                data-umami-event="click-examples"
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

      {/* How It Works — the command is the description */}
      <section>
        <h2>How It Works</h2>
        <div className="blocks-grid">
          <div className="block">
            <div className="block-header">
              <span className="meta-label">--run</span>
              <span className="block-num">001</span>
            </div>
            <h3>Commands</h3>
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
        <div className="idea-meta">
          <span className="meta-label">every 6h</span>
          <span className="meta-label">in 30m</span>
          <span className="meta-label">0 9 * * 1-5</span>
          <span className="meta-label">2026-07-01T09:00</span>
        </div>
      </section>

      {/* Put it in a file — the yaml and the reconcile output carry it */}
      <section>
        <h2>Put It in a File</h2>
        <div className="workflow-list">
          <div className="workflow-item">
            <div className="workflow-code copyable">
              <CopyButton
                text={`name: my-jobs\njobs:\n  backup:\n    schedule: "every 6h"\n    run: "restic backup ~/"\n  morning-brief:\n    schedule: "0 9 * * 1-5"\n    prompt: "Summarize my unread PRs"\n  slack-ping:\n    schedule: "every 15m"\n    webhook: "https://hooks.slack.com/T/B/X"\n    headers:\n      Authorization: "Bearer \${SLACK_TOKEN}"`}
              />
              <pre>
                <span className="hl-comment"># schedx.yaml</span>
                {"\n"}
                <span className="hl-flag">name:</span>{" "}
                <span className="hl-val">my-jobs</span>
                {"\n"}
                <span className="hl-flag">jobs:</span>
                {"\n"}
                {"  "}
                <span className="hl-cmd">backup:</span>
                {"\n"}
                {"    "}
                <span className="hl-flag">schedule:</span>{" "}
                <span className="hl-val">&quot;every 6h&quot;</span>
                {"\n"}
                {"    "}
                <span className="hl-flag">run:</span>{" "}
                <span className="hl-str">&quot;restic backup ~/&quot;</span>
                {"\n"}
                {"  "}
                <span className="hl-cmd">morning-brief:</span>
                {"\n"}
                {"    "}
                <span className="hl-flag">schedule:</span>{" "}
                <span className="hl-val">&quot;0 9 * * 1-5&quot;</span>
                {"\n"}
                {"    "}
                <span className="hl-flag">prompt:</span>{" "}
                <span className="hl-str">&quot;Summarize my unread PRs&quot;</span>
                {"\n"}
                {"  "}
                <span className="hl-cmd">slack-ping:</span>
                {"\n"}
                {"    "}
                <span className="hl-flag">schedule:</span>{" "}
                <span className="hl-val">&quot;every 15m&quot;</span>
                {"\n"}
                {"    "}
                <span className="hl-flag">webhook:</span>{" "}
                <span className="hl-str">
                  &quot;https://hooks.slack.com/T/B/X&quot;
                </span>
                {"\n"}
                {"    "}
                <span className="hl-flag">headers:</span>
                {"\n"}
                {"      "}
                <span className="hl-flag">Authorization:</span>{" "}
                <span className="hl-str">&quot;Bearer ${"{"}SLACK_TOKEN{"}"}&quot;</span>
              </pre>
            </div>
            <div className="workflow-code">
              <pre>
                <span className="hl-cmd">$ schedx up</span>
                {"\n"}
                <span className="hl-op">+</span> create backup [run] (aovboa)
                {"\n"}
                <span className="hl-op">+</span> create morning-brief [prompt] (xuv4gm)
                {"\n"}
                <span className="hl-op">+</span> create slack-ping [webhook] (9ocepi)
                {"\n"}
                <span className="hl-comment">
                  Applied manifest &apos;my-jobs&apos;: 3 created, 0 updated,{"\n"}
                  0 drift-corrected, 0 recreated, 0 removed, 0 unchanged.
                </span>
                {"\n\n"}
                <span className="hl-cmd">$ schedx up</span>
                {"\n"}
                <span className="hl-comment">
                  No changes. 3 job(s) up to date for manifest &apos;my-jobs&apos;.
                </span>
                {"\n\n"}
                <span className="hl-cmd">$ schedx down</span>
                {"\n"}
                <span className="hl-op">-</span> removed backup [run] (aovboa)
                {"\n"}
                <span className="hl-op">-</span> removed morning-brief [prompt] (xuv4gm)
                {"\n"}
                <span className="hl-op">-</span> removed slack-ping [webhook] (9ocepi)
                {"\n"}
                <span className="hl-comment">
                  Brought down manifest &apos;my-jobs&apos;: 3 job(s) removed.
                </span>
              </pre>
            </div>
          </div>
        </div>
        <div className="idea-meta">
          <span className="meta-label">docker-compose for schedules</span>
        </div>
      </section>

      {/* Watch it work — real output of the real binary, captured as a fixture */}
      <section>
        <h2>Watch It Work</h2>
        <div className="workflow-code">
          <pre>
            <span className="hl-cmd">$ schedx list</span>
            {"\n"}
            {"  "}
            <span className="hl-val">9ocepi</span>
            {"   slack-ping                              "}
            <span className="hl-flag">active</span>
            {"  webhook\n"}
            {"           every 15m  (*/15 * * * *)\n"}
            {"           "}
            <span className="hl-comment">Next: 12th Jun at 10:30am (in 14m)</span>
            {"\n\n"}
            {"  "}
            <span className="hl-val">aovboa</span>
            {"   backup                                  "}
            <span className="hl-flag">active</span>
            {"  run\n"}
            {"           every 6h  (0 */6 * * *)\n"}
            {"           "}
            <span className="hl-comment">Next: 12th Jun at 12:00pm (in 1h and 44m)</span>
            {"\n\n"}
            {"  "}
            <span className="hl-val">xuv4gm</span>
            {"   morning-brief                           "}
            <span className="hl-flag">active</span>
            {"  prompt\n"}
            {"           cron 0 9 * * 1-5\n"}
            {"           "}
            <span className="hl-comment">Next: 14th Jun at 9:00am (in 1 day and 22h)</span>
          </pre>
        </div>
        <div className="idea-meta">
          <span className="meta-label">schedx logs</span>
          <span className="meta-label">schedx history</span>
          <span className="meta-label">schedx pause / resume / skip</span>
          <span className="meta-label">--json on everything</span>
        </div>
      </section>

      {/* Footer / Install */}
      <section className="footer-section">
        <div className="cta-group">
          <div className="install-box">
            <code>curl -sL schedx.run/get | bash</code>
            <CopyButton text="curl -sL schedx.run/get | bash" umamiEvent="copy-install-command" />
            <span>[Bin]</span>
          </div>
          <div className="install-box">
            <code>cargo install schedx --locked</code>
            <CopyButton text="cargo install schedx --locked" umamiEvent="copy-cargo-install" />
            <span>[Rust]</span>
          </div>
          <div className="footer-meta">
            MIT Licensed. Written in Rust. State is plain files. No daemon, no cloud.
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
            href="https://github.com/Alireza29675/schedx/blob/main/docs/REFERENCE.md"
            target="_blank"
            rel="noopener noreferrer"
          >
            Reference
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
