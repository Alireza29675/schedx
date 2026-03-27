import { useMemo, useState, useCallback } from "react";
import { useParams, Link } from "react-router-dom";
import {
  Copy,
  DownloadSimple,
  ArrowLeft,
  Clock,
  HandPointing,
} from "@phosphor-icons/react";
import { useApi } from "../hooks/use-api";
import { runsApi } from "../api";
import type { RunRecord } from "../types";
import { StatusBadge } from "../components/shared/status-badge";
import { Button } from "../components/ui/button";
import { formatDurationBetween, formatDateTime } from "../lib/format";
import { STATUS_LABELS } from "../lib/constants";
import { cn } from "../lib/cn";

function getStatusExplanation(record: RunRecord): string {
  switch (record.status) {
    case "success":
      return "Completed successfully (exit code 0)";
    case "failed":
      return `Failed with exit code ${record.exit_code ?? "unknown"}`;
    case "timeout":
      return "Killed after exceeding timeout";
    case "skipped_overlap":
      return "Skipped — previous run still executing";
    case "internal_error":
      return "Internal scheduler error";
    default:
      return STATUS_LABELS[record.status] ?? record.status;
  }
}

function MetaItem({
  label,
  children,
}: {
  label: string;
  children: React.ReactNode;
}) {
  return (
    <div className="flex flex-col gap-1">
      <span className="text-xs text-text-muted uppercase tracking-wider">
        {label}
      </span>
      <span className="text-sm text-text-primary">{children}</span>
    </div>
  );
}

function CopyButton({
  text,
  label = "Copy",
}: {
  text: string;
  label?: string;
}) {
  const [copied, setCopied] = useState(false);

  const handleCopy = useCallback(() => {
    navigator.clipboard.writeText(text);
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  }, [text]);

  return (
    <button
      onClick={handleCopy}
      className="inline-flex items-center gap-1 text-text-muted hover:text-text-primary transition-colors"
      title={label}
    >
      <Copy size={14} />
      {copied && <span className="text-xs text-green-500">Copied</span>}
    </button>
  );
}

