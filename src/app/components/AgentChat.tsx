"use client";

import { useEffect, useState } from "react";

type LineKind = "user" | "tool" | "result" | "agent" | "divider" | "report" | "report-warn";

interface Step {
  kind: LineKind | "idle" | "dim";
  text?: string;
  mode?: "type" | "stamp";
  delayAfter: number;
}

// One scenario, told once: you ask, the agent schedules, then day-stamped
// reports keep arriving next to a cursor that never types again.
// The ⎿ line is byte-exact output of the real binary (fixture note in
// memory/tasks/schedx/2026-06-12-comms-revamp/TASK.md).
const TIMELINE: Step[] = [
  { kind: "user", text: "Check security logs every morning and flag anything suspicious", mode: "type", delayAfter: 500 },
  { kind: "tool", text: `Bash(schedx add "0 9 * * *" --name security-audit --prompt "Audit auth logs. Flag anything off.")`, mode: "stamp", delayAfter: 700 },
  { kind: "result", text: "Created job security-audit (cvbhlp). Next run: 13th Jun at 9:00am (in 22h and 17m)", mode: "stamp", delayAfter: 500 },
  { kind: "agent", text: "Scheduled. I'll audit every morning at 9.", mode: "type", delayAfter: 200 },
  { kind: "idle", delayAfter: 1500 },
  { kind: "dim", delayAfter: 0 },
  { kind: "divider", text: "sat 13 jun · 09:00", mode: "stamp", delayAfter: 150 },
  { kind: "report", text: "Audit clean. 0 failed logins, no new open ports. (run cpigad)", mode: "stamp", delayAfter: 1850 },
  { kind: "divider", text: "sun 14 jun · 09:00", mode: "stamp", delayAfter: 150 },
  { kind: "report", text: "Clean. New open port 5432 matches your deploy. Noted.", mode: "stamp", delayAfter: 1850 },
  { kind: "divider", text: "mon 15 jun · 09:00", mode: "stamp", delayAfter: 150 },
  { kind: "report-warn", text: "14 failed root logins from 1 IP — flagged. Wrote audit-report.md.", mode: "stamp", delayAfter: 5000 },
];

const TYPE_SPEED_USER = 38;
const TYPE_SPEED_AGENT = 24;
const FADE_MS = 600;
const RESTART_GAP_MS = 1000;

const ARIA_LABEL =
  "Animated Claude Code transcript: you ask once to check security logs every morning, " +
  "the agent schedules it with schedx, and day-stamped audit reports then arrive on " +
  "their own next to an idle cursor.";

// The conversation dims when the first report fires; reports never dim.
const DIMMABLE: ReadonlySet<LineKind> = new Set(["user", "tool", "result", "agent"]);

interface Line {
  kind: LineKind;
  text: string;
  charCount: number;
  stamped: boolean;
}

// Beat 8 (the held composite) as a static frame — every line shown, the ask
// tail dimmed, the idle cursor present. Used verbatim for reduced-motion.
const HELD_LINES: Line[] = TIMELINE.filter(
  (s): s is Step & { kind: LineKind } => s.kind !== "idle" && s.kind !== "dim",
).map((s) => ({ kind: s.kind, text: s.text ?? "", charCount: (s.text ?? "").length, stamped: true }));

