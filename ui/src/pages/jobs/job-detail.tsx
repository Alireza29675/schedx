import { useParams, useNavigate, Link } from "react-router-dom";
import { useMemo, useState } from "react";
import {
  Play,
  Pause,
  PlayCircle,
  Trash,
  PencilSimple,
  FastForward,
  Copy,
  ArrowLeft,
  Terminal,
  Robot,
  Globe,
  Clock,
} from "@phosphor-icons/react";
import { useApi } from "../../hooks/use-api";
import { useSSE } from "../../hooks/use-sse";
import { useToast } from "../../hooks/use-toast";
import { jobsApi, historyApi } from "../../api";
import type {
  Job,
  RunRecord,
  CommandAction,
  PromptAction,
  WebhookAction,
} from "../../types";
import { JobStatusBadge } from "../../components/shared/job-status-badge";
import { StatusBadge } from "../../components/shared/status-badge";
import { ActionTypeIcon } from "../../components/shared/action-type-icon";
import { TimeAgo } from "../../components/shared/time-ago";
import { Button } from "../../components/ui/button";
import { Tag } from "../../components/ui/tag";
import { ConfirmDialog } from "../../components/ui/confirm-dialog";
import { Skeleton } from "../../components/ui/skeleton";
import { EmptyState } from "../../components/ui/empty-state";
import {
  formatDuration,
  formatDurationBetween,
} from "../../lib/format";
import { cn } from "../../lib/cn";

