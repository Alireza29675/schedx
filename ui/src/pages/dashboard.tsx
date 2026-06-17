import { useMemo, useCallback } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import {
  Pause,
  CheckCircle,
  ArrowRight,
  Clock,
  Lightning,
  Timer,
  Heartbeat,
  WarningCircle,
  CalendarBlank,
  Spinner,
} from '@phosphor-icons/react';
import { useApi } from '../hooks/use-api';
import { useSSE } from '../hooks/use-sse';
import { jobsApi, historyApi, configApi } from '../api';
import type { JobListEntry, RunRecord, SystemStatus } from '../types';
import { StatusBadge } from '../components/shared/status-badge';
import { TimeAgo } from '../components/shared/time-ago';
import { Skeleton } from '../components/ui/skeleton';
import { Button } from '../components/ui/button';
import { EmptyState } from '../components/ui/empty-state';
import { formatDurationBetween, formatCountdown, timeAgo } from '../lib/format';
import { cn } from '../lib/cn';

function isToday(dateStr: string): boolean {
  const d = new Date(dateStr);
  const now = new Date();
  return (
    d.getFullYear() === now.getFullYear() &&
    d.getMonth() === now.getMonth() &&
    d.getDate() === now.getDate()
  );
}

function isWithin24h(dateStr: string): boolean {
  const d = new Date(dateStr);
  const now = new Date();
  return now.getTime() - d.getTime() < 24 * 60 * 60 * 1000;
}

const RUN_STATUS_COLORS: Record<string, string> = {
  success: 'bg-emerald-400',
  running: 'bg-blue-400',
  failed: 'bg-red-400',
  timeout: 'bg-amber-400',
  error: 'bg-red-400',
  cancelled: 'bg-zinc-400',
  skipped: 'bg-zinc-500',
};