function LogViewer({ runId }: { runId: string }) {
  const { data: logContent, loading } = useApi(
    () => runsApi.getLogs(runId),
    [runId],
  );
  const [logCopied, setLogCopied] = useState(false);

  const lines = useMemo(() => {
    if (!logContent) return [];
    return logContent.split("\n");
  }, [logContent]);

  const handleCopyLog = useCallback(() => {
    if (!logContent) return;
    navigator.clipboard.writeText(logContent);
    setLogCopied(true);
    setTimeout(() => setLogCopied(false), 1500);
  }, [logContent]);

  const handleDownload = useCallback(() => {
    if (!logContent) return;
    const blob = new Blob([logContent], { type: "text/plain" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `run-${runId}.log`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
  }, [logContent, runId]);

  return (
    <div className="bg-bg-panel border border-border rounded-lg flex flex-col flex-1 overflow-hidden">
      <div className="flex items-center justify-between px-4 py-3 border-b border-border">
        <h2 className="text-sm font-medium text-text-primary">Output</h2>
        <div className="flex items-center gap-2">
          <Button
            variant="ghost"
            onClick={handleCopyLog}
            disabled={!logContent}
          >
            <Copy size={14} />
            {logCopied ? "Copied" : "Copy Log"}
          </Button>
          <Button
            variant="ghost"
            onClick={handleDownload}
            disabled={!logContent}
          >
            <DownloadSimple size={14} />
            Download
          </Button>
        </div>
      </div>

      <div className="flex-1 overflow-auto bg-[#080808]">
        {loading ? (
          <div className="p-4 text-text-muted text-sm font-mono animate-pulse">
            Loading logs...
          </div>
        ) : !logContent ? (
          <div className="p-4 text-text-muted text-sm italic">
            No output captured for this run.
          </div>
        ) : (
          <div className="font-mono text-xs leading-5">
            {lines.map((line, i) => (
              <div
                key={i}
                className="flex hover:bg-white/[0.03] transition-colors"
              >
                <span className="w-[50px] shrink-0 text-right pr-3 py-0.5 text-text-muted/50 select-none border-r border-white/[0.06] bg-white/[0.02]">
                  {i + 1}
                </span>
                <span className="pl-3 py-0.5 text-text-secondary whitespace-pre-wrap break-all">
                  {line}
                </span>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

export default function RunDetailPage() {
  const { runId } = useParams<{ runId: string }>();

  const { data: record, loading } = useApi(
    () => runsApi.get(runId!),
    [runId],
  );

  if (loading) {
    return (
      <div className="flex flex-col gap-4 h-full animate-pulse">
        <div className="h-6 w-48 bg-bg-panel rounded" />
        <div className="h-40 bg-bg-panel rounded-lg" />
        <div className="flex-1 bg-bg-panel rounded-lg" />
      </div>
    );
  }

  if (!record) {
    return (
      <div className="flex flex-col items-center justify-center h-full gap-4">
        <p className="text-text-muted">Run not found.</p>
        <Link to="/history">
          <Button variant="secondary">
            <ArrowLeft size={14} />
            Back to History
          </Button>
        </Link>
      </div>
    );
  }

  const isScheduled = record.trigger === "scheduled";

  return (
    <div className="flex flex-col gap-4 h-full">
      {/* Header */}
      <div className="flex items-center gap-3">
        <Link
          to="/history"
          className="text-text-muted hover:text-text-primary transition-colors"
        >
          <ArrowLeft size={20} />
        </Link>
        <h1 className="text-xl font-semibold text-text-primary">
          Run{" "}
          <span className="font-mono text-text-code">
            {record.run_id.slice(0, 8)}
          </span>
        </h1>
      </div>

      {/* Metadata */}
      <div className="bg-bg-panel border border-border rounded-lg p-5">
        <div className="grid grid-cols-2 md:grid-cols-4 gap-x-8 gap-y-5">
          <MetaItem label="Run ID">
            <span className="inline-flex items-center gap-2">
              <span className="font-mono text-text-code">
                {record.run_id}
              </span>
              <CopyButton text={record.run_id} label="Copy Run ID" />
            </span>
          </MetaItem>

          <MetaItem label="Job">
            <Link
              to={`/jobs/${record.job_id}`}
              className="text-accent hover:underline"
            >
              {record.job_name}
            </Link>
          </MetaItem>

          <MetaItem label="Trigger">
            <span className="inline-flex items-center gap-1.5 px-2 py-0.5 text-xs border border-border rounded-md">
              {isScheduled ? (
                <Clock size={12} />
              ) : (
                <HandPointing size={12} />
              )}
              {isScheduled ? "Scheduled" : "Manual"}
            </span>
          </MetaItem>

          <MetaItem label="Scheduled For">
            {record.scheduled_for
              ? formatDateTime(record.scheduled_for)
              : "-"}
          </MetaItem>

          <MetaItem label="Started">
            {record.started_at ? formatDateTime(record.started_at) : "-"}
          </MetaItem>

          <MetaItem label="Finished">
            {record.finished_at ? formatDateTime(record.finished_at) : "-"}
          </MetaItem>

          <MetaItem label="Duration">
            {record.started_at && record.finished_at
              ? formatDurationBetween(record.started_at, record.finished_at)
              : "-"}
          </MetaItem>

          <MetaItem label="Status">
            <div className="flex flex-col gap-1">
              <StatusBadge status={record.status} />
              <span className="text-xs text-text-muted">
                {getStatusExplanation(record)}
              </span>
            </div>
          </MetaItem>

          <MetaItem label="Exit Code">
            <span
              className={cn(
                "font-mono",
                record.exit_code === 0
                  ? "text-text-muted"
                  : record.exit_code != null
                    ? "text-red-400"
                    : "text-text-muted",
              )}
            >
              {record.exit_code != null ? record.exit_code : "-"}
            </span>
          </MetaItem>
        </div>
      </div>

      {/* Log viewer */}
      <LogViewer runId={runId!} />
    </div>
  );
}
