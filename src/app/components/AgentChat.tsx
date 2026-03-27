"use client";

import { useEffect, useState, useRef } from "react";

interface ChatLine {
  type: "user" | "agent" | "tool" | "gap";
  text: string;
}

const SCENES: ChatLine[][] = [
  [
    { type: "user", text: "Check security logs daily and flag anything suspicious" },
    { type: "tool", text: `schedx add "0 9 * * *" \\\n  --prompt "Audit auth logs, open ports,\n  and failed logins. Alert if off."` },
    { type: "agent", text: "Scheduled. I'll audit every morning at 9." },
  ],
  [
    { type: "user", text: "After this deploy, check if everything is healthy" },
    { type: "tool", text: `schedx add "in 30m" \\\n  --prompt "Hit /health, check error\n  rates. Report back."` },
    { type: "agent", text: "Got it. I'll check back in 30 minutes." },
  ],
  [
    { type: "user", text: "Every Friday, summarize what the team shipped" },
    { type: "tool", text: `schedx add "0 17 * * 5" \\\n  --prompt "Pull merged PRs, summarize\n  the week's progress."` },
    { type: "agent", text: "Done. Weekly digest every Friday at 5pm." },
  ],
  [
    { type: "user", text: "Back up the database every 6 hours" },
    { type: "tool", text: `schedx add "every 6h" \\\n  --run "pg_dump prod >\n  /backups/$(date +%s).sql"` },
    { type: "agent", text: "Scheduled. Backups every 6 hours." },
  ],
  [
    { type: "user", text: "Monitor our API costs and warn me if they spike" },
    { type: "tool", text: `schedx add "0 8 * * *" \\\n  --prompt "Check API spend vs yesterday.\n  Alert if up more than 20%."` },
    { type: "agent", text: "On it. I'll check every morning at 8." },
  ],
];

const CHAR_SPEED = 16;
const SCENE_HOLD = 5000;
const LINE_PAUSE = 300;
const TOOL_PAUSE = 400;

export function AgentChat() {
  const [lines, setLines] = useState<
    { type: string; text: string; charCount: number; fading: boolean }[]
  >([]);
  const cancelRef = useRef(false);

  useEffect(() => {
    cancelRef.current = false;
    const sleep = (ms: number) =>
      new Promise<void>((r) => {
        const id = setTimeout(r, ms);
        const check = setInterval(() => {
          if (cancelRef.current) { clearTimeout(id); clearInterval(check); r(); }
        }, 50);
      });

    async function typeLine(type: string, text: string) {
      if (cancelRef.current) return;
      setLines((prev) => [...prev, { type, text, charCount: 0, fading: false }]);

      const speed = type === "tool" ? CHAR_SPEED * 0.4 : CHAR_SPEED;
      for (let c = 1; c <= text.length; c++) {
        if (cancelRef.current) return;
        await sleep(speed);
        setLines((prev) => {
          const next = [...prev];
          next[next.length - 1] = { ...next[next.length - 1], charCount: c };
          return next;
        });
      }
    }

    async function fadeAndClear() {
      if (cancelRef.current) return;
      setLines((prev) => prev.map((l) => ({ ...l, fading: true })));
      await sleep(600);
      if (!cancelRef.current) setLines([]);
    }

    async function run() {
      let i = 0;
      while (!cancelRef.current) {
        const scene = SCENES[i % SCENES.length];

        // User message
        await typeLine("user", scene[0].text);
        await sleep(LINE_PAUSE);

        // Tool use
        await sleep(TOOL_PAUSE);
        await typeLine("tool", scene[1].text);
        await sleep(LINE_PAUSE);

        // Agent confirmation
        await typeLine("agent", scene[2].text);

        // Hold the completed scene
        await sleep(SCENE_HOLD);

        // Fade out
        await fadeAndClear();
        await sleep(400);

        i++;
      }
    }

    run();
    return () => { cancelRef.current = true; };
  }, []);

  return (
    <div className="agent-chat">
      <div className="agent-chat-header">
        <div className="agent-chat-dots">
          <span />
          <span />
          <span />
        </div>
        <span className="agent-chat-title">claude code</span>
      </div>
      <div className="agent-chat-body">
        {lines.map((line, i) => {
          const displayed = line.text.slice(0, line.charCount);
          const isTyping = line.charCount < line.text.length;
          const fadeClass = line.fading ? " chat-fade" : "";

          if (line.type === "user") {
            return (
              <div key={i} className={`chat-line chat-user${fadeClass}`}>
                <span className="chat-prompt">&gt;</span>
                <span>
                  {displayed}
                  {isTyping && <span className="chat-cursor" />}
                </span>
              </div>
            );
          }

          if (line.type === "tool") {
            return (
              <div key={i} className={`chat-line chat-tool-wrap${fadeClass}`}>
                <div className="chat-tool-label">
                  <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                    <polyline points="4 17 10 11 4 5" />
                    <line x1="12" y1="19" x2="20" y2="19" />
                  </svg>
                  <span>bash</span>
                </div>
                <pre className="chat-tool-code">
                  {displayed}
                  {isTyping && <span className="chat-cursor" />}
                </pre>
              </div>
            );
          }

          if (line.type === "agent") {
            return (
              <div key={i} className={`chat-line chat-agent${fadeClass}`}>
                <span>
                  {displayed}
                  {isTyping && <span className="chat-cursor" />}
                </span>
              </div>
            );
          }

          return null;
        })}
      </div>
    </div>
  );
}