export default function DashboardPage() {
  const navigate = useNavigate();

  const jobs = useApi<JobListEntry[]>(() => jobsApi.list({}), []);
  const history = useApi<{ records: RunRecord[]; total: number }>(
    () => historyApi.list({ limit: 20 }),
    [],
  );
  const system = useApi<SystemStatus>(() => configApi.status(), []);

  useSSE((event: string, _data: string) => {
    if (event === 'job-changed') {
      jobs.refetch();
      history.refetch();
    }
    if (event === 'run-completed') {
      history.refetch();
      jobs.refetch();
    }
  });

  const stats = useMemo(() => {
    const jobList = jobs.data ?? [];
    const runs = history.data?.records ?? [];

    const activeCount = jobList.filter((j) => j.status === 'active').length;
    const pausedCount = jobList.filter((j) => j.status === 'paused').length;
    const completedToday = runs.filter(
      (r) => r.status === 'success' && r.started_at && isToday(r.started_at),
    ).length;

    const recentRuns24h = runs.filter((r) => r.started_at && isWithin24h(r.started_at));
    const failedRuns24h = recentRuns24h.filter((r) =>
      ['failed', 'timeout', 'error'].includes(r.status),
    );
    const failureRate =
      recentRuns24h.length > 0
        ? Math.round((failedRuns24h.length / recentRuns24h.length) * 100)
        : 0;

    const inFlight = jobList.filter((j) => j.in_flight);

    const upcoming = jobList
      .filter((j) => j.status === 'active' && j.next_run)
      .sort((a, b) => new Date(a.next_run!).getTime() - new Date(b.next_run!).getTime())
      .slice(0, 5);

    return { activeCount, pausedCount, completedToday, failureRate, failedRuns24h, recentRuns24h, inFlight, upcoming };
  }, [jobs.data, history.data]);

  const recentRuns = useMemo(() => {
    return (history.data?.records ?? []).slice(0, 10);
  }, [history.data]);

  const handleRunClick = useCallback(
    (runId: string) => {
      navigate(`/runs/${runId}`);
    },
    [navigate],
  );

  const loading = jobs.loading || history.loading || system.loading;

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="font-heading text-2xl font-semibold text-text-primary">Dashboard</h1>
          <p className="mt-1 text-sm text-text-secondary">Overview of your scheduled jobs</p>
        </div>
        <Link to="/jobs">
          <Button>
            View all jobs
            <ArrowRight className="ml-1.5" size={16} weight="bold" />
          </Button>
        </Link>
      </div>

      {/* Stat Cards */}
      <div className="grid grid-cols-1 gap-4 sm:grid-cols-3">
        <StatCard
          label="Active Jobs"
          value={stats.activeCount}
          loading={loading}
          icon={<Lightning size={16} weight="fill" className="text-emerald-400" />}
          dotColor="bg-emerald-400"
        />
        <StatCard
          label="Paused Jobs"
          value={stats.pausedCount}
          loading={loading}
          icon={<Pause size={16} weight="fill" className="text-amber-400" />}
          dotColor="bg-amber-400"
        />
        <StatCard
          label="Completed Today"
          value={stats.completedToday}
          loading={loading}
          icon={<CheckCircle size={16} weight="fill" className="text-blue-400" />}
          dotColor="bg-blue-400"
        />
      </div>

      {/* Main Grid */}
      <div className="grid grid-cols-1 gap-6 lg:grid-cols-3">
        {/* Left Column (2/3) */}
        <div className="space-y-6 lg:col-span-2">
          {/* Running Now */}
          {!loading && stats.inFlight.length > 0 && (
            <Panel title="Running Now" icon={<Spinner size={18} weight="bold" className="text-blue-400 animate-spin" />}>
              <div className="divide-y divide-border">
                {stats.inFlight.map((job) => (
                  <div
                    key={job.id}
                    className="flex items-center justify-between px-5 py-3"
                  >
                    <div className="flex items-center gap-3">
                      <span className="relative flex h-2.5 w-2.5">
                        <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-blue-400 opacity-75" />
                        <span className="relative inline-flex h-2.5 w-2.5 rounded-full bg-blue-400" />
                      </span>
                      <Link
                        to={`/jobs/${job.id}`}
                        className="font-mono text-sm text-text-primary hover:text-accent transition-colors"
                      >
                        {job.name}
                      </Link>
                    </div>
                    <span className="label-mono text-text-tertiary">
                      {job.last_run?.started_at ? formatDurationBetween(job.last_run.started_at, new Date().toISOString()) : '--'}
                    </span>
                  </div>
                ))}
              </div>
            </Panel>
          )}

          {/* Recent Runs */}
          <Panel title="Recent Runs" icon={<Clock size={18} weight="bold" className="text-text-secondary" />}>
            {loading ? (
              <div className="space-y-4 p-5">
                {Array.from({ length: 5 }).map((_, i) => (
                  <div key={i} className="flex items-start gap-4">
                    <Skeleton className="h-3 w-3 rounded-full mt-1.5" />
                    <div className="flex-1 space-y-2">
                      <Skeleton className="h-4 w-48" />
                      <Skeleton className="h-3 w-32" />
                    </div>
                  </div>
                ))}
              </div>
            ) : recentRuns.length === 0 ? (
              <div className="p-8">
                <EmptyState
                  icon={<Clock size={32} weight="thin" />}
                  title="No runs yet"
                  description="Runs will appear here once your jobs start executing."
                />
              </div>
            ) : (
              <div className="relative px-5 py-4">
                {/* Timeline connector line */}
                <div className="absolute left-[1.8rem] top-6 bottom-6 w-px bg-border" />

                <div className="space-y-0">
                  {recentRuns.map((run) => (
                    <button
                      key={run.run_id}
                      onClick={() => handleRunClick(run.run_id)}
                      className="group relative flex w-full items-start gap-4 rounded-lg px-1 py-3 text-left transition-colors hover:bg-bg-hover"
                    >
                      {/* Status dot */}
                      <div className="relative z-10 mt-1.5 flex-shrink-0">
                        <div
                          className={cn(
                            'h-3 w-3 rounded-full ring-4 ring-bg-panel',
                            RUN_STATUS_COLORS[run.status] ?? 'bg-zinc-500',
                          )}
                        />
                      </div>

                      {/* Content */}
                      <div className="flex-1 min-w-0">
                        <div className="flex items-center gap-2">
                          <span className="font-mono text-sm text-text-primary truncate">
                            {run.job_name ?? run.job_id}
                          </span>
                          <StatusBadge status={run.status} />
                        </div>
                        <div className="mt-1 flex items-center gap-3 text-xs text-text-tertiary">
                          <span>
                            {run.trigger === 'scheduled'
                              ? 'Triggered by schedule'
                              : run.trigger === 'manual'
                                ? 'Manual trigger'
                                : `Triggered by ${run.trigger ?? 'unknown'}`}
                          </span>
                          {run.started_at && run.finished_at && (
                            <>
                              <span className="text-border">&middot;</span>
                              <span>{formatDurationBetween(run.started_at, run.finished_at)}</span>
                            </>
                          )}
                          {run.started_at && (
                            <>
                              <span className="text-border">&middot;</span>
                              <TimeAgo date={run.started_at} />
                            </>
                          )}
                        </div>
                      </div>

                      {/* Arrow */}
                      <ArrowRight
                        size={14}
                        className="mt-2 flex-shrink-0 text-text-tertiary opacity-0 transition-opacity group-hover:opacity-100"
                      />
                    </button>
                  ))}
                </div>
              </div>
            )}
          </Panel>
        </div>

        {/* Right Column (1/3) */}
        <div className="space-y-6">
          {/* System Health */}
          <Panel title="System Health" icon={<Heartbeat size={18} weight="bold" className="text-text-secondary" />}>
            {loading ? (
              <div className="space-y-3 p-5">
                <Skeleton className="h-4 w-full" />
                <Skeleton className="h-4 w-3/4" />
                <Skeleton className="h-4 w-1/2" />
              </div>
            ) : system.error ? (
              <div className="flex items-center gap-2 p-5 text-sm text-red-400">
                <WarningCircle size={16} weight="fill" />
                <span>Unable to reach backend</span>
              </div>
            ) : system.data ? (
              <div className="divide-y divide-border">
                <HealthRow label="Backend" value={system.data.backend ?? 'unknown'} />
                <HealthRow
                  label="Last tick"
                  value={system.data.last_dispatch_at ? timeAgo(system.data.last_dispatch_at) : '--'}
                />
                <HealthRow label="Version" value={system.data.version ?? '--'} />
              </div>
            ) : null}
          </Panel>

          {/* Failure Rate */}
          <Panel title="Failure Rate (24h)" icon={<WarningCircle size={18} weight="bold" className="text-text-secondary" />}>
            {loading ? (
              <div className="space-y-3 p-5">
                <Skeleton className="h-10 w-20" />
                <Skeleton className="h-2 w-full rounded-full" />
              </div>
            ) : (
              <div className="p-5">
                <div className="flex items-baseline gap-2">
                  <span
                    className={cn(
                      'font-heading text-4xl font-semibold',
                      stats.failureRate === 0
                        ? 'text-emerald-400'
                        : stats.failureRate < 20
                          ? 'text-amber-400'
                          : 'text-red-400',
                    )}
                  >
                    {stats.failureRate}%
                  </span>
                  <span className="label-mono text-text-tertiary">
                    {stats.failedRuns24h.length}/{stats.recentRuns24h.length} runs
                  </span>
                </div>
                {/* Bar */}
                <div className="mt-3 h-2 w-full overflow-hidden rounded-full bg-bg-hover">
                  <div
                    className={cn(
                      'h-full rounded-full transition-all duration-500',
                      stats.failureRate === 0
                        ? 'bg-emerald-400'
                        : stats.failureRate < 20
                          ? 'bg-amber-400'
                          : 'bg-red-400',
                    )}
                    style={{ width: `${Math.max(stats.failureRate, 2)}%` }}
                  />
                </div>
              </div>
            )}
          </Panel>

          {/* Upcoming */}
          <Panel title="Upcoming" icon={<CalendarBlank size={18} weight="bold" className="text-text-secondary" />}>
            {loading ? (
              <div className="space-y-3 p-5">
                {Array.from({ length: 3 }).map((_, i) => (
                  <div key={i} className="space-y-1.5">
                    <Skeleton className="h-4 w-36" />
                    <Skeleton className="h-3 w-24" />
                  </div>
                ))}
              </div>
            ) : stats.upcoming.length === 0 ? (
              <div className="p-6">
                <EmptyState
                  icon={<CalendarBlank size={28} weight="thin" />}
                  title="Nothing scheduled"
                  description="No upcoming jobs to show."
                />
              </div>
            ) : (
              <div className="divide-y divide-border">
                {stats.upcoming.map((job) => (
                  <Link
                    key={job.id}
                    to={`/jobs/${job.id}`}
                    className="flex items-center justify-between px-5 py-3 transition-colors hover:bg-bg-hover"
                  >
                    <div className="min-w-0">
                      <p className="font-mono text-sm text-text-primary truncate">
                        {job.name}
                      </p>
                      <p className="label-mono mt-0.5 text-text-tertiary">
                        {job.schedule_input}
                      </p>
                    </div>
                    <div className="flex items-center gap-1.5 text-xs text-accent">
                      <Timer size={14} weight="bold" />
                      <span className="font-mono">
                        {job.next_run ? formatCountdown(job.next_run) : '--'}
                      </span>
                    </div>
                  </Link>
                ))}
              </div>
            )}
          </Panel>
        </div>
      </div>
    </div>
  );
}