export function AgentChat() {
  const [lines, setLines] = useState<Line[]>([]);
  const [dimmed, setDimmed] = useState(false);
  const [idleVisible, setIdleVisible] = useState(false);
  const [fading, setFading] = useState(false);

  useEffect(() => {
    // Reduced motion: render the spec's held composite frame statically and
    // never start the loop. The frame is designed to tell the whole story on
    // its own, so this is the on-concept opt-out (WCAG 2.2.2).
    if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) {
      setLines(HELD_LINES);
      setDimmed(true);
      setIdleVisible(true);
      return;
    }

    // Cancellation is per effect instance: a strict-mode remount must not be
    // able to revive the previous instance's loop (a shared ref reset would).
    let cancelled = false;
    // A cancelled sleep never resolves — its suspended async frame is GC'd,
    // so the loop simply stops at the next await. No post-cancel state writes,
    // and no leaked timers (every pending id is cleared on teardown).
    const pending = new Set<ReturnType<typeof setTimeout>>();
    const sleep = (ms: number) =>
      new Promise<void>((resolve) => {
        const id = setTimeout(() => {
          pending.delete(id);
          resolve();
        }, ms);
        pending.add(id);
      });

    async function typeLine(kind: LineKind, text: string, speed: number) {
      setLines((prev) => [...prev, { kind, text, charCount: 0, stamped: false }]);
      for (let c = 1; c <= text.length; c++) {
        await sleep(speed);
        setLines((prev) => {
          const next = [...prev];
          next[next.length - 1] = { ...next[next.length - 1], charCount: c };
          return next;
        });
      }
    }

    function stampLine(kind: LineKind, text: string) {
      setLines((prev) => [...prev, { kind, text, charCount: text.length, stamped: true }]);
    }

    async function run() {
      // Clear any residue a torn-down predecessor instance left behind.
      setLines([]);
      setDimmed(false);
      setIdleVisible(false);
      setFading(false);
      while (!cancelled) {
        for (const step of TIMELINE) {
          if (step.kind === "idle") {
            setIdleVisible(true);
          } else if (step.kind === "dim") {
            setDimmed(true);
          } else if (step.mode === "type") {
            const speed = step.kind === "user" ? TYPE_SPEED_USER : TYPE_SPEED_AGENT;
            await typeLine(step.kind, step.text ?? "", speed);
          } else {
            stampLine(step.kind, step.text ?? "");
          }
          await sleep(step.delayAfter);
        }
        // Loop seam: the body alone fades; the header persists, so the pane
        // reads as one continuous device rather than a restarting GIF.
        setFading(true);
        await sleep(FADE_MS);
        setLines([]);
        setDimmed(false);
        setIdleVisible(false);
        setFading(false);
        await sleep(RESTART_GAP_MS);
      }
    }

    // The loop is the whole pane; a stray throw should cost one cycle, not the
    // session. Restart after a beat unless we were torn down.
    function start() {
      run().catch(() => {
        if (!cancelled) setTimeout(start, RESTART_GAP_MS);
      });
    }
    start();

    return () => {
      cancelled = true;
      pending.forEach(clearTimeout);
      pending.clear();
    };
  }, []);

  return (
    <div className="agent-chat" role="img" aria-label={ARIA_LABEL}>
      <div className="agent-chat-header" aria-hidden="true">
        <div className="agent-chat-dots">
          <span />
          <span />
          <span />
        </div>
        <span className="agent-chat-title">claude code</span>
      </div>
      <div className={`agent-chat-body${fading ? " chat-fade" : ""}`}>
        {lines.map((line, i) => {
          const displayed = line.text.slice(0, line.charCount);
          const isTyping = !line.stamped && line.charCount < line.text.length;
          const dimClass = dimmed && DIMMABLE.has(line.kind) ? " chat-dim" : "";
          const stampClass = line.stamped ? " chat-stamp" : "";

          if (line.kind === "user") {
            return (
              <div key={i} className={`chat-line chat-user${dimClass}`}>
                <span className="chat-prompt">&gt;</span>
                <span>
                  {displayed}
                  {isTyping && <span className="chat-cursor" />}
                </span>
              </div>
            );
          }

          if (line.kind === "divider") {
            return (
              <div key={i} className={`chat-line chat-divider${stampClass}`}>
                <span>{line.text}</span>
              </div>
            );
          }

          const kindClass = {
            tool: "chat-tool-call",
            result: "chat-result",
            agent: "chat-agent-line",
            report: "chat-report",
            "report-warn": "chat-report chat-warn",
          }[line.kind];
          const dotClass = {
            tool: " chat-dot--ok",
            result: "",
            agent: "",
            report: " chat-dot--ok",
            "report-warn": " chat-dot--warn",
          }[line.kind];

          return (
            <div key={i} className={`chat-line ${kindClass}${dimClass}${stampClass}`}>
              <span className={`chat-dot${dotClass}`}>{line.kind === "result" ? "⎿" : "⏺"}</span>{" "}
              {displayed}
              {isTyping && <span className="chat-cursor" />}
            </div>
          );
        })}
        {idleVisible && (
          <div className="chat-line chat-user chat-stamp">
            <span className="chat-prompt">&gt;</span>
            <span>
              <span className="chat-cursor" />
            </span>
          </div>
        )}
      </div>
    </div>
  );
}