export default function JobDetail() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const { addToast } = useToast();

  const [skipCount, setSkipCount] = useState(1);
  const [showDeleteDialog, setShowDeleteDialog] = useState(false);
  const [actionLoading, setActionLoading] = useState<string | null>(null);

  const {
    data: job,
    loading: jobLoading,
    error: jobError,
    refetch: refetchJob,
  } = useApi<Job>(() => jobsApi.get(id!), [id]);

  const {
    data: historyData,
    loading: historyLoading,
    refetch: refetchHistory,
  } = useApi<{ records: RunRecord[]; total: number }>(
    () => historyApi.listForJob(id!, { limit: 10, offset: 0 }),
    [id]
  );

  useSSE((_event, data) => {
    try {
      const parsed = JSON.parse(data);
      if (parsed.job_id === id) {
        refetchJob();
        refetchHistory();
      }
    } catch {
      // ignore unparseable events
    }
  });

  const records = historyData?.records ?? [];

  const stats = useMemo(() => {
    if (!historyData) return null;
    const all = historyData.records;
    const total = historyData.total;
    const success = all.filter((r) => r.status === "success").length;
    const failed = all.filter((r) => r.status === "failed").length;
    const timeout = all.filter((r) => r.status === "timeout").length;
    const skipped = all.filter((r) => r.status === "skipped_overlap").length;
    const durations = all
      .filter((r) => r.started_at && r.finished_at)
      .map((r) => new Date(r.finished_at!).getTime() - new Date(r.started_at!).getTime());
    const avgDuration = durations.length > 0 ? durations.reduce((a, b) => a + b, 0) / durations.length : 0;
    const lastRun = all.length > 0 ? all[0] : null;
    const successRate = total > 0 ? Math.round((success / total) * 100) : 0;

    return { total, success, failed, timeout, skipped, avgDuration, lastRun, successRate };
  }, [historyData]);

  async function handleAction(action: string, fn: () => Promise<unknown>) {
    setActionLoading(action);
    try {
      await fn();
      refetchJob();
      refetchHistory();
      if (action === "delete") {
        addToast("success", "Job deleted");
        navigate("/jobs");
      } else {
        addToast("success", `Job ${action} successful`);
      }
    } catch (err: any) {
      addToast("error", err?.message ?? `Failed to ${action} job`);
    } finally {
      setActionLoading(null);
    }
  }

  function copyId() {
    if (!id) return;
    navigator.clipboard.writeText(id);
    addToast("success", "Copied to clipboard");
  }

  if (jobLoading) {
    return (
      <div className="space-y-6">
        <Skeleton className="h-10 w-64" />
        <div className="grid grid-cols-3 gap-6">
          <div className="col-span-2 space-y-6">
            <Skeleton className="h-40" />
            <Skeleton className="h-60" />
            <Skeleton className="h-80" />
          </div>
          <div className="space-y-6">
            <Skeleton className="h-64" />
            <Skeleton className="h-48" />
          </div>
        </div>
      </div>
    );
  }

  if (jobError || !job) {
    return (
      <EmptyState
        title="Job not found"
        description="The job you're looking for doesn't exist or has been removed."
        action={
          <Button onClick={() => navigate("/jobs")}>
            <ArrowLeft size={16} />
            Back to Jobs
          </Button>
        }
      />
    );
  }

  const action = job.action;
  const isPaused = job.status === "paused";

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="space-y-3">
        <Link
          to="/jobs"
          className="inline-flex items-center gap-1.5 text-sm text-text-secondary hover:text-text-primary transition-colors"
        >
          <ArrowLeft size={14} />
          Jobs
        </Link>

        <div className="flex items-start justify-between gap-4">
          <div className="space-y-2">
            <div className="flex items-center gap-3 flex-wrap">
              <h1 className="font-heading text-2xl text-text-primary">{job.name}</h1>
              <JobStatusBadge status={job.status} />
              <ActionTypeIcon type={action.type} />
            </div>

            <div className="flex items-center gap-2">
              <code className="text-xs font-mono text-text-tertiary">{job.id}</code>
              <button
                onClick={copyId}
                className="text-text-tertiary hover:text-text-secondary transition-colors"
                title="Copy ID"
              >
                <Copy size={14} />
              </button>
            </div>

            {job.tags && job.tags.length > 0 && (
              <div className="flex items-center gap-1.5 flex-wrap">
                {job.tags.map((tag) => (
                  <Tag key={tag}>{tag}</Tag>
                ))}
              </div>
            )}
          </div>
        </div>
      </div>

      {/* Grid */}
      <div className="grid grid-cols-3 gap-6">
        {/* Main content */}
        <div className="col-span-2 space-y-6">
          {/* Schedule */}
          <section className="bg-bg-panel border border-border rounded-lg p-5 space-y-4">
            <h2 className="label-mono">Schedule</h2>

            <div className="space-y-3">
              <div className="flex items-center gap-2">
                <Clock size={16} className="text-text-tertiary" />
                <code className="text-sm font-mono text-text-primary">{job.schedule_input}</code>
              </div>



              {job.skip_remaining != null && job.skip_remaining > 0 && (
                <div className="flex items-center gap-2 text-sm">
                  <FastForward size={16} className="text-amber-500" />
                  <span className="text-amber-500">
                    Skipping next {job.skip_remaining} run{job.skip_remaining > 1 ? "s" : ""}
                  </span>
                </div>
              )}
            </div>
          </section>

          {/* Action Configuration */}
          <section className="bg-bg-panel border border-border rounded-lg p-5 space-y-4">
            <h2 className="label-mono">Action</h2>
            <ActionDetail action={action} />
          </section>

          {/* Recent Runs */}
          <section className="bg-bg-panel border border-border rounded-lg overflow-hidden">
            <div className="flex items-center justify-between p-5 pb-0">
              <h2 className="label-mono">Recent Runs</h2>
              <Link
                to={`/history?job_id=${job.id}`}
                className="text-sm text-text-secondary hover:text-text-primary transition-colors"
              >
                View All
              </Link>
            </div>

            {historyLoading ? (
              <div className="p-5 space-y-3">
                {Array.from({ length: 5 }).map((_, i) => (
                  <Skeleton key={i} className="h-10" />
                ))}
              </div>
            ) : records.length === 0 ? (
              <div className="p-5">
                <p className="text-sm text-text-tertiary text-center py-8">No runs yet</p>
              </div>
            ) : (
              <div className="overflow-x-auto">
                <table className="w-full text-sm">
                  <thead>
                    <tr className="border-b border-border text-text-tertiary text-left">
                      <th className="px-5 py-3 font-medium">Run ID</th>
                      <th className="px-5 py-3 font-medium">Trigger</th>
                      <th className="px-5 py-3 font-medium">Started</th>
                      <th className="px-5 py-3 font-medium">Duration</th>
                      <th className="px-5 py-3 font-medium">Status</th>
                      <th className="px-5 py-3 font-medium">Exit Code</th>
                    </tr>
                  </thead>
                  <tbody>
                    {records.map((run) => (
                      <tr
                        key={run.run_id}
                        onClick={() => navigate(`/runs/${run.run_id}`)}
                        className="border-b border-border last:border-0 hover:bg-bg-hover cursor-pointer transition-colors"
                      >
                        <td className="px-5 py-3 font-mono text-xs text-text-secondary">
                          {run.run_id.slice(0, 8)}
                        </td>
                        <td className="px-5 py-3 text-text-secondary capitalize">
                          {run.trigger}
                        </td>
                        <td className="px-5 py-3 text-text-secondary">
                          {run.started_at ? <TimeAgo date={run.started_at} /> : "—"}
                        </td>
                        <td className="px-5 py-3 text-text-secondary font-mono text-xs">
                          {run.started_at && run.finished_at
                            ? formatDurationBetween(run.started_at, run.finished_at)
                            : run.started_at
                            ? "Running..."
                            : "—"}
                        </td>
                        <td className="px-5 py-3">
                          <StatusBadge status={run.status} />
                        </td>
                        <td className="px-5 py-3 font-mono text-xs text-text-secondary">
                          {run.exit_code != null ? run.exit_code : "—"}
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            )}
          </section>
        </div>

        {/* Sidebar */}
        <div className="space-y-6">
          {/* Actions */}
          <section className="bg-bg-panel border border-border rounded-lg p-5 space-y-3">
            <h2 className="label-mono mb-4">Actions</h2>

            {isPaused ? (
              <Button
                variant="primary"
                className="w-full"
                onClick={() => handleAction("resume", () => jobsApi.resume(id!))}
                disabled={actionLoading !== null}
              >
                <Play size={16} />
                Resume Job
              </Button>
            ) : (
              <Button
                variant="secondary"
                className="w-full"
                onClick={() => handleAction("pause", () => jobsApi.pause(id!))}
                disabled={actionLoading !== null}
              >
                <Pause size={16} />
                Pause Job
              </Button>
            )}

            <Button
              variant="secondary"
              className="w-full"
              onClick={() => handleAction("run", () => jobsApi.run(id!))}
              disabled={actionLoading !== null}
            >
              <PlayCircle size={16} />
              Run Now
            </Button>

            <div className="flex items-center gap-2">
              <input
                type="number"
                min={1}
                max={100}
                value={skipCount}
                onChange={(e) => setSkipCount(Math.max(1, parseInt(e.target.value) || 1))}
                className="flex-1 bg-bg-input border border-border rounded-md px-3 py-2 text-sm font-mono text-text-primary focus:outline-none focus:ring-1 focus:ring-border-focus"
              />
              <Button
                variant="secondary"
                onClick={() => handleAction("skip", () => jobsApi.skip(id!, skipCount))}
                disabled={actionLoading !== null}
              >
                <FastForward size={16} />
                Skip
              </Button>
            </div>

            <hr className="border-border" />

            <Button
              variant="secondary"
              className="w-full"
              onClick={() => navigate(`/jobs/${id}/edit`)}
            >
              <PencilSimple size={16} />
              Edit Job
            </Button>

            <Button
              variant="danger"
              className="w-full"
              onClick={() => setShowDeleteDialog(true)}
              disabled={actionLoading !== null}
            >
              <Trash size={16} />
              Delete Job
            </Button>
          </section>

          {/* Run Statistics */}
          <section className="bg-bg-panel border border-border rounded-lg p-5 space-y-4">
            <h2 className="label-mono">Run Statistics</h2>

            {stats ? (
              <div className="space-y-3">
                <StatRow label="Total Runs" value={String(stats.total)} />
                <StatRow
                  label="Success"
                  value={String(stats.success)}
                  valueClass="text-emerald-500"
                />
                <StatRow
                  label="Failed"
                  value={String(stats.failed)}
                  valueClass="text-red-500"
                />
                <StatRow
                  label="Timeout"
                  value={String(stats.timeout)}
                  valueClass="text-amber-500"
                />
                <StatRow
                  label="Skipped"
                  value={String(stats.skipped)}
                  valueClass="text-text-tertiary"
                />
                <hr className="border-border" />
                <StatRow
                  label="Success Rate"
                  value={`${stats.successRate}%`}
                  valueClass={stats.successRate >= 90 ? "text-emerald-500" : stats.successRate >= 50 ? "text-amber-500" : "text-red-500"}
                />
                <StatRow
                  label="Avg Duration"
                  value={stats.avgDuration > 0 ? formatDuration(stats.avgDuration) : "—"}
                />
                {stats.lastRun && (
                  <div className="pt-1">
                    <p className="text-xs text-text-tertiary mb-1">Last Run</p>
                    <div className="flex items-center gap-2">
                      <StatusBadge status={stats.lastRun.status} />
                      <span className="text-xs text-text-tertiary">
                        <TimeAgo date={stats.lastRun.started_at ?? stats.lastRun.scheduled_for} />
                      </span>
                    </div>
                  </div>
                )}
              </div>
            ) : (
              <p className="text-sm text-text-tertiary">No data yet</p>
            )}
          </section>
        </div>
      </div>

      {/* Delete confirmation */}
      <ConfirmDialog
        open={showDeleteDialog}
        onCancel={() => setShowDeleteDialog(false)}
        onConfirm={() => handleAction("delete", () => jobsApi.remove(id!))}
        title="Delete Job"
        description={`Are you sure you want to delete "${job.name}"? This action cannot be undone. All run history for this job will also be removed.`}
        confirmLabel="Delete"
        variant="danger"
      />
    </div>
  );
}

function ActionDetail({ action }: { action: Job["action"] }) {
  switch (action.type) {
    case "command": {
      const cmd = action as CommandAction;
      return (
        <div className="space-y-3">
          <div className="flex items-center gap-2 text-sm text-text-secondary">
            <Terminal size={16} />
            <span>Command</span>
            {cmd.shell && (
              <span className="text-xs bg-bg-subtle border border-border rounded px-1.5 py-0.5 font-mono">
                {cmd.shell}
              </span>
            )}
          </div>
          <pre className="bg-bg-subtle border border-border rounded-md p-4 text-sm font-mono text-text-primary overflow-x-auto whitespace-pre-wrap break-all">
            {cmd.command}
          </pre>
          {cmd.workdir && (
            <div className="flex items-center gap-2 text-sm text-text-tertiary">
              <span>Working directory:</span>
              <code className="font-mono text-text-secondary">{cmd.workdir}</code>
            </div>
          )}
        </div>
      );
    }
    case "prompt": {
      const prompt = action as PromptAction;
      return (
        <div className="space-y-3">
          <div className="flex items-center gap-2 text-sm text-text-secondary">
            <Robot size={16} />
            <span>Prompt</span>
            {prompt.agent && (
              <span className="text-xs bg-bg-subtle border border-border rounded px-1.5 py-0.5">
                {prompt.agent}
              </span>
            )}
          </div>
          <div className="bg-bg-subtle border border-border rounded-md p-4 text-sm text-text-primary whitespace-pre-wrap">
            {prompt.text}
          </div>
        </div>
      );
    }
    case "webhook": {
      const webhook = action as WebhookAction;
      return (
        <div className="space-y-3">
          <div className="flex items-center gap-2 text-sm text-text-secondary">
            <Globe size={16} />
            <span>Webhook</span>
          </div>
          <div className="flex items-center gap-2">
            <span className="text-xs font-mono font-bold bg-bg-subtle border border-border rounded px-2 py-1 uppercase">
              {webhook.method}
            </span>
            <code className="text-sm font-mono text-text-primary break-all">{webhook.url}</code>
          </div>
          {webhook.headers && Object.keys(webhook.headers).length > 0 && (
            <div className="space-y-1">
              <p className="text-xs text-text-tertiary font-medium">Headers</p>
              <div className="bg-bg-subtle border border-border rounded-md p-3 space-y-1">
                {Object.entries(webhook.headers).map(([key, value]) => (
                  <div key={key} className="text-xs font-mono">
                    <span className="text-text-secondary">{key}:</span>{" "}
                    <span className="text-text-tertiary">{value}</span>
                  </div>
                ))}
              </div>
            </div>
          )}
          {webhook.body && (
            <div className="space-y-1">
              <p className="text-xs text-text-tertiary font-medium">Body</p>
              <pre className="bg-bg-subtle border border-border rounded-md p-3 text-xs font-mono text-text-primary overflow-x-auto whitespace-pre-wrap break-all">
                {typeof webhook.body === "string"
                  ? webhook.body
                  : JSON.stringify(webhook.body, null, 2)}
              </pre>
            </div>
          )}
        </div>
      );
    }
    default:
      return (
        <p className="text-sm text-text-tertiary">Unknown action type</p>
      );
  }
}

function StatRow({
  label,
  value,
  valueClass,
}: {
  label: string;
  value: string;
  valueClass?: string;
}) {
  return (
    <div className="flex items-center justify-between text-sm">
      <span className="text-text-secondary">{label}</span>
      <span className={cn("font-mono", valueClass ?? "text-text-primary")}>{value}</span>
    </div>
  );
}