/* ------------------------------------------------------------------ */
/*  Sub-components                                                     */
/* ------------------------------------------------------------------ */

function StatCard({
  label,
  value,
  loading,
  icon,
  dotColor,
}: {
  label: string;
  value: number;
  loading: boolean;
  icon: React.ReactNode;
  dotColor: string;
}) {
  return (
    <div className="relative overflow-hidden rounded-lg border border-border bg-bg-panel p-5">
      {/* Dot indicator */}
      <div className={cn('absolute right-4 top-4 h-2 w-2 rounded-full', dotColor)} />

      <div className="flex items-center gap-2 text-text-secondary">
        {icon}
        <span className="label-mono">{label}</span>
      </div>

      {loading ? (
        <Skeleton className="mt-3 h-12 w-20" />
      ) : (
        <p className="mt-2 font-heading text-5xl font-semibold text-text-primary">{value}</p>
      )}
    </div>
  );
}

function Panel({
  title,
  icon,
  children,
}: {
  title: string;
  icon: React.ReactNode;
  children: React.ReactNode;
}) {
  return (
    <div className="overflow-hidden rounded-lg border border-border bg-bg-panel">
      <div className="flex items-center gap-2 border-b border-border px-5 py-3">
        {icon}
        <h2 className="font-heading text-sm font-semibold text-text-primary">{title}</h2>
      </div>
      {children}
    </div>
  );
}

function HealthRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex items-center justify-between px-5 py-3">
      <span className="label-mono text-text-tertiary">{label}</span>
      <span className="font-mono text-sm text-text-primary">{value}</span>
    </div>
  );
}
